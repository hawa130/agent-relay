use crate::models::FailureReason;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UsageSource {
    Local,
    Fallback,
    WebEnhanced,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UsageConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UsageStatus {
    Healthy,
    Warning,
    Exhausted,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageWindow {
    pub used_percent: Option<f64>,
    pub window_minutes: Option<i64>,
    pub reset_at: Option<DateTime<Utc>>,
    pub status: UsageStatus,
    pub exact: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSnapshot {
    pub profile_id: Option<String>,
    pub profile_name: Option<String>,
    pub source: UsageSource,
    pub confidence: UsageConfidence,
    pub stale: bool,
    pub last_refreshed_at: DateTime<Utc>,
    pub next_reset_at: Option<DateTime<Utc>>,
    pub session: UsageWindow,
    pub weekly: UsageWindow,
    pub auto_switch_reason: Option<FailureReason>,
    pub can_auto_switch: bool,
    pub message: Option<String>,
}
