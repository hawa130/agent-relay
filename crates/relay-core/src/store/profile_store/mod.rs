mod codec;
mod events;
mod profiles;
mod schema;
mod settings;

#[cfg(test)]
mod tests;

use self::codec::{
    failure_event_from_model, path_to_string, probe_identity_from_model, profile_from_model,
    slugify, stringify_agent_kind, stringify_auth_mode, stringify_outcome,
    stringify_probe_provider, stringify_reason, switch_history_from_model,
};
use self::schema::{
    SchemaState, inspect_schema_state, schema_incompatible_error, sqlite_url, sync_schema,
    validate_schema_queries,
};
use crate::models::{
    AgentKind, AppSettings, FailureEvent, FailureReason, Profile, ProfileProbeIdentity, RelayError,
    SwitchHistoryEntry, SwitchOutcome,
};
use crate::store::entities::{
    agent_settings, app_settings, failure_events, profile_probe_identities,
    profiles as profile_entities, switch_history,
};
use crate::{CodexSettings, CodexSettingsUpdateRequest};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
    IntoActiveModel, ModelTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::json;
use std::path::{Path, PathBuf};

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
        if !db_path.as_ref().exists() {
            return Ok(Self { connection: None });
        }

        let connection = Database::connect(sqlite_url(db_path.as_ref(), true)).await?;
        match inspect_schema_state(&connection).await? {
            SchemaState::Ready => {
                validate_schema_queries(&connection).await?;
                Ok(Self {
                    connection: Some(connection),
                })
            }
            SchemaState::Empty => Ok(Self { connection: None }),
            SchemaState::Syncable | SchemaState::Legacy | SchemaState::Incompatible => {
                Err(schema_incompatible_error())
            }
        }
    }

    async fn initialize(&self) -> Result<(), RelayError> {
        let connection = self.require_connection()?;
        match inspect_schema_state(connection).await? {
            SchemaState::Empty | SchemaState::Syncable | SchemaState::Ready => {}
            SchemaState::Legacy | SchemaState::Incompatible => {
                return Err(schema_incompatible_error());
            }
        }
        sync_schema(connection).await?;
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
}
