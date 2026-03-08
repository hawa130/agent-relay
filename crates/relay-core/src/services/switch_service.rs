use crate::adapters::{AgentAdapter, CodexAdapter};
use crate::models::{ActiveState, FailureReason, Profile, RelayError, SwitchOutcome, SwitchReport};
use crate::platform::RelayPaths;
use crate::store::{FileLogStore, FileStateStore, SqliteStore, SwitchHistoryRecord};
use chrono::{Duration, Utc};

pub fn switch_to_profile(
    store: &SqliteStore,
    state_store: &FileStateStore,
    log_store: &FileLogStore,
    adapter: &CodexAdapter,
    paths: &RelayPaths,
    profile: &Profile,
) -> Result<SwitchReport, RelayError> {
    if !profile.enabled {
        return Err(RelayError::Conflict(format!(
            "profile is disabled: {}",
            profile.id
        )));
    }

    let current_state = state_store.load()?;
    let previous_profile_id = current_state.active_profile_id.clone();

    log_store.append(
        "info",
        "switch.start",
        format!("switching to {}", profile.id),
    )?;

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
                last_error: None,
            };
            state_store.save(&next_state)?;
            store.record_switch(SwitchHistoryRecord {
                profile_id: Some(profile.id.clone()),
                previous_profile_id: previous_profile_id.clone(),
                outcome: SwitchOutcome::Success,
                reason: Some("manual".into()),
                checkpoint_id: Some(checkpoint.checkpoint_id.clone()),
                rollback_performed: false,
            })?;
            log_store.append("info", "switch.success", format!("active={}", profile.id))?;

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
            let settings = store.get_settings()?;
            let now = Utc::now();
            let next_state = ActiveState {
                active_profile_id: previous_profile_id.clone(),
                last_switch_at: Some(now),
                last_switch_result: SwitchOutcome::Failed,
                auto_switch_enabled: current_state.auto_switch_enabled,
                last_error: Some(error.to_string()),
            };
            state_store.save(&next_state)?;
            store.record_switch(SwitchHistoryRecord {
                profile_id: Some(profile.id.clone()),
                previous_profile_id,
                outcome: SwitchOutcome::Failed,
                reason: Some(error.to_string()),
                checkpoint_id: None,
                rollback_performed: true,
            })?;
            store.record_failure_event(
                Some(&profile.id),
                classify_failure_reason(&error),
                error.to_string(),
                Some(now + Duration::seconds(settings.cooldown_seconds)),
            )?;
            log_store.append("error", "switch.failed", error.to_string())?;
            Err(error)
        }
    }
}

fn classify_failure_reason(error: &RelayError) -> FailureReason {
    let message = error.to_string().to_lowercase();
    if message.contains("auth") {
        FailureReason::AuthInvalid
    } else if message.contains("quota") {
        FailureReason::QuotaExhausted
    } else if message.contains("rate") {
        FailureReason::RateLimited
    } else if matches!(error, RelayError::ExternalCommand(_)) {
        FailureReason::CommandFailed
    } else {
        FailureReason::ValidationFailed
    }
}
