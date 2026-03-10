use super::{
    AddProfileRequest, AgentLoginRequest, EditProfileRequest, ImportProfileRequest, RelayApp,
    profile_switch_eligibility,
};
use crate::models::{
    AgentKind, AgentLinkResult, Profile, ProfileDetail, ProfileListItem, ProfileProbeIdentity,
    ProfileRecoveryReport, RelayError,
};
use crate::services::{profile_service, usage_service};
use crate::store::{AddProfileRecord, ProfileUpdateRecord};
use std::fs;
use std::path::PathBuf;

impl RelayApp {
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

    pub async fn recover_profiles(
        &self,
        agent: AgentKind,
    ) -> Result<ProfileRecoveryReport, RelayError> {
        let adapter = self.adapters.adapter(&agent);
        let report = adapter.recover_profiles(&self.store, &self.paths).await?;
        self.clear_stale_active_state().await?;
        self.log_store.append(
            "info",
            "profile.recovered",
            format!("agent={agent:?} recovered={}", report.recovered.len()),
        )?;
        Ok(report)
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
}
