use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentKind {
    Codex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileAccountState {
    Healthy,
    AccountUnavailable,
}

impl Default for ProfileAccountState {
    fn default() -> Self {
        Self::Healthy
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMode {
    ConfigFilesystem,
    EnvReference,
    KeychainReference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub nickname: String,
    pub agent: AgentKind,
    pub priority: i32,
    pub enabled: bool,
    #[serde(default)]
    pub account_state: ProfileAccountState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_error_http_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_state_updated_at: Option<String>,
    pub agent_home: Option<String>,
    pub config_path: Option<String>,
    pub auth_mode: AuthMode,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}
