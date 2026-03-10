use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub auto_switch_enabled: bool,
    pub cooldown_seconds: i64,
    pub refresh_interval_seconds: i64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_switch_enabled: false,
            cooldown_seconds: 600,
            refresh_interval_seconds: 60,
        }
    }
}
