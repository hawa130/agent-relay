use crate::models::{
    FailureReason, Profile, ProfileProbeIdentity, RelayError, UsageConfidence, UsageSnapshot,
    UsageSource, UsageSourceMode, UsageStatus, UsageWindow,
};
use crate::platform::{find_binary, live_codex_home};
use crate::store::{FileUsageStore, SqliteStore};
use chrono::{DateTime, Duration, TimeZone, Utc};
use reqwest::blocking::{Client, ClientBuilder};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration as StdDuration, Instant};

const SNAPSHOT_STALE_AFTER_MINUTES: i64 = 15;
const APP_SERVER_TIMEOUT_SECS: u64 = 6;
const OFFICIAL_HTTP_TIMEOUT_SECS: u64 = 10;
const OFFICIAL_USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const OFFICIAL_REFRESH_URL: &str = "https://auth.openai.com/oauth/token";
const OFFICIAL_REFRESH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

pub fn build_active(
    store: &SqliteStore,
    usage_store: &FileUsageStore,
    active_profile: Option<&Profile>,
    live_home: &Path,
    source_mode: UsageSourceMode,
    allow_cache_writes: bool,
) -> Result<UsageSnapshot, RelayError> {
    refresh_profile(
        store,
        usage_store,
        active_profile,
        active_profile,
        live_home,
        source_mode,
        allow_cache_writes,
    )
}

pub fn refresh_profile(
    store: &SqliteStore,
    usage_store: &FileUsageStore,
    target_profile: Option<&Profile>,
    active_profile: Option<&Profile>,
    live_home: &Path,
    source_mode: UsageSourceMode,
    allow_cache_writes: bool,
) -> Result<UsageSnapshot, RelayError> {
    let local_home = resolve_local_home(target_profile, active_profile, live_home);
    let providers = provider_order(source_mode.clone());

    for provider in providers {
        let snapshot = match provider {
            Provider::Local => {
                collect_local_snapshot(target_profile, active_profile, local_home.as_deref())?
            }
            Provider::WebEnhanced => collect_web_enhanced_snapshot(store, target_profile)?,
            Provider::Fallback => collect_fallback_snapshot(store, target_profile)?,
        };

        if let Some(mut snapshot) = snapshot {
            maybe_note_fallback(&mut snapshot, source_mode);
            if allow_cache_writes && snapshot.profile_id.is_some() {
                usage_store.save_profile(&snapshot)?;
            }
            return Ok(snapshot);
        }
    }

    if let Some(profile_id) = target_profile.and_then(|profile| Some(profile.id.as_str())) {
        if let Some(mut snapshot) = usage_store.load_profile(profile_id)? {
            refresh_cache_metadata(&mut snapshot);
            snapshot.can_auto_switch = false;
            snapshot.auto_switch_reason = None;
            snapshot.message = Some("using cached usage snapshot".into());
            return Ok(snapshot);
        }
    }

    let snapshot = empty_snapshot(
        target_profile,
        UsageSource::Fallback,
        true,
        Some("usage unavailable from configured sources".into()),
    );
    if allow_cache_writes && snapshot.profile_id.is_some() {
        usage_store.save_profile(&snapshot)?;
    }
    Ok(snapshot)
}

pub fn load_profile_snapshot(
    usage_store: &FileUsageStore,
    profile: &Profile,
) -> Result<UsageSnapshot, RelayError> {
    if let Some(mut snapshot) = usage_store.load_profile(&profile.id)? {
        refresh_cache_metadata(&mut snapshot);
        apply_profile_context(&mut snapshot, Some(profile));
        return Ok(snapshot);
    }

    Ok(empty_snapshot(
        Some(profile),
        UsageSource::Fallback,
        true,
        Some("usage not fetched yet".into()),
    ))
}

pub fn list_profile_snapshots(
    usage_store: &FileUsageStore,
    profiles: &[Profile],
) -> Result<Vec<UsageSnapshot>, RelayError> {
    let cache = usage_store.load_all()?;
    let mut by_profile = HashMap::with_capacity(cache.len());
    for snapshot in cache {
        if let Some(profile_id) = snapshot.profile_id.clone() {
            by_profile.insert(profile_id, snapshot);
        }
    }

    let mut snapshots = Vec::with_capacity(profiles.len());
    for profile in profiles {
        if let Some(mut snapshot) = by_profile.remove(&profile.id) {
            refresh_cache_metadata(&mut snapshot);
            apply_profile_context(&mut snapshot, Some(profile));
            snapshots.push(snapshot);
        } else {
            snapshots.push(empty_snapshot(
                Some(profile),
                UsageSource::Fallback,
                true,
                Some("usage not fetched yet".into()),
            ));
        }
    }

    Ok(snapshots)
}

fn resolve_local_home(
    target_profile: Option<&Profile>,
    active_profile: Option<&Profile>,
    live_home: &Path,
) -> Option<PathBuf> {
    if target_profile.is_none() {
        return Some(live_home.to_path_buf());
    }

    if target_profile
        .zip(active_profile)
        .is_some_and(|(target, active)| target.id == active.id)
    {
        return Some(live_home.to_path_buf());
    }

    target_profile
        .and_then(|profile| profile.agent_home.as_ref())
        .map(PathBuf::from)
}

#[derive(Clone, Copy)]
enum Provider {
    Local,
    WebEnhanced,
    Fallback,
}

fn provider_order(mode: UsageSourceMode) -> [Provider; 3] {
    match mode {
        UsageSourceMode::Auto => [Provider::Local, Provider::WebEnhanced, Provider::Fallback],
        UsageSourceMode::Local => [Provider::Local, Provider::WebEnhanced, Provider::Fallback],
        UsageSourceMode::WebEnhanced => {
            [Provider::WebEnhanced, Provider::Local, Provider::Fallback]
        }
    }
}

fn maybe_note_fallback(snapshot: &mut UsageSnapshot, source_mode: UsageSourceMode) {
    if source_mode == UsageSourceMode::WebEnhanced && snapshot.source != UsageSource::WebEnhanced {
        snapshot.message = Some(match snapshot.message.take() {
            Some(existing) => format!("web-enhanced provider unavailable; {existing}"),
            None => "web-enhanced provider unavailable; fell back to local usage".into(),
        });
    }
}

fn collect_local_snapshot(
    target_profile: Option<&Profile>,
    active_profile: Option<&Profile>,
    local_home: Option<&Path>,
) -> Result<Option<UsageSnapshot>, RelayError> {
    let Some(local_home) = local_home else {
        return Ok(None);
    };
    if target_profile
        .zip(active_profile)
        .is_some_and(|(target, active)| target.id == active.id)
    {
        if let Some(snapshot) = collect_app_server_snapshot(target_profile, local_home)? {
            return Ok(Some(snapshot));
        }
    }

    collect_session_snapshot(target_profile, local_home)
}

fn collect_app_server_snapshot(
    profile: Option<&Profile>,
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
    };
    if stale {
        snapshot.message = Some("local usage data is stale".into());
    }
    apply_auto_switch_policy(&mut snapshot);
    Ok(Some(snapshot))
}

fn collect_web_enhanced_snapshot(
    store: &SqliteStore,
    profile: Option<&Profile>,
) -> Result<Option<UsageSnapshot>, RelayError> {
    let Some(profile) = profile else {
        return Ok(None);
    };
    let Some(identity) = store.get_probe_identity(&profile.id)? else {
        return Ok(None);
    };
    let snapshot = fetch_official_usage_snapshot(store, profile, identity)?;
    Ok(Some(snapshot))
}

fn fetch_official_usage_snapshot(
    store: &SqliteStore,
    profile: &Profile,
    mut identity: ProfileProbeIdentity,
) -> Result<UsageSnapshot, RelayError> {
    let mut response = official_usage_request(&identity)?;
    if should_refresh_official_response(&response)
        && identity
            .refresh_token()
            .is_some_and(|token| !token.is_empty())
    {
        identity = refresh_probe_identity(store, &identity)?;
        response = official_usage_request(&identity)?;
    }
    if response.http_code != 200 {
        return Ok(remote_error_snapshot(
            profile,
            &format!("official usage returned HTTP {}", response.http_code),
        ));
    }

    let payload: OfficialUsageResponse = serde_json::from_str(&response.body)
        .map_err(|error| RelayError::ExternalCommand(error.to_string()))?;
    let session = official_window(payload.rate_limit.primary_window.as_ref());
    let weekly = official_window(payload.rate_limit.secondary_window.as_ref());
    let mut snapshot = UsageSnapshot {
        profile_id: Some(profile.id.clone()),
        profile_name: Some(profile.nickname.clone()),
        source: UsageSource::WebEnhanced,
        confidence: UsageConfidence::High,
        stale: false,
        last_refreshed_at: Utc::now(),
        next_reset_at: next_reset_at(&session, &weekly),
        session,
        weekly,
        auto_switch_reason: None,
        can_auto_switch: false,
        message: Some("official usage API".into()),
    };
    apply_auto_switch_policy(&mut snapshot);
    Ok(snapshot)
}

fn should_refresh_official_response(response: &HttpResponse) -> bool {
    matches!(response.http_code, 401 | 403)
}

fn refresh_probe_identity(
    store: &SqliteStore,
    identity: &ProfileProbeIdentity,
) -> Result<ProfileProbeIdentity, RelayError> {
    let refresh_token = identity
        .refresh_token()
        .map(ToOwned::to_owned)
        .ok_or_else(|| RelayError::Validation("probe identity is missing refresh_token".into()))?;
    let body = serde_json::json!({
        "client_id": OFFICIAL_REFRESH_CLIENT_ID,
        "grant_type": "refresh_token",
        "refresh_token": refresh_token,
    })
    .to_string();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let response = run_http_json(
        &official_refresh_url(),
        reqwest::Method::POST,
        headers,
        Some(body),
    )?;
    if response.http_code != 200 {
        return Err(RelayError::ExternalCommand(format!(
            "official refresh returned HTTP {}",
            response.http_code
        )));
    }

    let refreshed: OfficialRefreshResponse = serde_json::from_str(&response.body)
        .map_err(|error| RelayError::ExternalCommand(error.to_string()))?;
    let updated = ProfileProbeIdentity::codex_official(
        identity.profile_id.clone(),
        identity
            .account_id()
            .map(ToOwned::to_owned)
            .ok_or_else(|| RelayError::Validation("probe identity is missing account_id".into()))?,
        refreshed.access_token,
        refreshed
            .refresh_token
            .or_else(|| identity.refresh_token().map(ToOwned::to_owned)),
        refreshed
            .id_token
            .or_else(|| identity.id_token().map(ToOwned::to_owned)),
        identity.email().map(ToOwned::to_owned),
        identity.plan_hint().map(ToOwned::to_owned),
        identity.created_at.clone(),
        Utc::now().to_rfc3339(),
    );

    store.upsert_probe_identity(&updated)
}

fn official_usage_request(identity: &ProfileProbeIdentity) -> Result<HttpResponse, RelayError> {
    let access_token = identity
        .access_token()
        .ok_or_else(|| RelayError::Validation("probe identity is missing access_token".into()))?;
    let account_id = identity
        .account_id()
        .ok_or_else(|| RelayError::Validation("probe identity is missing account_id".into()))?;
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        header_value(&format!("Bearer {access_token}"))?,
    );
    headers.insert(
        HeaderName::from_static("chatgpt-account-id"),
        header_value(account_id)?,
    );

    run_http_json(&official_usage_url(), reqwest::Method::GET, headers, None)
}

fn run_http_json(
    url: &str,
    method: reqwest::Method,
    headers: HeaderMap,
    body: Option<String>,
) -> Result<HttpResponse, RelayError> {
    if let Some(path) = url.strip_prefix("file://") {
        let body = fs::read_to_string(path)?;
        return Ok(HttpResponse {
            http_code: 200,
            body,
        });
    }

    let client = official_http_client()?;
    let mut request = client.request(method, url).headers(headers);
    if let Some(body) = body {
        request = request.body(body);
    }

    let response = request
        .send()
        .map_err(|error| RelayError::ExternalCommand(error.to_string()))?;
    let http_code = response.status().as_u16();
    let body = response
        .text()
        .map_err(|error| RelayError::ExternalCommand(error.to_string()))?;

    Ok(HttpResponse { http_code, body })
}

fn official_http_client() -> Result<Client, RelayError> {
    ClientBuilder::new()
        .timeout(StdDuration::from_secs(OFFICIAL_HTTP_TIMEOUT_SECS))
        .user_agent("relay")
        .build()
        .map_err(|error| RelayError::ExternalCommand(error.to_string()))
}

fn official_usage_url() -> String {
    env::var("RELAY_OFFICIAL_USAGE_URL").unwrap_or_else(|_| OFFICIAL_USAGE_URL.into())
}

fn official_refresh_url() -> String {
    env::var("RELAY_OFFICIAL_REFRESH_URL").unwrap_or_else(|_| OFFICIAL_REFRESH_URL.into())
}

fn header_value(value: &str) -> Result<HeaderValue, RelayError> {
    HeaderValue::from_str(value).map_err(|error| RelayError::Validation(error.to_string()))
}

fn official_window(window: Option<&OfficialRateLimitWindow>) -> UsageWindow {
    let Some(window) = window else {
        return UsageWindow {
            used_percent: None,
            window_minutes: None,
            reset_at: None,
            status: UsageStatus::Unknown,
            exact: false,
        };
    };

    let reset_at = window
        .reset_after_seconds
        .map(|seconds| Utc::now() + Duration::seconds(seconds));
    build_window(
        window.used_percent,
        window.limit_window_seconds.map(|seconds| seconds / 60),
        reset_at,
        true,
    )
}

fn remote_error_snapshot(profile: &Profile, message: &str) -> UsageSnapshot {
    let mut snapshot = empty_snapshot(
        Some(profile),
        UsageSource::WebEnhanced,
        true,
        Some(message.into()),
    );
    snapshot.confidence = UsageConfidence::Medium;
    snapshot
}

fn collect_fallback_snapshot(
    store: &SqliteStore,
    profile: Option<&Profile>,
) -> Result<Option<UsageSnapshot>, RelayError> {
    let Some(profile) = profile else {
        return Ok(None);
    };
    let mut events = store.list_failure_events(100)?;
    events.retain(|event| event.profile_id.as_deref() == Some(profile.id.as_str()));
    let Some(event) = events.into_iter().max_by_key(|event| event.created_at) else {
        return Ok(None);
    };

    let mut snapshot = empty_snapshot(
        Some(profile),
        UsageSource::Fallback,
        is_usage_stale(event.created_at),
        Some(event.message.clone()),
    );
    snapshot.last_refreshed_at = event.created_at;
    snapshot.confidence = UsageConfidence::Medium;

    match event.reason {
        FailureReason::SessionExhausted => snapshot.session.status = UsageStatus::Exhausted,
        FailureReason::WeeklyExhausted => snapshot.weekly.status = UsageStatus::Exhausted,
        FailureReason::QuotaExhausted => {
            snapshot.weekly.status = UsageStatus::Exhausted;
            snapshot.session.status = UsageStatus::Warning;
        }
        FailureReason::RateLimited => snapshot.session.status = UsageStatus::Warning,
        FailureReason::AuthInvalid => {
            snapshot.session.status = UsageStatus::Warning;
            snapshot.weekly.status = UsageStatus::Warning;
        }
        FailureReason::CommandFailed | FailureReason::ValidationFailed | FailureReason::Unknown => {
        }
    }

    apply_auto_switch_policy(&mut snapshot);
    Ok(Some(snapshot))
}

fn refresh_cache_metadata(snapshot: &mut UsageSnapshot) {
    snapshot.stale = is_usage_stale(snapshot.last_refreshed_at);
    if snapshot.stale && snapshot.message.is_none() {
        snapshot.message = Some("cached usage data is stale".into());
    }
}

fn apply_auto_switch_policy(snapshot: &mut UsageSnapshot) {
    snapshot.auto_switch_reason = None;
    snapshot.can_auto_switch = false;
    if snapshot.stale || snapshot.confidence != UsageConfidence::High {
        return;
    }

    if snapshot.session.status == UsageStatus::Exhausted {
        snapshot.auto_switch_reason = Some(FailureReason::SessionExhausted);
    } else if snapshot.weekly.status == UsageStatus::Exhausted {
        snapshot.auto_switch_reason = Some(FailureReason::WeeklyExhausted);
    }

    snapshot.can_auto_switch = snapshot.auto_switch_reason.is_some();
}

fn apply_profile_context(snapshot: &mut UsageSnapshot, profile: Option<&Profile>) {
    snapshot.profile_id = profile.map(|value| value.id.clone());
    snapshot.profile_name = profile.map(|value| value.nickname.clone());
}

fn empty_snapshot(
    profile: Option<&Profile>,
    source: UsageSource,
    stale: bool,
    message: Option<String>,
) -> UsageSnapshot {
    let now = Utc::now();
    UsageSnapshot {
        profile_id: profile.map(|value| value.id.clone()),
        profile_name: profile.map(|value| value.nickname.clone()),
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
            window_minutes: Some(10080),
            reset_at: None,
            status: UsageStatus::Unknown,
            exact: false,
        },
        auto_switch_reason: None,
        can_auto_switch: false,
        message,
    }
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
    snapshot: AppServerRateLimitSnapshot,
    profile: Option<&Profile>,
    message: &str,
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
        message: Some(message.into()),
    };
    apply_auto_switch_policy(&mut snapshot);
    snapshot
}

fn app_server_window(window: Option<AppServerRateLimitWindow>) -> UsageWindow {
    let Some(window) = window else {
        return UsageWindow {
            used_percent: None,
            window_minutes: None,
            reset_at: None,
            status: UsageStatus::Unknown,
            exact: false,
        };
    };

    build_window(
        window.used_percent,
        window.window_minutes,
        window.resets_at.and_then(timestamp_to_datetime),
        true,
    )
}

fn next_reset_at(session: &UsageWindow, weekly: &UsageWindow) -> Option<DateTime<Utc>> {
    match (session.reset_at, weekly.reset_at) {
        (Some(session_reset), Some(weekly_reset)) => Some(session_reset.min(weekly_reset)),
        (Some(session_reset), None) => Some(session_reset),
        (None, Some(weekly_reset)) => Some(weekly_reset),
        (None, None) => None,
    }
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

fn is_usage_stale(timestamp: DateTime<Utc>) -> bool {
    Utc::now() - timestamp > Duration::minutes(SNAPSHOT_STALE_AFTER_MINUTES)
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
        "codex app-server rate limit RPC",
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

struct HttpResponse {
    http_code: u16,
    body: String,
}

#[derive(Deserialize)]
struct OfficialUsageResponse {
    rate_limit: OfficialRateLimit,
}

#[derive(Deserialize)]
struct OfficialRateLimit {
    primary_window: Option<OfficialRateLimitWindow>,
    secondary_window: Option<OfficialRateLimitWindow>,
}

#[derive(Deserialize)]
struct OfficialRateLimitWindow {
    used_percent: Option<f64>,
    limit_window_seconds: Option<i64>,
    reset_after_seconds: Option<i64>,
}

#[derive(Deserialize)]
struct OfficialRefreshResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{build_active, list_profile_snapshots, load_profile_snapshot, refresh_profile};
    use crate::models::{AuthMode, FailureReason, UsageSource, UsageSourceMode, UsageStatus};
    use crate::store::{FileUsageStore, SqliteStore};
    use chrono::{Duration, Utc};
    use std::fs;
    use tempfile::tempdir;

    fn profile(id: &str, nickname: &str, home: &std::path::Path) -> crate::models::Profile {
        crate::models::Profile {
            id: id.into(),
            nickname: nickname.into(),
            agent: crate::models::AgentKind::Codex,
            priority: 100,
            enabled: true,
            agent_home: Some(home.to_string_lossy().into_owned()),
            config_path: Some(home.join("config.toml").to_string_lossy().into_owned()),
            auth_mode: AuthMode::ConfigFilesystem,
            metadata: serde_json::Value::Null,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    fn make_home(path: &std::path::Path, label: &str, age_minutes: i64) {
        fs::create_dir_all(path.join("sessions/2026/03/08")).expect("sessions");
        fs::write(path.join("config.toml"), format!("model = \"{label}\"")).expect("config");
        fs::write(path.join("auth.json"), format!("{{\"token\":\"{label}\"}}")).expect("auth");
        let timestamp = (Utc::now() - Duration::minutes(age_minutes)).to_rfc3339();
        fs::write(
            path.join("sessions/2026/03/08/rollout.jsonl"),
            format!(
                "{{\"timestamp\":\"{timestamp}\",\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\",\"info\":{{\"rate_limits\":{{\"primary\":{{\"used_percent\":41.0,\"window_minutes\":300,\"resets_at\":1772979934}},\"secondary\":{{\"used_percent\":12.0,\"window_minutes\":10080,\"resets_at\":1773566734}}}}}}}}}}\n"
            ),
        )
        .expect("usage session");
    }

    #[test]
    fn builds_local_usage_snapshot_from_profile_home() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let inactive_home = temp.path().join("inactive");
        make_home(&inactive_home, "inactive", 0);
        let inactive_profile = profile("p_inactive", "inactive", &inactive_home);

        let snapshot = refresh_profile(
            &store,
            &usage_store,
            Some(&inactive_profile),
            None,
            temp.path(),
            UsageSourceMode::Local,
            true,
        )
        .expect("usage");

        assert_eq!(snapshot.source, UsageSource::Local);
        assert_eq!(snapshot.profile_id.as_deref(), Some("p_inactive"));
        assert_eq!(snapshot.session.used_percent, Some(41.0));
        assert_eq!(snapshot.weekly.used_percent, Some(12.0));
    }

    #[test]
    fn loads_cached_placeholder_for_unknown_profile_usage() {
        let temp = tempdir().expect("tempdir");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let home = temp.path().join("home");
        make_home(&home, "home", 0);
        let profile = profile("p_unknown", "work", &home);

        let snapshot = load_profile_snapshot(&usage_store, &profile).expect("snapshot");

        assert_eq!(snapshot.profile_id.as_deref(), Some("p_unknown"));
        assert!(snapshot.stale);
        assert_eq!(snapshot.message.as_deref(), Some("usage not fetched yet"));
    }

    #[test]
    fn falls_back_to_failure_events_without_auto_switch() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let home = temp.path().join("fallback");
        fs::create_dir_all(&home).expect("home");
        fs::write(home.join("config.toml"), "model = \"fallback\"").expect("config");
        let profile = profile("p_fallback", "fallback", &home);
        store
            .record_failure_event_for_test(
                &profile.id,
                FailureReason::SessionExhausted,
                "session exhausted",
            )
            .expect("failure event");

        let snapshot = refresh_profile(
            &store,
            &usage_store,
            Some(&profile),
            None,
            temp.path(),
            UsageSourceMode::Local,
            true,
        )
        .expect("snapshot");

        assert_eq!(snapshot.source, UsageSource::Fallback);
        assert_eq!(snapshot.session.status, UsageStatus::Exhausted);
        assert!(!snapshot.can_auto_switch);
    }

    #[test]
    fn lists_snapshots_for_all_profiles() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let active_home = temp.path().join("active");
        let inactive_home = temp.path().join("inactive");
        make_home(&active_home, "active", 0);
        make_home(&inactive_home, "inactive", 20);
        let active_profile = profile("p_active", "active", &active_home);
        let inactive_profile = profile("p_inactive", "inactive", &inactive_home);

        let _ = build_active(
            &store,
            &usage_store,
            Some(&active_profile),
            &active_home,
            UsageSourceMode::Local,
            true,
        )
        .expect("active usage");
        let _ = refresh_profile(
            &store,
            &usage_store,
            Some(&inactive_profile),
            Some(&active_profile),
            &active_home,
            UsageSourceMode::Local,
            true,
        )
        .expect("inactive usage");

        let snapshots = list_profile_snapshots(
            &usage_store,
            &[active_profile.clone(), inactive_profile.clone()],
        )
        .expect("snapshots");

        assert_eq!(snapshots.len(), 2);
        assert!(
            snapshots
                .iter()
                .any(|snapshot| snapshot.profile_id.as_deref() == Some("p_active"))
        );
        assert!(
            snapshots
                .iter()
                .any(|snapshot| snapshot.profile_id.as_deref() == Some("p_inactive"))
        );
    }

    #[test]
    fn persists_unavailable_snapshot_for_profile_refresh() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let home = temp.path().join("missing-usage");
        fs::create_dir_all(&home).expect("home");
        fs::write(home.join("config.toml"), "model = \"missing\"").expect("config");
        let profile = profile("p_missing", "missing", &home);

        let refreshed = refresh_profile(
            &store,
            &usage_store,
            Some(&profile),
            None,
            temp.path(),
            UsageSourceMode::Local,
            true,
        )
        .expect("refresh");
        let cached = load_profile_snapshot(&usage_store, &profile).expect("cached");

        assert_eq!(refreshed.profile_id.as_deref(), Some("p_missing"));
        assert_eq!(
            refreshed.message.as_deref(),
            Some("usage unavailable from configured sources")
        );
        assert_eq!(cached.message, refreshed.message);
    }

    #[test]
    fn inactive_profile_without_agent_home_does_not_read_active_live_usage() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let active_home = temp.path().join("active");
        make_home(&active_home, "active", 0);
        let active_profile = profile("p_active", "active", &active_home);
        let inactive_profile = crate::models::Profile {
            id: "p_inactive".into(),
            nickname: "inactive".into(),
            agent: crate::models::AgentKind::Codex,
            priority: 100,
            enabled: true,
            agent_home: None,
            config_path: Some(
                active_home
                    .join("config.toml")
                    .to_string_lossy()
                    .into_owned(),
            ),
            auth_mode: AuthMode::ConfigFilesystem,
            metadata: serde_json::Value::Null,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        let snapshot = refresh_profile(
            &store,
            &usage_store,
            Some(&inactive_profile),
            Some(&active_profile),
            &active_home,
            UsageSourceMode::Local,
            true,
        )
        .expect("refresh");

        assert_eq!(snapshot.profile_id.as_deref(), Some("p_inactive"));
        assert_eq!(snapshot.source, UsageSource::Fallback);
        assert_eq!(snapshot.session.used_percent, None);
        assert_eq!(
            snapshot.message.as_deref(),
            Some("usage unavailable from configured sources")
        );
    }
}
