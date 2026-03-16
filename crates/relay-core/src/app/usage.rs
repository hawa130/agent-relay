use super::{BootstrapMode, RelayApp};
use crate::models::{FailureReason, Profile, RelayError, UsageSnapshot, UsageStatus};
use crate::services::usage_service;
use futures_util::stream::{self, StreamExt};

#[derive(Clone)]
struct UsageContext {
    active_profile: Option<Profile>,
    allow_cache_writes: bool,
}

impl RelayApp {
    pub async fn usage_report(&self) -> Result<UsageSnapshot, RelayError> {
        let context = self.usage_context().await?;
        let provider = context
            .active_profile
            .as_ref()
            .map(|profile| self.adapters.usage_provider(&profile.agent))
            .unwrap_or_else(|| self.adapters.primary_usage_provider());
        let source_mode = if let Some(profile) = context.active_profile.as_ref() {
            self.usage_source_mode_for_agent(&profile.agent).await?
        } else {
            self.default_usage_source_mode()
        };
        usage_service::build_active(
            &self.store,
            &self.usage_store,
            provider,
            context.active_profile.as_ref(),
            source_mode,
            context.allow_cache_writes,
        )
        .await
    }

    pub async fn profile_usage_report(&self, id: &str) -> Result<UsageSnapshot, RelayError> {
        let profile = self.store.get_profile(id).await?;
        usage_service::load_profile_snapshot(&self.usage_store, &profile).await
    }

    pub async fn refresh_usage_profile(&self, id: &str) -> Result<UsageSnapshot, RelayError> {
        let context = self.usage_context().await?;
        let profile = self.store.get_profile(id).await?;
        self.refresh_usage_snapshot(&profile, &context).await
    }

    pub async fn refresh_enabled_usage_reports(&self) -> Result<Vec<UsageSnapshot>, RelayError> {
        let profiles = self.store.list_enabled_profiles().await?;
        self.refresh_usage_for_profiles(&profiles).await
    }

    pub async fn refresh_all_usage_reports(&self) -> Result<Vec<UsageSnapshot>, RelayError> {
        let profiles = self.store.list_profiles().await?;
        self.refresh_usage_for_profiles(&profiles).await
    }

    async fn refresh_usage_for_profiles(
        &self,
        profiles: &[Profile],
    ) -> Result<Vec<UsageSnapshot>, RelayError> {
        let context = self.usage_context().await?;
        let concurrency = self.settings().await?.network_query_concurrency.max(1) as usize;
        let app = self.clone();
        let mut results = stream::iter(profiles.iter().cloned().enumerate().map(
            move |(index, profile)| {
                let app = app.clone();
                let context = context.clone();
                async move { (index, app.refresh_usage_snapshot(&profile, &context).await) }
            },
        ))
        .buffer_unordered(concurrency)
        .collect::<Vec<_>>()
        .await;

        results.sort_by_key(|(index, _)| *index);
        let mut snapshots = Vec::with_capacity(results.len());
        for (_, result) in results {
            snapshots.push(result?);
        }
        Ok(snapshots)
    }

    async fn refresh_usage_snapshot(
        &self,
        profile: &Profile,
        context: &UsageContext,
    ) -> Result<UsageSnapshot, RelayError> {
        let provider = self.adapters.usage_provider(&profile.agent);
        let snapshot = usage_service::refresh_profile(
            &self.store,
            &self.usage_store,
            provider,
            Some(profile),
            context.active_profile.as_ref(),
            self.usage_source_mode_for_agent(&profile.agent).await?,
            context.allow_cache_writes,
        )
        .await?;
        self.sync_usage_failure_events(profile, &snapshot).await?;
        Ok(snapshot)
    }

    async fn usage_context(&self) -> Result<UsageContext, RelayError> {
        let active_state = self.state_store.load().await?;
        Ok(UsageContext {
            active_profile: self.active_profile_from_state(&active_state).await?,
            allow_cache_writes: self.bootstrap_mode == BootstrapMode::ReadWrite,
        })
    }

    async fn sync_usage_failure_events(
        &self,
        profile: &Profile,
        snapshot: &UsageSnapshot,
    ) -> Result<(), RelayError> {
        let mut active_reasons = Vec::new();

        if matches!(snapshot.session.status, UsageStatus::Exhausted) {
            active_reasons.push(FailureReason::SessionExhausted);
            self.store
                .record_failure_event(
                    Some(profile.id.as_str()),
                    FailureReason::SessionExhausted,
                    "Session usage exhausted.",
                    None,
                )
                .await?;
        }

        if matches!(snapshot.weekly.status, UsageStatus::Exhausted) {
            active_reasons.push(FailureReason::WeeklyExhausted);
            self.store
                .record_failure_event(
                    Some(profile.id.as_str()),
                    FailureReason::WeeklyExhausted,
                    "Weekly usage exhausted.",
                    None,
                )
                .await?;
        }

        let resolved_reasons = [
            FailureReason::SessionExhausted,
            FailureReason::WeeklyExhausted,
        ]
        .into_iter()
        .filter(|reason| !active_reasons.contains(reason))
        .collect::<Vec<_>>();

        self.store
            .resolve_failure_events(&profile.id, &resolved_reasons)
            .await?;

        Ok(())
    }
}
