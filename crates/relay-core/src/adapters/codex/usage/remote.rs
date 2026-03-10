use crate::internal::usage_policy::{
    apply_auto_switch_policy, build_usage_window, next_reset_at, unknown_usage_window,
};
use crate::models::{
    Profile, ProfileProbeIdentity, RelayError, UsageConfidence, UsageSnapshot, UsageSource,
    UsageWindow,
};
use crate::store::SqliteStore;
use chrono::{Duration, Utc};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, ClientBuilder};
use serde::Deserialize;
use std::env;
use std::fs;
use std::time::Duration as StdDuration;

const OFFICIAL_HTTP_TIMEOUT_SECS: u64 = 10;
const OFFICIAL_USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const OFFICIAL_REFRESH_URL: &str = "https://auth.openai.com/oauth/token";
const OFFICIAL_REFRESH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

pub(crate) async fn collect_remote(
    store: &SqliteStore,
    profile: Option<&Profile>,
) -> Result<Option<UsageSnapshot>, RelayError> {
    let Some(profile) = profile else {
        return Ok(None);
    };
    let Some(identity) = store.get_probe_identity(&profile.id).await? else {
        return Ok(None);
    };
    let snapshot = fetch_official_usage_snapshot(store, profile, identity).await?;
    Ok(Some(snapshot))
}

async fn fetch_official_usage_snapshot(
    store: &SqliteStore,
    profile: &Profile,
    mut identity: ProfileProbeIdentity,
) -> Result<UsageSnapshot, RelayError> {
    let mut response = official_usage_request(&identity).await?;
    if should_refresh_official_response(&response)
        && identity
            .refresh_token()
            .is_some_and(|token| !token.is_empty())
    {
        identity = refresh_probe_identity(store, &identity).await?;
        response = official_usage_request(&identity).await?;
    }
    if response.http_code != 200 {
        return Ok(remote_error_snapshot(profile));
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
        message: None,
    };
    apply_auto_switch_policy(&mut snapshot);
    Ok(snapshot)
}

fn should_refresh_official_response(response: &HttpResponse) -> bool {
    matches!(response.http_code, 401 | 403)
}

async fn refresh_probe_identity(
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
    )
    .await?;
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

    store.upsert_probe_identity(&updated).await
}

async fn official_usage_request(
    identity: &ProfileProbeIdentity,
) -> Result<HttpResponse, RelayError> {
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

    run_http_json(&official_usage_url(), reqwest::Method::GET, headers, None).await
}

async fn run_http_json(
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
        .await
        .map_err(|error| RelayError::ExternalCommand(error.to_string()))?;
    let http_code = response.status().as_u16();
    let body = response
        .text()
        .await
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
        return unknown_usage_window(None);
    };

    let reset_at = window
        .reset_after_seconds
        .map(|seconds| Utc::now() + Duration::seconds(seconds));
    build_usage_window(
        window.used_percent,
        window.limit_window_seconds.map(|seconds| seconds / 60),
        reset_at,
        true,
    )
}

fn remote_error_snapshot(profile: &Profile) -> UsageSnapshot {
    UsageSnapshot {
        profile_id: Some(profile.id.clone()),
        profile_name: Some(profile.nickname.clone()),
        source: UsageSource::WebEnhanced,
        confidence: UsageConfidence::Medium,
        stale: true,
        last_refreshed_at: Utc::now(),
        next_reset_at: None,
        session: unknown_usage_window(Some(300)),
        weekly: unknown_usage_window(Some(10080)),
        auto_switch_reason: None,
        can_auto_switch: false,
        message: Some("Enhanced usage is currently unavailable.".into()),
    }
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
