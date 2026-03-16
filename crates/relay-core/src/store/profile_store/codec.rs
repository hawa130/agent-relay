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
        created_at: parse_timestamp(&model.created_at)?,
        updated_at: parse_timestamp(&model.updated_at)?,
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
        created_at: parse_timestamp(&model.created_at)?,
        updated_at: parse_timestamp(&model.updated_at)?,
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
        created_at: parse_timestamp(&model.created_at)?,
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
            .transpose()?,
        resolved_at: model
            .resolved_at
            .as_deref()
            .map(parse_timestamp)
            .transpose()?,
        created_at: parse_timestamp(&model.created_at)?,
    })
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, RelayError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| RelayError::Store(format!("invalid timestamp {value:?}: {error}")))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic_cases() {
        assert_eq!(slugify("Hello World"), "hello_world");
        assert_eq!(slugify("my-profile-123"), "my_profile_123");
        assert_eq!(slugify("  leading spaces  "), "leading_spaces");
        assert_eq!(slugify("UPPER"), "upper");
        assert_eq!(slugify("a__b"), "a_b"); // consecutive non-alnum chars collapse
    }

    #[test]
    fn slugify_special_characters() {
        assert_eq!(slugify("foo@bar.com"), "foo_bar_com");
        assert_eq!(slugify("!!!test!!!"), "test");
    }

    #[test]
    fn parse_timestamp_valid_rfc3339() {
        use chrono::Datelike;
        let result = parse_timestamp("2024-01-15T10:30:00Z");
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.year(), 2024);
    }

    #[test]
    fn parse_timestamp_invalid_input() {
        let result = parse_timestamp("not-a-date");
        assert!(result.is_err());
        match result.unwrap_err() {
            RelayError::Store(msg) => assert!(msg.contains("invalid timestamp")),
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn failure_reason_round_trip() {
        let variants = vec![
            FailureReason::SessionExhausted,
            FailureReason::WeeklyExhausted,
            FailureReason::AccountUnavailable,
            FailureReason::AuthInvalid,
            FailureReason::QuotaExhausted,
            FailureReason::RateLimited,
            FailureReason::CommandFailed,
            FailureReason::ValidationFailed,
            FailureReason::Unknown,
        ];
        for variant in variants {
            let stringified = stringify_reason(&variant);
            let parsed = parse_reason(stringified);
            assert_eq!(
                std::mem::discriminant(&variant),
                std::mem::discriminant(&parsed),
                "round-trip failed for {stringified}"
            );
        }
    }

    #[test]
    fn parse_reason_unknown_value() {
        assert!(matches!(parse_reason("garbage"), FailureReason::Unknown));
    }

    #[test]
    fn auth_mode_round_trip() {
        use crate::models::AuthMode;
        let modes = vec![
            AuthMode::ConfigFilesystem,
            AuthMode::EnvReference,
            AuthMode::KeychainReference,
        ];
        for mode in modes {
            let stringified = stringify_auth_mode(&mode);
            let parsed = parse_auth_mode(stringified);
            assert_eq!(
                std::mem::discriminant(&mode),
                std::mem::discriminant(&parsed),
                "round-trip failed for {stringified}"
            );
        }
    }

    #[test]
    fn parse_auth_mode_defaults_to_config_filesystem() {
        let parsed = parse_auth_mode("unknown-mode");
        assert!(matches!(parsed, crate::models::AuthMode::ConfigFilesystem));
    }
}
