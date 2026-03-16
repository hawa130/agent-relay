use crate::adapters::AgentAdapter;
use crate::models::{
    ActiveState, FailureReason, Profile, RelayError, SwitchOutcome, SwitchReport, SwitchTrigger,
};
use crate::platform::RelayPaths;
use crate::store::{FileLogStore, FileStateStore, SqliteStore, SwitchHistoryRecord};
use chrono::{Duration, Utc};

const SWITCH_FAILURE_REASONS: &[FailureReason] = &[
    FailureReason::AuthInvalid,
    FailureReason::QuotaExhausted,
    FailureReason::RateLimited,
    FailureReason::CommandFailed,
    FailureReason::ValidationFailed,
    FailureReason::Unknown,
];

pub async fn switch_to_profile(
    store: &SqliteStore,
    state_store: &FileStateStore,
    log_store: &FileLogStore,
    adapter: &dyn AgentAdapter,
    paths: &RelayPaths,
    profile: &Profile,
    trigger: SwitchTrigger,
) -> Result<SwitchReport, RelayError> {
    if !profile.enabled {
        return Err(RelayError::Conflict(format!(
            "profile is disabled: {}",
            profile.id
        )));
    }

    let current_state = state_store.load().await?;
    let previous_profile_id = current_state.active_profile_id.clone();

    log_store
        .append(
            "info".into(),
            "switch.start".into(),
            format!("switching to {}", profile.id),
        )
        .await?;

    match adapter
        .validate_profile(profile)
        .and_then(|_| adapter.activate(profile, &paths.snapshots_dir))
    {
        Ok(checkpoint) => {
            let switched_at = Utc::now();
            let next_state = ActiveState {
                active_profile_id: Some(profile.id.clone()),
                last_switch_at: Some(switched_at),
                last_switch_result: SwitchOutcome::Success,
                auto_switch_enabled: current_state.auto_switch_enabled,
            };
            state_store.save(&next_state).await?;
            if let Err(error) = store
                .record_switch(SwitchHistoryRecord {
                    profile_id: Some(profile.id.clone()),
                    previous_profile_id: previous_profile_id.clone(),
                    outcome: SwitchOutcome::Success,
                    reason: Some(stringify_trigger(trigger).into()),
                    checkpoint_id: Some(checkpoint.checkpoint_id.clone()),
                    rollback_performed: false,
                })
                .await
            {
                rollback_success_persistence(
                    adapter,
                    paths,
                    &checkpoint.checkpoint_id,
                    &current_state,
                    state_store,
                )
                .await?;
                return Err(error);
            }
            log_store
                .append(
                    "info".into(),
                    "switch.success".into(),
                    format!("active={}", profile.id),
                )
                .await?;
            if let Err(error) = store
                .resolve_failure_events(&profile.id, SWITCH_FAILURE_REASONS)
                .await
            {
                log_store
                    .append(
                        "warn".into(),
                        "switch.resolve_failure_events_failed".into(),
                        error.to_string(),
                    )
                    .await?;
            }

            Ok(SwitchReport {
                profile_id: profile.id.clone(),
                previous_profile_id,
                checkpoint_id: checkpoint.checkpoint_id,
                rollback_performed: false,
                switched_at,
                message: "switch completed".into(),
            })
        }
        Err(error) => {
            let settings = store.get_settings().await?;
            let now = Utc::now();
            let next_state = ActiveState {
                active_profile_id: previous_profile_id.clone(),
                last_switch_at: Some(now),
                last_switch_result: SwitchOutcome::Failed,
                auto_switch_enabled: current_state.auto_switch_enabled,
            };
            state_store.save(&next_state).await?;
            if let Err(persist_error) = store
                .record_switch_failure(
                    SwitchHistoryRecord {
                        profile_id: Some(profile.id.clone()),
                        previous_profile_id,
                        outcome: SwitchOutcome::Failed,
                        reason: Some(error.to_string()),
                        checkpoint_id: None,
                        rollback_performed: true,
                    },
                    classify_failure_reason(&error),
                    error.to_string(),
                    Some(now + Duration::seconds(settings.cooldown_seconds)),
                )
                .await
            {
                state_store.save(&current_state).await?;
                return Err(persist_error);
            }
            log_store
                .append("error".into(), "switch.failed".into(), error.to_string())
                .await?;
            Err(error)
        }
    }
}

fn stringify_trigger(trigger: SwitchTrigger) -> &'static str {
    match trigger {
        SwitchTrigger::Manual => "manual",
        SwitchTrigger::Auto => "auto",
    }
}

async fn rollback_success_persistence(
    adapter: &dyn AgentAdapter,
    paths: &RelayPaths,
    checkpoint_id: &str,
    previous_state: &ActiveState,
    state_store: &FileStateStore,
) -> Result<(), RelayError> {
    adapter.rollback_checkpoint(&paths.snapshots_dir, checkpoint_id)?;
    state_store.save(previous_state).await?;
    Ok(())
}

fn classify_failure_reason(error: &RelayError) -> FailureReason {
    match error {
        RelayError::Auth(_) => FailureReason::AuthInvalid,
        RelayError::QuotaExhausted(_) => FailureReason::QuotaExhausted,
        RelayError::RateLimited(_) => FailureReason::RateLimited,
        RelayError::ExternalCommand(_) => FailureReason::CommandFailed,
        RelayError::Validation(_) => FailureReason::ValidationFailed,
        _ => FailureReason::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::RelayPaths;
    use crate::store::{FileLogStore, FileStateStore};
    use chrono::Utc;
    use sea_orm::{ConnectionTrait, Database};
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn restores_live_files_and_state_when_switch_persistence_fails() {
        let temp = tempdir().expect("tempdir");
        let relay_root = temp.path().join("relay");
        let live_home = temp.path().join("live");
        let profile_home = temp.path().join("profile");
        fs::create_dir_all(&live_home).expect("live");
        fs::create_dir_all(&profile_home).expect("profile");
        fs::write(live_home.join("config.toml"), "model = 'old'").expect("live config");
        fs::write(live_home.join("auth.json"), "{\"token\":\"old\"}").expect("live auth");
        fs::write(profile_home.join("config.toml"), "model = 'new'").expect("profile config");
        fs::write(profile_home.join("auth.json"), "{\"token\":\"new\"}").expect("profile auth");

        let paths = RelayPaths::from_root(relay_root);
        paths.ensure_layout().expect("layout");
        let store = SqliteStore::new(&paths.db_path).await.expect("store");
        let state_store = FileStateStore::new(&paths.state_path);
        let log_store = FileLogStore::new(&paths.log_file);
        let adapter = crate::adapters::CodexAdapter::with_live_home(&live_home);

        let previous_state = ActiveState::default();
        state_store.save(&previous_state).await.expect("save state");

        let breaker = Database::connect(format!(
            "sqlite://{}?mode=rwc",
            paths.db_path.to_string_lossy()
        ))
        .await
        .expect("open breaker db");
        breaker
            .execute_unprepared("DROP TABLE switch_history")
            .await
            .expect("drop switch_history");
        drop(breaker);

        let profile = Profile {
            id: "p_new".into(),
            nickname: "New".into(),
            agent: crate::models::AgentKind::Codex,
            priority: 10,
            enabled: true,
            account_state: crate::models::ProfileAccountState::Healthy,
            account_error_http_status: None,
            account_state_updated_at: None,
            agent_home: Some(profile_home.to_string_lossy().into_owned()),
            config_path: Some(
                profile_home
                    .join("config.toml")
                    .to_string_lossy()
                    .into_owned(),
            ),
            auth_mode: crate::models::AuthMode::ConfigFilesystem,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let error = switch_to_profile(
            &store,
            &state_store,
            &log_store,
            &adapter,
            &paths,
            &profile,
            SwitchTrigger::Manual,
        )
        .await
        .expect_err("switch should fail when switch history persistence breaks");
        assert!(matches!(error, RelayError::Store(_)));

        let restored_state = state_store.load().await.expect("load state");
        assert_eq!(
            restored_state.active_profile_id,
            previous_state.active_profile_id
        );
        assert_eq!(
            fs::read_to_string(live_home.join("config.toml")).expect("live config"),
            "model = 'old'"
        );
        assert_eq!(
            fs::read_to_string(live_home.join("auth.json")).expect("live auth"),
            "{\"token\":\"old\"}"
        );
    }
}
