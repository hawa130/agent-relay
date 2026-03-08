use crate::adapters::{AgentAdapter, CodexAdapter};
use crate::models::{Profile, RelayError};
use crate::platform::RelayPaths;
use crate::services::codex_link_service;
use crate::store::{AddProfileRecord, ProfileUpdateRecord, SqliteStore};
use chrono::Utc;

pub fn add_profile(
    store: &SqliteStore,
    adapter: &CodexAdapter,
    record: AddProfileRecord,
) -> Result<Profile, RelayError> {
    validate_nickname(&record.nickname)?;
    validate_source_inputs(record.config_path.as_ref(), record.codex_home.as_ref())?;
    validate_source_paths(record.config_path.as_ref(), record.codex_home.as_ref())?;

    let candidate = candidate_profile_from_add_record(&record);
    adapter.validate_profile(&candidate)?;

    let profile = store.add_profile(record)?;
    adapter.validate_profile(&profile)?;
    Ok(profile)
}

pub fn edit_profile(
    store: &SqliteStore,
    adapter: &CodexAdapter,
    id: &str,
    update: ProfileUpdateRecord,
) -> Result<Profile, RelayError> {
    if id.trim().is_empty() {
        return Err(RelayError::InvalidInput(
            "profile id must not be empty".into(),
        ));
    }
    if let Some(nickname) = update.nickname.as_ref() {
        validate_nickname(nickname)?;
    }

    let current = store.get_profile(id)?;
    let candidate = Profile {
        id: current.id.clone(),
        nickname: update.nickname.clone().unwrap_or(current.nickname.clone()),
        agent: current.agent.clone(),
        priority: update.priority.unwrap_or(current.priority),
        enabled: current.enabled,
        agent_home: update
            .codex_home
            .clone()
            .unwrap_or_else(|| current.agent_home.clone().map(Into::into))
            .map(|path| path.to_string_lossy().into_owned()),
        config_path: update
            .config_path
            .clone()
            .unwrap_or_else(|| current.config_path.clone().map(Into::into))
            .map(|path| path.to_string_lossy().into_owned()),
        auth_mode: update
            .auth_mode
            .clone()
            .unwrap_or(current.auth_mode.clone()),
        metadata: current.metadata.clone(),
        created_at: current.created_at.clone(),
        updated_at: current.updated_at.clone(),
    };
    validate_source_inputs(
        candidate
            .config_path
            .as_ref()
            .map(std::path::PathBuf::from)
            .as_ref(),
        candidate
            .agent_home
            .as_ref()
            .map(std::path::PathBuf::from)
            .as_ref(),
    )?;
    validate_source_paths(
        candidate
            .config_path
            .as_ref()
            .map(std::path::PathBuf::from)
            .as_ref(),
        candidate
            .agent_home
            .as_ref()
            .map(std::path::PathBuf::from)
            .as_ref(),
    )?;
    adapter.validate_profile(&candidate)?;

    let profile = store.update_profile(id, update)?;
    adapter.validate_profile(&profile)?;
    Ok(profile)
}

pub fn remove_profile(store: &SqliteStore, id: &str) -> Result<Profile, RelayError> {
    if id.trim().is_empty() {
        return Err(RelayError::InvalidInput(
            "profile id must not be empty".into(),
        ));
    }
    store.remove_profile(id)
}

pub fn set_profile_enabled(
    store: &SqliteStore,
    id: &str,
    enabled: bool,
) -> Result<Profile, RelayError> {
    if id.trim().is_empty() {
        return Err(RelayError::InvalidInput(
            "profile id must not be empty".into(),
        ));
    }
    store.set_enabled(id, enabled)
}

pub fn import_codex_profile(
    store: &SqliteStore,
    adapter: &CodexAdapter,
    paths: &RelayPaths,
    nickname: Option<String>,
    priority: i32,
) -> Result<Profile, RelayError> {
    let snapshot_dir = paths
        .profiles_dir
        .join(format!("imported_{}", Utc::now().timestamp_millis()));
    adapter.import_live_profile(&snapshot_dir)?;
    let live_identity =
        codex_link_service::load_probe_identity_from_home("pending", adapter.live_home()).ok();

    let record = AddProfileRecord {
        nickname: nickname.unwrap_or_else(|| {
            live_identity
                .as_ref()
                .and_then(|identity| identity.email().map(ToOwned::to_owned))
                .unwrap_or_else(|| format!("Imported Codex {}", Utc::now().format("%Y%m%d-%H%M%S")))
        }),
        priority,
        config_path: Some(snapshot_dir.join("config.toml")),
        codex_home: Some(snapshot_dir),
        auth_mode: crate::models::AuthMode::ConfigFilesystem,
    };
    let profile = add_profile(store, adapter, record)?;

    if let Some(identity) = live_identity {
        let _ = store.upsert_probe_identity(&crate::models::ProfileProbeIdentity {
            profile_id: profile.id.clone(),
            ..identity
        });
    }

    Ok(profile)
}

fn validate_nickname(nickname: &str) -> Result<(), RelayError> {
    if nickname.trim().is_empty() {
        return Err(RelayError::InvalidInput(
            "profile nickname must not be empty".into(),
        ));
    }
    Ok(())
}

fn validate_source_inputs(
    config_path: Option<&std::path::PathBuf>,
    codex_home: Option<&std::path::PathBuf>,
) -> Result<(), RelayError> {
    if config_path.is_none() && codex_home.is_none() {
        return Err(RelayError::Validation(
            "profile must provide either config_path or agent_home".into(),
        ));
    }
    Ok(())
}

fn validate_source_paths(
    config_path: Option<&std::path::PathBuf>,
    codex_home: Option<&std::path::PathBuf>,
) -> Result<(), RelayError> {
    if let Some(path) = config_path {
        if !path.exists() {
            return Err(RelayError::Validation(format!(
                "config path does not exist: {}",
                path.display()
            )));
        }
    }

    if let Some(path) = codex_home {
        if !path.exists() {
            return Err(RelayError::Validation(format!(
                "agent home does not exist: {}",
                path.display()
            )));
        }
        if !path.is_dir() {
            return Err(RelayError::Validation(format!(
                "agent home is not a directory: {}",
                path.display()
            )));
        }
    }

    Ok(())
}

fn candidate_profile_from_add_record(record: &AddProfileRecord) -> Profile {
    let now = Utc::now().to_rfc3339();
    Profile {
        id: "candidate".into(),
        nickname: record.nickname.clone(),
        agent: crate::models::AgentKind::Codex,
        priority: record.priority,
        enabled: true,
        agent_home: record
            .codex_home
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        config_path: record
            .config_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        auth_mode: record.auth_mode.clone(),
        metadata: serde_json::json!({}),
        created_at: now.clone(),
        updated_at: now,
    }
}
