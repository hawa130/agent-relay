use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    pub created_at: String,
    pub updated_at: String,
}

impl ProfileProbeIdentity {
    pub fn codex_official(
        profile_id: String,
        account_id: String,
        access_token: String,
        refresh_token: Option<String>,
        id_token: Option<String>,
        email: Option<String>,
        plan_hint: Option<String>,
        created_at: String,
        updated_at: String,
    ) -> Self {
        Self {
            profile_id,
            provider: ProbeProvider::CodexOfficial,
            principal_id: Some(account_id.clone()),
            display_name: email.clone(),
            credentials: json!({
                "account_id": account_id,
                "access_token": access_token,
                "refresh_token": refresh_token,
                "id_token": id_token,
            }),
            metadata: json!({
                "email": email,
                "plan_hint": plan_hint,
            }),
            created_at,
            updated_at,
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
