use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentKind {
    Codex,
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
    #[serde(alias = "codex_home")]
    pub agent_home: Option<String>,
    pub config_path: Option<String>,
    pub auth_mode: AuthMode,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}
