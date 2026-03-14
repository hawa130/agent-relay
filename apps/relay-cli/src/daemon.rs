use crate::DaemonCommand;
use relay_core::{
    BootstrapMode, DaemonService, EngineConnectionState, RelayApp, RelayError, RpcErrorObject,
    RpcErrorResponse, RpcNotification, RpcRequest, RpcSuccessResponse,
};
use std::io::{self, BufRead, BufWriter, Write};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use tokio::sync::Notify;
use tokio::sync::mpsc;
use tokio::task::{JoinSet, LocalSet};
use tokio::time::{Duration, Instant, sleep_until};

#[derive(Clone, Copy)]
enum RefreshKind {
    Startup,
    Interval,
}

const DISABLED_REFRESH_DEADLINE: Duration = Duration::from_secs(60 * 60 * 24 * 365 * 100);

#[derive(Default)]
struct RuntimeSignals {
    startup_refresh_armed: AtomicBool,
    shutdown_requested: AtomicBool,
    startup_refresh_notify: Notify,
    interval_reset_notify: Notify,
    shutdown_notify: Notify,
}

impl RuntimeSignals {
    fn arm_startup_refresh(&self) {
        if !self.startup_refresh_armed.swap(true, Ordering::SeqCst) {
            self.startup_refresh_notify.notify_waiters();
        }
    }

    fn reset_interval_schedule(&self) {
        self.interval_reset_notify.notify_waiters();
    }

    fn request_shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::SeqCst);
        self.shutdown_notify.notify_waiters();
        self.startup_refresh_notify.notify_waiters();
        self.interval_reset_notify.notify_waiters();
    }

    fn shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::SeqCst)
    }
}

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
    let read_app = Rc::new(RelayApp::bootstrap_with_mode(BootstrapMode::ReadOnly).await?);

    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<String>();
    let (notification_tx, notification_rx) = mpsc::unbounded_channel::<RpcNotification>();
    let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<String>();
    spawn_stdin_reader(request_tx);

    let service = DaemonService::new(write_app, notification_tx);
    service.sync_network_query_concurrency().await?;
    let runtime_signals = Arc::new(RuntimeSignals::default());
    let scheduler_task = tokio::task::spawn_local(run_refresh_scheduler(
        service.clone(),
        Arc::clone(&runtime_signals),
    ));
    tokio::pin!(scheduler_task);
    let notification_task = tokio::task::spawn_local(run_notification_forwarder(
        notification_rx,
        outbound_tx.clone(),
    ));
    tokio::pin!(notification_task);

    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    let mut request_tasks = JoinSet::<()>::new();
    let mut scheduler_task_done = false;
    let mut notification_task_done = false;
    let mut stop_accepting_requests = false;

    loop {
        if scheduler_task_done && request_tasks.is_empty() && outbound_rx.is_empty() {
            break;
        }

        tokio::select! {
            maybe_line = request_rx.recv(), if !stop_accepting_requests => {
                let Some(line) = maybe_line else {
                    stop_accepting_requests = true;
                    runtime_signals.request_shutdown();
                    continue;
                };

                match serde_json::from_str::<RpcRequest>(&line) {
                    Ok(request) => {
                        if DaemonService::is_read_method(&request.method) {
                            let read_app = Rc::clone(&read_app);
                            let outbound_tx = outbound_tx.clone();
                            request_tasks.spawn_local(async move {
                                let payload = render_read_response(read_app, request).await;
                                let _ = outbound_tx.send(payload);
                            });
                        } else {
                            let service = service.clone();
                            let outbound_tx = outbound_tx.clone();
                            let runtime_signals = Arc::clone(&runtime_signals);
                            request_tasks.spawn_local(async move {
                                handle_write_request_task(
                                    service,
                                    outbound_tx,
                                    runtime_signals,
                                    request,
                                )
                                .await;
                            });
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
                        if outbound_tx.send(payload).is_err() {
                            stop_accepting_requests = true;
                        }
                    }
                }
            }
            maybe_outbound = outbound_rx.recv() => {
                let Some(payload) = maybe_outbound else {
                    continue;
                };
                write_line(&mut writer, &payload)?;
            }
            joined = request_tasks.join_next(), if !request_tasks.is_empty() => {
                if let Some(Err(error)) = joined {
                    return Err(RelayError::Internal(format!("daemon request worker failed: {error}")));
                }
            }
            result = &mut scheduler_task, if !scheduler_task_done => {
                scheduler_task_done = true;
                stop_accepting_requests = true;
                if let Err(error) = result {
                    return Err(RelayError::Internal(format!("daemon scheduler worker failed: {error}")));
                }
            }
            result = &mut notification_task, if !notification_task_done => {
                notification_task_done = true;
                if let Err(error) = result {
                    return Err(RelayError::Internal(format!("daemon notification worker failed: {error}")));
                }
            }
        }

        if service.shutdown_requested() {
            stop_accepting_requests = true;
        }
    }

    Ok(())
}

async fn run_refresh_scheduler(service: DaemonService, runtime_signals: Arc<RuntimeSignals>) {
    if service.shutdown_requested() || runtime_signals.shutdown_requested() {
        return;
    }

    while !runtime_signals.startup_refresh_armed.load(Ordering::SeqCst) {
        if runtime_signals.shutdown_requested() {
            return;
        }
        tokio::select! {
            _ = runtime_signals.startup_refresh_notify.notified() => {}
            _ = runtime_signals.shutdown_notify.notified() => return,
        }
        if service.shutdown_requested() || runtime_signals.shutdown_requested() {
            return;
        }
    }

    if automatic_refresh_enabled(&service).await {
        run_refresh(RefreshKind::Startup, service.clone()).await;
    }

    loop {
        if service.shutdown_requested() || runtime_signals.shutdown_requested() {
            break;
        }

        let next_refresh_at = schedule_next_refresh_at(&service).await;
        tokio::select! {
            _ = sleep_until(next_refresh_at) => {
                if service.shutdown_requested() || runtime_signals.shutdown_requested() {
                    break;
                }
                if automatic_refresh_enabled(&service).await {
                    run_refresh(RefreshKind::Interval, service.clone()).await;
                }
            }
            _ = runtime_signals.interval_reset_notify.notified() => {}
            _ = runtime_signals.shutdown_notify.notified() => break,
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

async fn render_read_response(read_app: Rc<RelayApp>, request: RpcRequest) -> String {
    let request_id = request.id.clone();
    match DaemonService::handle_read_request(read_app.as_ref(), request).await {
        Ok(response) => serialize_response(&response)
            .unwrap_or_else(|error| serialize_internal_error(None, error.to_string())),
        Err(error) => serialize_error(&RpcErrorResponse {
            jsonrpc: "2.0".into(),
            id: Some(request_id),
            error,
        })
        .unwrap_or_else(|err| serialize_internal_error(None, err.to_string())),
    }
}

async fn handle_write_request_task(
    service: DaemonService,
    outbound_tx: mpsc::UnboundedSender<String>,
    runtime_signals: Arc<RuntimeSignals>,
    request: RpcRequest,
) {
    let method = request.method.clone();
    let request_id = request.id.clone();
    let payload = match service.handle_request(request).await {
        Ok(response) => {
            if method == "session/subscribe" {
                runtime_signals.arm_startup_refresh();
            } else if method == "relay/settings/update" {
                runtime_signals.reset_interval_schedule();
            } else if method == "shutdown" {
                runtime_signals.request_shutdown();
            }
            serialize_response(&response)
                .unwrap_or_else(|error| serialize_internal_error(None, error.to_string()))
        }
        Err(error) => serialize_error(&RpcErrorResponse {
            jsonrpc: "2.0".into(),
            id: Some(request_id),
            error,
        })
        .unwrap_or_else(|err| serialize_internal_error(None, err.to_string())),
    };

    let _ = outbound_tx.send(payload);
}

async fn run_refresh(kind: RefreshKind, service: DaemonService) {
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
