use crate::models::{CodexOfficialProbeIdentity, ProfileProbeIdentity, RelayError};
use base64::Engine;
use chrono::Utc;
use std::fs;
use std::path::Path;

pub(crate) fn copy_login_auth(
    login_home: &Path,
    destination_home: &Path,
) -> Result<(), RelayError> {
    fs::create_dir_all(destination_home)?;
    super::copy_atomic(
        &login_home.join("auth.json"),
        &destination_home.join("auth.json"),
    )?;
    Ok(())
}

pub(crate) fn load_probe_identity_from_home(
    profile_id: &str,
    home: &Path,
) -> Result<ProfileProbeIdentity, RelayError> {
    let auth_path = home.join("auth.json");
    let contents = fs::read_to_string(&auth_path)?;
    let auth: CodexAuthFile =
        serde_json::from_str(&contents).map_err(|error| RelayError::Auth(error.to_string()))?;

    let tokens = auth
        .tokens
        .ok_or_else(|| RelayError::Auth("auth.json is missing tokens".into()))?;
    let account_id = tokens
        .account_id
        .clone()
        .ok_or_else(|| RelayError::Auth("auth.json is missing account_id".into()))?;
    let access_token = tokens
        .access_token
        .clone()
        .ok_or_else(|| RelayError::Auth("auth.json is missing access_token".into()))?;
    let id_token = tokens.id_token.clone();

    let now = Utc::now();
    Ok(ProfileProbeIdentity::codex_official(
        CodexOfficialProbeIdentity {
            profile_id: profile_id.into(),
            account_id,
            access_token,
            refresh_token: tokens.refresh_token,
            id_token: id_token.clone(),
            email: extract_email(id_token.as_deref()),
            plan_hint: None,
            created_at: now,
            updated_at: now,
        },
    ))
}

/// SAFETY: The JWT `id_token` is decoded without signature verification.
/// This is acceptable because the token is read from the local `auth.json` file
/// (written by the Codex CLI login flow) and is used only for display purposes
/// (extracting the user's email address).
pub(crate) fn extract_email(id_token: Option<&str>) -> Option<String> {
    let payload = id_token?.split('.').nth(1)?;
    let decoded = decode_base64url(payload)?;
    let claims: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    claims
        .get("email")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            claims
                .get("preferred_username")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
}

fn decode_base64url(value: &str) -> Option<Vec<u8>> {
    let mut normalized = value.replace('-', "+").replace('_', "/");
    let padding = (4 - normalized.len() % 4) % 4;
    normalized.extend(std::iter::repeat_n('=', padding));
    base64::engine::general_purpose::STANDARD
        .decode(normalized)
        .ok()
}

#[derive(serde::Deserialize)]
struct CodexAuthFile {
    tokens: Option<CodexAuthTokens>,
}

#[derive(serde::Deserialize)]
struct CodexAuthTokens {
    access_token: Option<String>,
    refresh_token: Option<String>,
    id_token: Option<String>,
    account_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use tempfile::tempdir;

    fn make_jwt(claims: &serde_json::Value) -> String {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256"}"#);
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_string(claims).unwrap());
        format!("{header}.{payload}.signature")
    }

    #[test]
    fn extract_email_from_valid_jwt() {
        let token = make_jwt(&serde_json::json!({"email": "user@example.com"}));
        let email = extract_email(Some(&token));
        assert_eq!(email.as_deref(), Some("user@example.com"));
    }

    #[test]
    fn extract_email_returns_none_for_none_input() {
        assert!(extract_email(None).is_none());
    }

    #[test]
    fn extract_email_returns_none_for_invalid_base64() {
        assert!(extract_email(Some("not.a.jwt")).is_none());
    }

    #[test]
    fn extract_email_falls_back_to_preferred_username() {
        let token = make_jwt(&serde_json::json!({"preferred_username": "alice"}));
        let email = extract_email(Some(&token));
        assert_eq!(email.as_deref(), Some("alice"));
    }

    #[test]
    fn extract_email_prefers_email_over_preferred_username() {
        let token = make_jwt(
            &serde_json::json!({"email": "real@example.com", "preferred_username": "fallback"}),
        );
        let email = extract_email(Some(&token));
        assert_eq!(email.as_deref(), Some("real@example.com"));
    }

    #[test]
    fn load_probe_identity_from_valid_auth_json() {
        let temp = tempdir().expect("tempdir");
        let home = temp.path();
        let auth_json = serde_json::json!({
            "tokens": {
                "access_token": "access-tok",
                "refresh_token": "refresh-tok",
                "id_token": make_jwt(&serde_json::json!({"email": "test@example.com"})),
                "account_id": "acct-123"
            }
        });
        std::fs::write(
            home.join("auth.json"),
            serde_json::to_string(&auth_json).unwrap(),
        )
        .expect("write auth.json");

        let identity = load_probe_identity_from_home("profile-1", home).expect("load identity");

        assert_eq!(identity.profile_id, "profile-1");
        assert_eq!(identity.principal_id.as_deref(), Some("acct-123"));
        assert_eq!(identity.display_name.as_deref(), Some("test@example.com"));
    }
}
