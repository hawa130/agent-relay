use crate::DaemonCommand;
use relay_core::{
    BootstrapMode, DaemonService, EngineConnectionState, RelayApp, RelayError, RpcErrorObject,
    RpcErrorResponse, RpcNotification, RpcRequest, RpcSuccessResponse,
};
use std::io::{self, BufRead, BufWriter, Write};
use std::sync::Arc;
use std::thread;
use tokio::sync::mpsc;
use tokio::task::{JoinSet, LocalSet};
use tokio::time::{Duration, Instant, sleep_until};

#[derive(Clone, Copy)]
enum RefreshKind {
    Startup,
    Interval,
}

const DISABLED_REFRESH_DEADLINE: Duration = Duration::from_secs(60 * 60 * 24 * 365 * 100);

pub(crate) async fn run(command: &DaemonCommand) -> Result<(), RelayError> {
    if !command.stdio {
        return Err(RelayError::InvalidInput(
            "daemon currently requires --stdio".into(),
        ));
    }

    let local = LocalSet::new();
    local.run_until(run_local()).await
}

async fn run_local() -> Result<(), RelayError> {
    let write_app = RelayApp::bootstrap_with_mode(BootstrapMode::ReadWrite).await?;
    let read_app = Arc::new(RelayApp::bootstrap_with_mode(BootstrapMode::ReadOnly).await?);

    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<String>();
    let (notification_tx, notification_rx) = mpsc::unbounded_channel::<RpcNotification>();
    let (write_tx, write_rx) = mpsc::unbounded_channel::<RpcRequest>();
    let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<String>();
    spawn_stdin_reader(request_tx);

    let write_task = tokio::task::spawn_local(run_write_service(
        DaemonService::new(write_app, notification_tx),
        write_rx,
        outbound_tx.clone(),
    ));
    tokio::pin!(write_task);
    let notification_task = tokio::task::spawn_local(run_notification_forwarder(
        notification_rx,
        outbound_tx.clone(),
    ));
    tokio::pin!(notification_task);

    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    let mut read_tasks = JoinSet::<String>::new();
    let mut write_task_done = false;
    let mut notification_task_done = false;
    let mut stop_accepting_requests = false;

    loop {
        if write_task_done && notification_task_done && read_tasks.is_empty() && outbound_rx.is_empty() {
            break;
        }

        tokio::select! {
            maybe_line = request_rx.recv(), if !stop_accepting_requests => {
                let Some(line) = maybe_line else {
                    stop_accepting_requests = true;
                    continue;
                };

                match serde_json::from_str::<RpcRequest>(&line) {
                    Ok(request) => {
                        if DaemonService::is_read_method(&request.method) {
                            let read_app = Arc::clone(&read_app);
                            read_tasks.spawn_local(
                                async move { render_read_response(read_app, request).await }
                            );
                        } else if write_tx.send(request).is_err() {
                            stop_accepting_requests = true;
                        }
                    }
                    Err(error) => {
                        let payload = serialize_error(&RpcErrorResponse {
                            jsonrpc: "2.0".into(),
                            id: None,
                            error: RpcErrorObject {
                                code: -32600,
                                message: error.to_string(),
                                data: None,
                            },
                        })?;
                        write_line(&mut writer, &payload)?;
                    }
                }
            }
            maybe_outbound = outbound_rx.recv() => {
                let Some(payload) = maybe_outbound else {
                    continue;
                };
                write_line(&mut writer, &payload)?;
            }
            joined = read_tasks.join_next(), if !read_tasks.is_empty() => {
                if let Some(result) = joined {
                    if let Ok(payload) = result {
                        write_line(&mut writer, &payload)?;
                    }
                }
            }
            result = &mut write_task, if !write_task_done => {
                write_task_done = true;
                stop_accepting_requests = true;
                if let Err(error) = result {
                    return Err(RelayError::Internal(format!("daemon write worker failed: {error}")));
                }
            }
            result = &mut notification_task, if !notification_task_done => {
                notification_task_done = true;
                if let Err(error) = result {
                    return Err(RelayError::Internal(format!("daemon notification worker failed: {error}")));
                }
            }
        }
    }

    Ok(())
}

async fn run_write_service(
    service: DaemonService,
    mut write_rx: mpsc::UnboundedReceiver<RpcRequest>,
    outbound_tx: mpsc::UnboundedSender<String>,
) {
    let mut startup_refresh_pending = true;
    let mut refresh_in_flight = false;
    let mut next_refresh_at = schedule_next_refresh_at(&service).await;
    let mut background_tasks = JoinSet::<WriteServiceEvent>::new();
    let mut service = service;

    loop {
        if service.shutdown_requested() && background_tasks.is_empty() {
            break;
        }

        tokio::select! {
            biased;
            joined = background_tasks.join_next(), if !background_tasks.is_empty() => {
                if let Some(result) = joined {
                    match result {
                        Ok(WriteServiceEvent::RefreshFinished) => {
                            refresh_in_flight = false;
                            next_refresh_at = schedule_next_refresh_at(&service).await;
                        }
                        Err(error) => {
                            panic!("daemon background task failed: {error}");
                        }
                    }
                }
            }
            maybe_request = write_rx.recv() => {
                let Some(request) = maybe_request else {
                    if background_tasks.is_empty() {
                        break;
                    }
                    continue;
                };
                let method = request.method.clone();
                let (payload, reset_refresh_deadline) =
                    render_write_response(&mut service, request).await;
                if outbound_tx.send(payload).is_err() {
                    break;
                }
                if reset_refresh_deadline {
                    next_refresh_at = schedule_next_refresh_at(&service).await;
                }

                if startup_refresh_pending && method == "session/subscribe" {
                    startup_refresh_pending = false;
                    if automatic_refresh_enabled(&service).await {
                        refresh_in_flight = true;
                        let background_service = service.clone();
                        background_tasks.spawn_local(async move {
                            run_refresh(RefreshKind::Startup, background_service).await;
                            WriteServiceEvent::RefreshFinished
                        });
                    }
                }

                if service.shutdown_requested() {
                    if background_tasks.is_empty() {
                        break;
                    }
                }
            }
            _ = sleep_until(next_refresh_at), if !refresh_in_flight && !service.shutdown_requested() => {
                refresh_in_flight = true;
                let background_service = service.clone();
                background_tasks.spawn_local(async move {
                    run_refresh(RefreshKind::Interval, background_service).await;
                    WriteServiceEvent::RefreshFinished
                });
            }
        }
    }
}

async fn run_notification_forwarder(
    mut notification_rx: mpsc::UnboundedReceiver<RpcNotification>,
    outbound_tx: mpsc::UnboundedSender<String>,
) {
    while let Some(notification) = notification_rx.recv().await {
        let payload = serialize_notification(&notification)
            .unwrap_or_else(|error| serialize_internal_error(None, error.to_string()));
        if outbound_tx.send(payload).is_err() {
            break;
        }
    }
}

async fn render_read_response(read_app: Arc<RelayApp>, request: RpcRequest) -> String {
    match DaemonService::handle_read_request(read_app.as_ref(), request).await {
        Ok(response) => serialize_response(&response)
            .unwrap_or_else(|error| serialize_internal_error(None, error.to_string())),
        Err(error) => serialize_error(&RpcErrorResponse {
            jsonrpc: "2.0".into(),
            id: None,
            error,
        })
        .unwrap_or_else(|err| serialize_internal_error(None, err.to_string())),
    }
}

async fn render_write_response(
    service: &mut DaemonService,
    request: RpcRequest,
) -> (String, bool) {
    let should_reset_refresh_deadline = request.method == "relay/settings/update";
    match service.handle_request(request).await {
        Ok(response) => (
            serialize_response(&response)
                .unwrap_or_else(|error| serialize_internal_error(None, error.to_string())),
            should_reset_refresh_deadline,
        ),
        Err(error) => (
            serialize_error(&RpcErrorResponse {
                jsonrpc: "2.0".into(),
                id: None,
                error,
            })
            .unwrap_or_else(|err| serialize_internal_error(None, err.to_string())),
            false,
        ),
    }
}

enum WriteServiceEvent {
    RefreshFinished,
}

async fn run_refresh(kind: RefreshKind, mut service: DaemonService) {
    let result = match kind {
        RefreshKind::Startup => service.startup_tick().await,
        RefreshKind::Interval => service.interval_tick().await,
    };

    if let Err(error) = result {
        let _ = service
            .publish_health_update(EngineConnectionState::Degraded, Some(error.to_string()))
            .await;
    }
}

fn spawn_stdin_reader(sender: mpsc::UnboundedSender<String>) {
    thread::spawn(move || {
        let stdin = io::stdin();
        let reader = stdin.lock();
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if sender.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
}

async fn next_refresh_deadline(service: &DaemonService) -> Duration {
    let seconds = service.current_refresh_interval().await.unwrap_or(60);
    if seconds <= 0 {
        return DISABLED_REFRESH_DEADLINE;
    }
    Duration::from_secs(seconds.max(15) as u64)
}

async fn schedule_next_refresh_at(service: &DaemonService) -> Instant {
    Instant::now() + next_refresh_deadline(service).await
}

async fn automatic_refresh_enabled(service: &DaemonService) -> bool {
    service.current_refresh_interval().await.unwrap_or(60) > 0
}

fn write_line(writer: &mut BufWriter<io::StdoutLock<'_>>, line: &str) -> Result<(), RelayError> {
    writer.write_all(line.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn serialize_response(response: &RpcSuccessResponse) -> Result<String, RelayError> {
    serde_json::to_string(response).map_err(|error| RelayError::Internal(error.to_string()))
}

fn serialize_error(response: &RpcErrorResponse) -> Result<String, RelayError> {
    serde_json::to_string(response).map_err(|error| RelayError::Internal(error.to_string()))
}

fn serialize_notification(notification: &RpcNotification) -> Result<String, RelayError> {
    serde_json::to_string(notification).map_err(|error| RelayError::Internal(error.to_string()))
}

fn serialize_internal_error(id: Option<String>, message: String) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": -32603,
            "message": message,
            "data": null
        }
    })
    .to_string()
}
