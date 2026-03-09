use crate::models::{
    AgentKind, AppSettings, FailureEvent, FailureReason, ProbeProvider, Profile,
    ProfileProbeIdentity, RelayError, SwitchHistoryEntry, SwitchOutcome, UsageSourceMode,
};
use crate::{CodexSettings, CodexSettingsUpdateRequest};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const CURRENT_SCHEMA_VERSION: i32 = 5;
const SCHEMA_V1_SQL: &str = "
CREATE TABLE IF NOT EXISTS profiles (
    id TEXT PRIMARY KEY,
    nickname TEXT NOT NULL,
    agent TEXT NOT NULL,
    priority INTEGER NOT NULL,
    enabled INTEGER NOT NULL,
    agent_home TEXT,
    config_path TEXT,
    auth_mode TEXT NOT NULL,
    metadata TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS switch_history (
    id TEXT PRIMARY KEY,
    profile_id TEXT,
    previous_profile_id TEXT,
    outcome TEXT NOT NULL,
    reason TEXT,
    checkpoint_id TEXT,
    rollback_performed INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    details TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS failure_events (
    id TEXT PRIMARY KEY,
    profile_id TEXT,
    reason TEXT NOT NULL,
    message TEXT NOT NULL,
    cooldown_until TEXT,
    created_at TEXT NOT NULL
);";
const SCHEMA_V2_SQL: &str = "
CREATE TABLE IF NOT EXISTS profile_probe_identities (
    profile_id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    account_id TEXT NOT NULL,
    access_token TEXT NOT NULL,
    refresh_token TEXT,
    id_token TEXT,
    email TEXT,
    plan_hint TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);";
const SCHEMA_V3_SQL: &str = "
DROP TABLE IF EXISTS profile_probe_identities;
CREATE TABLE profile_probe_identities (
    profile_id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    principal_id TEXT,
    display_name TEXT,
    credentials_json TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);";
const SCHEMA_V4_SQL: &str = "
DROP TABLE IF EXISTS switch_history;
DROP TABLE IF EXISTS failure_events;
DROP TABLE IF EXISTS app_settings;
DROP TABLE IF EXISTS profile_probe_identities;
DROP TABLE IF EXISTS profiles;
CREATE TABLE profiles (
    id TEXT PRIMARY KEY,
    nickname TEXT NOT NULL,
    agent TEXT NOT NULL,
    priority INTEGER NOT NULL,
    enabled INTEGER NOT NULL,
    agent_home TEXT,
    config_path TEXT,
    auth_mode TEXT NOT NULL,
    metadata TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE switch_history (
    id TEXT PRIMARY KEY,
    profile_id TEXT,
    previous_profile_id TEXT,
    outcome TEXT NOT NULL,
    reason TEXT,
    checkpoint_id TEXT,
    rollback_performed INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    details TEXT NOT NULL
);
CREATE TABLE failure_events (
    id TEXT PRIMARY KEY,
    profile_id TEXT,
    reason TEXT NOT NULL,
    message TEXT NOT NULL,
    cooldown_until TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE profile_probe_identities (
    profile_id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    principal_id TEXT,
    display_name TEXT,
    credentials_json TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);";
const SCHEMA_V5_SQL: &str = "
CREATE TABLE IF NOT EXISTS agent_settings (
    agent TEXT PRIMARY KEY,
    settings_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);";

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
    db_path: PathBuf,
    read_only: bool,
}

impl SqliteStore {
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self, RelayError> {
        let db_path = db_path.as_ref().to_path_buf();
        let store = Self {
            db_path,
            read_only: false,
        };
        store.initialize()?;
        Ok(store)
    }

    pub fn open_read_only(db_path: impl AsRef<Path>) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
            read_only: true,
        }
    }

    fn open(&self) -> Result<Connection, RelayError> {
        let connection = if self.read_only {
            Connection::open_with_flags(&self.db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        } else {
            Connection::open(&self.db_path)
        };
        connection.map_err(|error| RelayError::Store(error.to_string()))
    }

    fn initialize(&self) -> Result<(), RelayError> {
        if self.read_only {
            return Ok(());
        }
        let mut connection = self.open()?;
        self.run_migrations(&mut connection)?;
        self.ensure_default_settings(&connection)?;
        self.ensure_default_agent_settings(&connection)?;
        Ok(())
    }

    fn run_migrations(&self, connection: &mut Connection) -> Result<(), RelayError> {
        let current_version = self.schema_version_from(connection)?;
        if current_version > CURRENT_SCHEMA_VERSION {
            return Err(RelayError::Store(format!(
                "database schema version {current_version} is newer than supported version {CURRENT_SCHEMA_VERSION}"
            )));
        }

        let transaction = connection
            .transaction()
            .map_err(|error| RelayError::Store(error.to_string()))?;

        if current_version < 1 {
            transaction
                .execute_batch(SCHEMA_V1_SQL)
                .map_err(|error| RelayError::Store(error.to_string()))?;
            transaction
                .pragma_update(None, "user_version", 1)
                .map_err(|error| RelayError::Store(error.to_string()))?;
        }

        if current_version < 2 {
            transaction
                .execute_batch(SCHEMA_V2_SQL)
                .map_err(|error| RelayError::Store(error.to_string()))?;
            transaction
                .pragma_update(None, "user_version", 2)
                .map_err(|error| RelayError::Store(error.to_string()))?;
        }

        if current_version < 3 {
            transaction
                .execute_batch(SCHEMA_V3_SQL)
                .map_err(|error| RelayError::Store(error.to_string()))?;
            transaction
                .pragma_update(None, "user_version", 3)
                .map_err(|error| RelayError::Store(error.to_string()))?;
        }

        if current_version < 4 {
            transaction
                .execute_batch(SCHEMA_V4_SQL)
                .map_err(|error| RelayError::Store(error.to_string()))?;
            transaction
                .pragma_update(None, "user_version", 4)
                .map_err(|error| RelayError::Store(error.to_string()))?;
        }

        if current_version < 5 {
            transaction
                .execute_batch(SCHEMA_V5_SQL)
                .map_err(|error| RelayError::Store(error.to_string()))?;
            migrate_legacy_codex_settings(&transaction)?;
            transaction
                .pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)
                .map_err(|error| RelayError::Store(error.to_string()))?;
        }

        transaction
            .commit()
            .map_err(|error| RelayError::Store(error.to_string()))?;
        Ok(())
    }

    fn schema_version_from(&self, connection: &Connection) -> Result<i32, RelayError> {
        connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|error| RelayError::Store(error.to_string()))
    }

    pub fn schema_version(&self) -> Result<i32, RelayError> {
        let connection = self.open()?;
        self.schema_version_from(&connection)
    }

    fn ensure_default_settings(&self, connection: &Connection) -> Result<(), RelayError> {
        let defaults = AppSettings::default();
        connection
            .execute(
                "INSERT OR IGNORE INTO app_settings (key, value) VALUES ('auto_switch_enabled', ?1)",
                [if defaults.auto_switch_enabled { "true" } else { "false" }],
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;
        connection
            .execute(
                "INSERT OR IGNORE INTO app_settings (key, value) VALUES ('cooldown_seconds', ?1)",
                [defaults.cooldown_seconds.to_string()],
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;
        connection
            .execute(
                "DELETE FROM app_settings WHERE key = 'usage_source_mode'",
                [],
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;
        Ok(())
    }

    fn ensure_default_agent_settings(&self, connection: &Connection) -> Result<(), RelayError> {
        let defaults = CodexSettings::default();
        let timestamp = Utc::now().to_rfc3339();
        connection
            .execute(
                "INSERT OR IGNORE INTO agent_settings (agent, settings_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?3)",
                params![
                    stringify_agent_kind(&AgentKind::Codex),
                    serde_json::to_string(&defaults)
                        .map_err(|error| RelayError::Store(error.to_string()))?,
                    timestamp,
                ],
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;
        Ok(())
    }

    fn row_to_profile(row: &rusqlite::Row<'_>) -> Result<Profile, rusqlite::Error> {
        let agent = parse_agent_kind(row.get::<_, String>(2)?.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::other(error.to_string())),
            )
        })?;
        let auth_mode = parse_auth_mode(row.get::<_, String>(7)?.as_str());
        let metadata_text: String = row.get(8)?;
        let metadata: Value = serde_json::from_str(&metadata_text).unwrap_or(Value::Null);

        Ok(Profile {
            id: row.get(0)?,
            nickname: row.get(1)?,
            agent,
            priority: row.get(3)?,
            enabled: row.get::<_, i64>(4)? != 0,
            agent_home: row.get(5)?,
            config_path: row.get(6)?,
            auth_mode,
            metadata,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    }

    fn row_to_probe_identity(
        row: &rusqlite::Row<'_>,
    ) -> Result<ProfileProbeIdentity, rusqlite::Error> {
        let provider =
            parse_probe_provider(row.get::<_, String>(1)?.as_str()).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::other(error.to_string())),
                )
            })?;

        Ok(ProfileProbeIdentity {
            profile_id: row.get(0)?,
            provider,
            principal_id: row.get(2)?,
            display_name: row.get(3)?,
            credentials: serde_json::from_str::<Value>(&row.get::<_, String>(4)?)
                .unwrap_or(Value::Null),
            metadata: serde_json::from_str::<Value>(&row.get::<_, String>(5)?)
                .unwrap_or(Value::Null),
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    }

    pub fn list_profiles(&self) -> Result<Vec<Profile>, RelayError> {
        if self.read_only && !self.db_path.exists() {
            return Ok(Vec::new());
        }
        let connection = self.open()?;
        let mut statement = connection
            .prepare(
                "SELECT id, nickname, agent, priority, enabled, agent_home, config_path, auth_mode, metadata, created_at, updated_at
                 FROM profiles
                 ORDER BY priority ASC, nickname ASC",
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;

        let rows = statement
            .query_map([], Self::row_to_profile)
            .map_err(|error| RelayError::Store(error.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| RelayError::Store(error.to_string()))
    }

    pub fn list_enabled_profiles(&self) -> Result<Vec<Profile>, RelayError> {
        Ok(self
            .list_profiles()?
            .into_iter()
            .filter(|profile| profile.enabled)
            .collect())
    }

    pub fn get_profile(&self, id: &str) -> Result<Profile, RelayError> {
        if self.read_only && !self.db_path.exists() {
            return Err(RelayError::NotFound(format!("profile not found: {id}")));
        }
        let connection = self.open()?;
        let mut statement = connection
            .prepare(
                "SELECT id, nickname, agent, priority, enabled, agent_home, config_path, auth_mode, metadata, created_at, updated_at
                 FROM profiles
                 WHERE id = ?1",
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;

        statement
            .query_row([id], Self::row_to_profile)
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    RelayError::NotFound(format!("profile not found: {id}"))
                }
                other => RelayError::Store(other.to_string()),
            })
    }

    pub fn add_profile(&self, record: AddProfileRecord) -> Result<Profile, RelayError> {
        let now = Utc::now().to_rfc3339();
        let id = format!(
            "p_{}_{}",
            Utc::now().timestamp_millis(),
            slugify(&record.nickname)
        );
        let metadata_text = json!({}).to_string();
        let auth_mode = stringify_auth_mode(&record.auth_mode);

        let connection = self.open()?;
        connection
            .execute(
                "INSERT INTO profiles (id, nickname, agent, priority, enabled, agent_home, config_path, auth_mode, metadata, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    id,
                    record.nickname,
                    stringify_agent_kind(&record.agent),
                    record.priority,
                    record.agent_home.as_ref().map(|path| path_to_string(path)),
                    record.config_path.as_ref().map(|path| path_to_string(path)),
                    auth_mode,
                    metadata_text,
                    now,
                    now,
                ],
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;

        self.get_profile(&id)
    }

    pub fn update_profile(
        &self,
        id: &str,
        update: ProfileUpdateRecord,
    ) -> Result<Profile, RelayError> {
        let current = self.get_profile(id)?;
        let connection = self.open()?;
        let updated_at = Utc::now().to_rfc3339();
        let nickname = update.nickname.unwrap_or(current.nickname);
        let priority = update.priority.unwrap_or(current.priority);
        let config_path = update
            .config_path
            .unwrap_or_else(|| current.config_path.map(PathBuf::from));
        let agent_home = update
            .agent_home
            .unwrap_or_else(|| current.agent_home.map(PathBuf::from));
        let auth_mode = update.auth_mode.unwrap_or(current.auth_mode);

        let affected = connection
            .execute(
                "UPDATE profiles
                 SET nickname = ?2, priority = ?3, agent_home = ?4, config_path = ?5, auth_mode = ?6, updated_at = ?7
                 WHERE id = ?1",
                params![
                    id,
                    nickname,
                    priority,
                    agent_home.as_ref().map(|path| path_to_string(path)),
                    config_path.as_ref().map(|path| path_to_string(path)),
                    stringify_auth_mode(&auth_mode),
                    updated_at,
                ],
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;

        if affected == 0 {
            return Err(RelayError::NotFound(format!("profile not found: {id}")));
        }

        self.get_profile(id)
    }

    pub fn remove_profile(&self, id: &str) -> Result<Profile, RelayError> {
        let profile = self.get_profile(id)?;
        let connection = self.open()?;
        let affected = connection
            .execute("DELETE FROM profiles WHERE id = ?1", [id])
            .map_err(|error| RelayError::Store(error.to_string()))?;
        connection
            .execute(
                "DELETE FROM profile_probe_identities WHERE profile_id = ?1",
                [id],
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;
        if affected == 0 {
            return Err(RelayError::NotFound(format!("profile not found: {id}")));
        }
        Ok(profile)
    }

    pub fn get_probe_identity(
        &self,
        profile_id: &str,
    ) -> Result<Option<ProfileProbeIdentity>, RelayError> {
        if self.read_only && !self.db_path.exists() {
            return Ok(None);
        }
        let connection = self.open()?;
        let mut statement = connection
            .prepare(
                "SELECT profile_id, provider, principal_id, display_name, credentials_json, metadata_json, created_at, updated_at
                 FROM profile_probe_identities
                 WHERE profile_id = ?1",
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;

        statement
            .query_row([profile_id], Self::row_to_probe_identity)
            .optional()
            .map_err(|error| RelayError::Store(error.to_string()))
    }

    pub fn upsert_probe_identity(
        &self,
        identity: &ProfileProbeIdentity,
    ) -> Result<ProfileProbeIdentity, RelayError> {
        let connection = self.open()?;
        connection
            .execute(
                "INSERT INTO profile_probe_identities
                 (profile_id, provider, principal_id, display_name, credentials_json, metadata_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(profile_id) DO UPDATE SET
                    provider = excluded.provider,
                    principal_id = excluded.principal_id,
                    display_name = excluded.display_name,
                    credentials_json = excluded.credentials_json,
                    metadata_json = excluded.metadata_json,
                    updated_at = excluded.updated_at",
                params![
                    identity.profile_id,
                    stringify_probe_provider(&identity.provider),
                    identity.principal_id,
                    identity.display_name,
                    identity.credentials.to_string(),
                    identity.metadata.to_string(),
                    identity.created_at,
                    identity.updated_at,
                ],
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;
        self.get_probe_identity(&identity.profile_id)?
            .ok_or_else(|| RelayError::Store("failed to reload probe identity".into()))
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<Profile, RelayError> {
        let connection = self.open()?;
        let affected = connection
            .execute(
                "UPDATE profiles SET enabled = ?2, updated_at = ?3 WHERE id = ?1",
                params![
                    id,
                    if enabled { 1_i64 } else { 0_i64 },
                    Utc::now().to_rfc3339()
                ],
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;
        if affected == 0 {
            return Err(RelayError::NotFound(format!("profile not found: {id}")));
        }
        self.get_profile(id)
    }

    pub fn get_settings(&self) -> Result<AppSettings, RelayError> {
        if self.read_only && !self.db_path.exists() {
            return Ok(AppSettings::default());
        }
        let connection = self.open()?;
        let auto_switch_enabled = connection
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'auto_switch_enabled'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| RelayError::Store(error.to_string()))?
            .map(|value| value == "true")
            .unwrap_or(false);
        let cooldown_seconds = connection
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'cooldown_seconds'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| RelayError::Store(error.to_string()))?
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(600);
        Ok(AppSettings {
            auto_switch_enabled,
            cooldown_seconds,
        })
    }

    pub fn set_auto_switch_enabled(&self, enabled: bool) -> Result<AppSettings, RelayError> {
        self.set_setting(
            "auto_switch_enabled",
            if enabled { "true" } else { "false" },
        )?;
        self.get_settings()
    }

    pub fn set_cooldown_seconds(&self, value: i64) -> Result<AppSettings, RelayError> {
        self.set_setting("cooldown_seconds", &value.to_string())?;
        self.get_settings()
    }

    pub fn codex_settings(&self) -> Result<CodexSettings, RelayError> {
        if self.read_only && !self.db_path.exists() {
            return Ok(CodexSettings::default());
        }
        let connection = self.open()?;
        let settings = connection
            .query_row(
                "SELECT settings_json FROM agent_settings WHERE agent = ?1",
                [stringify_agent_kind(&AgentKind::Codex)],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| RelayError::Store(error.to_string()))?
            .map(|value| {
                serde_json::from_str::<CodexSettings>(&value)
                    .map_err(|error| RelayError::Store(error.to_string()))
            })
            .transpose()?
            .unwrap_or_default();
        Ok(settings)
    }

    pub fn update_codex_settings(
        &self,
        request: CodexSettingsUpdateRequest,
    ) -> Result<CodexSettings, RelayError> {
        let current = self.codex_settings()?;
        let settings = CodexSettings {
            usage_source_mode: request
                .usage_source_mode
                .unwrap_or(current.usage_source_mode),
        };
        let timestamp = Utc::now().to_rfc3339();
        let payload = serde_json::to_string(&settings)
            .map_err(|error| RelayError::Store(error.to_string()))?;
        let connection = self.open()?;
        connection
            .execute(
                "INSERT INTO agent_settings (agent, settings_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?3)
                 ON CONFLICT(agent) DO UPDATE SET
                    settings_json = excluded.settings_json,
                    updated_at = excluded.updated_at",
                params![stringify_agent_kind(&AgentKind::Codex), payload, timestamp],
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;
        self.codex_settings()
    }

    fn set_setting(&self, key: &str, value: &str) -> Result<(), RelayError> {
        let connection = self.open()?;
        connection
            .execute(
                "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;
        Ok(())
    }

    pub fn record_switch(
        &self,
        record: SwitchHistoryRecord,
    ) -> Result<SwitchHistoryEntry, RelayError> {
        let connection = self.open()?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| RelayError::Store(error.to_string()))?;
        let entry = insert_switch_history(&transaction, record)?;
        transaction
            .commit()
            .map_err(|error| RelayError::Store(error.to_string()))?;
        Ok(entry)
    }

    pub fn record_switch_failure(
        &self,
        record: SwitchHistoryRecord,
        failure_reason: FailureReason,
        failure_message: impl AsRef<str>,
        cooldown_until: Option<DateTime<Utc>>,
    ) -> Result<(SwitchHistoryEntry, FailureEvent), RelayError> {
        let mut connection = self.open()?;
        let transaction = connection
            .transaction()
            .map_err(|error| RelayError::Store(error.to_string()))?;
        let entry = insert_switch_history(&transaction, record)?;
        let event = insert_failure_event(
            &transaction,
            entry.profile_id.as_deref(),
            failure_reason,
            failure_message,
            cooldown_until,
        )?;
        transaction
            .commit()
            .map_err(|error| RelayError::Store(error.to_string()))?;
        Ok((entry, event))
    }

    pub fn list_switch_history(&self, limit: usize) -> Result<Vec<SwitchHistoryEntry>, RelayError> {
        if self.read_only && !self.db_path.exists() {
            return Ok(Vec::new());
        }
        let connection = self.open()?;
        let mut statement = connection
            .prepare(
                "SELECT id, profile_id, previous_profile_id, outcome, reason, checkpoint_id, rollback_performed, created_at
                 FROM switch_history
                 ORDER BY created_at DESC
                 LIMIT ?1",
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;
        let rows = statement
            .query_map([limit as i64], |row| {
                let created_at_text: String = row.get(7)?;
                let created_at = DateTime::parse_from_rfc3339(&created_at_text)
                    .map(|value| value.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                Ok(SwitchHistoryEntry {
                    id: row.get(0)?,
                    profile_id: row.get(1)?,
                    previous_profile_id: row.get(2)?,
                    outcome: parse_outcome(row.get::<_, String>(3)?.as_str()),
                    reason: row.get(4)?,
                    checkpoint_id: row.get(5)?,
                    rollback_performed: row.get::<_, i64>(6)? != 0,
                    created_at,
                })
            })
            .map_err(|error| RelayError::Store(error.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| RelayError::Store(error.to_string()))
    }

    pub fn record_failure_event(
        &self,
        profile_id: Option<&str>,
        reason: FailureReason,
        message: impl AsRef<str>,
        cooldown_until: Option<DateTime<Utc>>,
    ) -> Result<FailureEvent, RelayError> {
        let connection = self.open()?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| RelayError::Store(error.to_string()))?;
        let event =
            insert_failure_event(&transaction, profile_id, reason, message, cooldown_until)?;
        transaction
            .commit()
            .map_err(|error| RelayError::Store(error.to_string()))?;
        Ok(event)
    }

    pub fn list_failure_events(&self, limit: usize) -> Result<Vec<FailureEvent>, RelayError> {
        if self.read_only && !self.db_path.exists() {
            return Ok(Vec::new());
        }
        let connection = self.open()?;
        let mut statement = connection
            .prepare(
                "SELECT id, profile_id, reason, message, cooldown_until, created_at
                 FROM failure_events
                 ORDER BY created_at DESC
                 LIMIT ?1",
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;
        let rows = statement
            .query_map([limit as i64], |row| {
                let created_at_text: String = row.get(5)?;
                let created_at = DateTime::parse_from_rfc3339(&created_at_text)
                    .map(|value| value.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let cooldown_until = row
                    .get::<_, Option<String>>(4)?
                    .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
                    .map(|value| value.with_timezone(&Utc));
                Ok(FailureEvent {
                    id: row.get(0)?,
                    profile_id: row.get(1)?,
                    reason: parse_reason(row.get::<_, String>(2)?.as_str()),
                    message: row.get(3)?,
                    cooldown_until,
                    created_at,
                })
            })
            .map_err(|error| RelayError::Store(error.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| RelayError::Store(error.to_string()))
    }

    #[cfg(test)]
    pub fn record_failure_event_for_test(
        &self,
        profile_id: &str,
        reason: FailureReason,
        message: impl AsRef<str>,
    ) -> Result<FailureEvent, RelayError> {
        let mut connection = self.open()?;
        let transaction = connection
            .transaction()
            .map_err(|error| RelayError::Store(error.to_string()))?;
        let event = insert_failure_event(&transaction, Some(profile_id), reason, message, None)?;
        transaction
            .commit()
            .map_err(|error| RelayError::Store(error.to_string()))?;
        Ok(event)
    }
}

fn insert_switch_history(
    connection: &Connection,
    record: SwitchHistoryRecord,
) -> Result<SwitchHistoryEntry, RelayError> {
    let id = format!("sw_{}", Utc::now().timestamp_millis());
    let created_at = Utc::now();
    let outcome = stringify_outcome(&record.outcome);
    let details = json!({}).to_string();

    connection
        .execute(
            "INSERT INTO switch_history (id, profile_id, previous_profile_id, outcome, reason, checkpoint_id, rollback_performed, created_at, details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                record.profile_id,
                record.previous_profile_id,
                outcome,
                record.reason,
                record.checkpoint_id,
                if record.rollback_performed { 1_i64 } else { 0_i64 },
                created_at.to_rfc3339(),
                details,
            ],
        )
        .map_err(|error| RelayError::Store(error.to_string()))?;

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

fn insert_failure_event(
    connection: &Connection,
    profile_id: Option<&str>,
    reason: FailureReason,
    message: impl AsRef<str>,
    cooldown_until: Option<DateTime<Utc>>,
) -> Result<FailureEvent, RelayError> {
    let id = format!("ev_{}", Utc::now().timestamp_millis());
    let created_at = Utc::now();
    let event = FailureEvent {
        id: id.clone(),
        profile_id: profile_id.map(ToOwned::to_owned),
        reason: reason.clone(),
        message: message.as_ref().to_string(),
        cooldown_until,
        created_at,
    };

    connection
        .execute(
            "INSERT INTO failure_events (id, profile_id, reason, message, cooldown_until, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.id,
                event.profile_id,
                stringify_reason(&event.reason),
                event.message,
                event.cooldown_until.map(|value| value.to_rfc3339()),
                event.created_at.to_rfc3339(),
            ],
        )
        .map_err(|error| RelayError::Store(error.to_string()))?;

    Ok(event)
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

fn migrate_legacy_codex_settings(connection: &Connection) -> Result<(), RelayError> {
    let existing = connection
        .query_row(
            "SELECT 1 FROM agent_settings WHERE agent = ?1",
            [stringify_agent_kind(&AgentKind::Codex)],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| RelayError::Store(error.to_string()))?;
    if existing.is_some() {
        return Ok(());
    }

    let usage_source_mode = connection
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'usage_source_mode'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| RelayError::Store(error.to_string()))?
        .as_deref()
        .map(parse_usage_source_mode)
        .transpose()?
        .unwrap_or(UsageSourceMode::Auto);
    let settings = CodexSettings { usage_source_mode };
    let timestamp = Utc::now().to_rfc3339();

    connection
        .execute(
            "INSERT INTO agent_settings (agent, settings_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?3)",
            params![
                stringify_agent_kind(&AgentKind::Codex),
                serde_json::to_string(&settings)
                    .map_err(|error| RelayError::Store(error.to_string()))?,
                timestamp,
            ],
        )
        .map_err(|error| RelayError::Store(error.to_string()))?;
    Ok(())
}

fn parse_usage_source_mode(value: &str) -> Result<UsageSourceMode, RelayError> {
    match value {
        "auto" => Ok(UsageSourceMode::Auto),
        "local" => Ok(UsageSourceMode::Local),
        "web-enhanced" => Ok(UsageSourceMode::WebEnhanced),
        other => Err(RelayError::Store(format!(
            "unsupported usage source mode: {other}"
        ))),
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
    use rusqlite::Connection;
    use tempfile::tempdir;

    #[test]
    fn profile_settings_events_and_switch_history_round_trip() {
        let temp = tempdir().expect("tempdir");
        let db_path = temp.path().join("relay.db");
        let store = SqliteStore::new(&db_path).expect("store");
        assert_eq!(
            store.schema_version().expect("schema version"),
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
            .expect("updated");
        assert_eq!(updated.nickname, "Work Updated");

        let settings = store.set_auto_switch_enabled(true).expect("settings");
        assert!(settings.auto_switch_enabled);

        let event = store
            .record_failure_event(
                Some(&created.id),
                FailureReason::ValidationFailed,
                "validation failed",
                None,
            )
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
            .expect("switch");
        assert_eq!(switch_entry.checkpoint_id.as_deref(), Some("ckpt"));

        assert_eq!(store.list_failure_events(10).expect("events").len(), 1);
        assert_eq!(store.list_switch_history(10).expect("history").len(), 1);
        assert_eq!(store.list_profiles().expect("profiles").len(), 1);
    }

    #[test]
    fn initializes_legacy_database_with_version_stamp() {
        let temp = tempdir().expect("tempdir");
        let db_path = temp.path().join("relay.db");

        let connection = Connection::open(&db_path).expect("connection");
        connection
            .execute_batch(SCHEMA_V1_SQL)
            .expect("legacy schema without user_version");

        let store = SqliteStore::new(&db_path).expect("store");
        assert_eq!(
            store.schema_version().expect("schema version"),
            CURRENT_SCHEMA_VERSION
        );
    }

    #[test]
    fn probe_identity_round_trips_generic_payload() {
        let temp = tempdir().expect("tempdir");
        let db_path = temp.path().join("relay.db");
        let store = SqliteStore::new(&db_path).expect("store");

        let identity = ProfileProbeIdentity {
            profile_id: "p_test".into(),
            provider: ProbeProvider::CodexOfficial,
            principal_id: Some("acct-123".into()),
            display_name: Some("user@example.com".into()),
            credentials: json!({
                "account_id": "acct-123",
                "access_token": "access-token",
                "refresh_token": "refresh-token"
            }),
            metadata: json!({
                "email": "user@example.com",
                "plan_hint": "team"
            }),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        let stored = store.upsert_probe_identity(&identity).expect("upsert");
        assert_eq!(stored.account_id(), Some("acct-123"));
        assert_eq!(stored.access_token(), Some("access-token"));
        assert_eq!(stored.email(), Some("user@example.com"));
        assert_eq!(stored.plan_hint(), Some("team"));
    }

    #[test]
    fn codex_settings_round_trip() {
        let temp = tempdir().expect("tempdir");
        let db_path = temp.path().join("relay.db");
        let store = SqliteStore::new(&db_path).expect("store");

        let defaults = store.codex_settings().expect("default codex settings");
        assert_eq!(defaults.usage_source_mode, UsageSourceMode::Auto);

        let updated = store
            .update_codex_settings(CodexSettingsUpdateRequest {
                usage_source_mode: Some(UsageSourceMode::WebEnhanced),
            })
            .expect("updated codex settings");
        assert_eq!(updated.usage_source_mode, UsageSourceMode::WebEnhanced);
        assert_eq!(
            store.codex_settings().expect("reloaded codex settings"),
            updated
        );
    }

    #[test]
    fn migrates_legacy_global_usage_source_mode_into_codex_settings() {
        let temp = tempdir().expect("tempdir");
        let db_path = temp.path().join("relay.db");
        let connection = Connection::open(&db_path).expect("open db");
        connection.execute_batch(SCHEMA_V4_SQL).expect("schema v4");
        connection
            .pragma_update(None, "user_version", 4)
            .expect("user version");
        connection
            .execute(
                "INSERT INTO app_settings (key, value) VALUES ('auto_switch_enabled', 'false')",
                [],
            )
            .expect("auto switch");
        connection
            .execute(
                "INSERT INTO app_settings (key, value) VALUES ('cooldown_seconds', '600')",
                [],
            )
            .expect("cooldown");
        connection
            .execute(
                "INSERT INTO app_settings (key, value) VALUES ('usage_source_mode', 'web-enhanced')",
                [],
            )
            .expect("legacy usage source mode");
        drop(connection);

        let store = SqliteStore::new(&db_path).expect("migrated store");
        assert_eq!(
            store.schema_version().expect("schema version"),
            CURRENT_SCHEMA_VERSION
        );
        assert_eq!(
            store
                .codex_settings()
                .expect("codex settings")
                .usage_source_mode,
            UsageSourceMode::WebEnhanced
        );
    }
}
