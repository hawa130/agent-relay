use crate::models::UsageSourceMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodexSettings {
    pub usage_source_mode: UsageSourceMode,
}

impl Default for CodexSettings {
    fn default() -> Self {
        Self {
            usage_source_mode: UsageSourceMode::Auto,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CodexSettingsUpdateRequest {
    pub usage_source_mode: Option<UsageSourceMode>,
}
