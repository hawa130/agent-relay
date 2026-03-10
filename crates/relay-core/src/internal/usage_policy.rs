use crate::models::{FailureReason, UsageConfidence, UsageSnapshot, UsageStatus, UsageWindow};
use chrono::{DateTime, Duration, Utc};

const SNAPSHOT_STALE_AFTER_MINUTES: i64 = 15;

pub(crate) fn build_usage_window(
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

pub(crate) fn unknown_usage_window(window_minutes: Option<i64>) -> UsageWindow {
    build_usage_window(None, window_minutes, None, false)
}

pub(crate) fn next_reset_at(session: &UsageWindow, weekly: &UsageWindow) -> Option<DateTime<Utc>> {
    match (session.reset_at, weekly.reset_at) {
        (Some(session_reset), Some(weekly_reset)) => Some(session_reset.min(weekly_reset)),
        (Some(session_reset), None) => Some(session_reset),
        (None, Some(weekly_reset)) => Some(weekly_reset),
        (None, None) => None,
    }
}

pub(crate) fn apply_auto_switch_policy(snapshot: &mut UsageSnapshot) {
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

pub(crate) fn is_usage_stale(timestamp: DateTime<Utc>) -> bool {
    Utc::now() - timestamp > Duration::minutes(SNAPSHOT_STALE_AFTER_MINUTES)
}
