use crate::models::{
    AgentKind, AppSettings, FailureEvent, FailureReason, ProbeProvider, Profile,
    ProfileProbeIdentity, RelayError, SwitchHistoryEntry, SwitchOutcome,
};
use crate::store::entities::{
    agent_settings, app_settings, failure_events, profile_probe_identities, profiles,
    switch_history,
};
use crate::store::migrations::Migrator;
use crate::{CodexSettings, CodexSettingsUpdateRequest};
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend,
    EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, QuerySelect, Set, Statement,
    TransactionTrait,
};
use sea_orm_migration::MigratorTrait;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const CURRENT_SCHEMA_VERSION: i32 = 1;
const MIGRATIONS_TABLE: &str = "seaql_migrations";

#[derive(Debug, Clone)]
pub struct AddProfileRecord {
    pub agent: AgentKind,
    pub nickname: String,
    pub priority: i32,
    pub config_path: Option<PathBuf>,
    pub agent_home: Option<PathBuf>,
    pub auth_mode: crate::models::AuthMode,
}

#[derive(Debug, Clone, Default)]
pub struct ProfileUpdateRecord {
    pub nickname: Option<String>,
    pub priority: Option<i32>,
    pub config_path: Option<Option<PathBuf>>,
    pub agent_home: Option<Option<PathBuf>>,
    pub auth_mode: Option<crate::models::AuthMode>,
}

#[derive(Debug, Clone)]
pub struct SwitchHistoryRecord {
    pub profile_id: Option<String>,
    pub previous_profile_id: Option<String>,
    pub outcome: SwitchOutcome,
    pub reason: Option<String>,
    pub checkpoint_id: Option<String>,
    pub rollback_performed: bool,
}

#[derive(Debug, Clone)]
pub struct SqliteStore {
    connection: Option<DatabaseConnection>,
}

impl SqliteStore {
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self, RelayError> {
        let connection = Database::connect(sqlite_url(db_path.as_ref(), false)).await?;
        let store = Self {
            connection: Some(connection),
        };
        store.initialize().await?;
        Ok(store)
    }

    pub async fn open_read_only(db_path: impl AsRef<Path>) -> Result<Self, RelayError> {
        let connection = if db_path.as_ref().exists() {
            Some(Database::connect(sqlite_url(db_path.as_ref(), true)).await?)
        } else {
            None
        };
        Ok(Self { connection })
    }

    async fn initialize(&self) -> Result<(), RelayError> {
        let connection = self.require_connection()?;
        Migrator::up(connection, None).await?;
        self.ensure_default_settings(connection).await?;
        self.ensure_default_agent_settings(connection).await?;
        Ok(())
    }

    fn connection(&self) -> Option<&DatabaseConnection> {
        self.connection.as_ref()
    }

    fn require_connection(&self) -> Result<&DatabaseConnection, RelayError> {
        self.connection()
            .ok_or_else(|| RelayError::Store("database is not available".into()))
    }

    pub async fn schema_version(&self) -> Result<i32, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(0);
        };
        if !has_table(&connection, MIGRATIONS_TABLE).await? {
            return Ok(0);
        }

        let statement = Statement::from_string(
            DbBackend::Sqlite,
            format!("SELECT COUNT(1) AS count FROM {MIGRATIONS_TABLE}"),
        );
        let row = connection.query_one(statement).await?;
        let count = row
            .as_ref()
            .and_then(|value| value.try_get::<i64>("", "count").ok())
            .unwrap_or_default();
        Ok((count as i32).min(CURRENT_SCHEMA_VERSION))
    }

    async fn ensure_default_settings(
        &self,
        connection: &DatabaseConnection,
    ) -> Result<(), RelayError> {
        let defaults = AppSettings::default();
        self.set_setting_value(
            connection,
            "auto_switch_enabled",
            if defaults.auto_switch_enabled {
                "true"
            } else {
                "false"
            },
        )
        .await?;
        self.set_setting_value(
            connection,
            "cooldown_seconds",
            &defaults.cooldown_seconds.to_string(),
        )
        .await?;

        if let Some(model) = app_settings::Entity::find_by_id("usage_source_mode")
            .one(connection)
            .await?
        {
            model.delete(connection).await?;
        }
        Ok(())
    }

    async fn ensure_default_agent_settings(
        &self,
        connection: &DatabaseConnection,
    ) -> Result<(), RelayError> {
        if agent_settings::Entity::find_by_id(stringify_agent_kind(&AgentKind::Codex))
            .one(connection)
            .await?
            .is_some()
        {
            return Ok(());
        }

        let defaults = CodexSettings::default();
        let timestamp = Utc::now().to_rfc3339();
        agent_settings::ActiveModel {
            agent: Set(stringify_agent_kind(&AgentKind::Codex).to_string()),
            settings_json: Set(serde_json::to_string(&defaults)
                .map_err(|error| RelayError::Store(error.to_string()))?),
            created_at: Set(timestamp.clone()),
            updated_at: Set(timestamp),
        }
        .insert(connection)
        .await?;
        Ok(())
    }

    pub async fn list_profiles(&self) -> Result<Vec<Profile>, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(Vec::new());
        };

        profiles::Entity::find()
            .order_by_asc(profiles::Column::Priority)
            .order_by_asc(profiles::Column::Nickname)
            .all(connection)
            .await?
            .into_iter()
            .map(profile_from_model)
            .collect()
    }

    pub async fn list_enabled_profiles(&self) -> Result<Vec<Profile>, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(Vec::new());
        };

        profiles::Entity::find()
            .filter(profiles::Column::Enabled.eq(true))
            .order_by_asc(profiles::Column::Priority)
            .order_by_asc(profiles::Column::Nickname)
            .all(connection)
            .await?
            .into_iter()
            .map(profile_from_model)
            .collect()
    }

    pub async fn get_profile(&self, id: &str) -> Result<Profile, RelayError> {
        let Some(connection) = self.connection() else {
            return Err(RelayError::NotFound(format!("profile not found: {id}")));
        };

        profiles::Entity::find_by_id(id.to_string())
            .one(connection)
            .await?
            .ok_or_else(|| RelayError::NotFound(format!("profile not found: {id}")))
            .and_then(profile_from_model)
    }

    pub async fn add_profile(&self, record: AddProfileRecord) -> Result<Profile, RelayError> {
        let connection = self.require_connection()?;
        let now = Utc::now().to_rfc3339();
        let id = format!(
            "p_{}_{}",
            Utc::now().timestamp_millis(),
            slugify(&record.nickname)
        );

        profiles::ActiveModel {
            id: Set(id.clone()),
            nickname: Set(record.nickname),
            agent: Set(stringify_agent_kind(&record.agent).to_string()),
            priority: Set(record.priority),
            enabled: Set(true),
            agent_home: Set(record.agent_home.as_ref().map(|path| path_to_string(path))),
            config_path: Set(record.config_path.as_ref().map(|path| path_to_string(path))),
            auth_mode: Set(stringify_auth_mode(&record.auth_mode).to_string()),
            metadata: Set(json!({}).to_string()),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(connection)
        .await?;

        self.get_profile(&id).await
    }

    pub async fn update_profile(
        &self,
        id: &str,
        update: ProfileUpdateRecord,
    ) -> Result<Profile, RelayError> {
        let current = self.get_profile(id).await?;
        let connection = self.require_connection()?;
        let model = profiles::Entity::find_by_id(id.to_string())
            .one(connection)
            .await?
            .ok_or_else(|| RelayError::NotFound(format!("profile not found: {id}")))?;
        let mut active = model.into_active_model();

        let nickname = update.nickname.unwrap_or(current.nickname);
        let priority = update.priority.unwrap_or(current.priority);
        let config_path = update
            .config_path
            .unwrap_or_else(|| current.config_path.map(PathBuf::from));
        let agent_home = update
            .agent_home
            .unwrap_or_else(|| current.agent_home.map(PathBuf::from));
        let auth_mode = update.auth_mode.unwrap_or(current.auth_mode);

        active.nickname = Set(nickname);
        active.priority = Set(priority);
        active.agent_home = Set(agent_home.as_ref().map(|path| path_to_string(path)));
        active.config_path = Set(config_path.as_ref().map(|path| path_to_string(path)));
        active.auth_mode = Set(stringify_auth_mode(&auth_mode).to_string());
        active.updated_at = Set(Utc::now().to_rfc3339());
        active.update(connection).await?;

        self.get_profile(id).await
    }

    pub async fn remove_profile(&self, id: &str) -> Result<Profile, RelayError> {
        let profile = self.get_profile(id).await?;
        let connection = self.require_connection()?;
        let transaction = connection.begin().await?;

        if let Some(identity) = profile_probe_identities::Entity::find_by_id(id.to_string())
            .one(&transaction)
            .await?
        {
            identity.delete(&transaction).await?;
        }

        let model = profiles::Entity::find_by_id(id.to_string())
            .one(&transaction)
            .await?
            .ok_or_else(|| RelayError::NotFound(format!("profile not found: {id}")))?;
        model.delete(&transaction).await?;
        transaction.commit().await?;

        Ok(profile)
    }

    pub async fn get_probe_identity(
        &self,
        profile_id: &str,
    ) -> Result<Option<ProfileProbeIdentity>, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(None);
        };

        profile_probe_identities::Entity::find_by_id(profile_id.to_string())
            .one(connection)
            .await?
            .map(probe_identity_from_model)
            .transpose()
    }

    pub async fn upsert_probe_identity(
        &self,
        identity: &ProfileProbeIdentity,
    ) -> Result<ProfileProbeIdentity, RelayError> {
        let connection = self.require_connection()?;
        if let Some(model) =
            profile_probe_identities::Entity::find_by_id(identity.profile_id.clone())
                .one(connection)
                .await?
        {
            let mut active = model.into_active_model();
            active.provider = Set(stringify_probe_provider(&identity.provider).to_string());
            active.principal_id = Set(identity.principal_id.clone());
            active.display_name = Set(identity.display_name.clone());
            active.credentials_json = Set(identity.credentials.to_string());
            active.metadata_json = Set(identity.metadata.to_string());
            active.updated_at = Set(identity.updated_at.clone());
            active.update(connection).await?;
        } else {
            profile_probe_identities::ActiveModel {
                profile_id: Set(identity.profile_id.clone()),
                provider: Set(stringify_probe_provider(&identity.provider).to_string()),
                principal_id: Set(identity.principal_id.clone()),
                display_name: Set(identity.display_name.clone()),
                credentials_json: Set(identity.credentials.to_string()),
                metadata_json: Set(identity.metadata.to_string()),
                created_at: Set(identity.created_at.clone()),
                updated_at: Set(identity.updated_at.clone()),
            }
            .insert(connection)
            .await?;
        }

        self.get_probe_identity(&identity.profile_id)
            .await?
            .ok_or_else(|| RelayError::Store("failed to reload probe identity".into()))
    }

    pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<Profile, RelayError> {
        let connection = self.require_connection()?;
        let model = profiles::Entity::find_by_id(id.to_string())
            .one(connection)
            .await?
            .ok_or_else(|| RelayError::NotFound(format!("profile not found: {id}")))?;
        let mut active = model.into_active_model();
        active.enabled = Set(enabled);
        active.updated_at = Set(Utc::now().to_rfc3339());
        active.update(connection).await?;
        self.get_profile(id).await
    }

    pub async fn get_settings(&self) -> Result<AppSettings, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(AppSettings::default());
        };
        let auto_switch_enabled = app_settings::Entity::find_by_id("auto_switch_enabled")
            .one(connection)
            .await?
            .map(|value| value.value == "true")
            .unwrap_or(false);
        let cooldown_seconds = app_settings::Entity::find_by_id("cooldown_seconds")
            .one(connection)
            .await?
            .and_then(|value| value.value.parse::<i64>().ok())
            .unwrap_or(600);

        Ok(AppSettings {
            auto_switch_enabled,
            cooldown_seconds,
        })
    }

    pub async fn set_auto_switch_enabled(&self, enabled: bool) -> Result<AppSettings, RelayError> {
        let connection = self.require_connection()?;
        self.set_setting_value(
            connection,
            "auto_switch_enabled",
            if enabled { "true" } else { "false" },
        )
        .await?;
        self.get_settings().await
    }

    pub async fn set_cooldown_seconds(&self, value: i64) -> Result<AppSettings, RelayError> {
        let connection = self.require_connection()?;
        self.set_setting_value(connection, "cooldown_seconds", &value.to_string())
            .await?;
        self.get_settings().await
    }

    pub async fn codex_settings(&self) -> Result<CodexSettings, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(CodexSettings::default());
        };

        agent_settings::Entity::find_by_id(stringify_agent_kind(&AgentKind::Codex))
            .one(connection)
            .await?
            .map(|value| {
                serde_json::from_str::<CodexSettings>(&value.settings_json)
                    .map_err(|error| RelayError::Store(error.to_string()))
            })
            .transpose()?
            .map(Ok)
            .unwrap_or_else(|| Ok(CodexSettings::default()))
    }

    pub async fn update_codex_settings(
        &self,
        request: CodexSettingsUpdateRequest,
    ) -> Result<CodexSettings, RelayError> {
        let current = self.codex_settings().await?;
        let settings = CodexSettings {
            usage_source_mode: request
                .usage_source_mode
                .unwrap_or(current.usage_source_mode),
        };
        let payload = serde_json::to_string(&settings)
            .map_err(|error| RelayError::Store(error.to_string()))?;
        let timestamp = Utc::now().to_rfc3339();
        let connection = self.require_connection()?;

        if let Some(model) =
            agent_settings::Entity::find_by_id(stringify_agent_kind(&AgentKind::Codex))
                .one(connection)
                .await?
        {
            let mut active = model.into_active_model();
            active.settings_json = Set(payload);
            active.updated_at = Set(timestamp);
            active.update(connection).await?;
        } else {
            agent_settings::ActiveModel {
                agent: Set(stringify_agent_kind(&AgentKind::Codex).to_string()),
                settings_json: Set(payload),
                created_at: Set(timestamp.clone()),
                updated_at: Set(timestamp),
            }
            .insert(connection)
            .await?;
        }

        self.codex_settings().await
    }

    async fn set_setting_value(
        &self,
        connection: &DatabaseConnection,
        key: &str,
        value: &str,
    ) -> Result<(), RelayError> {
        if let Some(model) = app_settings::Entity::find_by_id(key.to_string())
            .one(connection)
            .await?
        {
            let mut active = model.into_active_model();
            active.value = Set(value.to_string());
            active.update(connection).await?;
        } else {
            app_settings::ActiveModel {
                key: Set(key.to_string()),
                value: Set(value.to_string()),
            }
            .insert(connection)
            .await?;
        }
        Ok(())
    }

    pub async fn record_switch(
        &self,
        record: SwitchHistoryRecord,
    ) -> Result<SwitchHistoryEntry, RelayError> {
        let connection = self.require_connection()?;
        let transaction = connection.begin().await?;
        let entry = insert_switch_history(&transaction, record).await?;
        transaction.commit().await?;
        Ok(entry)
    }

    pub async fn record_switch_failure(
        &self,
        record: SwitchHistoryRecord,
        failure_reason: FailureReason,
        failure_message: impl AsRef<str>,
        cooldown_until: Option<DateTime<Utc>>,
    ) -> Result<(SwitchHistoryEntry, FailureEvent), RelayError> {
        let connection = self.require_connection()?;
        let transaction = connection.begin().await?;
        let entry = insert_switch_history(&transaction, record).await?;
        let event = insert_failure_event(
            &transaction,
            entry.profile_id.as_deref(),
            failure_reason,
            failure_message,
            cooldown_until,
        )
        .await?;
        transaction.commit().await?;
        Ok((entry, event))
    }

    pub async fn list_switch_history(
        &self,
        limit: usize,
    ) -> Result<Vec<SwitchHistoryEntry>, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(Vec::new());
        };

        switch_history::Entity::find()
            .order_by_desc(switch_history::Column::CreatedAt)
            .limit(limit as u64)
            .all(connection)
            .await?
            .into_iter()
            .map(switch_history_from_model)
            .collect()
    }

    pub async fn record_failure_event(
        &self,
        profile_id: Option<&str>,
        reason: FailureReason,
        message: impl AsRef<str>,
        cooldown_until: Option<DateTime<Utc>>,
    ) -> Result<FailureEvent, RelayError> {
        let connection = self.require_connection()?;
        let transaction = connection.begin().await?;
        let event =
            insert_failure_event(&transaction, profile_id, reason, message, cooldown_until).await?;
        transaction.commit().await?;
        Ok(event)
    }

    pub async fn list_failure_events(&self, limit: usize) -> Result<Vec<FailureEvent>, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(Vec::new());
        };

        failure_events::Entity::find()
            .order_by_desc(failure_events::Column::CreatedAt)
            .limit(limit as u64)
            .all(connection)
            .await?
            .into_iter()
            .map(failure_event_from_model)
            .collect()
    }

    #[cfg(test)]
    pub async fn record_failure_event_for_test(
        &self,
        profile_id: &str,
        reason: FailureReason,
        message: impl AsRef<str>,
    ) -> Result<FailureEvent, RelayError> {
        self.record_failure_event(Some(profile_id), reason, message, None)
            .await
    }
}

async fn has_table(connection: &DatabaseConnection, table: &str) -> Result<bool, RelayError> {
    let statement = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "SELECT 1 AS present FROM sqlite_master WHERE type = 'table' AND name = ? LIMIT 1",
        [table.into()],
    );
    Ok(connection.query_one(statement).await?.is_some())
}

async fn insert_switch_history<C>(
    connection: &C,
    record: SwitchHistoryRecord,
) -> Result<SwitchHistoryEntry, RelayError>
where
    C: ConnectionTrait,
{
    let id = format!("sw_{}", Utc::now().timestamp_millis());
    let created_at = Utc::now();

    switch_history::ActiveModel {
        id: Set(id.clone()),
        profile_id: Set(record.profile_id.clone()),
        previous_profile_id: Set(record.previous_profile_id.clone()),
        outcome: Set(stringify_outcome(&record.outcome).to_string()),
        reason: Set(record.reason.clone()),
        checkpoint_id: Set(record.checkpoint_id.clone()),
        rollback_performed: Set(record.rollback_performed),
        created_at: Set(created_at.to_rfc3339()),
        details: Set(json!({}).to_string()),
    }
    .insert(connection)
    .await?;

    Ok(SwitchHistoryEntry {
        id,
        profile_id: record.profile_id,
        previous_profile_id: record.previous_profile_id,
        outcome: record.outcome,
        reason: record.reason,
        checkpoint_id: record.checkpoint_id,
        rollback_performed: record.rollback_performed,
        created_at,
    })
}

async fn insert_failure_event<C>(
    connection: &C,
    profile_id: Option<&str>,
    reason: FailureReason,
    message: impl AsRef<str>,
    cooldown_until: Option<DateTime<Utc>>,
) -> Result<FailureEvent, RelayError>
where
    C: ConnectionTrait,
{
    let event = FailureEvent {
        id: format!("ev_{}", Utc::now().timestamp_millis()),
        profile_id: profile_id.map(ToOwned::to_owned),
        reason: reason.clone(),
        message: message.as_ref().to_string(),
        cooldown_until,
        created_at: Utc::now(),
    };

    failure_events::ActiveModel {
        id: Set(event.id.clone()),
        profile_id: Set(event.profile_id.clone()),
        reason: Set(stringify_reason(&event.reason).to_string()),
        message: Set(event.message.clone()),
        cooldown_until: Set(event.cooldown_until.map(|value| value.to_rfc3339())),
        created_at: Set(event.created_at.to_rfc3339()),
    }
    .insert(connection)
    .await?;

    Ok(event)
}

fn profile_from_model(model: profiles::Model) -> Result<Profile, RelayError> {
    Ok(Profile {
        id: model.id,
        nickname: model.nickname,
        agent: parse_agent_kind(&model.agent)?,
        priority: model.priority,
        enabled: model.enabled,
        agent_home: model.agent_home,
        config_path: model.config_path,
        auth_mode: parse_auth_mode(&model.auth_mode),
        metadata: serde_json::from_str(&model.metadata).unwrap_or(Value::Null),
        created_at: model.created_at,
        updated_at: model.updated_at,
    })
}

fn probe_identity_from_model(
    model: profile_probe_identities::Model,
) -> Result<ProfileProbeIdentity, RelayError> {
    Ok(ProfileProbeIdentity {
        profile_id: model.profile_id,
        provider: parse_probe_provider(&model.provider)?,
        principal_id: model.principal_id,
        display_name: model.display_name,
        credentials: serde_json::from_str(&model.credentials_json).unwrap_or(Value::Null),
        metadata: serde_json::from_str(&model.metadata_json).unwrap_or(Value::Null),
        created_at: model.created_at,
        updated_at: model.updated_at,
    })
}

fn switch_history_from_model(
    model: switch_history::Model,
) -> Result<SwitchHistoryEntry, RelayError> {
    Ok(SwitchHistoryEntry {
        id: model.id,
        profile_id: model.profile_id,
        previous_profile_id: model.previous_profile_id,
        outcome: parse_outcome(&model.outcome),
        reason: model.reason,
        checkpoint_id: model.checkpoint_id,
        rollback_performed: model.rollback_performed,
        created_at: parse_timestamp(&model.created_at),
    })
}

fn failure_event_from_model(model: failure_events::Model) -> Result<FailureEvent, RelayError> {
    Ok(FailureEvent {
        id: model.id,
        profile_id: model.profile_id,
        reason: parse_reason(&model.reason),
        message: model.message,
        cooldown_until: model
            .cooldown_until
            .as_deref()
            .map(parse_timestamp)
            .map(Some)
            .unwrap_or(None),
        created_at: parse_timestamp(&model.created_at),
    })
}

fn parse_timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn sqlite_url(path: &Path, read_only: bool) -> String {
    let mode = if read_only { "?mode=ro" } else { "?mode=rwc" };
    format!("sqlite://{}{}", path.to_string_lossy(), mode)
}

fn stringify_auth_mode(mode: &crate::models::AuthMode) -> &'static str {
    match mode {
        crate::models::AuthMode::ConfigFilesystem => "config-filesystem",
        crate::models::AuthMode::EnvReference => "env-reference",
        crate::models::AuthMode::KeychainReference => "keychain-reference",
    }
}

fn parse_auth_mode(value: &str) -> crate::models::AuthMode {
    match value {
        "env-reference" => crate::models::AuthMode::EnvReference,
        "keychain-reference" => crate::models::AuthMode::KeychainReference,
        _ => crate::models::AuthMode::ConfigFilesystem,
    }
}

fn stringify_probe_provider(provider: &ProbeProvider) -> &'static str {
    match provider {
        ProbeProvider::CodexOfficial => "codex-official",
    }
}

fn parse_probe_provider(value: &str) -> Result<ProbeProvider, RelayError> {
    match value {
        "codex-official" => Ok(ProbeProvider::CodexOfficial),
        other => Err(RelayError::Store(format!(
            "unsupported probe provider: {other}"
        ))),
    }
}

fn stringify_agent_kind(kind: &AgentKind) -> &'static str {
    match kind {
        AgentKind::Codex => "codex",
    }
}

fn parse_agent_kind(value: &str) -> Result<AgentKind, RelayError> {
    match value {
        "codex" => Ok(AgentKind::Codex),
        other => Err(RelayError::Validation(format!(
            "unknown agent kind: {other}"
        ))),
    }
}

fn stringify_reason(reason: &FailureReason) -> &'static str {
    match reason {
        FailureReason::SessionExhausted => "session-exhausted",
        FailureReason::WeeklyExhausted => "weekly-exhausted",
        FailureReason::AuthInvalid => "auth-invalid",
        FailureReason::QuotaExhausted => "quota-exhausted",
        FailureReason::RateLimited => "rate-limited",
        FailureReason::CommandFailed => "command-failed",
        FailureReason::ValidationFailed => "validation-failed",
        FailureReason::Unknown => "unknown",
    }
}

fn parse_reason(value: &str) -> FailureReason {
    match value {
        "session-exhausted" => FailureReason::SessionExhausted,
        "weekly-exhausted" => FailureReason::WeeklyExhausted,
        "auth-invalid" => FailureReason::AuthInvalid,
        "quota-exhausted" => FailureReason::QuotaExhausted,
        "rate-limited" => FailureReason::RateLimited,
        "command-failed" => FailureReason::CommandFailed,
        "validation-failed" => FailureReason::ValidationFailed,
        _ => FailureReason::Unknown,
    }
}

fn stringify_outcome(outcome: &SwitchOutcome) -> &'static str {
    match outcome {
        SwitchOutcome::NotRun => "not-run",
        SwitchOutcome::Success => "success",
        SwitchOutcome::Failed => "failed",
    }
}

fn parse_outcome(value: &str) -> SwitchOutcome {
    match value {
        "success" => SwitchOutcome::Success,
        "failed" => SwitchOutcome::Failed,
        _ => SwitchOutcome::NotRun,
    }
}

fn slugify(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
        } else if !output.ends_with('_') {
            output.push('_');
        }
    }
    output.trim_matches('_').to_string()
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::UsageSourceMode;
    use tempfile::tempdir;

    #[tokio::test]
    async fn profile_settings_events_and_switch_history_round_trip() {
        let temp = tempdir().expect("tempdir");
        let db_path = temp.path().join("relay.db");
        let store = SqliteStore::new(&db_path).await.expect("store");
        assert_eq!(
            store.schema_version().await.expect("schema version"),
            CURRENT_SCHEMA_VERSION
        );

        let created = store
            .add_profile(AddProfileRecord {
                agent: AgentKind::Codex,
                nickname: "Work".into(),
                priority: 10,
                config_path: Some(temp.path().join("config.toml")),
                agent_home: Some(temp.path().join(".codex-work")),
                auth_mode: crate::models::AuthMode::ConfigFilesystem,
            })
            .await
            .expect("profile");
        assert!(created.enabled);

        let updated = store
            .update_profile(
                &created.id,
                ProfileUpdateRecord {
                    nickname: Some("Work Updated".into()),
                    ..ProfileUpdateRecord::default()
                },
            )
            .await
            .expect("updated");
        assert_eq!(updated.nickname, "Work Updated");

        let settings = store.set_auto_switch_enabled(true).await.expect("settings");
        assert!(settings.auto_switch_enabled);

        let event = store
            .record_failure_event(
                Some(&created.id),
                FailureReason::ValidationFailed,
                "validation failed",
                None,
            )
            .await
            .expect("event");
        assert_eq!(event.profile_id.as_deref(), Some(created.id.as_str()));

        let switch_entry = store
            .record_switch(SwitchHistoryRecord {
                profile_id: Some(created.id.clone()),
                previous_profile_id: None,
                outcome: SwitchOutcome::Success,
                reason: Some("manual".into()),
                checkpoint_id: Some("ckpt".into()),
                rollback_performed: false,
            })
            .await
            .expect("switch");
        assert_eq!(switch_entry.checkpoint_id.as_deref(), Some("ckpt"));

        assert_eq!(
            store.list_failure_events(10).await.expect("events").len(),
            1
        );
        assert_eq!(
            store.list_switch_history(10).await.expect("history").len(),
            1
        );
        assert_eq!(store.list_profiles().await.expect("profiles").len(), 1);
    }

    #[tokio::test]
    async fn codex_settings_round_trip() {
        let temp = tempdir().expect("tempdir");
        let db_path = temp.path().join("relay.db");
        let store = SqliteStore::new(&db_path).await.expect("store");

        let defaults = store
            .codex_settings()
            .await
            .expect("default codex settings");
        assert_eq!(defaults.usage_source_mode, UsageSourceMode::Auto);

        let updated = store
            .update_codex_settings(CodexSettingsUpdateRequest {
                usage_source_mode: Some(UsageSourceMode::WebEnhanced),
            })
            .await
            .expect("updated codex settings");
        assert_eq!(updated.usage_source_mode, UsageSourceMode::WebEnhanced);
        assert_eq!(
            store
                .codex_settings()
                .await
                .expect("reloaded codex settings"),
            updated
        );
    }
}
