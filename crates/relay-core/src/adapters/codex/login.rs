use super::CodexAdapter;
use super::auth::{copy_login_auth, load_probe_identity_from_home};
use crate::adapters::AgentAdapter;
use crate::app::AgentLoginMode;
use crate::models::{
    AgentKind, AgentLinkResult, AuthMode, Profile, ProfileProbeIdentity, RelayError,
};
use crate::platform::RelayPaths;
use crate::services::profile_service;
use crate::store::{AddProfileRecord, SqliteStore};
use chrono::Utc;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::thread::JoinHandle;
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
    mode: AgentLoginMode,
) -> Result<AgentLinkResult, RelayError> {
    let login_home = prepare_login_home()?;
    run_codex_login(&login_home, mode)?;
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

fn run_codex_login(login_home: &Path, mode: AgentLoginMode) -> Result<(), RelayError> {
    let Some(binary) = crate::platform::find_binary("codex") else {
        return Err(RelayError::ExternalCommand("codex binary not found".into()));
    };

    let mut command = Command::new(binary);
    command.arg("login");
    if mode == AgentLoginMode::DeviceAuth {
        command.arg("--device-auth");
    }

    let mut child = command
        .env("CODEX_HOME", login_home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| RelayError::ExternalCommand(error.to_string()))?;

    let forward_output = mode == AgentLoginMode::DeviceAuth;
    let stdout_handle = child
        .stdout
        .take()
        .map(|pipe| spawn_output_reader(pipe, forward_output));
    let stderr_handle = child
        .stderr
        .take()
        .map(|pipe| spawn_output_reader(pipe, forward_output));

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
            let _ = stdout_handle
                .map(JoinHandle::join)
                .transpose()
                .map_err(|_| RelayError::Internal("codex login stdout reader panicked".into()))?;
            let _ = stderr_handle
                .map(JoinHandle::join)
                .transpose()
                .map_err(|_| RelayError::Internal("codex login stderr reader panicked".into()))?;
            return Err(RelayError::ExternalCommand(
                match mode {
                    AgentLoginMode::Browser => "codex login timed out waiting for browser sign-in",
                    AgentLoginMode::DeviceAuth => {
                        "codex login timed out waiting for device authorization"
                    }
                }
                .into(),
            ));
        }

        thread::sleep(Duration::from_millis(CODEX_LOGIN_POLL_MILLIS));
    };

    let stdout = stdout_handle
        .map(JoinHandle::join)
        .transpose()
        .map_err(|_| RelayError::Internal("codex login stdout reader panicked".into()))?
        .unwrap_or_default();
    let stderr = stderr_handle
        .map(JoinHandle::join)
        .transpose()
        .map_err(|_| RelayError::Internal("codex login stderr reader panicked".into()))?
        .unwrap_or_default();

    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr);
        let stdout = String::from_utf8_lossy(&stdout);
        let message = stderr
            .trim()
            .strip_suffix('\n')
            .unwrap_or(stderr.trim())
            .trim();
        let fallback = stdout
            .trim()
            .strip_suffix('\n')
            .unwrap_or(stdout.trim())
            .trim();
        return Err(RelayError::ExternalCommand(if !message.is_empty() {
            message.into()
        } else if !fallback.is_empty() {
            fallback.into()
        } else {
            "codex login did not complete successfully".into()
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

fn spawn_output_reader<R>(mut reader: R, forward_output: bool) -> JoinHandle<Vec<u8>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer = Vec::new();
        let mut chunk = [0_u8; 1024];
        let mut stderr = std::io::stderr();

        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(count) => {
                    buffer.extend_from_slice(&chunk[..count]);
                    if forward_output {
                        let _ = stderr.write_all(&chunk[..count]);
                        let _ = stderr.flush();
                    }
                }
                Err(_) => break,
            }
        }

        buffer
    })
}
