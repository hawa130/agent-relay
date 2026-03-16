use super::{ActivityEventsQuery, RelayApp, SystemSettingsUpdateRequest};
use crate::models::{AppSettings, FailureEvent, RelayError, SwitchReport, SwitchTrigger};
use crate::services::{policy_service, switch_service};

fn validate_refresh_interval_seconds(value: i64) -> Result<(), RelayError> {
    if value != 0 && !(15..=900).contains(&value) {
        return Err(RelayError::InvalidInput(
            "refresh interval must be 0 or between 15 and 900 seconds".into(),
        ));
    }
    Ok(())
}

fn validate_network_query_concurrency(value: i64) -> Result<(), RelayError> {
    if !(1..=32).contains(&value) {
        return Err(RelayError::InvalidInput(
            "network query concurrency must be between 1 and 32".into(),
        ));
    }
    Ok(())
}

fn validate_cooldown_seconds(value: i64) -> Result<(), RelayError> {
    if !(0..=86400).contains(&value) {
        return Err(RelayError::InvalidInput(
            "cooldown seconds must be between 0 and 86400".into(),
        ));
    }
    Ok(())
}

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
        let active_state = self.state_store.load().await?;
        let profiles = self.store.list_enabled_profiles().await?;
        let usage_snapshots = self.usage_store.load_all().await?;
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
        let mut state = self.state_store.load().await?;
        state.auto_switch_enabled = enabled;
        self.state_store.save(&state).await?;
        self.log_store
            .append(
                "info".into(),
                "auto_switch.updated".into(),
                format!("enabled={enabled}"),
            )
            .await?;
        Ok(settings)
    }

    pub async fn set_cooldown_seconds(&self, value: i64) -> Result<AppSettings, RelayError> {
        validate_cooldown_seconds(value)?;
        let settings = self.store.set_cooldown_seconds(value).await?;
        self.log_store
            .append(
                "info".into(),
                "cooldown.updated".into(),
                format!("seconds={value}"),
            )
            .await?;
        Ok(settings)
    }

    pub async fn set_refresh_interval_seconds(
        &self,
        value: i64,
    ) -> Result<AppSettings, RelayError> {
        validate_refresh_interval_seconds(value)?;
        let settings = self.store.set_refresh_interval_seconds(value).await?;
        self.log_store
            .append(
                "info".into(),
                "refresh_interval.updated".into(),
                format!("seconds={value}"),
            )
            .await?;
        Ok(settings)
    }

    pub async fn set_network_query_concurrency(
        &self,
        value: i64,
    ) -> Result<AppSettings, RelayError> {
        validate_network_query_concurrency(value)?;
        let settings = self.store.set_network_query_concurrency(value).await?;
        self.log_store
            .append(
                "info".into(),
                "network_query_concurrency.updated".into(),
                format!("value={value}"),
            )
            .await?;
        Ok(settings)
    }

    pub async fn update_system_settings(
        &self,
        request: SystemSettingsUpdateRequest,
    ) -> Result<AppSettings, RelayError> {
        if let Some(value) = request.refresh_interval_seconds {
            validate_refresh_interval_seconds(value)?;
        }
        if let Some(value) = request.network_query_concurrency {
            validate_network_query_concurrency(value)?;
        }

        if let Some(value) = request.cooldown_seconds {
            validate_cooldown_seconds(value)?;
        }

        if let Some(enabled) = request.auto_switch_enabled {
            self.set_auto_switch_enabled(enabled).await?;
        }
        if let Some(value) = request.cooldown_seconds {
            self.set_cooldown_seconds(value).await?;
        }
        if let Some(value) = request.refresh_interval_seconds {
            self.set_refresh_interval_seconds(value).await?;
        }
        if let Some(value) = request.network_query_concurrency {
            self.set_network_query_concurrency(value).await?;
        }
        self.store.get_settings().await
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
