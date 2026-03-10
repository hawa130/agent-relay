use crate::models::SwitchOutcome;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwitchTrigger {
    Manual,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchHistoryEntry {
    pub id: String,
    pub profile_id: Option<String>,
    pub previous_profile_id: Option<String>,
    pub outcome: SwitchOutcome,
    pub reason: Option<String>,
    pub checkpoint_id: Option<String>,
    pub rollback_performed: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchReport {
    pub profile_id: String,
    pub previous_profile_id: Option<String>,
    pub checkpoint_id: String,
    pub rollback_performed: bool,
    pub switched_at: DateTime<Utc>,
    pub message: String,
}
