use crate::adapters::UsageProvider;
use crate::internal::usage_policy::{
    apply_auto_switch_policy, is_usage_stale, unknown_usage_window,
};
use crate::models::{
    FailureReason, Profile, RelayError, UsageConfidence, UsageRemoteError, UsageSnapshot,
    UsageSource, UsageSourceMode, UsageStatus,
};
use crate::store::{FileUsageStore, SqliteStore};
use chrono::Utc;

pub async fn build_active(
    store: &SqliteStore,
    usage_store: &FileUsageStore,
    provider: &dyn UsageProvider,
    active_profile: Option<&Profile>,
    source_mode: UsageSourceMode,
    allow_cache_writes: bool,
) -> Result<UsageSnapshot, RelayError> {
    refresh_profile(
        store,
        usage_store,
        provider,
        active_profile,
        active_profile,
        source_mode,
        allow_cache_writes,
    )
    .await
}

pub async fn refresh_profile(
    store: &SqliteStore,
    usage_store: &FileUsageStore,
    provider: &dyn UsageProvider,
    target_profile: Option<&Profile>,
    active_profile: Option<&Profile>,
    source_mode: UsageSourceMode,
    allow_cache_writes: bool,
) -> Result<UsageSnapshot, RelayError> {
    let providers = provider_order(source_mode.clone());
    let mut remote_failure: Option<RemoteFailureContext> = None;

    for current in providers {
        let snapshot = match current {
            Provider::Local => provider.collect_local_usage(target_profile, active_profile)?,
            Provider::WebEnhanced => {
                match provider.collect_remote_usage(store, target_profile).await {
                    Ok(snapshot) => snapshot,
                    Err(error) => {
                        remote_failure = Some(RemoteFailureContext {
                            message: error.to_string(),
                            remote_error: None,
                        });
                        continue;
                    }
                }
            }
            Provider::Fallback => collect_fallback_snapshot(store, target_profile).await?,
        };

        if let Some(mut snapshot) = snapshot {
            if matches!(current, Provider::WebEnhanced)
                && snapshot.source == UsageSource::WebEnhanced
                && snapshot.remote_error.is_some()
            {
                remote_failure = Some(RemoteFailureContext::from_snapshot(&snapshot));
                continue;
            }
            if should_continue_to_next_provider(current, source_mode.clone(), &snapshot) {
                continue;
            }
            maybe_note_fallback(&mut snapshot, source_mode.clone(), remote_failure.as_ref());
            if allow_cache_writes && snapshot.profile_id.is_some() {
                usage_store.save_profile(&snapshot)?;
            }
            return Ok(snapshot);
        }
    }

    if let Some(profile_id) = target_profile.map(|profile| profile.id.as_str()) {
        if let Some(mut snapshot) = usage_store.load_profile(profile_id)? {
            refresh_cache_metadata(&mut snapshot);
            snapshot.can_auto_switch = false;
            snapshot.auto_switch_reason = None;
            snapshot.message = Some(cache_fallback_message(
                remote_failure
                    .as_ref()
                    .map(|failure| failure.message.as_str()),
            ));
            snapshot.remote_error = remote_failure
                .as_ref()
                .and_then(|failure| failure.remote_error.clone());
            return Ok(snapshot);
        }
    }

    let snapshot = empty_snapshot(
        target_profile,
        UsageSource::Fallback,
        true,
        Some(unavailable_message(
            remote_failure
                .as_ref()
                .map(|failure| failure.message.as_str()),
        )),
        remote_failure
            .as_ref()
            .and_then(|failure| failure.remote_error.clone()),
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
        Some("Usage has not been fetched yet.".into()),
        None,
    ))
}

pub fn list_profile_snapshots(
    usage_store: &FileUsageStore,
    profiles: &[Profile],
) -> Result<Vec<UsageSnapshot>, RelayError> {
    let cache = usage_store.load_all()?;
    let mut by_profile = std::collections::HashMap::with_capacity(cache.len());
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
                Some("Usage has not been fetched yet.".into()),
                None,
            ));
        }
    }

    Ok(snapshots)
}

#[derive(Clone, Copy)]
enum Provider {
    Local,
    WebEnhanced,
    Fallback,
}

fn provider_order(mode: UsageSourceMode) -> [Provider; 3] {
    match mode {
        UsageSourceMode::Auto => [Provider::WebEnhanced, Provider::Local, Provider::Fallback],
        UsageSourceMode::Local => [Provider::Local, Provider::WebEnhanced, Provider::Fallback],
        UsageSourceMode::WebEnhanced => {
            [Provider::WebEnhanced, Provider::Local, Provider::Fallback]
        }
    }
}

fn maybe_note_fallback(
    snapshot: &mut UsageSnapshot,
    source_mode: UsageSourceMode,
    remote_failure: Option<&RemoteFailureContext>,
) {
    if let Some(failure) = remote_failure {
        snapshot.message = Some(match snapshot.source {
            UsageSource::Local => local_fallback_message(&failure.message),
            UsageSource::Fallback | UsageSource::WebEnhanced => {
                cache_fallback_message(Some(&failure.message))
            }
        });
        snapshot.remote_error = failure.remote_error.clone();
        return;
    }

    if matches!(
        source_mode,
        UsageSourceMode::Auto | UsageSourceMode::WebEnhanced
    ) && snapshot.source == UsageSource::Local
    {
        snapshot.message = Some(local_fallback_message_without_detail());
    }
}

async fn collect_fallback_snapshot(
    store: &SqliteStore,
    profile: Option<&Profile>,
) -> Result<Option<UsageSnapshot>, RelayError> {
    let Some(profile) = profile else {
        return Ok(None);
    };
    let mut events = store.list_failure_events(100).await?;
    events.retain(|event| event.profile_id.as_deref() == Some(profile.id.as_str()));
    let Some(event) = events.into_iter().max_by_key(|event| event.created_at) else {
        return Ok(None);
    };

    let mut snapshot = empty_snapshot(
        Some(profile),
        UsageSource::Fallback,
        is_usage_stale(event.created_at),
        Some("Usage may be unavailable due to a recent failure.".into()),
        None,
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
        snapshot.message = Some("Usage may be outdated.".into());
    }
}

fn local_fallback_message(detail: &str) -> String {
    format!(
        "{} {}",
        local_fallback_message_without_detail(),
        detail.trim()
    )
}

fn local_fallback_message_without_detail() -> String {
    "Using local usage because enhanced usage is unavailable.".into()
}

fn cache_fallback_message(detail: Option<&str>) -> String {
    match detail.map(str::trim).filter(|detail| !detail.is_empty()) {
        Some(detail) => format!("Usage may be outdated. {detail}"),
        None => "Usage may be outdated.".into(),
    }
}

fn unavailable_message(detail: Option<&str>) -> String {
    match detail.map(str::trim).filter(|detail| !detail.is_empty()) {
        Some(detail) => format!("Usage is currently unavailable. {detail}"),
        None => "Usage is currently unavailable.".into(),
    }
}

fn should_continue_to_next_provider(
    provider: Provider,
    mode: UsageSourceMode,
    snapshot: &UsageSnapshot,
) -> bool {
    matches!(mode, UsageSourceMode::Auto)
        && matches!(provider, Provider::WebEnhanced)
        && snapshot.source == UsageSource::WebEnhanced
        && (snapshot.stale || snapshot.confidence != UsageConfidence::High)
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
    remote_error: Option<UsageRemoteError>,
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
        session: unknown_usage_window(Some(300)),
        weekly: unknown_usage_window(Some(10080)),
        auto_switch_reason: None,
        can_auto_switch: false,
        message,
        remote_error,
    }
}

#[derive(Clone)]
struct RemoteFailureContext {
    message: String,
    remote_error: Option<UsageRemoteError>,
}

impl RemoteFailureContext {
    fn from_snapshot(snapshot: &UsageSnapshot) -> Self {
        Self {
            message: snapshot.message.clone().unwrap_or_default(),
            remote_error: snapshot.remote_error.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{build_active, list_profile_snapshots, load_profile_snapshot, refresh_profile};
    use crate::adapters::UsageProvider;
    use crate::adapters::codex::CodexAdapter;
    use crate::models::{
        AuthMode, FailureReason, RelayError, UsageConfidence, UsageRemoteError,
        UsageRemoteErrorKind, UsageSnapshot, UsageSource, UsageSourceMode, UsageStatus,
        UsageWindow,
    };
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

    struct FakeUsageProvider {
        local: Result<Option<UsageSnapshot>, RelayError>,
        remote: Result<Option<UsageSnapshot>, RelayError>,
    }

    #[async_trait::async_trait(?Send)]
    impl UsageProvider for FakeUsageProvider {
        fn collect_local_usage(
            &self,
            _target_profile: Option<&crate::models::Profile>,
            _active_profile: Option<&crate::models::Profile>,
        ) -> Result<Option<UsageSnapshot>, RelayError> {
            self.local.clone()
        }

        async fn collect_remote_usage(
            &self,
            _store: &SqliteStore,
            _target_profile: Option<&crate::models::Profile>,
        ) -> Result<Option<UsageSnapshot>, RelayError> {
            self.remote.clone()
        }
    }

    fn synthetic_snapshot(source: UsageSource, message: Option<&str>) -> UsageSnapshot {
        UsageSnapshot {
            profile_id: Some("p_test".into()),
            profile_name: Some("test".into()),
            source,
            confidence: UsageConfidence::High,
            stale: false,
            last_refreshed_at: Utc::now(),
            next_reset_at: None,
            session: UsageWindow {
                used_percent: Some(20.0),
                window_minutes: Some(300),
                reset_at: None,
                status: UsageStatus::Healthy,
                exact: true,
            },
            weekly: UsageWindow {
                used_percent: Some(30.0),
                window_minutes: Some(10080),
                reset_at: None,
                status: UsageStatus::Healthy,
                exact: true,
            },
            auto_switch_reason: None,
            can_auto_switch: false,
            message: message.map(str::to_string),
            remote_error: None,
        }
    }

    fn remote_error_snapshot(
        message: &str,
        kind: UsageRemoteErrorKind,
        http_status: Option<u16>,
    ) -> UsageSnapshot {
        let mut snapshot = synthetic_snapshot(UsageSource::WebEnhanced, Some(message));
        snapshot.stale = true;
        snapshot.confidence = UsageConfidence::Medium;
        snapshot.remote_error = Some(UsageRemoteError { kind, http_status });
        snapshot
    }

    #[tokio::test]
    async fn builds_local_usage_snapshot_from_profile_home() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).await.expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let inactive_home = temp.path().join("inactive");
        make_home(&inactive_home, "inactive", 0);
        let inactive_profile = profile("p_inactive", "inactive", &inactive_home);
        let provider = CodexAdapter::with_live_home(temp.path());

        let snapshot = refresh_profile(
            &store,
            &usage_store,
            &provider,
            Some(&inactive_profile),
            None,
            UsageSourceMode::Local,
            true,
        )
        .await
        .expect("usage");

        assert_eq!(snapshot.source, UsageSource::Local);
        assert_eq!(snapshot.profile_id.as_deref(), Some("p_inactive"));
        assert_eq!(snapshot.session.used_percent, Some(41.0));
        assert_eq!(snapshot.weekly.used_percent, Some(12.0));
    }

    #[tokio::test]
    async fn auto_mode_prefers_web_enhanced_usage_when_available() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).await.expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let home = temp.path().join("home");
        make_home(&home, "home", 0);
        let profile = profile("p_web", "web", &home);
        let provider = FakeUsageProvider {
            local: Ok(Some(synthetic_snapshot(
                UsageSource::Local,
                Some("local usage"),
            ))),
            remote: Ok(Some(synthetic_snapshot(UsageSource::WebEnhanced, None))),
        };

        let snapshot = refresh_profile(
            &store,
            &usage_store,
            &provider,
            Some(&profile),
            None,
            UsageSourceMode::Auto,
            true,
        )
        .await
        .expect("snapshot");

        assert_eq!(snapshot.source, UsageSource::WebEnhanced);
        assert_eq!(snapshot.message, None);
    }

    #[tokio::test]
    async fn auto_mode_notes_when_web_enhanced_falls_back_to_local() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).await.expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let home = temp.path().join("home");
        make_home(&home, "home", 0);
        let profile = profile("p_local", "local", &home);
        let provider = FakeUsageProvider {
            local: Ok(Some(synthetic_snapshot(UsageSource::Local, None))),
            remote: Ok(None),
        };

        let snapshot = refresh_profile(
            &store,
            &usage_store,
            &provider,
            Some(&profile),
            None,
            UsageSourceMode::Auto,
            true,
        )
        .await
        .expect("snapshot");

        assert_eq!(snapshot.source, UsageSource::Local);
        assert_eq!(
            snapshot.message.as_deref(),
            Some("Using local usage because enhanced usage is unavailable.")
        );
    }

    #[tokio::test]
    async fn auto_mode_skips_stale_web_snapshot_and_uses_local() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).await.expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let home = temp.path().join("home");
        make_home(&home, "home", 0);
        let profile = profile("p_local", "local", &home);
        let mut stale_remote = synthetic_snapshot(
            UsageSource::WebEnhanced,
            Some("Enhanced usage is currently unavailable."),
        );
        stale_remote.stale = true;
        stale_remote.confidence = UsageConfidence::Medium;
        let provider = FakeUsageProvider {
            local: Ok(Some(synthetic_snapshot(UsageSource::Local, None))),
            remote: Ok(Some(stale_remote)),
        };

        let snapshot = refresh_profile(
            &store,
            &usage_store,
            &provider,
            Some(&profile),
            None,
            UsageSourceMode::Auto,
            true,
        )
        .await
        .expect("snapshot");

        assert_eq!(snapshot.source, UsageSource::Local);
        assert_eq!(
            snapshot.message.as_deref(),
            Some("Using local usage because enhanced usage is unavailable.")
        );
    }

    #[tokio::test]
    async fn auto_mode_uses_local_snapshot_and_includes_remote_failure_detail() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).await.expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let home = temp.path().join("home");
        make_home(&home, "home", 0);
        let profile = profile("p_local", "local", &home);
        let provider = FakeUsageProvider {
            local: Ok(Some(synthetic_snapshot(UsageSource::Local, None))),
            remote: Ok(Some(remote_error_snapshot(
                "Codex connection failed: failed to fetch codex rate limits: GET https://chatgpt.com/backend-api/wham/usage failed: 402 Payment Required; content-type=application/json; body={\"detail\":{\"code\":\"deactivated_workspace\"}}",
                UsageRemoteErrorKind::Other,
                Some(402),
            ))),
        };

        let snapshot = refresh_profile(
            &store,
            &usage_store,
            &provider,
            Some(&profile),
            None,
            UsageSourceMode::Auto,
            true,
        )
        .await
        .expect("snapshot");

        assert_eq!(snapshot.source, UsageSource::Local);
        assert_eq!(
            snapshot.message.as_deref(),
            Some(
                "Using local usage because enhanced usage is unavailable. Codex connection failed: failed to fetch codex rate limits: GET https://chatgpt.com/backend-api/wham/usage failed: 402 Payment Required; content-type=application/json; body={\"detail\":{\"code\":\"deactivated_workspace\"}}"
            )
        );
        assert_eq!(
            snapshot.remote_error,
            Some(UsageRemoteError {
                kind: UsageRemoteErrorKind::Other,
                http_status: Some(402),
            })
        );
    }

    #[tokio::test]
    async fn web_enhanced_mode_falls_back_to_local_on_remote_failure() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).await.expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let home = temp.path().join("home");
        make_home(&home, "home", 0);
        let profile = profile("p_local", "local", &home);
        let provider = FakeUsageProvider {
            local: Ok(Some(synthetic_snapshot(UsageSource::Local, None))),
            remote: Ok(Some(remote_error_snapshot(
                "Codex connection failed: failed to fetch codex rate limits: GET https://chatgpt.com/backend-api/wham/usage failed: operation timed out",
                UsageRemoteErrorKind::Network,
                None,
            ))),
        };

        let snapshot = refresh_profile(
            &store,
            &usage_store,
            &provider,
            Some(&profile),
            None,
            UsageSourceMode::WebEnhanced,
            true,
        )
        .await
        .expect("snapshot");

        assert_eq!(snapshot.source, UsageSource::Local);
        assert_eq!(
            snapshot.message.as_deref(),
            Some(
                "Using local usage because enhanced usage is unavailable. Codex connection failed: failed to fetch codex rate limits: GET https://chatgpt.com/backend-api/wham/usage failed: operation timed out"
            )
        );
        assert_eq!(
            snapshot.remote_error,
            Some(UsageRemoteError {
                kind: UsageRemoteErrorKind::Network,
                http_status: None,
            })
        );
    }

    #[tokio::test]
    async fn remote_failure_uses_cached_snapshot_message_when_no_fresh_provider_succeeds() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).await.expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let home = temp.path().join("home");
        make_home(&home, "home", 0);
        let profile = profile("p_cached", "cached", &home);
        let mut cached = synthetic_snapshot(UsageSource::Local, Some("local usage"));
        cached.profile_id = Some(profile.id.clone());
        cached.profile_name = Some(profile.nickname.clone());
        usage_store.save_profile(&cached).expect("save cache");
        let provider = FakeUsageProvider {
            local: Ok(None),
            remote: Ok(Some(remote_error_snapshot(
                "Codex connection failed: failed to fetch codex rate limits: GET https://chatgpt.com/backend-api/wham/usage failed: dns error",
                UsageRemoteErrorKind::Network,
                None,
            ))),
        };

        let snapshot = refresh_profile(
            &store,
            &usage_store,
            &provider,
            Some(&profile),
            None,
            UsageSourceMode::Auto,
            true,
        )
        .await
        .expect("snapshot");

        assert_eq!(
            snapshot.message.as_deref(),
            Some(
                "Usage may be outdated. Codex connection failed: failed to fetch codex rate limits: GET https://chatgpt.com/backend-api/wham/usage failed: dns error"
            )
        );
        assert_eq!(
            snapshot.remote_error,
            Some(UsageRemoteError {
                kind: UsageRemoteErrorKind::Network,
                http_status: None,
            })
        );
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
        assert_eq!(
            snapshot.message.as_deref(),
            Some("Usage has not been fetched yet.")
        );
    }

    #[tokio::test]
    async fn falls_back_to_failure_events_without_auto_switch() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).await.expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let home = temp.path().join("fallback");
        fs::create_dir_all(&home).expect("home");
        fs::write(home.join("config.toml"), "model = \"fallback\"").expect("config");
        let profile = profile("p_fallback", "fallback", &home);
        let provider = CodexAdapter::with_live_home(temp.path());
        store
            .record_failure_event_for_test(
                &profile.id,
                FailureReason::SessionExhausted,
                "session exhausted",
            )
            .await
            .expect("failure event");

        let snapshot = refresh_profile(
            &store,
            &usage_store,
            &provider,
            Some(&profile),
            None,
            UsageSourceMode::Local,
            true,
        )
        .await
        .expect("snapshot");

        assert_eq!(snapshot.source, UsageSource::Fallback);
        assert_eq!(snapshot.session.status, UsageStatus::Exhausted);
        assert!(!snapshot.can_auto_switch);
    }

    #[tokio::test]
    async fn lists_snapshots_for_all_profiles() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).await.expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let active_home = temp.path().join("active");
        let inactive_home = temp.path().join("inactive");
        make_home(&active_home, "active", 0);
        make_home(&inactive_home, "inactive", 20);
        let active_profile = profile("p_active", "active", &active_home);
        let inactive_profile = profile("p_inactive", "inactive", &inactive_home);
        let provider = CodexAdapter::with_live_home(&active_home);

        let _ = build_active(
            &store,
            &usage_store,
            &provider,
            Some(&active_profile),
            UsageSourceMode::Local,
            true,
        )
        .await
        .expect("active usage");
        let _ = refresh_profile(
            &store,
            &usage_store,
            &provider,
            Some(&inactive_profile),
            Some(&active_profile),
            UsageSourceMode::Local,
            true,
        )
        .await
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

    #[tokio::test]
    async fn persists_unavailable_snapshot_for_profile_refresh() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).await.expect("store");
        let usage_store = FileUsageStore::new(temp.path().join("usage.json"));
        let home = temp.path().join("missing-usage");
        fs::create_dir_all(&home).expect("home");
        fs::write(home.join("config.toml"), "model = \"missing\"").expect("config");
        let profile = profile("p_missing", "missing", &home);
        let provider = CodexAdapter::with_live_home(temp.path());

        let refreshed = refresh_profile(
            &store,
            &usage_store,
            &provider,
            Some(&profile),
            None,
            UsageSourceMode::Local,
            true,
        )
        .await
        .expect("refresh");
        let cached = load_profile_snapshot(&usage_store, &profile).expect("cached");

        assert_eq!(refreshed.profile_id.as_deref(), Some("p_missing"));
        assert_eq!(
            refreshed.message.as_deref(),
            Some("Usage is currently unavailable.")
        );
        assert_eq!(cached.message, refreshed.message);
    }

    #[tokio::test]
    async fn inactive_profile_without_agent_home_does_not_read_active_live_usage() {
        let temp = tempdir().expect("tempdir");
        let relay_db = temp.path().join("relay.db");
        let store = SqliteStore::new(&relay_db).await.expect("store");
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
        let provider = CodexAdapter::with_live_home(&active_home);

        let snapshot = refresh_profile(
            &store,
            &usage_store,
            &provider,
            Some(&inactive_profile),
            Some(&active_profile),
            UsageSourceMode::Local,
            true,
        )
        .await
        .expect("refresh");

        assert_eq!(snapshot.profile_id.as_deref(), Some("p_inactive"));
        assert_eq!(snapshot.source, UsageSource::Fallback);
        assert_eq!(snapshot.session.used_percent, None);
        assert_eq!(
            snapshot.message.as_deref(),
            Some("Usage is currently unavailable.")
        );
    }
}
