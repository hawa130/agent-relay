use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub platform: String,
    pub relay_home: String,
    pub relay_db_path: String,
    pub relay_log_path: String,
    pub live_codex_home: String,
    pub codex_binary: Option<String>,
    pub codex_home: Option<String>,
    pub codex_home_exists: bool,
    pub managed_files: Vec<String>,
}
