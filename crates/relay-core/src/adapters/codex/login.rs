use super::CodexAdapter;
use super::auth::{copy_login_auth, load_probe_identity_from_home};
use crate::adapters::AgentAdapter;
use crate::models::{
    AgentKind, AgentLinkResult, AuthMode, Profile, ProfileProbeIdentity, RelayError,
};
use crate::platform::RelayPaths;
use crate::services::profile_service;
use crate::store::{AddProfileRecord, SqliteStore};
use chrono::Utc;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const CODEX_LOGIN_TIMEOUT_SECS: u64 = 300;
const CODEX_LOGIN_POLL_MILLIS: u64 = 250;

pub(crate) fn import_profile(
    adapter: &CodexAdapter,
    store: &SqliteStore,
    paths: &RelayPaths,
    nickname: Option<String>,
    priority: i32,
) -> Result<Profile, RelayError> {
    let snapshot_dir = paths
        .profiles_dir
        .join(format!("imported_{}", Utc::now().timestamp_millis()));
    adapter.import_live_profile(&snapshot_dir)?;
    let live_identity = load_probe_identity_from_home("pending", adapter.live_home()).ok();

    let record = AddProfileRecord {
        agent: AgentKind::Codex,
        nickname: nickname.unwrap_or_else(|| {
            live_identity
                .as_ref()
                .and_then(|identity| identity.email().map(ToOwned::to_owned))
                .unwrap_or_else(|| format!("Imported Codex {}", Utc::now().format("%Y%m%d-%H%M%S")))
        }),
        priority,
        config_path: Some(snapshot_dir.join("config.toml")),
        agent_home: Some(snapshot_dir),
        auth_mode: AuthMode::ConfigFilesystem,
    };
    let profile = profile_service::add_profile(store, adapter, record)?;

    if let Some(identity) = live_identity {
        let _ = store.upsert_probe_identity(&ProfileProbeIdentity {
            profile_id: profile.id.clone(),
            ..identity
        });
    }

    Ok(profile)
}

pub(crate) fn login_profile(
    adapter: &CodexAdapter,
    store: &SqliteStore,
    profiles_dir: &Path,
    nickname: Option<String>,
    priority: i32,
) -> Result<AgentLinkResult, RelayError> {
    let login_home = prepare_login_home()?;
    run_codex_login(&login_home)?;
    let identity = load_probe_identity_from_home("pending", &login_home)?;

    let snapshot_dir = profiles_dir.join(format!("login_{}", Utc::now().timestamp_millis()));
    adapter.import_live_profile(&snapshot_dir)?;
    copy_login_auth(&login_home, &snapshot_dir)?;

    let profile = store.add_profile(AddProfileRecord {
        agent: AgentKind::Codex,
        nickname: nickname.unwrap_or_else(|| {
            identity
                .email()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("Codex {}", Utc::now().format("%Y%m%d-%H%M%S")))
        }),
        priority,
        config_path: Some(snapshot_dir.join("config.toml")),
        agent_home: Some(snapshot_dir),
        auth_mode: AuthMode::ConfigFilesystem,
    })?;

    let probe_identity = store.upsert_probe_identity(&ProfileProbeIdentity {
        profile_id: profile.id.clone(),
        ..identity
    })?;

    Ok(AgentLinkResult {
        profile,
        probe_identity,
        activated: false,
    })
}

pub(crate) fn relink_profile(
    adapter: &CodexAdapter,
    store: &SqliteStore,
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
    let Some(binary) = crate::platform::find_binary("codex") else {
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
