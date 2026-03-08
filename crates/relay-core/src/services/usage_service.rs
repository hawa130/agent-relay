use crate::models::{
    FailureEvent, FailureReason, Profile, RelayError, UsageConfidence, UsageSnapshot, UsageSource,
    UsageStatus, UsageWindow,
};
use crate::platform::{find_binary, live_codex_home};
use crate::store::{FileUsageStore, SqliteStore};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration as StdDuration, Instant};

const STALE_AFTER_MINUTES: i64 = 15;
const APP_SERVER_TIMEOUT_SECS: u64 = 6;

pub fn build(
    store: &SqliteStore,
    usage_store: &FileUsageStore,
    active_profile: Option<&Profile>,
    live_home: &Path,
) -> Result<UsageSnapshot, RelayError> {
    if let Some(snapshot) = collect_app_server_snapshot(active_profile, live_home)? {
        usage_store.save(&snapshot)?;
        return Ok(snapshot);
    }

    if let Some(snapshot) = collect_local_snapshot(active_profile, live_home)? {
        usage_store.save(&snapshot)?;
        return Ok(snapshot);
    }

    if let Some(snapshot) = collect_fallback_snapshot(store, active_profile)? {
        usage_store.save(&snapshot)?;
        return Ok(snapshot);
    }

    if let Some(mut snapshot) = usage_store.load()? {
        apply_profile_context(&mut snapshot, active_profile);
        snapshot.stale = true;
        snapshot.can_auto_switch = false;
        snapshot.auto_switch_reason = None;
        snapshot.message = Some("using cached usage snapshot".into());
        return Ok(snapshot);
    }

    Ok(empty_snapshot(
        active_profile,
        UsageSource::Fallback,
        true,
        Some("usage unavailable from local sources".into()),
    ))
}

fn collect_app_server_snapshot(
    active_profile: Option<&Profile>,
    live_home: &Path,
) -> Result<Option<UsageSnapshot>, RelayError> {
    let Some(expected_live_home) = live_codex_home() else {
        return Ok(None);
    };
    let live_home = canonicalize_lossy(live_home);
    let expected_live_home = canonicalize_lossy(&expected_live_home);
    if live_home != expected_live_home {
        return Ok(None);
    }
    if !looks_like_real_codex_auth(&live_home.join("auth.json")) {
        return Ok(None);
    }

    let Some(binary) = find_binary("codex") else {
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

            if let Some(snapshot) = parse_app_server_message(message, active_profile)? {
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
    active_profile: Option<&Profile>,
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
        active_profile,
        "codex app-server rate limit RPC",
    )))
}

fn collect_local_snapshot(
    active_profile: Option<&Profile>,
    live_home: &Path,
) -> Result<Option<UsageSnapshot>, RelayError> {
    let sessions_dir = live_home.join("sessions");
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

    let stale = Utc::now() - reading.last_refreshed_at > Duration::minutes(STALE_AFTER_MINUTES);
    let session = build_window(
        Some(reading.primary_used_percent),
        Some(reading.primary_window_minutes),
        Some(reading.primary_resets_at),
        true,
    );
    let weekly = build_window(
        reading.secondary_used_percent,
        reading.secondary_window_minutes,
        reading.secondary_resets_at,
        reading.secondary_used_percent.is_some(),
    );

    let mut snapshot = UsageSnapshot {
        profile_id: None,
        profile_name: None,
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
    };
    apply_profile_context(&mut snapshot, active_profile);
    apply_auto_switch_policy(&mut snapshot);
    if snapshot.stale {
        snapshot.message = Some("local usage data is stale".into());
    } else {
        snapshot.message = Some("local session log fallback".into());
    }
    Ok(Some(snapshot))
}

fn collect_fallback_snapshot(
    store: &SqliteStore,
    active_profile: Option<&Profile>,
) -> Result<Option<UsageSnapshot>, RelayError> {
    let active_profile_id = active_profile.map(|profile| profile.id.as_str());
    let events = store.list_failure_events(50)?;
    let event = select_relevant_failure_event(&events, active_profile_id);
    let Some(event) = event else {
        return Ok(None);
    };

    let mut snapshot = empty_snapshot(
        active_profile,
        UsageSource::Fallback,
        false,
        Some(event.message.clone()),
    );
    snapshot.last_refreshed_at = event.created_at;
    snapshot.confidence = UsageConfidence::Medium;
    snapshot.auto_switch_reason = fallback_auto_switch_reason(&event.reason);
    snapshot.can_auto_switch = snapshot.auto_switch_reason.is_some();

    match event.reason {
        FailureReason::SessionExhausted => snapshot.session.status = UsageStatus::Exhausted,
        FailureReason::WeeklyExhausted => snapshot.weekly.status = UsageStatus::Exhausted,
        FailureReason::QuotaExhausted => {
            snapshot.weekly.status = UsageStatus::Exhausted;
            snapshot.session.status = UsageStatus::Warning;
        }
        FailureReason::RateLimited => snapshot.session.status = UsageStatus::Warning,
        FailureReason::AuthInvalid => {}
        FailureReason::CommandFailed | FailureReason::ValidationFailed | FailureReason::Unknown => {
            snapshot.can_auto_switch = false;
            snapshot.auto_switch_reason = None;
        }
    }

    Ok(Some(snapshot))
}

fn apply_auto_switch_policy(snapshot: &mut UsageSnapshot) {
    if snapshot.auto_switch_reason.is_none() {
        snapshot.auto_switch_reason = crate::services::policy_service::auto_switch_reason(snapshot);
    }
    snapshot.can_auto_switch = snapshot.auto_switch_reason.is_some() && !snapshot.stale;
}

fn apply_profile_context(snapshot: &mut UsageSnapshot, active_profile: Option<&Profile>) {
    snapshot.profile_id = active_profile.map(|profile| profile.id.clone());
    snapshot.profile_name = active_profile.map(|profile| profile.nickname.clone());
}

fn empty_snapshot(
    active_profile: Option<&Profile>,
    source: UsageSource,
    stale: bool,
    message: Option<String>,
) -> UsageSnapshot {
    let now = Utc::now();
    let mut snapshot = UsageSnapshot {
        profile_id: None,
        profile_name: None,
        source,
        confidence: UsageConfidence::Low,
        stale,
        last_refreshed_at: now,
        next_reset_at: None,
        session: UsageWindow {
            used_percent: None,
            window_minutes: Some(300),
            reset_at: None,
            status: UsageStatus::Unknown,
            exact: false,
        },
        weekly: UsageWindow {
            used_percent: None,
            window_minutes: Some(10_080),
            reset_at: None,
            status: UsageStatus::Unknown,
            exact: false,
        },
        auto_switch_reason: None,
        can_auto_switch: false,
        message,
    };
    apply_profile_context(&mut snapshot, active_profile);
    snapshot
}

fn build_window(
    used_percent: Option<f64>,
    window_minutes: Option<i64>,
    reset_at: Option<DateTime<Utc>>,
    exact: bool,
) -> UsageWindow {
    let status = match used_percent {
        Some(value) if value >= 100.0 => UsageStatus::Exhausted,
        Some(value) if value >= 80.0 => UsageStatus::Warning,
        Some(_) => UsageStatus::Healthy,
        None => UsageStatus::Unknown,
    };

    UsageWindow {
        used_percent,
        window_minutes,
        reset_at,
        status,
        exact,
    }
}

fn snapshot_from_rate_limit_snapshot(
    rate_limits: AppServerRateLimitSnapshot,
    active_profile: Option<&Profile>,
    message: &str,
) -> UsageSnapshot {
    let session = app_server_window(rate_limits.primary.clone());
    let weekly = app_server_window(rate_limits.secondary.clone());
    let last_refreshed_at = Utc::now();
    let mut snapshot = UsageSnapshot {
        profile_id: None,
        profile_name: None,
        source: UsageSource::Local,
        confidence: UsageConfidence::High,
        stale: false,
        last_refreshed_at,
        next_reset_at: next_reset_at(&session, &weekly),
        session,
        weekly,
        auto_switch_reason: None,
        can_auto_switch: false,
        message: Some(message.into()),
    };
    apply_profile_context(&mut snapshot, active_profile);
    apply_auto_switch_policy(&mut snapshot);
    snapshot
}

fn app_server_window(window: Option<AppServerRateLimitWindow>) -> UsageWindow {
    build_window(
        window.as_ref().map(|window| window.used_percent),
        window
            .as_ref()
            .and_then(|window| window.window_duration_mins),
        window
            .as_ref()
            .and_then(|window| window.resets_at)
            .and_then(from_unix_seconds),
        window.is_some(),
    )
}

fn next_reset_at(session: &UsageWindow, weekly: &UsageWindow) -> Option<DateTime<Utc>> {
    match (session.reset_at, weekly.reset_at) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn select_relevant_failure_event<'a>(
    events: &'a [FailureEvent],
    active_profile_id: Option<&str>,
) -> Option<&'a FailureEvent> {
    events.iter().find(|event| match active_profile_id {
        Some(profile_id) => event.profile_id.as_deref() == Some(profile_id),
        None => true,
    })
}

fn fallback_auto_switch_reason(reason: &FailureReason) -> Option<FailureReason> {
    match reason {
        FailureReason::SessionExhausted
        | FailureReason::WeeklyExhausted
        | FailureReason::AuthInvalid
        | FailureReason::QuotaExhausted
        | FailureReason::RateLimited => Some(reason.clone()),
        FailureReason::CommandFailed | FailureReason::ValidationFailed | FailureReason::Unknown => {
            None
        }
    }
}

fn collect_jsonl_files(root: &Path) -> Result<Vec<PathBuf>, RelayError> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();

    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry.file_type()?.is_dir() {
                pending.push(entry_path);
            } else if entry_path.extension().is_some_and(|value| value == "jsonl") {
                files.push(entry_path);
            }
        }
    }

    files.sort();
    Ok(files)
}

fn parse_latest_usage_from_file(path: &Path) -> Result<Option<LocalUsageReading>, RelayError> {
    let contents = fs::read_to_string(path)?;
    let mut newest = None;

    for line in contents.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };

        if value.get("type").and_then(Value::as_str) != Some("event_msg") {
            continue;
        }
        if value.pointer("/payload/type").and_then(Value::as_str) != Some("token_count") {
            continue;
        }

        let timestamp = value
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(parse_rfc3339)
            .unwrap_or_else(Utc::now);

        let Some(primary) = value.pointer("/payload/info/rate_limits/primary") else {
            continue;
        };

        let reading = LocalUsageReading {
            last_refreshed_at: timestamp,
            primary_used_percent: primary
                .get("used_percent")
                .and_then(Value::as_f64)
                .unwrap_or_default(),
            primary_window_minutes: primary
                .get("window_minutes")
                .and_then(Value::as_i64)
                .unwrap_or(300),
            primary_resets_at: primary
                .get("resets_at")
                .and_then(Value::as_i64)
                .and_then(from_unix_seconds)
                .unwrap_or(timestamp + Duration::minutes(300)),
            secondary_used_percent: value
                .pointer("/payload/info/rate_limits/secondary/used_percent")
                .and_then(Value::as_f64),
            secondary_window_minutes: value
                .pointer("/payload/info/rate_limits/secondary/window_minutes")
                .and_then(Value::as_i64),
            secondary_resets_at: value
                .pointer("/payload/info/rate_limits/secondary/resets_at")
                .and_then(Value::as_i64)
                .and_then(from_unix_seconds),
        };

        if newest.as_ref().is_none_or(|current: &LocalUsageReading| {
            reading.last_refreshed_at > current.last_refreshed_at
        }) {
            newest = Some(reading);
        }
    }

    Ok(newest)
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn from_unix_seconds(value: i64) -> Option<DateTime<Utc>> {
    DateTime::<Utc>::from_timestamp(value, 0)
}

fn canonicalize_lossy(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn looks_like_real_codex_auth(path: &Path) -> bool {
    let Ok(contents) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<Value>(&contents) else {
        return false;
    };

    value.get("OPENAI_API_KEY").is_some()
        || value.get("auth_mode").is_some()
        || value.get("tokens").is_some()
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

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppServerRateLimitsResponse {
    rate_limits: AppServerRateLimitSnapshot,
    rate_limits_by_limit_id: Option<std::collections::HashMap<String, AppServerRateLimitSnapshot>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppServerRateLimitSnapshot {
    primary: Option<AppServerRateLimitWindow>,
    secondary: Option<AppServerRateLimitWindow>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppServerRateLimitWindow {
    used_percent: f64,
    window_duration_mins: Option<i64>,
    resets_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AuthMode;
    use crate::store::AddProfileRecord;
    use tempfile::tempdir;

    fn make_profile() -> Profile {
        Profile {
            id: "p1".into(),
            nickname: "Work".into(),
            agent: crate::models::AgentKind::Codex,
            priority: 10,
            enabled: true,
            agent_home: Some("/tmp/work".into()),
            config_path: Some("/tmp/work/config.toml".into()),
            auth_mode: AuthMode::ConfigFilesystem,
            metadata: serde_json::json!({}),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn builds_local_usage_snapshot_from_session_jsonl() {
        let temp = tempdir().expect("tempdir");
        let sessions_dir = temp.path().join("sessions/2026/03/08");
        fs::create_dir_all(&sessions_dir).expect("sessions");
        fs::write(
            sessions_dir.join("rollout.jsonl"),
            "{\"timestamp\":\"2026-03-08T12:39:47.628Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"rate_limits\":{\"primary\":{\"used_percent\":41.0,\"window_minutes\":300,\"resets_at\":1772979934},\"secondary\":{\"used_percent\":12.0,\"window_minutes\":10080,\"resets_at\":1773566734}}}}}\n",
        )
        .expect("session");

        let store = SqliteStore::new(temp.path().join("relay.db")).expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let snapshot =
            build(&store, &usage_store, Some(&make_profile()), temp.path()).expect("usage");

        assert_eq!(snapshot.source, UsageSource::Local);
        assert_eq!(snapshot.profile_name.as_deref(), Some("Work"));
        assert_eq!(snapshot.session.used_percent, Some(41.0));
        assert_eq!(snapshot.weekly.used_percent, Some(12.0));
        assert!(snapshot.stale);
    }

    #[test]
    fn parses_app_server_rate_limit_response() {
        let value = serde_json::json!({
            "id": 2,
            "result": {
                "rateLimits": {
                    "limitId": "codex",
                    "limitName": null,
                    "primary": {
                        "usedPercent": 17,
                        "windowDurationMins": 300,
                        "resetsAt": 1772989560
                    },
                    "secondary": {
                        "usedPercent": 28,
                        "windowDurationMins": 10080,
                        "resetsAt": 1773297378
                    },
                    "credits": null,
                    "planType": "team"
                },
                "rateLimitsByLimitId": {
                    "codex": {
                        "limitId": "codex",
                        "limitName": null,
                        "primary": {
                            "usedPercent": 17,
                            "windowDurationMins": 300,
                            "resetsAt": 1772989560
                        },
                        "secondary": {
                            "usedPercent": 28,
                            "windowDurationMins": 10080,
                            "resetsAt": 1773297378
                        },
                        "credits": null,
                        "planType": "team"
                    }
                }
            }
        });

        let snapshot = parse_app_server_message(value, Some(&make_profile()))
            .expect("parse response")
            .expect("snapshot");
        assert_eq!(snapshot.source, UsageSource::Local);
        assert_eq!(snapshot.session.used_percent, Some(17.0));
        assert_eq!(snapshot.weekly.used_percent, Some(28.0));
        assert_eq!(
            snapshot.message.as_deref(),
            Some("codex app-server rate limit RPC")
        );
    }

    #[test]
    fn falls_back_to_failure_events_when_local_usage_missing() {
        let temp = tempdir().expect("tempdir");
        let store = SqliteStore::new(temp.path().join("relay.db")).expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let profile = store
            .add_profile(AddProfileRecord {
                nickname: "Work".into(),
                priority: 10,
                config_path: Some(temp.path().join("config.toml")),
                codex_home: Some(temp.path().join("work-home")),
                auth_mode: AuthMode::ConfigFilesystem,
            })
            .expect("profile");
        store
            .record_failure_event(
                Some(&profile.id),
                FailureReason::RateLimited,
                "rate limited by codex",
                None,
            )
            .expect("event");

        let snapshot = build(&store, &usage_store, Some(&profile), temp.path()).expect("usage");

        assert_eq!(snapshot.source, UsageSource::Fallback);
        assert_eq!(
            snapshot.auto_switch_reason,
            Some(FailureReason::RateLimited)
        );
        assert!(snapshot.can_auto_switch);
        assert_eq!(snapshot.session.status, UsageStatus::Warning);
        assert_eq!(snapshot.weekly.status, UsageStatus::Unknown);
    }
}
