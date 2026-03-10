use super::{ActivityEventsQuery, RelayApp, SystemSettingsUpdateRequest};
use crate::models::{AppSettings, FailureEvent, RelayError, SwitchReport, SwitchTrigger};
use crate::services::{events_service, policy_service, switch_service};

impl RelayApp {
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
            SwitchTrigger::Manual,
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
            SwitchTrigger::Auto,
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

    pub async fn set_cooldown_seconds(&self, value: i64) -> Result<AppSettings, RelayError> {
        let settings = self.store.set_cooldown_seconds(value).await?;
        self.log_store
            .append("info", "cooldown.updated", format!("seconds={value}"))?;
        Ok(settings)
    }

    pub async fn set_refresh_interval_seconds(
        &self,
        value: i64,
    ) -> Result<AppSettings, RelayError> {
        if !(15..=900).contains(&value) {
            return Err(RelayError::InvalidInput(
                "refresh interval must be between 15 and 900 seconds".into(),
            ));
        }
        let settings = self.store.set_refresh_interval_seconds(value).await?;
        self.log_store.append(
            "info",
            "refresh_interval.updated",
            format!("seconds={value}"),
        )?;
        Ok(settings)
    }

    pub async fn update_system_settings(
        &self,
        request: SystemSettingsUpdateRequest,
    ) -> Result<AppSettings, RelayError> {
        if let Some(enabled) = request.auto_switch_enabled {
            return self.set_auto_switch_enabled(enabled).await;
        }
        if let Some(value) = request.cooldown_seconds {
            return self.set_cooldown_seconds(value).await;
        }
        if let Some(value) = request.refresh_interval_seconds {
            return self.set_refresh_interval_seconds(value).await;
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
}
