use crate::models::{ActiveState, AppSettings, FailureEvent, Profile, UsageSnapshot};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatusReport {
    pub relay_home: String,
    pub live_agent_home: String,
    pub profile_count: usize,
    pub active_state: ActiveState,
    pub settings: AppSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDetail {
    pub profile: Profile,
    pub is_active: bool,
    pub usage: Option<UsageSnapshot>,
    pub last_failure_event: Option<FailureEvent>,
    pub switch_eligible: bool,
    pub switch_ineligibility_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileListItem {
    pub profile: Profile,
    pub is_active: bool,
    pub usage_summary: Option<UsageSnapshot>,
}
