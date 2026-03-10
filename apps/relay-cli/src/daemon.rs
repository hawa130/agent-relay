use crate::DaemonCommand;
use relay_core::{
    BootstrapMode, DaemonService, EngineConnectionState, RelayApp, RelayError, RpcErrorResponse,
    RpcNotification, RpcRequest, RpcSuccessResponse,
};
use std::io::{self, BufRead, BufWriter, Write};
use std::thread;
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant, sleep};

pub(crate) async fn run(command: &DaemonCommand) -> Result<(), RelayError> {
    if !command.stdio {
        return Err(RelayError::InvalidInput(
            "daemon currently requires --stdio".into(),
        ));
    }

    let app = RelayApp::bootstrap_with_mode(BootstrapMode::ReadWrite).await?;
    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<String>();
    let (notification_tx, mut notification_rx) = mpsc::unbounded_channel::<RpcNotification>();
    spawn_stdin_reader(request_tx);

    let mut service = DaemonService::new(app, notification_tx);
    if let Err(error) = service.startup_tick().await {
        let _ = service
            .publish_health_update(EngineConnectionState::Degraded, Some(error.to_string()))
            .await;
    }

    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    let refresh_sleep = sleep(next_refresh_deadline(&service).await);
    tokio::pin!(refresh_sleep);

    loop {
        tokio::select! {
            maybe_line = request_rx.recv() => {
                let Some(line) = maybe_line else {
                    break;
                };
                let outbound = match serde_json::from_str::<RpcRequest>(&line) {
                    Ok(request) => match service.handle_request(request).await {
                        Ok(response) => serialize_response(&response)?,
                        Err(error) => serialize_error(&RpcErrorResponse {
                            jsonrpc: "2.0".into(),
                            id: None,
                            error,
                        })?,
                    },
                    Err(error) => serialize_error(&RpcErrorResponse {
                        jsonrpc: "2.0".into(),
                        id: None,
                        error: relay_core::RpcErrorObject {
                            code: -32600,
                            message: error.to_string(),
                            data: None,
                        },
                    })?,
                };
                write_line(&mut writer, &outbound)?;
                if service.shutdown_requested() {
                    break;
                }
                refresh_sleep
                    .as_mut()
                    .reset(Instant::now() + next_refresh_deadline(&service).await);
            }
            maybe_notification = notification_rx.recv() => {
                let Some(notification) = maybe_notification else {
                    continue;
                };
                let payload = serialize_notification(&notification)?;
                write_line(&mut writer, &payload)?;
            }
            _ = &mut refresh_sleep => {
                if let Err(error) = service.interval_tick().await {
                    let _ = service
                        .publish_health_update(EngineConnectionState::Degraded, Some(error.to_string()))
                        .await;
                }
                refresh_sleep
                    .as_mut()
                    .reset(Instant::now() + next_refresh_deadline(&service).await);
            }
        }
    }

    Ok(())
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
    Duration::from_secs(seconds.max(15) as u64)
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
