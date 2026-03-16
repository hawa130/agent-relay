use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProbeProvider {
    CodexOfficial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileProbeIdentity {
    pub profile_id: String,
    pub provider: ProbeProvider,
    pub principal_id: Option<String>,
    pub display_name: Option<String>,
    pub credentials: Value,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CodexOfficialProbeIdentity {
    pub profile_id: String,
    pub account_id: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub email: Option<String>,
    pub plan_hint: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProfileProbeIdentity {
    pub fn codex_official(input: CodexOfficialProbeIdentity) -> Self {
        Self {
            profile_id: input.profile_id,
            provider: ProbeProvider::CodexOfficial,
            principal_id: Some(input.account_id.clone()),
            display_name: input.email.clone(),
            credentials: json!({
                "account_id": input.account_id,
                "access_token": input.access_token,
                "refresh_token": input.refresh_token,
                "id_token": input.id_token,
            }),
            metadata: json!({
                "email": input.email,
                "plan_hint": input.plan_hint,
            }),
            created_at: input.created_at,
            updated_at: input.updated_at,
        }
    }

    pub fn account_id(&self) -> Option<&str> {
        json_get_string(&self.credentials, "account_id").or(self.principal_id.as_deref())
    }

    pub fn access_token(&self) -> Option<&str> {
        json_get_string(&self.credentials, "access_token")
    }

    pub fn refresh_token(&self) -> Option<&str> {
        json_get_string(&self.credentials, "refresh_token")
    }

    pub fn id_token(&self) -> Option<&str> {
        json_get_string(&self.credentials, "id_token")
    }

    pub fn email(&self) -> Option<&str> {
        json_get_string(&self.metadata, "email").or(self.display_name.as_deref())
    }

    pub fn plan_hint(&self) -> Option<&str> {
        json_get_string(&self.metadata, "plan_hint")
    }
}

fn json_get_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}
