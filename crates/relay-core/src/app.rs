use crate::adapters::AdapterRegistry;
use crate::models::{
    AgentKind, AgentLinkResult, AppSettings, DiagnosticsExport, DoctorReport, FailureEvent,
    FailureReason, LogTail, Profile, ProfileDetail, ProfileListItem, ProfileProbeIdentity,
    RelayError, StatusReport, SwitchReport, SystemStatusReport, UsageSnapshot, UsageSourceMode,
    UsageStatus,
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
use crate::{CodexSettings, CodexSettingsUpdateRequest};
use std::fs;
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
    pub reason: Option<FailureReason>,
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

    pub async fn list_profiles(&self) -> Result<Vec<Profile>, RelayError> {
        self.store.list_profiles().await
    }

    pub async fn list_profiles_with_usage(&self) -> Result<Vec<ProfileListItem>, RelayError> {
        let profiles = self.store.list_profiles().await?;
        let active_state = self.state_store.load()?;
        let snapshots = usage_service::list_profile_snapshots(&self.usage_store, &profiles)?;
        let items = profiles
            .into_iter()
            .zip(snapshots)
            .map(|(profile, usage_summary)| ProfileListItem {
                is_active: active_state.active_profile_id.as_deref() == Some(profile.id.as_str()),
                profile,
                usage_summary: Some(usage_summary),
            })
            .collect();
        Ok(items)
    }

    pub async fn profile_detail(&self, id: &str) -> Result<ProfileDetail, RelayError> {
        let profile = self.store.get_profile(id).await?;
        let active_state = self.state_store.load()?;
        let usage = self.usage_store.load_profile(id)?;
        let last_failure_event = self
            .store
            .list_failure_events(200)
            .await?
            .into_iter()
            .find(|event| event.profile_id.as_deref() == Some(id));
        let (switch_eligible, switch_ineligibility_reason) =
            profile_switch_eligibility(&profile, usage.as_ref());

        Ok(ProfileDetail {
            is_active: active_state.active_profile_id.as_deref() == Some(id),
            profile,
            usage,
            last_failure_event,
            switch_eligible,
            switch_ineligibility_reason,
        })
    }

    pub async fn current_profile_detail(&self) -> Result<ProfileDetail, RelayError> {
        let active_state = self.state_store.load()?;
        let active_profile_id = active_state
            .active_profile_id
            .ok_or_else(|| RelayError::NotFound("no active profile".into()))?;
        self.profile_detail(&active_profile_id).await
    }

    pub async fn add_profile(&self, request: AddProfileRequest) -> Result<Profile, RelayError> {
        let adapter = self.adapters.adapter(&request.agent);
        let profile = profile_service::add_profile(
            &self.store,
            adapter,
            AddProfileRecord {
                agent: request.agent,
                nickname: request.nickname,
                priority: request.priority,
                config_path: request.config_path,
                agent_home: request.agent_home,
                auth_mode: request.auth_mode,
            },
        )
        .await?;
        self.log_store
            .append("info", "profile.added", format!("id={}", profile.id))?;
        Ok(profile)
    }

    pub async fn edit_profile(
        &self,
        id: &str,
        request: EditProfileRequest,
    ) -> Result<Profile, RelayError> {
        let current = self.store.get_profile(id).await?;
        let adapter = self.adapters.adapter(&current.agent);
        let profile = profile_service::edit_profile(
            &self.store,
            adapter,
            id,
            ProfileUpdateRecord {
                nickname: request.nickname,
                priority: request.priority,
                config_path: request.config_path,
                agent_home: request.agent_home,
                auth_mode: request.auth_mode,
            },
        )
        .await?;
        self.log_store
            .append("info", "profile.updated", format!("id={}", profile.id))?;
        Ok(profile)
    }

    pub async fn import_profile(
        &self,
        request: ImportProfileRequest,
    ) -> Result<Profile, RelayError> {
        let adapter = self.adapters.adapter(&request.agent);
        let profile = adapter
            .import_profile(&self.store, &self.paths, request.nickname, request.priority)
            .await?;
        self.sync_active_profile(&profile)?;
        self.log_store
            .append("info", "profile.imported", format!("id={}", profile.id))?;
        Ok(profile)
    }

    pub async fn login_profile(
        &self,
        request: AgentLoginRequest,
    ) -> Result<AgentLinkResult, RelayError> {
        let adapter = self.adapters.adapter(&request.agent);
        let result = adapter
            .login_profile(
                &self.store,
                &self.paths.profiles_dir,
                request.nickname,
                request.priority,
                request.mode,
            )
            .await?;
        let _ = self.refresh_usage_profile(&result.profile.id).await;
        self.log_store.append(
            "info",
            "profile.logged_in",
            format!("id={}", result.profile.id),
        )?;
        Ok(result)
    }

    pub async fn relink_profile(
        &self,
        agent: AgentKind,
        id: &str,
    ) -> Result<ProfileProbeIdentity, RelayError> {
        let profile = self.store.get_profile(id).await?;
        if profile.agent != agent {
            return Err(RelayError::Conflict(format!(
                "profile {} belongs to {:?}, not {:?}",
                profile.id, profile.agent, agent
            )));
        }
        let adapter = self.adapters.adapter(&agent);
        let identity = adapter.relink_profile(&self.store, &profile).await?;
        let _ = self.refresh_usage_profile(id).await;
        self.log_store
            .append("info", "profile.relinked", format!("id={id}"))?;
        Ok(identity)
    }

    pub async fn remove_profile(&self, id: &str) -> Result<Profile, RelayError> {
        let active_state = self.state_store.load()?;
        let removing_active = active_state.active_profile_id.as_deref() == Some(id);
        if removing_active {
            if let Some(replacement) = self.next_enabled_profile_excluding(id).await? {
                self.switch_to_profile(&replacement.id).await?;
            }
        }

        let profile = profile_service::remove_profile(&self.store, id).await?;
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

    pub async fn set_profile_enabled(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<Profile, RelayError> {
        let profile = profile_service::set_profile_enabled(&self.store, id, enabled).await?;
        self.log_store.append(
            "info",
            "profile.enabled_changed",
            format!("id={} enabled={enabled}", profile.id),
        )?;
        Ok(profile)
    }

    pub async fn switch_to_profile(&self, id: &str) -> Result<SwitchReport, RelayError> {
        let profile = self.store.get_profile(id).await?;
        let adapter = self.adapters.adapter(&profile.agent);
        switch_service::switch_to_profile(
            &self.store,
            &self.state_store,
            &self.log_store,
            adapter,
            &self.paths,
            &profile,
        )
        .await
    }

    pub async fn switch_next_profile(&self) -> Result<SwitchReport, RelayError> {
        let active_state = self.state_store.load()?;
        let profiles = self.store.list_enabled_profiles().await?;
        let usage_snapshots = self.usage_store.load_all()?;
        let events = self.store.list_failure_events(100).await?;
        let next = policy_service::select_next_profile(
            &profiles,
            &usage_snapshots,
            active_state.active_profile_id.as_deref(),
            &events,
        )?;
        let adapter = self.adapters.adapter(&next.agent);
        switch_service::switch_to_profile(
            &self.store,
            &self.state_store,
            &self.log_store,
            adapter,
            &self.paths,
            &next,
        )
        .await
    }

    pub async fn set_auto_switch_enabled(&self, enabled: bool) -> Result<AppSettings, RelayError> {
        let settings = self.store.set_auto_switch_enabled(enabled).await?;
        let mut state = self.state_store.load()?;
        state.auto_switch_enabled = enabled;
        self.state_store.save(&state)?;
        self.log_store
            .append("info", "auto_switch.updated", format!("enabled={enabled}"))?;
        Ok(settings)
    }

    pub async fn update_system_settings(
        &self,
        request: SystemSettingsUpdateRequest,
    ) -> Result<AppSettings, RelayError> {
        if let Some(enabled) = request.auto_switch_enabled {
            return self.set_auto_switch_enabled(enabled).await;
        }
        self.store.get_settings().await
    }

    pub async fn list_failure_events(&self, limit: usize) -> Result<Vec<FailureEvent>, RelayError> {
        events_service::list_failure_events(&self.store, limit).await
    }

    pub async fn list_activity_events(
        &self,
        query: ActivityEventsQuery,
    ) -> Result<Vec<FailureEvent>, RelayError> {
        let mut events = self.store.list_failure_events(query.limit.max(200)).await?;
        if let Some(profile_id) = query.profile_id.as_deref() {
            events.retain(|event| event.profile_id.as_deref() == Some(profile_id));
        }
        if let Some(reason) = query.reason.as_ref() {
            events.retain(|event| &event.reason == reason);
        }
        events.truncate(query.limit);
        Ok(events)
    }

    pub async fn usage_report(&self) -> Result<UsageSnapshot, RelayError> {
        let active_state = self.state_store.load()?;
        let active_profile = match active_state.active_profile_id.as_deref() {
            Some(profile_id) => Some(self.store.get_profile(profile_id).await?),
            None => None,
        };
        let provider = active_profile
            .as_ref()
            .map(|profile| self.adapters.usage_provider(&profile.agent))
            .unwrap_or_else(|| self.adapters.primary_usage_provider());
        let source_mode = if let Some(profile) = active_profile.as_ref() {
            self.usage_source_mode_for_agent(&profile.agent).await?
        } else {
            self.default_usage_source_mode()
        };
        usage_service::build_active(
            &self.store,
            &self.usage_store,
            provider,
            active_profile.as_ref(),
            source_mode,
            self.bootstrap_mode == BootstrapMode::ReadWrite,
        )
        .await
    }

    pub async fn profile_usage_report(&self, id: &str) -> Result<UsageSnapshot, RelayError> {
        let profile = self.store.get_profile(id).await?;
        usage_service::load_profile_snapshot(&self.usage_store, &profile)
    }

    pub async fn list_usage_reports(&self) -> Result<Vec<UsageSnapshot>, RelayError> {
        let profiles = self.store.list_profiles().await?;
        usage_service::list_profile_snapshots(&self.usage_store, &profiles)
    }

    pub async fn refresh_usage_profile(&self, id: &str) -> Result<UsageSnapshot, RelayError> {
        let active_state = self.state_store.load()?;
        let active_profile = match active_state.active_profile_id.as_deref() {
            Some(profile_id) => Some(self.store.get_profile(profile_id).await?),
            None => None,
        };
        let profile = self.store.get_profile(id).await?;
        let provider = self.adapters.usage_provider(&profile.agent);
        usage_service::refresh_profile(
            &self.store,
            &self.usage_store,
            provider,
            Some(&profile),
            active_profile.as_ref(),
            self.usage_source_mode_for_agent(&profile.agent).await?,
            self.bootstrap_mode == BootstrapMode::ReadWrite,
        )
        .await
    }

    pub async fn refresh_enabled_usage_reports(&self) -> Result<Vec<UsageSnapshot>, RelayError> {
        let profiles = self.store.list_enabled_profiles().await?;
        self.refresh_usage_for_profiles(&profiles).await
    }

    pub async fn refresh_all_usage_reports(&self) -> Result<Vec<UsageSnapshot>, RelayError> {
        let profiles = self.store.list_profiles().await?;
        self.refresh_usage_for_profiles(&profiles).await
    }

    pub async fn update_codex_settings(
        &self,
        request: CodexSettingsUpdateRequest,
    ) -> Result<CodexSettings, RelayError> {
        self.store.update_codex_settings(request).await
    }

    async fn refresh_usage_for_profiles(
        &self,
        profiles: &[Profile],
    ) -> Result<Vec<UsageSnapshot>, RelayError> {
        let active_state = self.state_store.load()?;
        let active_profile = match active_state.active_profile_id.as_deref() {
            Some(profile_id) => Some(self.store.get_profile(profile_id).await?),
            None => None,
        };
        let mut snapshots = Vec::with_capacity(profiles.len());
        for profile in profiles {
            let provider = self.adapters.usage_provider(&profile.agent);
            snapshots.push(
                usage_service::refresh_profile(
                    &self.store,
                    &self.usage_store,
                    provider,
                    Some(profile),
                    active_profile.as_ref(),
                    self.usage_source_mode_for_agent(&profile.agent).await?,
                    self.bootstrap_mode == BootstrapMode::ReadWrite,
                )
                .await?,
            );
        }
        Ok(snapshots)
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

    fn sync_active_profile(&self, profile: &Profile) -> Result<(), RelayError> {
        let mut state = self.state_store.load()?;
        if state.active_profile_id.as_deref() == Some(profile.id.as_str()) {
            return Ok(());
        }

        state.active_profile_id = Some(profile.id.clone());
        state.last_error = None;
        self.state_store.save(&state)
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
