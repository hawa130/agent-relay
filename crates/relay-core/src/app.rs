use crate::adapters::CodexAdapter;
use crate::models::{
    AppSettings, DiagnosticsExport, DoctorReport, FailureEvent, LogTail, Profile, RelayError,
    StatusReport, SwitchReport, UsageSnapshot,
};
use crate::platform::RelayPaths;
use crate::services::{
    diagnostics_service, doctor_service, events_service, policy_service, profile_service,
    status_service, switch_service, usage_service,
};
use crate::store::{
    AddProfileRecord, FileLogStore, FileStateStore, FileUsageStore, ProfileUpdateRecord,
    SqliteStore,
};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone)]
pub struct AddProfileRequest {
    pub nickname: String,
    pub priority: i32,
    pub config_path: Option<PathBuf>,
    pub agent_home: Option<PathBuf>,
    pub auth_mode: crate::models::AuthMode,
}

#[derive(Debug, Clone, Default)]
pub struct EditProfileRequest {
    pub nickname: Option<String>,
    pub priority: Option<i32>,
    pub config_path: Option<Option<PathBuf>>,
    pub agent_home: Option<Option<PathBuf>>,
    pub auth_mode: Option<crate::models::AuthMode>,
}

pub struct RelayApp {
    paths: RelayPaths,
    store: SqliteStore,
    state_store: FileStateStore,
    usage_store: FileUsageStore,
    log_store: FileLogStore,
    codex_adapter: CodexAdapter,
    bootstrap_mode: BootstrapMode,
}

impl RelayApp {
    pub fn bootstrap() -> Result<Self, RelayError> {
        Self::bootstrap_with_mode(BootstrapMode::ReadWrite)
    }

    pub fn bootstrap_read_only() -> Result<Self, RelayError> {
        Self::bootstrap_with_mode(BootstrapMode::ReadOnly)
    }

    pub fn bootstrap_with_mode(bootstrap_mode: BootstrapMode) -> Result<Self, RelayError> {
        let paths = RelayPaths::from_env()?;
        if bootstrap_mode == BootstrapMode::ReadWrite {
            paths.ensure_layout()?;
        }

        let store = match bootstrap_mode {
            BootstrapMode::ReadOnly => SqliteStore::open_read_only(&paths.db_path),
            BootstrapMode::ReadWrite => SqliteStore::new(&paths.db_path)?,
        };
        let state_store = FileStateStore::new(&paths.state_path);
        let usage_store = FileUsageStore::new(&paths.usage_path);
        let log_store = FileLogStore::new(&paths.log_file);
        let codex_adapter = CodexAdapter::new()?;

        Ok(Self {
            paths,
            store,
            state_store,
            usage_store,
            log_store,
            codex_adapter,
            bootstrap_mode,
        })
    }

    pub fn doctor_report(&self) -> Result<DoctorReport, RelayError> {
        doctor_service::run(&self.paths)
    }

    pub fn status_report(&self) -> Result<StatusReport, RelayError> {
        let active_state = self.state_store.load()?;
        let settings = self.store.get_settings()?;
        status_service::build(
            &self.paths,
            &self.store,
            active_state,
            settings,
            &self.codex_adapter,
        )
    }

    pub fn settings(&self) -> Result<AppSettings, RelayError> {
        self.store.get_settings()
    }

    pub fn list_profiles(&self) -> Result<Vec<Profile>, RelayError> {
        self.store.list_profiles()
    }

    pub fn add_profile(&self, request: AddProfileRequest) -> Result<Profile, RelayError> {
        let profile = profile_service::add_profile(
            &self.store,
            &self.codex_adapter,
            AddProfileRecord {
                nickname: request.nickname,
                priority: request.priority,
                config_path: request.config_path,
                codex_home: request.agent_home,
                auth_mode: request.auth_mode,
            },
        )?;
        self.log_store
            .append("info", "profile.added", format!("id={}", profile.id))?;
        Ok(profile)
    }

    pub fn edit_profile(
        &self,
        id: &str,
        request: EditProfileRequest,
    ) -> Result<Profile, RelayError> {
        let profile = profile_service::edit_profile(
            &self.store,
            &self.codex_adapter,
            id,
            ProfileUpdateRecord {
                nickname: request.nickname,
                priority: request.priority,
                config_path: request.config_path,
                codex_home: request.agent_home,
                auth_mode: request.auth_mode,
            },
        )?;
        self.log_store
            .append("info", "profile.updated", format!("id={}", profile.id))?;
        Ok(profile)
    }

    pub fn import_codex_profile(
        &self,
        nickname: Option<String>,
        priority: i32,
    ) -> Result<Profile, RelayError> {
        let profile = profile_service::import_codex_profile(
            &self.store,
            &self.codex_adapter,
            &self.paths,
            nickname,
            priority,
        )?;
        self.log_store
            .append("info", "profile.imported", format!("id={}", profile.id))?;
        Ok(profile)
    }

    pub fn remove_profile(&self, id: &str) -> Result<Profile, RelayError> {
        let profile = profile_service::remove_profile(&self.store, id)?;
        if let Some(home) = profile.agent_home.as_ref() {
            let path = PathBuf::from(home);
            if path.starts_with(&self.paths.profiles_dir) && path.exists() {
                fs::remove_dir_all(path)?;
            }
        }
        let mut state = self.state_store.load()?;
        if state.active_profile_id.as_deref() == Some(profile.id.as_str()) {
            state.active_profile_id = None;
            state.last_switch_result = crate::models::SwitchOutcome::NotRun;
            state.last_error = None;
            self.state_store.save(&state)?;
        }
        self.log_store
            .append("info", "profile.removed", format!("id={}", profile.id))?;
        Ok(profile)
    }

    pub fn set_profile_enabled(&self, id: &str, enabled: bool) -> Result<Profile, RelayError> {
        let profile = profile_service::set_profile_enabled(&self.store, id, enabled)?;
        self.log_store.append(
            "info",
            "profile.enabled_changed",
            format!("id={} enabled={enabled}", profile.id),
        )?;
        Ok(profile)
    }

    pub fn switch_to_profile(&self, id: &str) -> Result<SwitchReport, RelayError> {
        let profile = self.store.get_profile(id)?;
        switch_service::switch_to_profile(
            &self.store,
            &self.state_store,
            &self.log_store,
            &self.codex_adapter,
            &self.paths,
            &profile,
        )
    }

    pub fn switch_next_profile(&self) -> Result<SwitchReport, RelayError> {
        let active_state = self.state_store.load()?;
        let profiles = self.store.list_enabled_profiles()?;
        let events = self.store.list_failure_events(100)?;
        let next = policy_service::select_next_profile(
            &profiles,
            active_state.active_profile_id.as_deref(),
            &events,
        )?;
        switch_service::switch_to_profile(
            &self.store,
            &self.state_store,
            &self.log_store,
            &self.codex_adapter,
            &self.paths,
            &next,
        )
    }

    pub fn set_auto_switch_enabled(&self, enabled: bool) -> Result<AppSettings, RelayError> {
        let settings = self.store.set_auto_switch_enabled(enabled)?;
        let mut state = self.state_store.load()?;
        state.auto_switch_enabled = enabled;
        self.state_store.save(&state)?;
        self.log_store
            .append("info", "auto_switch.updated", format!("enabled={enabled}"))?;
        Ok(settings)
    }

    pub fn list_failure_events(&self, limit: usize) -> Result<Vec<FailureEvent>, RelayError> {
        events_service::list_failure_events(&self.store, limit)
    }

    pub fn usage_report(&self) -> Result<UsageSnapshot, RelayError> {
        let active_state = self.state_store.load()?;
        let active_profile = active_state
            .active_profile_id
            .as_deref()
            .map(|profile_id| self.store.get_profile(profile_id))
            .transpose()?;
        usage_service::build(
            &self.store,
            &self.usage_store,
            active_profile.as_ref(),
            self.codex_adapter.live_home(),
            self.bootstrap_mode == BootstrapMode::ReadWrite,
        )
    }

    pub fn logs_tail(&self, lines: usize) -> Result<LogTail, RelayError> {
        self.log_store.tail(lines)
    }

    pub fn diagnostics_export(&self) -> Result<DiagnosticsExport, RelayError> {
        let doctor = self.doctor_report()?;
        let status = self.status_report()?;
        let active_state = self.state_store.load()?;
        let usage = self.usage_report()?;
        diagnostics_service::export_bundle(
            &self.paths,
            &self.store,
            &self.log_store,
            &doctor,
            &status,
            &active_state,
            &usage,
        )
    }
}
