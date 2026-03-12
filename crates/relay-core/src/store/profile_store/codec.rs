use crate::models::{
    AgentKind, FailureEvent, FailureReason, ProbeProvider, Profile, ProfileAccountState,
    ProfileProbeIdentity, RelayError, SwitchHistoryEntry, SwitchOutcome,
};
use crate::store::entities::{failure_events, profile_probe_identities, profiles, switch_history};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::path::Path;

pub(super) fn profile_from_model(model: profiles::Model) -> Result<Profile, RelayError> {
    Ok(Profile {
        id: model.id,
        nickname: model.nickname,
        agent: parse_agent_kind(&model.agent)?,
        priority: model.priority,
        enabled: model.enabled,
        account_state: parse_profile_account_state(model.account_state.as_deref()),
        account_error_http_status: model
            .account_error_http_status
            .and_then(|value| u16::try_from(value).ok()),
        account_state_updated_at: model.account_state_updated_at,
        agent_home: model.agent_home,
        config_path: model.config_path,
        auth_mode: parse_auth_mode(&model.auth_mode),
        metadata: serde_json::from_str(&model.metadata).unwrap_or(Value::Null),
        created_at: model.created_at,
        updated_at: model.updated_at,
    })
}

pub(super) fn probe_identity_from_model(
    model: profile_probe_identities::Model,
) -> Result<ProfileProbeIdentity, RelayError> {
    Ok(ProfileProbeIdentity {
        profile_id: model.profile_id,
        provider: parse_probe_provider(&model.provider)?,
        principal_id: model.principal_id,
        display_name: model.display_name,
        credentials: serde_json::from_str(&model.credentials_json).unwrap_or(Value::Null),
        metadata: serde_json::from_str(&model.metadata_json).unwrap_or(Value::Null),
        created_at: model.created_at,
        updated_at: model.updated_at,
    })
}

pub(super) fn switch_history_from_model(
    model: switch_history::Model,
) -> Result<SwitchHistoryEntry, RelayError> {
    Ok(SwitchHistoryEntry {
        id: model.id,
        profile_id: model.profile_id,
        previous_profile_id: model.previous_profile_id,
        outcome: parse_outcome(&model.outcome),
        reason: model.reason,
        checkpoint_id: model.checkpoint_id,
        rollback_performed: model.rollback_performed,
        created_at: parse_timestamp(&model.created_at),
    })
}

pub(super) fn failure_event_from_model(
    model: failure_events::Model,
) -> Result<FailureEvent, RelayError> {
    Ok(FailureEvent {
        id: model.id,
        profile_id: model.profile_id,
        reason: parse_reason(&model.reason),
        message: model.message,
        cooldown_until: model
            .cooldown_until
            .as_deref()
            .map(parse_timestamp)
            .map(Some)
            .unwrap_or(None),
        resolved_at: model
            .resolved_at
            .as_deref()
            .map(parse_timestamp)
            .map(Some)
            .unwrap_or(None),
        created_at: parse_timestamp(&model.created_at),
    })
}

fn parse_timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

pub(super) fn stringify_auth_mode(mode: &crate::models::AuthMode) -> &'static str {
    match mode {
        crate::models::AuthMode::ConfigFilesystem => "config-filesystem",
        crate::models::AuthMode::EnvReference => "env-reference",
        crate::models::AuthMode::KeychainReference => "keychain-reference",
    }
}

fn parse_auth_mode(value: &str) -> crate::models::AuthMode {
    match value {
        "env-reference" => crate::models::AuthMode::EnvReference,
        "keychain-reference" => crate::models::AuthMode::KeychainReference,
        _ => crate::models::AuthMode::ConfigFilesystem,
    }
}

pub(super) fn stringify_probe_provider(provider: &ProbeProvider) -> &'static str {
    match provider {
        ProbeProvider::CodexOfficial => "codex-official",
    }
}

fn parse_probe_provider(value: &str) -> Result<ProbeProvider, RelayError> {
    match value {
        "codex-official" => Ok(ProbeProvider::CodexOfficial),
        other => Err(RelayError::Store(format!(
            "unsupported probe provider: {other}"
        ))),
    }
}

pub(super) fn stringify_agent_kind(kind: &AgentKind) -> &'static str {
    match kind {
        AgentKind::Codex => "codex",
    }
}

fn parse_agent_kind(value: &str) -> Result<AgentKind, RelayError> {
    match value {
        "codex" => Ok(AgentKind::Codex),
        other => Err(RelayError::Validation(format!(
            "unknown agent kind: {other}"
        ))),
    }
}

pub(super) fn stringify_reason(reason: &FailureReason) -> &'static str {
    match reason {
        FailureReason::SessionExhausted => "session-exhausted",
        FailureReason::WeeklyExhausted => "weekly-exhausted",
        FailureReason::AccountUnavailable => "account-unavailable",
        FailureReason::AuthInvalid => "auth-invalid",
        FailureReason::QuotaExhausted => "quota-exhausted",
        FailureReason::RateLimited => "rate-limited",
        FailureReason::CommandFailed => "command-failed",
        FailureReason::ValidationFailed => "validation-failed",
        FailureReason::Unknown => "unknown",
    }
}

fn parse_reason(value: &str) -> FailureReason {
    match value {
        "session-exhausted" => FailureReason::SessionExhausted,
        "weekly-exhausted" => FailureReason::WeeklyExhausted,
        "account-unavailable" => FailureReason::AccountUnavailable,
        "auth-invalid" => FailureReason::AuthInvalid,
        "quota-exhausted" => FailureReason::QuotaExhausted,
        "rate-limited" => FailureReason::RateLimited,
        "command-failed" => FailureReason::CommandFailed,
        "validation-failed" => FailureReason::ValidationFailed,
        _ => FailureReason::Unknown,
    }
}

pub(super) fn stringify_profile_account_state(state: &ProfileAccountState) -> &'static str {
    match state {
        ProfileAccountState::Healthy => "healthy",
        ProfileAccountState::AccountUnavailable => "account-unavailable",
    }
}

fn parse_profile_account_state(value: Option<&str>) -> ProfileAccountState {
    match value {
        Some("account-unavailable") => ProfileAccountState::AccountUnavailable,
        _ => ProfileAccountState::Healthy,
    }
}

pub(super) fn stringify_outcome(outcome: &SwitchOutcome) -> &'static str {
    match outcome {
        SwitchOutcome::NotRun => "not-run",
        SwitchOutcome::Success => "success",
        SwitchOutcome::Failed => "failed",
    }
}

fn parse_outcome(value: &str) -> SwitchOutcome {
    match value {
        "success" => SwitchOutcome::Success,
        "failed" => SwitchOutcome::Failed,
        _ => SwitchOutcome::NotRun,
    }
}

pub(super) fn slugify(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
        } else if !output.ends_with('_') {
            output.push('_');
        }
    }
    output.trim_matches('_').to_string()
}

pub(super) fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
