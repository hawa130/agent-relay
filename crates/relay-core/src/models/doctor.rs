use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub platform: String,
    pub relay_home: String,
    pub relay_db_path: String,
    pub relay_log_path: String,
    pub primary_agent: crate::models::AgentKind,
    pub live_agent_home: String,
    pub agent_binary: Option<String>,
    pub default_agent_home: Option<String>,
    pub default_agent_home_exists: bool,
    pub managed_files: Vec<String>,
}
