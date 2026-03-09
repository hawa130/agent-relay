use crate::adapters::AgentAdapter;
use crate::models::{CodexLinkResult, Profile, ProfileProbeIdentity, RelayError};
use crate::platform::find_binary;
use crate::store::{AddProfileRecord, SqliteStore};
use base64::Engine;
use chrono::Utc;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const CODEX_LOGIN_TIMEOUT_SECS: u64 = 300;
const CODEX_LOGIN_POLL_MILLIS: u64 = 250;

pub fn login_new_profile(
    store: &SqliteStore,
    adapter: &dyn AgentAdapter,
    profiles_dir: &Path,
    nickname: Option<String>,
    priority: i32,
) -> Result<CodexLinkResult, RelayError> {
    let login_home = prepare_login_home()?;
    run_codex_login(&login_home)?;
    let identity = load_probe_identity_from_home("pending", &login_home)?;

    let snapshot_dir = profiles_dir.join(format!("login_{}", Utc::now().timestamp_millis()));
    adapter.import_live_profile(&snapshot_dir)?;
    copy_login_auth(&login_home, &snapshot_dir)?;

    let profile = store.add_profile(AddProfileRecord {
        agent: crate::models::AgentKind::Codex,
        nickname: nickname.unwrap_or_else(|| {
            identity
                .email()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("Codex {}", Utc::now().format("%Y%m%d-%H%M%S")))
        }),
        priority,
        config_path: Some(snapshot_dir.join("config.toml")),
        agent_home: Some(snapshot_dir),
        auth_mode: crate::models::AuthMode::ConfigFilesystem,
    })?;

    let probe_identity = store.upsert_probe_identity(&ProfileProbeIdentity {
        profile_id: profile.id.clone(),
        ..identity
    })?;

    Ok(CodexLinkResult {
        profile,
        probe_identity,
        activated: false,
    })
}

pub fn relink_profile(
    store: &SqliteStore,
    adapter: &dyn AgentAdapter,
    profile: &Profile,
) -> Result<ProfileProbeIdentity, RelayError> {
    let live_home = adapter.live_home();
    if let Some(agent_home) = profile.agent_home.as_ref() {
        copy_login_auth(live_home, Path::new(agent_home))?;
    }
    let identity = load_probe_identity_from_home(&profile.id, live_home)?;
    store.upsert_probe_identity(&identity)
}

fn prepare_login_home() -> Result<PathBuf, RelayError> {
    let path = std::env::temp_dir().join(format!(
        "relay-codex-login-{}",
        Utc::now().timestamp_millis()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn run_codex_login(login_home: &Path) -> Result<(), RelayError> {
    let Some(binary) = find_binary("codex") else {
        return Err(RelayError::ExternalCommand("codex binary not found".into()));
    };

    let mut child = Command::new(binary)
        .arg("login")
        .env("CODEX_HOME", login_home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| RelayError::ExternalCommand(error.to_string()))?;

    let deadline = Instant::now() + Duration::from_secs(CODEX_LOGIN_TIMEOUT_SECS);
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| RelayError::ExternalCommand(error.to_string()))?
        {
            break status;
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(RelayError::ExternalCommand(
                "codex login timed out waiting for browser sign-in".into(),
            ));
        }

        thread::sleep(Duration::from_millis(CODEX_LOGIN_POLL_MILLIS));
    };

    if !status.success() {
        let mut stderr = String::new();
        if let Some(mut pipe) = child.stderr.take() {
            let _ = pipe.read_to_string(&mut stderr);
        }
        return Err(RelayError::ExternalCommand(if stderr.trim().is_empty() {
            "codex login did not complete successfully".into()
        } else {
            stderr.trim().into()
        }));
    }

    let auth_path = login_home.join("auth.json");
    if !auth_path.exists() {
        return Err(RelayError::Validation(
            "codex login completed without creating auth.json".into(),
        ));
    }

    Ok(())
}

pub fn copy_login_auth(login_home: &Path, destination_home: &Path) -> Result<(), RelayError> {
    fs::create_dir_all(destination_home)?;
    fs::copy(
        login_home.join("auth.json"),
        destination_home.join("auth.json"),
    )?;
    Ok(())
}

pub fn load_probe_identity_from_home(
    profile_id: &str,
    home: &Path,
) -> Result<ProfileProbeIdentity, RelayError> {
    let auth_path = home.join("auth.json");
    let contents = fs::read_to_string(&auth_path)?;
    let auth: CodexAuthFile = serde_json::from_str(&contents)
        .map_err(|error| RelayError::Validation(error.to_string()))?;

    let tokens = auth
        .tokens
        .ok_or_else(|| RelayError::Validation("auth.json is missing tokens".into()))?;
    let account_id = tokens
        .account_id
        .clone()
        .ok_or_else(|| RelayError::Validation("auth.json is missing account_id".into()))?;
    let access_token = tokens
        .access_token
        .clone()
        .ok_or_else(|| RelayError::Validation("auth.json is missing access_token".into()))?;
    let id_token = tokens.id_token.clone();

    let now = Utc::now().to_rfc3339();
    Ok(ProfileProbeIdentity::codex_official(
        profile_id.into(),
        account_id,
        access_token,
        tokens.refresh_token,
        id_token.clone(),
        extract_email(id_token.as_deref()),
        None,
        now.clone(),
        now,
    ))
}

fn extract_email(id_token: Option<&str>) -> Option<String> {
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
