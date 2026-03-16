use crate::models::{
    FailureEvent, FailureReason, Profile, ProfileAccountState, RelayError, UsageConfidence,
    UsageSnapshot,
};
use chrono::Utc;

pub fn select_next_profile(
    profiles: &[Profile],
    usage_snapshots: &[UsageSnapshot],
    active_profile_id: Option<&str>,
    failure_events: &[FailureEvent],
) -> Result<Profile, RelayError> {
    let eligible = profiles
        .iter()
        .filter(|profile| profile.enabled)
        .filter(|profile| profile.account_state == ProfileAccountState::Healthy)
        .filter(|profile| !is_in_cooldown(profile, failure_events))
        .collect::<Vec<_>>();

    if eligible.is_empty() {
        return Err(RelayError::NotFound("no eligible profile available".into()));
    }

    let healthy = eligible
        .iter()
        .copied()
        .filter(|profile| is_healthy_candidate(profile, usage_snapshots))
        .collect::<Vec<_>>();

    if healthy.is_empty() {
        return Err(RelayError::Conflict(
            "all enabled profiles are exhausted or unavailable for auto-switch".into(),
        ));
    }

    if let Some(active_profile_id) = active_profile_id {
        if healthy.len() == 1 && healthy[0].id == active_profile_id {
            return Err(RelayError::NotFound("no next profile available".into()));
        }

        if let Some(index) = healthy
            .iter()
            .position(|profile| profile.id == active_profile_id)
        {
            return Ok(healthy[(index + 1) % healthy.len()].clone());
        }
    }

    Ok(healthy[0].clone())
}

fn is_in_cooldown(profile: &Profile, events: &[FailureEvent]) -> bool {
    let now = Utc::now();
    events.iter().any(|event| {
        event.profile_id.as_deref() == Some(profile.id.as_str())
            && event.resolved_at.is_none()
            && event.cooldown_until.is_some_and(|until| until > now)
    })
}

fn is_healthy_candidate(profile: &Profile, snapshots: &[UsageSnapshot]) -> bool {
    snapshots
        .iter()
        .find(|snapshot| snapshot.profile_id.as_deref() == Some(profile.id.as_str()))
        .is_some_and(|snapshot| {
            !snapshot.stale
                && snapshot.confidence == UsageConfidence::High
                && !matches!(
                    snapshot.session.status,
                    crate::models::UsageStatus::Exhausted
                )
                && !matches!(
                    snapshot.weekly.status,
                    crate::models::UsageStatus::Exhausted
                )
        })
}

pub fn auto_switch_reason(snapshot: &UsageSnapshot) -> Option<FailureReason> {
    if snapshot.stale || snapshot.confidence != UsageConfidence::High {
        return None;
    }

    if matches!(
        snapshot.session.status,
        crate::models::UsageStatus::Exhausted
    ) {
        return Some(FailureReason::SessionExhausted);
    }

    if matches!(
        snapshot.weekly.status,
        crate::models::UsageStatus::Exhausted
    ) {
        return Some(FailureReason::WeeklyExhausted);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{auto_switch_reason, select_next_profile};
    use crate::models::{
        AgentKind, AuthMode, FailureEvent, FailureReason, Profile, UsageConfidence, UsageSnapshot,
        UsageSource, UsageStatus, UsageWindow,
    };
    use chrono::{DateTime, Duration, Utc};
    use serde_json::json;

    #[test]
    fn select_next_profile_skips_exhausted_candidates() {
        let profiles = vec![profile("p1", "alpha", 10), profile("p2", "beta", 20)];
        let snapshots = vec![
            exhausted_snapshot("p1"),
            healthy_snapshot("p2", UsageConfidence::High),
        ];

        let selected =
            select_next_profile(&profiles, &snapshots, Some("p1"), &[]).expect("next profile");

        assert_eq!(selected.id, "p2");
    }

    #[test]
    fn select_next_profile_reports_no_next_when_only_active_candidate_is_healthy() {
        let profiles = vec![
            profile("p1", "alpha", 10),
            profile("p2", "beta", 20),
            profile("p3", "gamma", 30),
            profile("p4", "delta", 40),
        ];
        let mut stale = healthy_snapshot("p2", UsageConfidence::High);
        stale.stale = true;
        let weekly_exhausted = UsageSnapshot {
            weekly: UsageWindow {
                status: UsageStatus::Exhausted,
                ..usage_window(UsageStatus::Healthy)
            },
            ..healthy_snapshot("p3", UsageConfidence::High)
        };
        let error = select_next_profile(
            &profiles,
            &[
                healthy_snapshot("p1", UsageConfidence::High),
                stale,
                weekly_exhausted,
                healthy_snapshot("p4", UsageConfidence::Medium),
            ],
            Some("p1"),
            &[],
        )
        .expect_err("expected no next profile");

        match error {
            crate::models::RelayError::NotFound(message) => {
                assert_eq!(message, "no next profile available");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn select_next_profile_returns_conflict_when_all_candidates_are_unhealthy() {
        let profiles = vec![profile("p1", "alpha", 10), profile("p2", "beta", 20)];
        let error = select_next_profile(
            &profiles,
            &[
                exhausted_snapshot("p1"),
                healthy_snapshot("p2", UsageConfidence::Medium),
            ],
            Some("p1"),
            &[],
        )
        .expect_err("expected conflict");

        match error {
            crate::models::RelayError::Conflict(message) => {
                assert_eq!(
                    message,
                    "all enabled profiles are exhausted or unavailable for auto-switch"
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn select_next_profile_skips_account_unavailable_candidates() {
        let mut unavailable = profile("p1", "alpha", 10);
        unavailable.account_state = crate::models::ProfileAccountState::AccountUnavailable;
        let healthy = profile("p2", "beta", 20);
        let selected = select_next_profile(
            &[unavailable, healthy.clone()],
            &[
                healthy_snapshot("p1", UsageConfidence::High),
                healthy_snapshot("p2", UsageConfidence::High),
            ],
            Some("p1"),
            &[],
        )
        .expect("next profile");

        assert_eq!(selected.id, healthy.id);
    }

    #[test]
    fn select_next_profile_respects_cooldown_after_usage_filtering() {
        let profiles = vec![profile("p1", "alpha", 10), profile("p2", "beta", 20)];
        let events = vec![FailureEvent {
            id: "evt-1".into(),
            profile_id: Some("p2".into()),
            reason: FailureReason::CommandFailed,
            message: "cooldown".into(),
            cooldown_until: Some(Utc::now() + Duration::minutes(5)),
            resolved_at: None,
            created_at: Utc::now(),
        }];

        let error = select_next_profile(
            &profiles,
            &[
                exhausted_snapshot("p1"),
                healthy_snapshot("p2", UsageConfidence::High),
            ],
            Some("p1"),
            &events,
        )
        .expect_err("expected conflict");

        match error {
            crate::models::RelayError::Conflict(message) => {
                assert_eq!(
                    message,
                    "all enabled profiles are exhausted or unavailable for auto-switch"
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn auto_switch_reason_only_marks_high_confidence_non_stale_exhaustion() {
        let snapshot = exhausted_snapshot("p1");
        assert_eq!(
            auto_switch_reason(&snapshot),
            Some(FailureReason::SessionExhausted)
        );

        let stale = UsageSnapshot {
            stale: true,
            ..exhausted_snapshot("p1")
        };
        assert_eq!(auto_switch_reason(&stale), None);
    }

    fn profile(id: &str, nickname: &str, priority: i32) -> Profile {
        Profile {
            id: id.into(),
            nickname: nickname.into(),
            agent: AgentKind::Codex,
            priority,
            enabled: true,
            account_state: crate::models::ProfileAccountState::Healthy,
            account_error_http_status: None,
            account_state_updated_at: None,
            agent_home: Some(format!("/tmp/{id}")),
            config_path: Some(format!("/tmp/{id}/config.toml")),
            auth_mode: AuthMode::ConfigFilesystem,
            metadata: json!({}),
            created_at: "2026-03-09T00:00:00Z".parse::<DateTime<Utc>>().unwrap(),
            updated_at: "2026-03-09T00:00:00Z".parse::<DateTime<Utc>>().unwrap(),
        }
    }

    fn healthy_snapshot(profile_id: &str, confidence: UsageConfidence) -> UsageSnapshot {
        UsageSnapshot {
            profile_id: Some(profile_id.into()),
            profile_name: Some(profile_id.into()),
            source: UsageSource::Local,
            confidence,
            stale: false,
            last_refreshed_at: Utc::now(),
            next_reset_at: None,
            session: usage_window(UsageStatus::Healthy),
            weekly: usage_window(UsageStatus::Healthy),
            auto_switch_reason: None,
            can_auto_switch: false,
            message: Some("ok".into()),
            remote_error: None,
        }
    }

    fn exhausted_snapshot(profile_id: &str) -> UsageSnapshot {
        UsageSnapshot {
            session: usage_window(UsageStatus::Exhausted),
            auto_switch_reason: Some(FailureReason::SessionExhausted),
            can_auto_switch: true,
            ..healthy_snapshot(profile_id, UsageConfidence::High)
        }
    }

    fn usage_window(status: UsageStatus) -> UsageWindow {
        UsageWindow {
            used_percent: Some(95.0),
            window_minutes: Some(300),
            reset_at: None,
            status,
            exact: true,
        }
    }
}
