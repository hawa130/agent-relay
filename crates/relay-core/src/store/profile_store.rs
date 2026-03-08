use crate::models::{
    AppSettings, FailureEvent, FailureReason, Profile, RelayError, SwitchHistoryEntry,
    SwitchOutcome,
};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AddProfileRecord {
    pub nickname: String,
    pub priority: i32,
    pub config_path: Option<PathBuf>,
    pub codex_home: Option<PathBuf>,
    pub auth_mode: crate::models::AuthMode,
}

#[derive(Debug, Clone, Default)]
pub struct ProfileUpdateRecord {
    pub nickname: Option<String>,
    pub priority: Option<i32>,
    pub config_path: Option<Option<PathBuf>>,
    pub codex_home: Option<Option<PathBuf>>,
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
}

impl SqliteStore {
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self, RelayError> {
        let db_path = db_path.as_ref().to_path_buf();
        let store = Self { db_path };
        store.initialize()?;
        Ok(store)
    }

    fn open(&self) -> Result<Connection, RelayError> {
        Connection::open(&self.db_path).map_err(|error| RelayError::Store(error.to_string()))
    }

    fn initialize(&self) -> Result<(), RelayError> {
        let connection = self.open()?;
        connection
            .execute_batch(
                "BEGIN;
                CREATE TABLE IF NOT EXISTS profiles (
                    id TEXT PRIMARY KEY,
                    nickname TEXT NOT NULL,
                    agent TEXT NOT NULL,
                    priority INTEGER NOT NULL,
                    enabled INTEGER NOT NULL,
                    codex_home TEXT,
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
                );
                COMMIT;",
            )
            .map_err(|error| RelayError::Store(error.to_string()))?;
        self.ensure_default_settings()?;
        Ok(())
    }

    fn ensure_default_settings(&self) -> Result<(), RelayError> {
        let defaults = AppSettings::default();
        let connection = self.open()?;
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
        Ok(())
    }

    fn row_to_profile(row: &rusqlite::Row<'_>) -> Result<Profile, rusqlite::Error> {
        let auth_mode = parse_auth_mode(row.get::<_, String>(7)?.as_str());
        let metadata_text: String = row.get(8)?;
        let metadata: Value = serde_json::from_str(&metadata_text).unwrap_or(Value::Null);

        Ok(Profile {
            id: row.get(0)?,
            nickname: row.get(1)?,
            agent: crate::models::AgentKind::Codex,
            priority: row.get(3)?,
            enabled: row.get::<_, i64>(4)? != 0,
            codex_home: row.get(5)?,
            config_path: row.get(6)?,
            auth_mode,
            metadata,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    }

    pub fn list_profiles(&self) -> Result<Vec<Profile>, RelayError> {
        let connection = self.open()?;
        let mut statement = connection
            .prepare(
                "SELECT id, nickname, agent, priority, enabled, codex_home, config_path, auth_mode, metadata, created_at, updated_at
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
        let connection = self.open()?;
        let mut statement = connection
            .prepare(
                "SELECT id, nickname, agent, priority, enabled, codex_home, config_path, auth_mode, metadata, created_at, updated_at
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
                "INSERT INTO profiles (id, nickname, agent, priority, enabled, codex_home, config_path, auth_mode, metadata, created_at, updated_at)
                 VALUES (?1, ?2, 'codex', ?3, 1, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    id,
                    record.nickname,
                    record.priority,
                    record.codex_home.as_ref().map(path_to_string),
                    record.config_path.as_ref().map(path_to_string),
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
        let codex_home = update
            .codex_home
            .unwrap_or_else(|| current.codex_home.map(PathBuf::from));
        let auth_mode = update.auth_mode.unwrap_or(current.auth_mode);

        let affected = connection
            .execute(
                "UPDATE profiles
                 SET nickname = ?2, priority = ?3, codex_home = ?4, config_path = ?5, auth_mode = ?6, updated_at = ?7
                 WHERE id = ?1",
                params![
                    id,
                    nickname,
                    priority,
                    codex_home.as_ref().map(path_to_string),
                    config_path.as_ref().map(path_to_string),
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
        if affected == 0 {
            return Err(RelayError::NotFound(format!("profile not found: {id}")));
        }
        Ok(profile)
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
        let id = format!("sw_{}", Utc::now().timestamp_millis());
        let created_at = Utc::now();
        let outcome = stringify_outcome(&record.outcome);
        let details = json!({}).to_string();

        let connection = self.open()?;
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

    pub fn list_switch_history(&self, limit: usize) -> Result<Vec<SwitchHistoryEntry>, RelayError> {
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

        let connection = self.open()?;
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

    pub fn list_failure_events(&self, limit: usize) -> Result<Vec<FailureEvent>, RelayError> {
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

fn stringify_reason(reason: &FailureReason) -> &'static str {
    match reason {
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

fn path_to_string(path: &PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn profile_settings_events_and_switch_history_round_trip() {
        let temp = tempdir().expect("tempdir");
        let db_path = temp.path().join("relay.db");
        let store = SqliteStore::new(&db_path).expect("store");

        let created = store
            .add_profile(AddProfileRecord {
                nickname: "Work".into(),
                priority: 10,
                config_path: Some(temp.path().join("config.toml")),
                codex_home: Some(temp.path().join(".codex-work")),
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
}
