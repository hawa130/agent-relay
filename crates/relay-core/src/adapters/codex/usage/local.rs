use super::super::CodexAdapter;
use crate::internal::usage_policy::{
    apply_auto_switch_policy, build_usage_window, is_usage_stale, next_reset_at,
    unknown_usage_window,
};
use crate::models::{Profile, RelayError, UsageConfidence, UsageSnapshot, UsageSource};
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration as StdDuration, Instant};

const APP_SERVER_TIMEOUT_SECS: u64 = 6;

pub(crate) fn collect_local(
    adapter: &CodexAdapter,
    target_profile: Option<&Profile>,
    active_profile: Option<&Profile>,
) -> Result<Option<UsageSnapshot>, RelayError> {
    let Some(local_home) = resolve_local_home(adapter, target_profile, active_profile) else {
        return Ok(None);
    };
    if target_profile
        .zip(active_profile)
        .is_some_and(|(target, active)| target.id == active.id)
    {
        if let Some(snapshot) = collect_app_server_snapshot(adapter, target_profile, &local_home)? {
            return Ok(Some(snapshot));
        }
    }

    collect_session_snapshot(target_profile, &local_home)
}

fn resolve_local_home(
    adapter: &CodexAdapter,
    target_profile: Option<&Profile>,
    active_profile: Option<&Profile>,
) -> Option<PathBuf> {
    if target_profile.is_none() {
        return Some(adapter.live_home().to_path_buf());
    }

    if target_profile
        .zip(active_profile)
        .is_some_and(|(target, active)| target.id == active.id)
    {
        return Some(adapter.live_home().to_path_buf());
    }

    target_profile
        .and_then(|profile| profile.agent_home.as_ref())
        .map(PathBuf::from)
}

fn collect_app_server_snapshot(
    adapter: &CodexAdapter,
    profile: Option<&Profile>,
    live_home: &Path,
) -> Result<Option<UsageSnapshot>, RelayError> {
    let live_home = canonicalize_lossy(live_home);
    let expected_live_home = canonicalize_lossy(adapter.live_home());
    if live_home != expected_live_home {
        return Ok(None);
    }
    if !looks_like_real_codex_auth(&live_home.join("auth.json")) {
        return Ok(None);
    }

    let Some(binary) = crate::platform::find_binary("codex") else {
        return Ok(None);
    };

    let mut child = match Command::new(binary)
        .arg("app-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return Ok(None),
    };

    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Ok(None);
    };

    let (sender, receiver) = mpsc::channel::<Value>();
    let reader_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else {
                break;
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            if sender.send(value).is_err() {
                break;
            }
        }
    });

    let result = (|| -> Result<Option<UsageSnapshot>, RelayError> {
        let Some(mut stdin) = child.stdin.take() else {
            return Ok(None);
        };

        write_app_server_request(&mut stdin, initialize_request())?;
        write_app_server_request(&mut stdin, account_rate_limits_request())?;
        stdin
            .flush()
            .map_err(|error| RelayError::Io(error.to_string()))?;

        let deadline = Instant::now() + StdDuration::from_secs(APP_SERVER_TIMEOUT_SECS);
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let Ok(message) = receiver.recv_timeout(remaining) else {
                break;
            };

            if let Some(snapshot) = parse_app_server_message(message, profile)? {
                return Ok(Some(snapshot));
            }
        }

        Ok(None)
    })();

    let _ = child.kill();
    let _ = child.wait();
    let _ = reader_handle.join();
    result
}

fn collect_session_snapshot(
    profile: Option<&Profile>,
    home: &Path,
) -> Result<Option<UsageSnapshot>, RelayError> {
    let sessions_dir = home.join("sessions");
    if !sessions_dir.exists() {
        return Ok(None);
    }

    let mut newest: Option<LocalUsageReading> = None;
    for path in collect_jsonl_files(&sessions_dir)? {
        let candidate = parse_latest_usage_from_file(&path)?;
        if candidate
            .as_ref()
            .zip(newest.as_ref())
            .is_some_and(|(left, right)| left.last_refreshed_at <= right.last_refreshed_at)
        {
            continue;
        }
        if candidate.is_some() {
            newest = candidate;
        }
    }

    let Some(reading) = newest else {
        return Ok(None);
    };

    let stale = is_usage_stale(reading.last_refreshed_at);
    let session = build_usage_window(
        Some(reading.primary_used_percent),
        Some(reading.primary_window_minutes),
        Some(reading.primary_resets_at),
        true,
    );
    let weekly = build_usage_window(
        reading.secondary_used_percent,
        reading.secondary_window_minutes,
        reading.secondary_resets_at,
        reading.secondary_used_percent.is_some(),
    );

    let mut snapshot = UsageSnapshot {
        profile_id: profile.map(|value| value.id.clone()),
        profile_name: profile.map(|value| value.nickname.clone()),
        source: UsageSource::Local,
        confidence: UsageConfidence::High,
        stale,
        last_refreshed_at: reading.last_refreshed_at,
        next_reset_at: next_reset_at(&session, &weekly),
        session,
        weekly,
        auto_switch_reason: None,
        can_auto_switch: false,
        message: None,
        remote_error: None,
    };
    if stale {
        snapshot.message = Some("Usage may be outdated.".into());
    }
    apply_auto_switch_policy(&mut snapshot);
    Ok(Some(snapshot))
}

fn snapshot_from_rate_limit_snapshot(
    snapshot: AppServerRateLimitSnapshot,
    profile: Option<&Profile>,
    message: Option<&str>,
) -> UsageSnapshot {
    let session = app_server_window(snapshot.primary);
    let weekly = app_server_window(snapshot.secondary);
    let last_refreshed_at = Utc::now();
    let mut snapshot = UsageSnapshot {
        profile_id: profile.map(|value| value.id.clone()),
        profile_name: profile.map(|value| value.nickname.clone()),
        source: UsageSource::Local,
        confidence: UsageConfidence::High,
        stale: false,
        last_refreshed_at,
        next_reset_at: next_reset_at(&session, &weekly),
        session,
        weekly,
        auto_switch_reason: None,
        can_auto_switch: false,
        message: message.map(str::to_string),
        remote_error: None,
    };
    apply_auto_switch_policy(&mut snapshot);
    snapshot
}

fn app_server_window(window: Option<AppServerRateLimitWindow>) -> crate::models::UsageWindow {
    let Some(window) = window else {
        return unknown_usage_window(None);
    };

    build_usage_window(
        window.used_percent,
        window.window_minutes,
        window.resets_at.and_then(timestamp_to_datetime),
        true,
    )
}

fn parse_latest_usage_from_file(path: &Path) -> Result<Option<LocalUsageReading>, RelayError> {
    let contents = fs::read_to_string(path)?;
    let mut newest: Option<LocalUsageReading> = None;

    for line in contents.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let payload = value
            .pointer("/payload/type")
            .and_then(Value::as_str)
            .filter(|kind| *kind == "token_count")
            .and_then(|_| value.pointer("/payload/info/rate_limits"))
            .cloned()
            .or_else(|| value.pointer("/payload/rate_limits").cloned());

        let Some(rate_limits) = payload else {
            continue;
        };

        let timestamp = value
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(parse_timestamp)
            .unwrap_or_else(Utc::now);
        let primary = rate_limits.get("primary");
        let secondary = rate_limits.get("secondary");
        let reading = LocalUsageReading {
            last_refreshed_at: timestamp,
            primary_used_percent: primary
                .and_then(|item| item.get("used_percent"))
                .and_then(Value::as_f64)
                .unwrap_or_default(),
            primary_window_minutes: primary
                .and_then(|item| item.get("window_minutes"))
                .and_then(Value::as_i64)
                .unwrap_or(300),
            primary_resets_at: primary
                .and_then(|item| item.get("resets_at"))
                .and_then(Value::as_i64)
                .and_then(timestamp_to_datetime)
                .unwrap_or_else(Utc::now),
            secondary_used_percent: secondary
                .and_then(|item| item.get("used_percent"))
                .and_then(Value::as_f64),
            secondary_window_minutes: secondary
                .and_then(|item| item.get("window_minutes"))
                .and_then(Value::as_i64),
            secondary_resets_at: secondary
                .and_then(|item| item.get("resets_at"))
                .and_then(Value::as_i64)
                .and_then(timestamp_to_datetime),
        };

        if newest
            .as_ref()
            .is_none_or(|current| reading.last_refreshed_at > current.last_refreshed_at)
        {
            newest = Some(reading);
        }
    }

    Ok(newest)
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

fn collect_jsonl_files(root: &Path) -> Result<Vec<PathBuf>, RelayError> {
    let mut results = Vec::new();
    collect_jsonl_files_recursive(root, &mut results)?;
    Ok(results)
}

fn collect_jsonl_files_recursive(
    root: &Path,
    results: &mut Vec<PathBuf>,
) -> Result<(), RelayError> {
    if !root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files_recursive(&path, results)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
            results.push(path);
        }
    }

    Ok(())
}

fn timestamp_to_datetime(timestamp: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_opt(timestamp, 0).single()
}

fn looks_like_real_codex_auth(path: &Path) -> bool {
    fs::read_to_string(path)
        .ok()
        .and_then(|value| serde_json::from_str::<Value>(&value).ok())
        .is_some_and(|json| {
            json.get("token")
                .and_then(Value::as_str)
                .is_some_and(|token| !token.trim().is_empty())
        })
}

fn canonicalize_lossy(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn write_app_server_request(stdin: &mut impl Write, value: Value) -> Result<(), RelayError> {
    serde_json::to_writer(&mut *stdin, &value)
        .map_err(|error| RelayError::Internal(error.to_string()))?;
    stdin
        .write_all(b"\n")
        .map_err(|error| RelayError::Io(error.to_string()))?;
    Ok(())
}

fn initialize_request() -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "clientInfo": {
                "name": "relay",
                "title": "Relay",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": {
                "experimentalApi": true
            }
        }
    })
}

fn account_rate_limits_request() -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "account/rateLimits/read",
        "params": {}
    })
}

fn parse_app_server_message(
    value: Value,
    profile: Option<&Profile>,
) -> Result<Option<UsageSnapshot>, RelayError> {
    let Some(id) = value.get("id").and_then(Value::as_i64) else {
        return Ok(None);
    };

    if id != 2 || value.get("error").is_some() {
        return Ok(None);
    }

    let Some(result) = value.get("result") else {
        return Ok(None);
    };
    let response: AppServerRateLimitsResponse = serde_json::from_value(result.clone())
        .map_err(|error| RelayError::Internal(error.to_string()))?;
    let rate_limits = response
        .rate_limits_by_limit_id
        .as_ref()
        .and_then(|limits| limits.get("codex"))
        .cloned()
        .unwrap_or(response.rate_limits);

    Ok(Some(snapshot_from_rate_limit_snapshot(
        rate_limits,
        profile,
        None,
    )))
}

#[derive(Debug, Clone)]
struct LocalUsageReading {
    last_refreshed_at: DateTime<Utc>,
    primary_used_percent: f64,
    primary_window_minutes: i64,
    primary_resets_at: DateTime<Utc>,
    secondary_used_percent: Option<f64>,
    secondary_window_minutes: Option<i64>,
    secondary_resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct AppServerRateLimitsResponse {
    #[serde(default)]
    rate_limits: AppServerRateLimitSnapshot,
    #[serde(default)]
    rate_limits_by_limit_id: Option<HashMap<String, AppServerRateLimitSnapshot>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct AppServerRateLimitSnapshot {
    #[serde(default)]
    primary: Option<AppServerRateLimitWindow>,
    #[serde(default)]
    secondary: Option<AppServerRateLimitWindow>,
}

#[derive(Debug, Clone, Deserialize)]
struct AppServerRateLimitWindow {
    #[serde(default)]
    used_percent: Option<f64>,
    #[serde(default)]
    window_minutes: Option<i64>,
    #[serde(default)]
    resets_at: Option<i64>,
}
