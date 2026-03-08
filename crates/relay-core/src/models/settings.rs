use crate::models::UsageSourceMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub auto_switch_enabled: bool,
    pub cooldown_seconds: i64,
    pub usage_source_mode: UsageSourceMode,
    pub menu_open_refresh_stale_after_seconds: i64,
    pub usage_background_refresh_enabled: bool,
    pub usage_background_refresh_interval_seconds: i64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_switch_enabled: false,
            cooldown_seconds: 600,
            usage_source_mode: UsageSourceMode::Auto,
            menu_open_refresh_stale_after_seconds: 10,
            usage_background_refresh_enabled: true,
            usage_background_refresh_interval_seconds: 120,
        }
    }
}
