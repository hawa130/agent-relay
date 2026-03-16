use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SwitchOutcome {
    NotRun,
    Success,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveState {
    pub active_profile_id: Option<String>,
    pub last_switch_at: Option<DateTime<Utc>>,
    pub last_switch_result: SwitchOutcome,
    pub auto_switch_enabled: bool,
}

impl Default for ActiveState {
    fn default() -> Self {
        Self {
            active_profile_id: None,
            last_switch_at: None,
            last_switch_result: SwitchOutcome::NotRun,
            auto_switch_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchCheckpoint {
    pub checkpoint_id: String,
    pub backup_paths: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReport {
    pub relay_home: String,
    pub live_agent_home: String,
    pub profile_count: usize,
    pub active_state: ActiveState,
    pub settings: crate::models::AppSettings,
}
