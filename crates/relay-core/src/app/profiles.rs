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
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

impl RelayApp {
    pub async fn profile(&self, id: &str) -> Result<Profile, RelayError> {
        self.store.get_profile(id).await
    }

    pub async fn list_profiles(&self) -> Result<Vec<Profile>, RelayError> {
        self.store.list_profiles().await
    }

    pub async fn list_profiles_with_usage(&self) -> Result<Vec<ProfileListItem>, RelayError> {
        let profiles = self.store.list_profiles().await?;
        let active_state = self.state_store.load().await?;
        let snapshots = usage_service::list_profile_snapshots(&self.usage_store, &profiles).await?;
        let current_failure_events = self.store.list_current_failure_events(None).await?;
        let mut current_events_by_profile =
            std::collections::HashMap::<String, Vec<crate::FailureEvent>>::new();
        for event in current_failure_events {
            if let Some(profile_id) = event.profile_id.clone() {
                current_events_by_profile
                    .entry(profile_id)
                    .or_default()
                    .push(event);
            }
        }
        let items = profiles
            .into_iter()
            .zip(snapshots)
            .map(|(profile, usage_summary)| ProfileListItem {
                is_active: active_state.active_profile_id.as_deref() == Some(profile.id.as_str()),
                current_failure_events: current_events_by_profile
                    .remove(&profile.id)
                    .unwrap_or_default(),
                profile,
                usage_summary: Some(usage_summary),
            })
            .collect();
        Ok(items)
    }

    pub async fn profile_detail(&self, id: &str) -> Result<ProfileDetail, RelayError> {
        let profile = self.store.get_profile(id).await?;
        let active_state = self.state_store.load().await?;
        let usage = self.usage_store.load_profile(id).await?;
        let current_failure_events = self.store.list_current_failure_events(Some(id)).await?;
        let (switch_eligible, switch_ineligibility_reason) =
            profile_switch_eligibility(&profile, usage.as_ref());

        Ok(ProfileDetail {
            is_active: active_state.active_profile_id.as_deref() == Some(id),
            profile,
            usage,
            current_failure_events,
            switch_eligible,
            switch_ineligibility_reason,
        })
    }

    pub async fn current_profile_detail(&self) -> Result<ProfileDetail, RelayError> {
        let active_state = self.state_store.load().await?;
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
            .append(
                "info".into(),
                "profile.added".into(),
                format!("id={}", profile.id),
            )
            .await?;
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
            .append(
                "info".into(),
                "profile.updated".into(),
                format!("id={}", profile.id),
            )
            .await?;
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
        self.sync_active_profile(&profile).await?;
        self.log_store
            .append(
                "info".into(),
                "profile.imported".into(),
                format!("id={}", profile.id),
            )
            .await?;
        Ok(profile)
    }

    pub async fn login_profile(
        &self,
        request: AgentLoginRequest,
    ) -> Result<AgentLinkResult, RelayError> {
        self.login_profile_cancellable(request, Arc::new(AtomicBool::new(false)))
            .await
    }

    pub async fn login_profile_cancellable(
        &self,
        request: AgentLoginRequest,
        cancel_requested: Arc<AtomicBool>,
    ) -> Result<AgentLinkResult, RelayError> {
        let adapter = self.adapters.adapter(&request.agent);
        let result = adapter
            .login_profile(
                &self.store,
                &self.paths.profiles_dir,
                request.nickname,
                request.priority,
                request.mode,
                cancel_requested,
            )
            .await?;
        let _ = self.refresh_usage_profile(&result.profile.id).await;
        self.log_store
            .append(
                "info".into(),
                "profile.logged_in".into(),
                format!("id={}", result.profile.id),
            )
            .await?;
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
            .append("info".into(), "profile.relinked".into(), format!("id={id}"))
            .await?;
        Ok(identity)
    }

    pub async fn recover_profiles(
        &self,
        agent: AgentKind,
    ) -> Result<ProfileRecoveryReport, RelayError> {
        let adapter = self.adapters.adapter(&agent);
        let report = adapter.recover_profiles(&self.store, &self.paths).await?;
        self.clear_stale_active_state().await?;
        self.log_store
            .append(
                "info".into(),
                "profile.recovered".into(),
                format!("agent={agent:?} recovered={}", report.recovered.len()),
            )
            .await?;
        Ok(report)
    }

    pub async fn remove_profile(&self, id: &str) -> Result<Profile, RelayError> {
        let active_state = self.state_store.load().await?;
        let removing_active = active_state.active_profile_id.as_deref() == Some(id);
        if removing_active {
            if let Some(replacement) = self.next_enabled_profile_excluding(id).await? {
                self.switch_to_profile(&replacement.id).await?;
            }
        }

        let profile = profile_service::remove_profile(&self.store, id).await?;
        if let Some(home) = profile.agent_home.as_ref() {
            let path = PathBuf::from(home);
            let profiles_dir = self.paths.profiles_dir.clone();
            if path.starts_with(&profiles_dir) && path.exists() {
                let path_owned = path.clone();
                tokio::task::spawn_blocking(move || std::fs::remove_dir_all(path_owned))
                    .await
                    .map_err(|e| RelayError::Internal(format!("blocking task failed: {e}")))?
                    .map_err(RelayError::from)?;
            }
        }
        let mut state = self.state_store.load().await?;
        if state.active_profile_id.as_deref() == Some(profile.id.as_str()) {
            state.active_profile_id = None;
            state.last_switch_result = crate::models::SwitchOutcome::NotRun;
            self.state_store.save(&state).await?;
        }
        self.log_store
            .append(
                "info".into(),
                "profile.removed".into(),
                format!("id={}", profile.id),
            )
            .await?;
        Ok(profile)
    }

    pub async fn set_profile_enabled(
        &self,
        id: &str,
        enabled: bool,
    ) -> Result<Profile, RelayError> {
        let profile = profile_service::set_profile_enabled(&self.store, id, enabled).await?;
        self.log_store
            .append(
                "info".into(),
                "profile.enabled_changed".into(),
                format!("id={} enabled={enabled}", profile.id),
            )
            .await?;
        Ok(profile)
    }
}
