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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveredProfile {
    pub source_dir: String,
    pub profile: Profile,
    pub probe_identity_restored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedRecoveredProfile {
    pub source_dir: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileRecoveryReport {
    pub scanned_dirs: usize,
    pub recovered: Vec<RecoveredProfile>,
    pub skipped: Vec<SkippedRecoveredProfile>,
}
