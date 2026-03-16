use crate::adapters::AgentAdapter;
use crate::models::RelayError;
use crate::models::{Profile, ProfileAccountState};
use crate::store::{AddProfileRecord, ProfileUpdateRecord, SqliteStore};
use chrono::Utc;

pub async fn add_profile(
    store: &SqliteStore,
    adapter: &dyn AgentAdapter,
    record: AddProfileRecord,
) -> Result<Profile, RelayError> {
    validate_nickname(&record.nickname)?;
    validate_source_inputs(record.config_path.as_ref(), record.agent_home.as_ref())?;
    validate_source_paths(record.config_path.as_ref(), record.agent_home.as_ref())?;

    let candidate = candidate_profile_from_add_record(&record);
    adapter.validate_profile(&candidate)?;

    let profile = store.add_profile(record).await?;
    adapter.validate_profile(&profile)?;
    Ok(profile)
}

pub async fn edit_profile(
    store: &SqliteStore,
    adapter: &dyn AgentAdapter,
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

    let current = store.get_profile(id).await?;
    let candidate = Profile {
        id: current.id.clone(),
        nickname: update.nickname.clone().unwrap_or(current.nickname.clone()),
        agent: current.agent,
        priority: update.priority.unwrap_or(current.priority),
        enabled: current.enabled,
        account_state: current.account_state,
        account_error_http_status: current.account_error_http_status,
        account_state_updated_at: current.account_state_updated_at.clone(),
        agent_home: update
            .agent_home
            .clone()
            .unwrap_or_else(|| current.agent_home.clone().map(Into::into))
            .map(|path| path.to_string_lossy().into_owned()),
        config_path: update
            .config_path
            .clone()
            .unwrap_or_else(|| current.config_path.clone().map(Into::into))
            .map(|path| path.to_string_lossy().into_owned()),
        auth_mode: update.auth_mode.unwrap_or(current.auth_mode),
        metadata: current.metadata.clone(),
        created_at: current.created_at,
        updated_at: current.updated_at,
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

    let profile = store.update_profile(id, update).await?;
    adapter.validate_profile(&profile)?;
    Ok(profile)
}

pub async fn remove_profile(store: &SqliteStore, id: &str) -> Result<Profile, RelayError> {
    if id.trim().is_empty() {
        return Err(RelayError::InvalidInput(
            "profile id must not be empty".into(),
        ));
    }
    store.remove_profile(id).await
}

pub async fn set_profile_enabled(
    store: &SqliteStore,
    id: &str,
    enabled: bool,
) -> Result<Profile, RelayError> {
    if id.trim().is_empty() {
        return Err(RelayError::InvalidInput(
            "profile id must not be empty".into(),
        ));
    }
    store.set_enabled(id, enabled).await
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
    agent_home: Option<&std::path::PathBuf>,
) -> Result<(), RelayError> {
    if config_path.is_none() && agent_home.is_none() {
        return Err(RelayError::Validation(
            "profile must provide either config_path or agent_home".into(),
        ));
    }
    Ok(())
}

fn validate_source_paths(
    config_path: Option<&std::path::PathBuf>,
    agent_home: Option<&std::path::PathBuf>,
) -> Result<(), RelayError> {
    if let Some(path) = config_path {
        if !path.exists() {
            return Err(RelayError::Validation(format!(
                "config path does not exist: {}",
                path.display()
            )));
        }
    }

    if let Some(path) = agent_home {
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
    let now = Utc::now();
    Profile {
        id: "candidate".into(),
        nickname: record.nickname.clone(),
        agent: record.agent,
        priority: record.priority,
        enabled: true,
        account_state: ProfileAccountState::Healthy,
        account_error_http_status: None,
        account_state_updated_at: None,
        agent_home: record
            .agent_home
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        config_path: record
            .config_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        auth_mode: record.auth_mode,
        metadata: serde_json::json!({}),
        created_at: now,
        updated_at: now,
    }
}
