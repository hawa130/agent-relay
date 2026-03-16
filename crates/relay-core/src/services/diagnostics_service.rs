use crate::models::{DiagnosticsExport, RelayError, UsageSnapshot};
use crate::platform::RelayPaths;
use crate::store::{FileLogStore, SqliteStore};
use crate::{ActiveState, DoctorReport, StatusReport};
use chrono::Utc;
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::Path;

pub async fn export_bundle(
    paths: &RelayPaths,
    store: &SqliteStore,
    log_store: &FileLogStore,
    doctor: &DoctorReport,
    status: &StatusReport,
    active_state: &ActiveState,
    usage: &UsageSnapshot,
) -> Result<DiagnosticsExport, RelayError> {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let bundle_dir = paths.exports_dir.join(format!("diagnostics_{timestamp}"));
    let archive_path = paths
        .exports_dir
        .join(format!("diagnostics_{timestamp}.zip"));

    // Collect async data before spawning blocking task
    let profiles_json = store.list_profiles().await?;
    let events_json = store.list_failure_events(100).await?;
    let switch_history_json = store.list_switch_history(100).await?;

    let doctor = doctor.clone();
    let status = status.clone();
    let active_state = active_state.clone();
    let usage = usage.clone();
    let log_path = log_store.path().to_path_buf();
    let bundle_dir_clone = bundle_dir.clone();
    let archive_path_clone = archive_path.clone();

    tokio::task::spawn_blocking(move || {
        fs::create_dir_all(&bundle_dir_clone)?;

        write_json(&bundle_dir_clone.join("doctor.json"), &doctor)?;
        write_json(&bundle_dir_clone.join("status.json"), &status)?;
        write_json(&bundle_dir_clone.join("active_state.json"), &active_state)?;
        write_json(&bundle_dir_clone.join("usage.json"), &usage)?;
        write_json(&bundle_dir_clone.join("profiles.json"), &profiles_json)?;
        write_json(&bundle_dir_clone.join("events.json"), &events_json)?;
        write_json(
            &bundle_dir_clone.join("switch_history.json"),
            &switch_history_json,
        )?;
        write_json(
            &bundle_dir_clone.join("build.json"),
            &serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "relay_home": doctor.relay_home,
                "agent_home_env": {
                    "name": doctor.agent_home_env_name,
                    "value": doctor.agent_home_env_value,
                },
            }),
        )?;

        if log_path.exists() {
            fs::copy(&log_path, bundle_dir_clone.join("relay.log"))?;
        }

        let archive_file = fs::File::create(&archive_path_clone)
            .map_err(|error| RelayError::Io(error.to_string()))?;
        let mut zip_writer = zip::ZipWriter::new(archive_file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for entry in
            fs::read_dir(&bundle_dir_clone).map_err(|error| RelayError::Io(error.to_string()))?
        {
            let entry = entry.map_err(|error| RelayError::Io(error.to_string()))?;
            let path = entry.path();
            if path.is_file() {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                zip_writer
                    .start_file(&name, options)
                    .map_err(|error| RelayError::Internal(error.to_string()))?;
                let contents =
                    fs::read(&path).map_err(|error| RelayError::Io(error.to_string()))?;
                zip_writer
                    .write_all(&contents)
                    .map_err(|error| RelayError::Io(error.to_string()))?;
            }
        }
        zip_writer
            .finish()
            .map_err(|error| RelayError::Internal(error.to_string()))?;

        Ok::<(), RelayError>(())
    })
    .await
    .map_err(|e| RelayError::Internal(format!("blocking task failed: {e}")))??;

    Ok(DiagnosticsExport {
        archive_path: archive_path.to_string_lossy().into_owned(),
        bundle_dir: bundle_dir.to_string_lossy().into_owned(),
        created_at: Utc::now(),
    })
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), RelayError> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|error| RelayError::Internal(error.to_string()))?;
    fs::write(path, body)?;
    Ok(())
}
