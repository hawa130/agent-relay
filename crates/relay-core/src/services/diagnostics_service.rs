use crate::models::{DiagnosticsExport, RelayError, UsageSnapshot};
use crate::platform::RelayPaths;
use crate::store::{FileLogStore, SqliteStore};
use crate::{ActiveState, DoctorReport, StatusReport};
use chrono::Utc;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn export_bundle(
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
    fs::create_dir_all(&bundle_dir)?;

    write_json(&bundle_dir.join("doctor.json"), doctor)?;
    write_json(&bundle_dir.join("status.json"), status)?;
    write_json(&bundle_dir.join("active_state.json"), active_state)?;
    write_json(&bundle_dir.join("usage.json"), usage)?;
    write_json(&bundle_dir.join("profiles.json"), &store.list_profiles()?)?;
    write_json(
        &bundle_dir.join("events.json"),
        &store.list_failure_events(100)?,
    )?;
    write_json(
        &bundle_dir.join("switch_history.json"),
        &store.list_switch_history(100)?,
    )?;
    write_json(
        &bundle_dir.join("build.json"),
        &serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "relay_home": paths.relay_home,
            "codex_home_env": std::env::var("CODEX_HOME").ok(),
        }),
    )?;

    if log_store.path().exists() {
        fs::copy(log_store.path(), bundle_dir.join("relay.log"))?;
    }

    let output = Command::new("zip")
        .args(["-qr", archive_path.to_string_lossy().as_ref(), "."])
        .current_dir(&bundle_dir)
        .output()
        .map_err(|error| RelayError::ExternalCommand(error.to_string()))?;

    if !output.status.success() {
        return Err(RelayError::ExternalCommand(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

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
