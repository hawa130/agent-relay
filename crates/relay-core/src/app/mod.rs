mod profiles;
mod switching;
mod usage;

use crate::adapters::AdapterRegistry;
use crate::models::{
    AgentKind, AppSettings, DiagnosticsExport, DoctorReport, LogTail, Profile, RelayError,
    StatusReport, SystemStatusReport, UsageSnapshot, UsageSourceMode, UsageStatus,
};
use crate::platform::RelayPaths;
use crate::services::{diagnostics_service, doctor_service, status_service};
use crate::store::{FileLogStore, FileStateStore, FileUsageStore, SqliteStore};
use crate::{CodexSettings, CodexSettingsUpdateRequest};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentLoginMode {
    Browser,
    DeviceAuth,
}

#[derive(Debug, Clone)]
pub struct AddProfileRequest {
    pub agent: AgentKind,
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
    adapters: AdapterRegistry,
    bootstrap_mode: BootstrapMode,
}

#[derive(Debug, Clone)]
pub struct AgentLoginRequest {
    pub agent: AgentKind,
    pub nickname: Option<String>,
    pub priority: i32,
    pub mode: AgentLoginMode,
}

#[derive(Debug, Clone)]
pub struct ImportProfileRequest {
    pub agent: AgentKind,
    pub nickname: Option<String>,
    pub priority: i32,
}

#[derive(Debug, Clone, Default)]
pub struct SystemSettingsUpdateRequest {
    pub auto_switch_enabled: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ActivityEventsQuery {
    pub limit: usize,
    pub profile_id: Option<String>,
    pub reason: Option<crate::models::FailureReason>,
}

impl RelayApp {
    pub async fn bootstrap() -> Result<Self, RelayError> {
        Self::bootstrap_with_mode(BootstrapMode::ReadWrite).await
    }

    pub async fn bootstrap_read_only() -> Result<Self, RelayError> {
        Self::bootstrap_with_mode(BootstrapMode::ReadOnly).await
    }

    pub async fn bootstrap_with_mode(bootstrap_mode: BootstrapMode) -> Result<Self, RelayError> {
        let paths = RelayPaths::from_env()?;
        if bootstrap_mode == BootstrapMode::ReadWrite {
            paths.ensure_layout()?;
        }

        let store = match bootstrap_mode {
            BootstrapMode::ReadOnly => SqliteStore::open_read_only(&paths.db_path).await?,
            BootstrapMode::ReadWrite => SqliteStore::new(&paths.db_path).await?,
        };
        let state_store = FileStateStore::new(&paths.state_path);
        let usage_store = FileUsageStore::new(&paths.usage_path);
        let log_store = FileLogStore::new(&paths.log_file);
        let adapters = AdapterRegistry::new()?;

        Ok(Self {
            paths,
            store,
            state_store,
            usage_store,
            log_store,
            adapters,
            bootstrap_mode,
        })
    }

    pub fn doctor_report(&self) -> Result<DoctorReport, RelayError> {
        doctor_service::run(&self.paths)
    }

    pub async fn status_report(&self) -> Result<StatusReport, RelayError> {
        let active_state = self.state_store.load()?;
        let settings = self.store.get_settings().await?;
        status_service::build(
            &self.paths,
            &self.store,
            active_state,
            settings,
            self.adapters.primary(),
        )
        .await
    }

    pub async fn system_status(&self) -> Result<SystemStatusReport, RelayError> {
        let status = self.status_report().await?;
        Ok(SystemStatusReport {
            relay_home: status.relay_home,
            live_agent_home: status.live_agent_home,
            profile_count: status.profile_count,
            active_state: status.active_state,
            settings: status.settings,
        })
    }

    pub async fn settings(&self) -> Result<AppSettings, RelayError> {
        self.store.get_settings().await
    }

    pub async fn codex_settings(&self) -> Result<CodexSettings, RelayError> {
        self.store.codex_settings().await
    }

    pub async fn update_codex_settings(
        &self,
        request: CodexSettingsUpdateRequest,
    ) -> Result<CodexSettings, RelayError> {
        self.store.update_codex_settings(request).await
    }

    pub fn logs_tail(&self, lines: usize) -> Result<LogTail, RelayError> {
        self.log_store.tail(lines)
    }

    pub async fn diagnostics_export(&self) -> Result<DiagnosticsExport, RelayError> {
        let doctor = self.doctor_report()?;
        let status = self.status_report().await?;
        let active_state = self.state_store.load()?;
        let usage = self.usage_report().await?;
        diagnostics_service::export_bundle(
            &self.paths,
            &self.store,
            &self.log_store,
            &doctor,
            &status,
            &active_state,
            &usage,
        )
        .await
    }

    async fn next_enabled_profile_excluding(
        &self,
        excluded_id: &str,
    ) -> Result<Option<Profile>, RelayError> {
        Ok(self
            .store
            .list_enabled_profiles()
            .await?
            .into_iter()
            .find(|profile| profile.id != excluded_id))
    }

    async fn active_profile_from_state(
        &self,
        active_state: &crate::models::ActiveState,
    ) -> Result<Option<Profile>, RelayError> {
        match active_state.active_profile_id.as_deref() {
            Some(profile_id) => Ok(Some(self.store.get_profile(profile_id).await?)),
            None => Ok(None),
        }
    }

    async fn usage_source_mode_for_agent(
        &self,
        agent: &AgentKind,
    ) -> Result<UsageSourceMode, RelayError> {
        match agent {
            AgentKind::Codex => Ok(self.store.codex_settings().await?.usage_source_mode),
        }
    }

    fn default_usage_source_mode(&self) -> UsageSourceMode {
        match self.adapters.primary_kind() {
            AgentKind::Codex => CodexSettings::default().usage_source_mode,
        }
    }

    async fn clear_stale_active_state(&self) -> Result<(), RelayError> {
        let mut state = self.state_store.load()?;
        let Some(active_profile_id) = state.active_profile_id.as_deref() else {
            return Ok(());
        };
        if self.store.get_profile(active_profile_id).await.is_ok() {
            return Ok(());
        }

        state.active_profile_id = None;
        state.last_switch_result = crate::models::SwitchOutcome::NotRun;
        state.last_error = None;
        self.state_store.save(&state)?;
        Ok(())
    }

    fn sync_active_profile(&self, profile: &Profile) -> Result<(), RelayError> {
        let mut state = self.state_store.load()?;
        if state.active_profile_id.as_deref() == Some(profile.id.as_str()) {
            return Ok(());
        }

        state.active_profile_id = Some(profile.id.clone());
        state.last_error = None;
        self.state_store.save(&state)
    }
}

fn profile_switch_eligibility(
    profile: &Profile,
    usage: Option<&UsageSnapshot>,
) -> (bool, Option<String>) {
    if !profile.enabled {
        return (false, Some("profile is disabled".into()));
    }

    let Some(snapshot) = usage else {
        return (true, None);
    };

    if snapshot.stale {
        return (false, Some("usage snapshot is stale".into()));
    }

    if snapshot.session.status == UsageStatus::Exhausted
        || snapshot.weekly.status == UsageStatus::Exhausted
        || snapshot.auto_switch_reason.is_some()
    {
        return (
            false,
            Some("usage is exhausted or unavailable for activation".into()),
        );
    }

    (true, None)
}
