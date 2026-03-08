use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProbeProvider {
    CodexOfficial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileProbeIdentity {
    pub profile_id: String,
    pub provider: ProbeProvider,
    pub account_id: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub email: Option<String>,
    pub plan_hint: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
