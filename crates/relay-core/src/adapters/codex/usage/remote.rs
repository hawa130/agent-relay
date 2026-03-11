use crate::internal::usage_policy::{
    apply_auto_switch_policy, build_usage_window, next_reset_at, unknown_usage_window,
};
use crate::models::{
    AppSettings, Profile, ProfileProbeIdentity, RelayError, UsageConfidence, UsageRemoteError,
    UsageRemoteErrorKind, UsageSnapshot, UsageSource, UsageWindow,
};
use crate::store::SqliteStore;
use base64::Engine;
use chrono::{Duration, Utc};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, ClientBuilder};
use serde::Deserialize;
use serde_json::Value;
use std::borrow::Cow;
use std::env;
use std::fs;
use std::time::Duration as StdDuration;

const OFFICIAL_HTTP_TIMEOUT_SECS: u64 = 10;
const OFFICIAL_USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const OFFICIAL_REFRESH_URL: &str = "https://auth.openai.com/oauth/token";
const OFFICIAL_REFRESH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const ERROR_BODY_PREVIEW_LIMIT: usize = 512;

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
    if should_refresh_probe_identity(store, &identity).await?
        && identity
            .refresh_token()
            .is_some_and(|token| !token.is_empty())
    {
        identity = match refresh_probe_identity(store, &identity).await {
            Ok(identity) => identity,
            Err(RefreshProbeIdentityError::Remote(failure)) => {
                return Ok(remote_error_snapshot(profile, &failure));
            }
            Err(RefreshProbeIdentityError::Relay(error)) => return Err(error),
        };
    }

    let mut response = match official_usage_request(&identity).await {
        Ok(response) => response,
        Err(failure) => return Ok(remote_error_snapshot(profile, &failure)),
    };
    if should_refresh_official_response(&response)
        && identity
            .refresh_token()
            .is_some_and(|token| !token.is_empty())
    {
        identity = match refresh_probe_identity(store, &identity).await {
            Ok(identity) => identity,
            Err(RefreshProbeIdentityError::Remote(failure)) => {
                return Ok(remote_error_snapshot(profile, &failure));
            }
            Err(RefreshProbeIdentityError::Relay(error)) => return Err(error),
        };
        response = match official_usage_request(&identity).await {
            Ok(response) => response,
            Err(failure) => return Ok(remote_error_snapshot(profile, &failure)),
        };
    }
    if response.http_code != 200 {
        return Ok(remote_error_snapshot(
            profile,
            &http_failure("failed to fetch codex rate limits", &response),
        ));
    }

    let payload: OfficialUsageResponse = match serde_json::from_str(&response.body) {
        Ok(payload) => payload,
        Err(error) => {
            return Ok(remote_error_snapshot(
                profile,
                &decode_failure("failed to decode codex rate limits response", error),
            ));
        }
    };
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
        remote_error: None,
    };
    apply_auto_switch_policy(&mut snapshot);
    Ok(snapshot)
}

fn should_refresh_official_response(response: &HttpResponse) -> bool {
    matches!(response.http_code, 401 | 403)
}

async fn should_refresh_probe_identity(
    store: &SqliteStore,
    identity: &ProfileProbeIdentity,
) -> Result<bool, RelayError> {
    let Some(refresh_token) = identity.refresh_token() else {
        return Ok(false);
    };
    if refresh_token.is_empty() {
        return Ok(false);
    }

    let Some(access_token) = identity.access_token() else {
        return Ok(true);
    };

    let Some(expiry) = jwt_expiry(access_token) else {
        return Ok(true);
    };

    let refresh_threshold = token_refresh_threshold(store.get_settings().await?);
    Ok(expiry - Utc::now() <= refresh_threshold)
}

fn token_refresh_threshold(settings: AppSettings) -> Duration {
    Duration::seconds(settings.refresh_interval_seconds.max(600))
}

fn jwt_expiry(token: &str) -> Option<chrono::DateTime<Utc>> {
    let mut segments = token.split('.');
    let _header = segments.next()?;
    let payload = segments.next()?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let value: Value = serde_json::from_slice(&decoded).ok()?;
    let exp = value.get("exp")?.as_i64()?;
    chrono::DateTime::<Utc>::from_timestamp(exp, 0)
}

async fn refresh_probe_identity(
    store: &SqliteStore,
    identity: &ProfileProbeIdentity,
) -> Result<ProfileProbeIdentity, RefreshProbeIdentityError> {
    let refresh_token = identity
        .refresh_token()
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            RefreshProbeIdentityError::Relay(RelayError::Validation(
                "probe identity is missing refresh_token".into(),
            ))
        })?;
    let body = serde_json::json!({
        "client_id": OFFICIAL_REFRESH_CLIENT_ID,
        "grant_type": "refresh_token",
        "refresh_token": refresh_token,
    })
    .to_string();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let response = run_http_json(
        "failed to refresh codex access token",
        &official_refresh_url(),
        reqwest::Method::POST,
        headers,
        Some(body),
    )
    .await
    .map_err(RefreshProbeIdentityError::Remote)?;
    if response.http_code != 200 {
        return Err(RefreshProbeIdentityError::Remote(http_failure(
            "failed to refresh codex access token",
            &response,
        )));
    }

    let refreshed: OfficialRefreshResponse =
        serde_json::from_str(&response.body).map_err(|error| {
            RefreshProbeIdentityError::Remote(decode_failure(
                "failed to decode codex refresh token response",
                error,
            ))
        })?;
    let updated = ProfileProbeIdentity::codex_official(
        identity.profile_id.clone(),
        identity
            .account_id()
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                RefreshProbeIdentityError::Relay(RelayError::Validation(
                    "probe identity is missing account_id".into(),
                ))
            })?,
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

    store
        .upsert_probe_identity(&updated)
        .await
        .map_err(RefreshProbeIdentityError::Relay)
}

async fn official_usage_request(
    identity: &ProfileProbeIdentity,
) -> Result<HttpResponse, RemoteUsageFailure> {
    let access_token = identity.access_token().ok_or_else(|| {
        other_failure(
            "failed to fetch codex rate limits",
            "probe identity is missing access_token".into(),
        )
    })?;
    let account_id = identity.account_id().ok_or_else(|| {
        other_failure(
            "failed to fetch codex rate limits",
            "probe identity is missing account_id".into(),
        )
    })?;
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        header_value(&format!("Bearer {access_token}")).map_err(|error| {
            other_failure("failed to fetch codex rate limits", error.to_string())
        })?,
    );
    headers.insert(
        HeaderName::from_static("chatgpt-account-id"),
        header_value(account_id).map_err(|error| {
            other_failure("failed to fetch codex rate limits", error.to_string())
        })?,
    );

    run_http_json(
        "failed to fetch codex rate limits",
        &official_usage_url(),
        reqwest::Method::GET,
        headers,
        None,
    )
    .await
}

async fn run_http_json(
    operation: &'static str,
    url: &str,
    method: reqwest::Method,
    headers: HeaderMap,
    body: Option<String>,
) -> Result<HttpResponse, RemoteUsageFailure> {
    if let Some(path) = url.strip_prefix("file://") {
        let body = fs::read_to_string(path)
            .map_err(|error| other_failure(operation, error.to_string()))?;
        return Ok(HttpResponse {
            method,
            url: url.to_string(),
            http_code: 200,
            reason_phrase: "OK".into(),
            content_type: Some("application/json".into()),
            body,
        });
    }

    let client =
        official_http_client().map_err(|error| other_failure(operation, error.to_string()))?;
    let mut request = client.request(method.clone(), url).headers(headers);
    if let Some(body) = body {
        request = request.body(body);
    }

    let response = request
        .send()
        .await
        .map_err(|error| transport_failure(operation, method.as_str(), url, error))?;
    let http_code = response.status().as_u16();
    let reason_phrase = response
        .status()
        .canonical_reason()
        .unwrap_or("Unknown Status")
        .to_string();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let body = response
        .text()
        .await
        .map_err(|error| transport_failure(operation, method.as_str(), url, error))?;

    Ok(HttpResponse {
        method,
        url: url.to_string(),
        http_code,
        reason_phrase,
        content_type,
        body,
    })
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

fn transport_failure(
    operation: &str,
    method: &str,
    url: &str,
    error: reqwest::Error,
) -> RemoteUsageFailure {
    RemoteUsageFailure {
        message: format!("Codex connection failed: {operation}: {method} {url} failed: {error}"),
        remote_error: UsageRemoteError {
            kind: UsageRemoteErrorKind::Network,
            http_status: None,
        },
    }
}

fn http_failure(operation: &str, response: &HttpResponse) -> RemoteUsageFailure {
    let content_type = response.content_type.as_deref().unwrap_or("unknown");
    RemoteUsageFailure {
        message: format!(
            "Codex connection failed: {operation}: {} {} failed: {} {}; content-type={content_type}; body={}",
            response.method.as_str(),
            response.url,
            response.http_code,
            response.reason_phrase,
            body_preview(&response.body),
        ),
        remote_error: UsageRemoteError {
            kind: UsageRemoteErrorKind::Other,
            http_status: Some(response.http_code),
        },
    }
}

fn decode_failure(operation: &str, error: impl std::fmt::Display) -> RemoteUsageFailure {
    other_failure(operation, error.to_string())
}

fn other_failure(operation: &str, detail: String) -> RemoteUsageFailure {
    RemoteUsageFailure {
        message: format!("Codex connection failed: {operation}: {detail}"),
        remote_error: UsageRemoteError {
            kind: UsageRemoteErrorKind::Other,
            http_status: None,
        },
    }
}

fn body_preview(body: &str) -> Cow<'_, str> {
    let normalized = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return Cow::Borrowed("<empty>");
    }
    if normalized.chars().count() <= ERROR_BODY_PREVIEW_LIMIT {
        return Cow::Owned(normalized);
    }

    let mut truncated = normalized
        .chars()
        .take(ERROR_BODY_PREVIEW_LIMIT)
        .collect::<String>();
    truncated.push_str("...");
    Cow::Owned(truncated)
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

fn remote_error_snapshot(profile: &Profile, failure: &RemoteUsageFailure) -> UsageSnapshot {
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
        message: Some(failure.message.clone()),
        remote_error: Some(failure.remote_error.clone()),
    }
}

struct HttpResponse {
    method: reqwest::Method,
    url: String,
    http_code: u16,
    reason_phrase: String,
    content_type: Option<String>,
    body: String,
}

#[derive(Debug, Clone)]
struct RemoteUsageFailure {
    message: String,
    remote_error: UsageRemoteError,
}

enum RefreshProbeIdentityError {
    Remote(RemoteUsageFailure),
    Relay(RelayError),
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
    use super::{
        HttpResponse, body_preview, http_failure, jwt_expiry, should_refresh_probe_identity,
        token_refresh_threshold, transport_failure,
    };
    use crate::models::{
        AppSettings, ProfileProbeIdentity, UsageRemoteError, UsageRemoteErrorKind,
    };
    use crate::store::SqliteStore;
    use base64::Engine;
    use chrono::{Duration, Utc};
    use reqwest::Method;
    use tempfile::tempdir;

    #[test]
    fn jwt_expiry_decodes_exp_claim() {
        let expiry = Utc::now() + Duration::minutes(30);
        let token = jwt_with_expiry(expiry);
        let decoded = jwt_expiry(&token).expect("jwt expiry");
        assert_eq!(decoded.timestamp(), expiry.timestamp());
    }

    #[test]
    fn token_refresh_threshold_uses_max_of_ten_minutes_and_refresh_interval() {
        let short_interval = AppSettings {
            refresh_interval_seconds: 60,
            ..AppSettings::default()
        };
        assert_eq!(
            token_refresh_threshold(short_interval),
            Duration::minutes(10)
        );

        let long_interval = AppSettings {
            refresh_interval_seconds: 900,
            ..AppSettings::default()
        };
        assert_eq!(
            token_refresh_threshold(long_interval),
            Duration::minutes(15)
        );
    }

    #[tokio::test]
    async fn non_jwt_tokens_refresh_on_every_request_when_refresh_token_exists() {
        let temp = tempdir().expect("tempdir");
        let store = SqliteStore::new(temp.path().join("relay.db"))
            .await
            .expect("store");
        let identity = probe_identity("not-a-jwt", Some("refresh-token"));
        assert!(
            should_refresh_probe_identity(&store, &identity)
                .await
                .expect("should refresh"),
            "non-jwt token should refresh eagerly"
        );
    }

    #[tokio::test]
    async fn jwt_refresh_uses_refresh_interval_threshold() {
        let temp = tempdir().expect("tempdir");
        let store = SqliteStore::new(temp.path().join("relay.db"))
            .await
            .expect("store");
        store
            .set_refresh_interval_seconds(900)
            .await
            .expect("set refresh interval");

        let fresh_identity = probe_identity(
            &jwt_with_expiry(Utc::now() + Duration::minutes(20)),
            Some("refresh-token"),
        );
        assert!(
            !should_refresh_probe_identity(&store, &fresh_identity)
                .await
                .expect("fresh token"),
            "token beyond threshold should not refresh"
        );

        let stale_identity = probe_identity(
            &jwt_with_expiry(Utc::now() + Duration::minutes(10)),
            Some("refresh-token"),
        );
        assert!(
            should_refresh_probe_identity(&store, &stale_identity)
                .await
                .expect("stale token"),
            "token inside threshold should refresh"
        );
    }

    #[test]
    fn http_failure_includes_status_headers_and_body_preview() {
        let error = http_failure(
            "failed to fetch codex rate limits",
            &HttpResponse {
                method: Method::GET,
                url: "https://chatgpt.com/backend-api/wham/usage".into(),
                http_code: 402,
                reason_phrase: "Payment Required".into(),
                content_type: Some("application/json".into()),
                body: "{\"detail\":{\"code\":\"deactivated_workspace\"}}".into(),
            },
        );

        assert_eq!(
            error.message,
            "Codex connection failed: failed to fetch codex rate limits: GET https://chatgpt.com/backend-api/wham/usage failed: 402 Payment Required; content-type=application/json; body={\"detail\":{\"code\":\"deactivated_workspace\"}}"
        );
        assert_eq!(
            error.remote_error,
            UsageRemoteError {
                kind: UsageRemoteErrorKind::Other,
                http_status: Some(402),
            }
        );
    }

    #[test]
    fn body_preview_normalizes_and_truncates() {
        let body = format!("{}\n{}", "a".repeat(400), "b".repeat(400));
        let preview = body_preview(&body);
        assert!(preview.contains(" "));
        assert!(preview.ends_with("..."));
        assert!(preview.len() <= 515);
    }

    #[test]
    fn body_preview_uses_empty_marker() {
        assert_eq!(body_preview(" \n\t ").as_ref(), "<empty>");
    }

    #[test]
    fn transport_failure_is_prefixed_consistently() {
        let client = reqwest::Client::new();
        let error = transport_failure(
            "failed to fetch codex rate limits",
            "GET",
            "https://chatgpt.com/backend-api/wham/usage",
            client
                .get("http://[::1")
                .build()
                .expect_err("invalid URL should fail"),
        );

        assert!(error
            .message
            .starts_with("Codex connection failed: failed to fetch codex rate limits: GET https://chatgpt.com/backend-api/wham/usage failed: "));
        assert_eq!(
            error.remote_error,
            UsageRemoteError {
                kind: UsageRemoteErrorKind::Network,
                http_status: None,
            }
        );
    }

    fn probe_identity(access_token: &str, refresh_token: Option<&str>) -> ProfileProbeIdentity {
        let now = Utc::now().to_rfc3339();
        ProfileProbeIdentity::codex_official(
            "p_test".into(),
            "acct".into(),
            access_token.into(),
            refresh_token.map(str::to_string),
            None,
            Some("test@example.com".into()),
            Some("plus".into()),
            now.clone(),
            now,
        )
    }

    fn jwt_with_expiry(expiry: chrono::DateTime<Utc>) -> String {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(format!(r#"{{"exp":{}}}"#, expiry.timestamp()).as_bytes());
        format!("{header}.{payload}.signature")
    }
}
