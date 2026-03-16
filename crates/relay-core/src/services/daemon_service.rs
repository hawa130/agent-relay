use crate::models::{
    ActiveStateUpdatedPayload, ActivityEventsParams, ActivityEventsResult,
    ActivityEventsUpdatedPayload, ActivityLogsUpdatedPayload, ActivityRefreshResult,
    AddProfileParams, DoctorUpdatedPayload, EditProfileParams, EngineConnectionState, EngineState,
    HealthUpdatedPayload, ImportProfileParams, InitialState, InitializeParams, InitializeResult,
    JSONRPC_VERSION, LoginProfileParams, LogsTailParams, LogsTailResult, ProfileIdParams,
    ProfilesUpdatedPayload, QueryStateItem, QueryStateKey, QueryStateKind, QueryStateStatus,
    QueryStateUpdatedPayload, RefreshUsageParams, RefreshUsageResult, RelayRpcTopic, RelayTaskKind,
    RpcNotification, RpcRequest, RpcServerCapabilities, RpcServerInfo, RpcSuccessResponse,
    SessionUpdate, SetProfileEnabledParams, SettingsResult, SettingsUpdateParams,
    SettingsUpdatedPayload, SubscribeParams, SubscribeResult, SwitchCompletedPayload,
    SwitchFailedPayload, SwitchTrigger, TaskCancelParams, TaskCancelResult, TaskStartResult,
    TaskUpdatedPayload, UsageGetParams, UsageResult, UsageUpdateTrigger, UsageUpdatedPayload,
    rpc_from_error, rpc_internal_error, rpc_invalid_params, rpc_invalid_request,
    rpc_method_not_found,
};
use crate::services::query_coordinator::QueryCoordinator;
use crate::services::task_manager::{TaskCancellationHandle, TaskManager};
use crate::{
    ActivityEventsQuery, CodexSettingsUpdateRequest, FailureReason, Profile, RelayApp, RelayError,
    SystemSettingsUpdateRequest,
};
use chrono::Utc;
use futures_util::stream::StreamExt;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::mpsc;

struct DaemonSessionState {
    subscribed_topics: HashSet<RelayRpcTopic>,
    query_states: BTreeMap<String, QueryStateItem>,
    connection_state: EngineConnectionState,
}

impl Default for DaemonSessionState {
    fn default() -> Self {
        Self {
            subscribed_topics: HashSet::new(),
            query_states: BTreeMap::new(),
            connection_state: EngineConnectionState::Starting,
        }
    }
}

#[derive(Clone)]
struct DaemonHub {
    notifications: mpsc::UnboundedSender<RpcNotification>,
    seq: Arc<AtomicU64>,
    state: Arc<Mutex<DaemonSessionState>>,
}

impl DaemonHub {
    fn new(notifications: mpsc::UnboundedSender<RpcNotification>) -> Self {
        Self {
            notifications,
            seq: Arc::new(AtomicU64::new(0)),
            state: Arc::new(Mutex::new(DaemonSessionState::default())),
        }
    }

    fn subscribe(&self, topics: &[RelayRpcTopic]) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        state.subscribed_topics.extend(topics.iter().copied());
    }

    fn unsubscribe(&self, topics: &[RelayRpcTopic]) -> Vec<RelayRpcTopic> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        for topic in topics {
            state.subscribed_topics.remove(topic);
        }
        state.subscribed_topics.iter().copied().collect()
    }

    fn connection_state(&self) -> EngineConnectionState {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .connection_state
    }

    fn set_connection_state(&self, value: EngineConnectionState) {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .connection_state = value;
    }

    fn query_state_snapshot(&self) -> Vec<QueryStateItem> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .query_states
            .values()
            .cloned()
            .collect()
    }

    fn set_query_state(&self, item: QueryStateItem) {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .query_states
            .insert(query_state_storage_key(&item.key), item);
    }

    fn clear_query_state(&self, key: &QueryStateKey) {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .query_states
            .remove(&query_state_storage_key(key));
    }

    fn publish<T: serde::Serialize>(
        &self,
        topic: RelayRpcTopic,
        payload: T,
    ) -> Result<(), RelayError> {
        if !self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .subscribed_topics
            .contains(&topic)
        {
            return Ok(());
        }
        let seq = self.seq.fetch_add(1, Ordering::Relaxed) + 1;
        self.notifications
            .send(RpcNotification {
                jsonrpc: JSONRPC_VERSION.into(),
                method: "session/update".into(),
                params: SessionUpdate {
                    topic,
                    seq,
                    timestamp: Utc::now(),
                    payload: serde_json::to_value(payload)
                        .map_err(|error| RelayError::Internal(error.to_string()))?,
                },
            })
            .map_err(|error| RelayError::Internal(error.to_string()))
    }
}

#[derive(Clone)]
pub struct DaemonService {
    app: RelayApp,
    hub: DaemonHub,
    started_at: chrono::DateTime<Utc>,
    usage_queries: QueryCoordinator<QueryStateKey, crate::UsageSnapshot, RelayError>,
    tasks: TaskManager,
    profile_write_lock: Arc<AsyncMutex<()>>,
    settings_lock: Arc<AsyncMutex<()>>,
    shutdown_requested: Arc<std::sync::atomic::AtomicBool>,
}

impl DaemonService {
    const DEFAULT_ACTIVITY_EVENTS_LIMIT: usize = 10;
    const DEFAULT_ACTIVITY_LOG_LINES: usize = 25;

    pub fn new(app: RelayApp, notifications: mpsc::UnboundedSender<RpcNotification>) -> Self {
        Self {
            app,
            hub: DaemonHub::new(notifications),
            started_at: Utc::now(),
            usage_queries: QueryCoordinator::new(10),
            tasks: TaskManager::new(),
            profile_write_lock: Arc::new(AsyncMutex::new(())),
            settings_lock: Arc::new(AsyncMutex::new(())),
            shutdown_requested: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn shutdown_requested(&self) -> bool {
        self.shutdown_requested
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub async fn current_refresh_interval(&self) -> Result<i64, RelayError> {
        Ok(self.app.settings().await?.refresh_interval_seconds)
    }

    pub async fn sync_network_query_concurrency(&self) -> Result<(), RelayError> {
        let limit = self.app.settings().await?.network_query_concurrency.max(1) as usize;
        self.usage_queries.set_limit(limit);
        Ok(())
    }

    pub async fn startup_tick(&self) -> Result<(), RelayError> {
        self.refresh_and_maybe_auto_switch(UsageUpdateTrigger::Startup)
            .await
    }

    pub async fn interval_tick(&self) -> Result<(), RelayError> {
        self.refresh_and_maybe_auto_switch(UsageUpdateTrigger::Interval)
            .await
    }

    pub fn is_read_method(method: &str) -> bool {
        matches!(
            method,
            "relay/doctor/get"
                | "relay/status/get"
                | "relay/profiles/list"
                | "relay/profiles/get"
                | "relay/usage/get"
                | "relay/settings/get"
                | "relay/activity/events/list"
                | "relay/activity/logs/tail"
        )
    }

    pub async fn handle_read_request(
        app: &RelayApp,
        request: RpcRequest,
    ) -> Result<RpcSuccessResponse, crate::RpcErrorObject> {
        if request.jsonrpc != JSONRPC_VERSION {
            return Err(rpc_invalid_request("jsonrpc must equal 2.0"));
        }

        let result = match request.method.as_str() {
            "relay/doctor/get" => serialize(app.doctor_report().map_err(|e| rpc_from_error(&e))?)?,
            "relay/status/get" => {
                serialize(app.system_status().await.map_err(|e| rpc_from_error(&e))?)?
            }
            "relay/profiles/list" => serialize(
                app.list_profiles_with_usage()
                    .await
                    .map_err(|e| rpc_from_error(&e))?,
            )?,
            "relay/profiles/get" => {
                let params: ProfileIdParams = parse_params(request.params)?;
                serialize(
                    app.profile_detail(&params.profile_id)
                        .await
                        .map_err(|e| rpc_from_error(&e))?,
                )?
            }
            "relay/usage/get" => {
                let params: UsageGetParams = parse_params(request.params)?;
                let snapshot = if let Some(profile_id) = params.profile_id {
                    app.profile_usage_report(&profile_id)
                        .await
                        .map_err(|e| rpc_from_error(&e))?
                } else {
                    app.usage_report().await.map_err(|e| rpc_from_error(&e))?
                };
                serialize(UsageResult { snapshot })?
            }
            "relay/settings/get" => serialize(SettingsResult {
                app: app.settings().await.map_err(|e| rpc_from_error(&e))?,
                codex: app.codex_settings().await.map_err(|e| rpc_from_error(&e))?,
            })?,
            "relay/activity/events/list" => {
                let params: ActivityEventsParams = parse_params(request.params)?;
                serialize(ActivityEventsResult {
                    events: app
                        .list_activity_events(ActivityEventsQuery {
                            limit: params.limit,
                            profile_id: params.profile_id,
                            reason: params.reason.map(parse_failure_reason).transpose()?,
                        })
                        .await
                        .map_err(|e| rpc_from_error(&e))?,
                })?
            }
            "relay/activity/logs/tail" => {
                let params: LogsTailParams = parse_params(request.params)?;
                serialize(LogsTailResult {
                    logs: app
                        .logs_tail(params.lines)
                        .await
                        .map_err(|e| rpc_from_error(&e))?,
                })?
            }
            other => return Err(rpc_method_not_found(other)),
        };

        Ok(RpcSuccessResponse {
            jsonrpc: JSONRPC_VERSION.into(),
            id: request.id,
            result,
        })
    }

    pub async fn handle_request(
        &self,
        request: RpcRequest,
    ) -> Result<RpcSuccessResponse, crate::RpcErrorObject> {
        if Self::is_read_method(&request.method) {
            return Self::handle_read_request(&self.app, request).await;
        }

        if request.jsonrpc != JSONRPC_VERSION {
            return Err(rpc_invalid_request("jsonrpc must equal 2.0"));
        }

        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params).await?,
            "session/subscribe" => self.handle_subscribe(request.params).await?,
            "session/unsubscribe" => self.handle_unsubscribe(request.params).await?,
            "relay/profiles/add" => {
                let _guard = self.profile_write_lock.lock().await;
                let params: AddProfileParams = parse_params(request.params)?;
                let profile = self
                    .app
                    .add_profile(params.request)
                    .await
                    .map_err(|e| rpc_from_error(&e))?;
                self.publish_profiles_updated()
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                self.publish_active_state_updated()
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                serialize(profile)?
            }
            "relay/profiles/edit" => {
                let _guard = self.profile_write_lock.lock().await;
                let params: EditProfileParams = parse_params(request.params)?;
                let profile = self
                    .app
                    .edit_profile(&params.profile_id, params.request)
                    .await
                    .map_err(|e| rpc_from_error(&e))?;
                self.publish_profiles_updated()
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                self.publish_active_state_updated()
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                serialize(profile)?
            }
            "relay/profiles/import" => {
                let _guard = self.profile_write_lock.lock().await;
                let params: ImportProfileParams = parse_params(request.params)?;
                let profile = self
                    .app
                    .import_profile(params.request)
                    .await
                    .map_err(|e| rpc_from_error(&e))?;
                self.publish_profiles_updated()
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                self.publish_active_state_updated()
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                serialize(profile)?
            }
            "relay/profiles/login/start" => {
                serialize(self.handle_login_start(request.params).await?)?
            }
            "relay/profiles/remove" => {
                let _guard = self.profile_write_lock.lock().await;
                let params: ProfileIdParams = parse_params(request.params)?;
                let profile = self
                    .app
                    .remove_profile(&params.profile_id)
                    .await
                    .map_err(|e| rpc_from_error(&e))?;
                self.publish_profiles_updated()
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                self.publish_active_state_updated()
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                serialize(profile)?
            }
            "relay/profiles/set_enabled" => {
                let _guard = self.profile_write_lock.lock().await;
                let params: SetProfileEnabledParams = parse_params(request.params)?;
                let profile = self
                    .app
                    .set_profile_enabled(&params.profile_id, params.enabled)
                    .await
                    .map_err(|e| rpc_from_error(&e))?;
                self.publish_profiles_updated()
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                self.publish_active_state_updated()
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                serialize(profile)?
            }
            "relay/usage/refresh" => serialize(self.handle_refresh(request.params).await?)?,
            "relay/activity/refresh" => serialize(self.handle_activity_refresh().await?)?,
            "relay/doctor/refresh" => serialize(self.handle_doctor_refresh().await?)?,
            "relay/switch/activate" => {
                let _guard = self.profile_write_lock.lock().await;
                let params: ProfileIdParams = parse_params(request.params)?;
                serialize(
                    self.handle_switch_result(
                        self.app.switch_to_profile(&params.profile_id).await,
                        SwitchTrigger::Manual,
                        Some(params.profile_id),
                    )
                    .await?,
                )?
            }
            "relay/switch/next" => {
                let _guard = self.profile_write_lock.lock().await;
                serialize(
                    self.handle_switch_result(
                        self.app.switch_next_profile().await,
                        SwitchTrigger::Manual,
                        None,
                    )
                    .await?,
                )?
            }
            "relay/settings/update" => {
                serialize(self.handle_settings_update(request.params).await?)?
            }
            "relay/tasks/cancel" => serialize(self.handle_task_cancel(request.params).await?)?,
            "relay/activity/diagnostics/export" => serialize(
                self.app
                    .diagnostics_export()
                    .await
                    .map_err(|e| rpc_from_error(&e))?,
            )?,
            "shutdown" => {
                self.shutdown_requested
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                serialize(serde_json::json!({ "accepted": true }))?
            }
            other => return Err(rpc_method_not_found(other)),
        };

        Ok(RpcSuccessResponse {
            jsonrpc: JSONRPC_VERSION.into(),
            id: request.id,
            result,
        })
    }

    async fn handle_initialize(&self, params: Value) -> Result<Value, crate::RpcErrorObject> {
        let _: InitializeParams = parse_params(params)?;
        self.hub.set_connection_state(EngineConnectionState::Ready);
        serialize(InitializeResult {
            protocol_version: "1".into(),
            server_info: RpcServerInfo {
                name: "agrelay".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            capabilities: RpcServerCapabilities {
                supports_subscriptions: true,
                supports_health_updates: true,
            },
            initial_state: InitialState {
                status: self
                    .app
                    .system_status()
                    .await
                    .map_err(|e| rpc_from_error(&e))?,
                profiles: self
                    .app
                    .list_profiles_with_usage()
                    .await
                    .map_err(|e| rpc_from_error(&e))?,
                codex_settings: self
                    .app
                    .codex_settings()
                    .await
                    .map_err(|e| rpc_from_error(&e))?,
                engine: EngineState {
                    started_at: self.started_at,
                    connection_state: self.hub.connection_state(),
                },
            },
        })
    }

    async fn handle_subscribe(&self, params: Value) -> Result<Value, crate::RpcErrorObject> {
        let params: SubscribeParams = parse_params(params)?;
        self.hub.subscribe(&params.topics);
        self.publish_subscribed_snapshots(&params.topics)
            .await
            .map_err(|e| rpc_internal_error(e.to_string()))?;
        serialize(SubscribeResult {
            subscribed_topics: params.topics,
        })
    }

    async fn handle_unsubscribe(&self, params: Value) -> Result<Value, crate::RpcErrorObject> {
        let params: SubscribeParams = parse_params(params)?;
        serialize(SubscribeResult {
            subscribed_topics: self.hub.unsubscribe(&params.topics),
        })
    }

    async fn handle_refresh(
        &self,
        params: Value,
    ) -> Result<RefreshUsageResult, crate::RpcErrorObject> {
        let params: RefreshUsageParams = parse_params(params)?;
        let snapshots = if let Some(profile_id) = params.profile_id {
            vec![
                self.refresh_usage_profile_with_query_state(
                    &profile_id,
                    UsageUpdateTrigger::Manual,
                )
                .await
                .map_err(|e| rpc_from_error(&e))?,
            ]
        } else if params.include_disabled {
            let profiles = self
                .app
                .list_profiles()
                .await
                .map_err(|e| rpc_from_error(&e))?;
            self.refresh_usage_profiles(profiles, UsageUpdateTrigger::Manual)
                .await
                .map_err(|e| rpc_from_error(&e))?
        } else {
            let profiles = self
                .app
                .list_profiles()
                .await
                .map_err(|e| rpc_from_error(&e))?
                .into_iter()
                .filter(|profile| profile.enabled)
                .collect();
            self.refresh_usage_profiles(profiles, UsageUpdateTrigger::Manual)
                .await
                .map_err(|e| rpc_from_error(&e))?
        };
        self.publish_profiles_updated()
            .await
            .map_err(|e| rpc_internal_error(e.to_string()))?;
        let events = self
            .load_recent_activity_events()
            .await
            .map_err(|e| rpc_internal_error(e.to_string()))?;
        self.publish_activity_events_updated(events)
            .await
            .map_err(|e| rpc_internal_error(e.to_string()))?;
        self.publish_active_state_updated()
            .await
            .map_err(|e| rpc_internal_error(e.to_string()))?;
        Ok(RefreshUsageResult { snapshots })
    }

    async fn handle_activity_refresh(
        &self,
    ) -> Result<ActivityRefreshResult, crate::RpcErrorObject> {
        let events = self
            .load_recent_activity_events()
            .await
            .map_err(|e| rpc_from_error(&e))?;
        let logs = self
            .app
            .logs_tail(Self::DEFAULT_ACTIVITY_LOG_LINES)
            .await
            .map_err(|e| rpc_from_error(&e))?;
        self.publish_activity_events_updated(events.clone())
            .await
            .map_err(|e| rpc_internal_error(e.to_string()))?;
        self.publish_activity_logs_updated(logs.clone())
            .await
            .map_err(|e| rpc_internal_error(e.to_string()))?;
        Ok(ActivityRefreshResult { events, logs })
    }

    async fn handle_doctor_refresh(&self) -> Result<crate::DoctorReport, crate::RpcErrorObject> {
        let report = self.app.doctor_report().map_err(|e| rpc_from_error(&e))?;
        self.publish_doctor_updated(report.clone())
            .await
            .map_err(|e| rpc_internal_error(e.to_string()))?;
        Ok(report)
    }

    async fn handle_login_start(
        &self,
        params: Value,
    ) -> Result<TaskStartResult, crate::RpcErrorObject> {
        let params: LoginProfileParams = parse_params(params)?;
        let cancel_requested = Arc::new(AtomicBool::new(false));
        let cancel_handle = {
            let cancel_requested = cancel_requested.clone();
            TaskCancellationHandle::new(move || {
                cancel_requested.store(true, std::sync::atomic::Ordering::SeqCst);
            })
        };
        let pending = self.tasks.start(RelayTaskKind::ProfileLogin, cancel_handle);
        let task_id = pending.task_id.clone();
        self.publish_task_updated(pending.clone())
            .await
            .map_err(|e| rpc_internal_error(e.to_string()))?;

        let service = self.clone();
        tokio::task::spawn_local(async move {
            service
                .run_login_task(task_id, params.request, cancel_requested)
                .await;
        });

        Ok(TaskStartResult {
            task_id: pending.task_id,
            kind: RelayTaskKind::ProfileLogin,
            accepted: true,
        })
    }

    async fn handle_task_cancel(
        &self,
        params: Value,
    ) -> Result<TaskCancelResult, crate::RpcErrorObject> {
        let params: TaskCancelParams = parse_params(params)?;
        Ok(TaskCancelResult {
            accepted: self.tasks.cancel(&params.task_id),
        })
    }

    async fn handle_settings_update(
        &self,
        params: Value,
    ) -> Result<SettingsResult, crate::RpcErrorObject> {
        let _guard = self.settings_lock.lock().await;
        let params: SettingsUpdateParams = parse_params(params)?;
        let app = if let Some(app_patch) = params.app {
            self.app
                .update_system_settings(SystemSettingsUpdateRequest {
                    auto_switch_enabled: app_patch.auto_switch_enabled,
                    cooldown_seconds: app_patch.cooldown_seconds,
                    refresh_interval_seconds: app_patch.refresh_interval_seconds,
                    network_query_concurrency: app_patch.network_query_concurrency,
                })
                .await
                .map_err(|e| rpc_from_error(&e))?
        } else {
            self.app.settings().await.map_err(|e| rpc_from_error(&e))?
        };
        let codex = if let Some(codex_patch) = params.codex {
            self.app
                .update_codex_settings(CodexSettingsUpdateRequest {
                    usage_source_mode: codex_patch.usage_source_mode,
                })
                .await
                .map_err(|e| rpc_from_error(&e))?
        } else {
            self.app
                .codex_settings()
                .await
                .map_err(|e| rpc_from_error(&e))?
        };
        let settings = SettingsResult { app, codex };
        self.sync_network_query_concurrency()
            .await
            .map_err(|e| rpc_internal_error(e.to_string()))?;
        self.publish_settings_updated(settings.clone())
            .await
            .map_err(|e| rpc_internal_error(e.to_string()))?;
        Ok(settings)
    }

    async fn handle_switch_result(
        &self,
        result: Result<crate::SwitchReport, RelayError>,
        trigger: SwitchTrigger,
        profile_id: Option<String>,
    ) -> Result<crate::SwitchReport, crate::RpcErrorObject> {
        match result {
            Ok(report) => {
                self.publish_switch_completed(report.clone(), trigger)
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                let events = self
                    .load_recent_activity_events()
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                self.publish_activity_events_updated(events)
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                self.publish_profiles_updated()
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                self.publish_active_state_updated()
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                Ok(report)
            }
            Err(error) => {
                self.publish_switch_failed(&error, profile_id, trigger)
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                let events = self
                    .load_recent_activity_events()
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                self.publish_activity_events_updated(events)
                    .await
                    .map_err(|e| rpc_internal_error(e.to_string()))?;
                Err(rpc_from_error(&error))
            }
        }
    }

    async fn refresh_and_maybe_auto_switch(
        &self,
        trigger: UsageUpdateTrigger,
    ) -> Result<(), RelayError> {
        let profiles: Vec<Profile> = self
            .app
            .list_profiles()
            .await?
            .into_iter()
            .filter(|profile| profile.enabled)
            .collect();
        let snapshots = self.refresh_usage_profiles(profiles, trigger).await?;
        self.publish_profiles_updated().await?;
        let events = self.load_recent_activity_events().await?;
        self.publish_activity_events_updated(events).await?;
        self.publish_active_state_updated().await?;

        let settings = self.app.settings().await?;
        if !settings.auto_switch_enabled {
            return Ok(());
        }
        let status = self.app.system_status().await?;
        let Some(active_profile_id) = status.active_state.active_profile_id else {
            return Ok(());
        };
        let Some(active_snapshot) = snapshots
            .iter()
            .find(|snapshot| snapshot.profile_id.as_deref() == Some(active_profile_id.as_str()))
        else {
            return Ok(());
        };
        let active_profile = self.app.profile(&active_profile_id).await?;
        let account_requires_switch =
            active_profile.account_state == crate::models::ProfileAccountState::AccountUnavailable
                || active_snapshot.remote_error.as_ref().is_some_and(|error| {
                    error.kind == crate::models::UsageRemoteErrorKind::Account
                });
        if !active_snapshot.can_auto_switch && !account_requires_switch {
            return Ok(());
        }

        let _guard = self.profile_write_lock.lock().await;
        match self.app.switch_next_profile().await {
            Ok(report) => {
                self.publish_switch_completed(report, SwitchTrigger::Auto)
                    .await?;
                let events = self.load_recent_activity_events().await?;
                self.publish_activity_events_updated(events).await?;
                self.publish_profiles_updated().await?;
                let profiles: Vec<Profile> = self
                    .app
                    .list_profiles()
                    .await?
                    .into_iter()
                    .filter(|profile| profile.enabled)
                    .collect();
                let _ = self
                    .refresh_usage_profiles(profiles, UsageUpdateTrigger::PostSwitch)
                    .await?;
                self.publish_active_state_updated().await?;
            }
            Err(error) => {
                self.publish_switch_failed(&error, Some(active_profile_id), SwitchTrigger::Auto)
                    .await?;
                let events = self.load_recent_activity_events().await?;
                self.publish_activity_events_updated(events).await?;
            }
        }

        Ok(())
    }

    async fn publish_usage_updated(
        &self,
        snapshots: Vec<crate::UsageSnapshot>,
        trigger: UsageUpdateTrigger,
    ) -> Result<(), RelayError> {
        self.hub.publish(
            RelayRpcTopic::UsageUpdated,
            UsageUpdatedPayload { snapshots, trigger },
        )
    }

    async fn publish_query_state_updated(&self) -> Result<(), RelayError> {
        self.hub.publish(
            RelayRpcTopic::QueryStateUpdated,
            QueryStateUpdatedPayload {
                states: self.hub.query_state_snapshot(),
            },
        )
    }

    async fn publish_settings_updated(&self, settings: SettingsResult) -> Result<(), RelayError> {
        self.hub.publish(
            RelayRpcTopic::SettingsUpdated,
            SettingsUpdatedPayload { settings },
        )
    }

    async fn publish_profiles_updated(&self) -> Result<(), RelayError> {
        let profiles = self.app.list_profiles_with_usage().await?;
        self.hub.publish(
            RelayRpcTopic::ProfilesUpdated,
            ProfilesUpdatedPayload { profiles },
        )
    }

    async fn publish_activity_events_updated(
        &self,
        events: Vec<crate::FailureEvent>,
    ) -> Result<(), RelayError> {
        self.hub.publish(
            RelayRpcTopic::ActivityEventsUpdated,
            ActivityEventsUpdatedPayload { events },
        )
    }

    async fn publish_activity_logs_updated(&self, logs: crate::LogTail) -> Result<(), RelayError> {
        self.hub.publish(
            RelayRpcTopic::ActivityLogsUpdated,
            ActivityLogsUpdatedPayload { logs },
        )
    }

    async fn publish_doctor_updated(&self, report: crate::DoctorReport) -> Result<(), RelayError> {
        self.hub.publish(
            RelayRpcTopic::DoctorUpdated,
            DoctorUpdatedPayload { report },
        )
    }

    async fn publish_active_state_updated(&self) -> Result<(), RelayError> {
        let status = self.app.system_status().await?;
        let active_profile = self
            .app
            .list_profiles_with_usage()
            .await?
            .into_iter()
            .find(|item| {
                status.active_state.active_profile_id.as_deref() == Some(item.profile.id.as_str())
            });
        self.hub.publish(
            RelayRpcTopic::ActiveStateUpdated,
            ActiveStateUpdatedPayload {
                active_state: status.active_state,
                active_profile,
            },
        )
    }

    async fn publish_switch_completed(
        &self,
        report: crate::SwitchReport,
        trigger: SwitchTrigger,
    ) -> Result<(), RelayError> {
        self.hub.publish(
            RelayRpcTopic::SwitchCompleted,
            SwitchCompletedPayload { report, trigger },
        )
    }

    async fn publish_switch_failed(
        &self,
        error: &RelayError,
        profile_id: Option<String>,
        trigger: SwitchTrigger,
    ) -> Result<(), RelayError> {
        self.hub.publish(
            RelayRpcTopic::SwitchFailed,
            SwitchFailedPayload {
                error_code: error.code().as_str().to_string(),
                message: error.to_string(),
                profile_id,
                trigger,
            },
        )
    }

    async fn publish_task_updated(&self, task: crate::TaskUpdate) -> Result<(), RelayError> {
        self.hub
            .publish(RelayRpcTopic::TaskUpdated, TaskUpdatedPayload { task })
    }

    pub async fn publish_health_update(
        &self,
        state: EngineConnectionState,
        detail: Option<String>,
    ) -> Result<(), RelayError> {
        self.hub.set_connection_state(state);
        self.hub.publish(
            RelayRpcTopic::HealthUpdated,
            HealthUpdatedPayload { state, detail },
        )
    }

    async fn publish_subscribed_snapshots(
        &self,
        topics: &[RelayRpcTopic],
    ) -> Result<(), RelayError> {
        for topic in topics {
            match topic {
                RelayRpcTopic::UsageUpdated => {
                    let profiles = self.app.list_profiles_with_usage().await?;
                    let snapshots = profiles
                        .into_iter()
                        .filter_map(|item| item.usage_summary)
                        .collect::<Vec<_>>();
                    self.publish_usage_updated(snapshots, UsageUpdateTrigger::Startup)
                        .await?;
                }
                RelayRpcTopic::QueryStateUpdated => {
                    self.publish_query_state_updated().await?;
                }
                RelayRpcTopic::ActiveStateUpdated => {
                    self.publish_active_state_updated().await?;
                }
                RelayRpcTopic::SettingsUpdated => {
                    let settings = SettingsResult {
                        app: self.app.settings().await?,
                        codex: self.app.codex_settings().await?,
                    };
                    self.publish_settings_updated(settings).await?;
                }
                RelayRpcTopic::ProfilesUpdated => {
                    self.publish_profiles_updated().await?;
                }
                RelayRpcTopic::ActivityEventsUpdated => {
                    let events = self.load_recent_activity_events().await?;
                    self.publish_activity_events_updated(events).await?;
                }
                RelayRpcTopic::ActivityLogsUpdated => {
                    let logs = self.app.logs_tail(Self::DEFAULT_ACTIVITY_LOG_LINES).await?;
                    self.publish_activity_logs_updated(logs).await?;
                }
                RelayRpcTopic::DoctorUpdated => {
                    let report = self.app.doctor_report()?;
                    self.publish_doctor_updated(report).await?;
                }
                RelayRpcTopic::SwitchCompleted
                | RelayRpcTopic::SwitchFailed
                | RelayRpcTopic::TaskUpdated => {}
                RelayRpcTopic::HealthUpdated => {
                    self.publish_health_update(EngineConnectionState::Ready, None)
                        .await?;
                }
            }
        }
        Ok(())
    }

    async fn load_recent_activity_events(&self) -> Result<Vec<crate::FailureEvent>, RelayError> {
        self.app
            .list_activity_events(ActivityEventsQuery {
                limit: Self::DEFAULT_ACTIVITY_EVENTS_LIMIT,
                profile_id: None,
                reason: None,
            })
            .await
    }

    async fn refresh_usage_profiles(
        &self,
        profiles: Vec<Profile>,
        trigger: UsageUpdateTrigger,
    ) -> Result<Vec<crate::UsageSnapshot>, RelayError> {
        self.sync_network_query_concurrency().await?;
        let concurrency = self.app.settings().await?.network_query_concurrency.max(1) as usize;
        let service = self.clone();
        let mut results = futures_util::stream::iter(profiles.into_iter().enumerate().map(
            move |(index, profile)| {
                let service = service.clone();
                async move {
                    (
                        index,
                        service
                            .refresh_usage_profile_with_query_state(&profile.id, trigger)
                            .await,
                    )
                }
            },
        ))
        .buffer_unordered(concurrency)
        .collect::<Vec<_>>()
        .await;

        results.sort_by_key(|(index, _)| *index);

        let mut snapshots = Vec::with_capacity(results.len());
        let mut first_error = None;
        for (_, result) in results {
            match result {
                Ok(snapshot) => snapshots.push(snapshot),
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        if snapshots.is_empty() {
            if let Some(error) = first_error {
                return Err(error);
            }
        }
        Ok(snapshots)
    }

    async fn refresh_usage_profile_with_query_state(
        &self,
        profile_id: &str,
        trigger: UsageUpdateTrigger,
    ) -> Result<crate::UsageSnapshot, RelayError> {
        let key = usage_profile_query_key(profile_id);
        self.set_query_state_pending(key.clone(), trigger).await?;
        let app = self.app.clone();
        let profile_id = profile_id.to_string();
        match self
            .usage_queries
            .run(key.clone(), move || {
                let app = app.clone();
                let profile_id = profile_id.clone();
                async move { app.refresh_usage_profile(&profile_id).await }
            })
            .await
        {
            Ok(snapshot) => {
                self.publish_usage_updated(vec![snapshot.clone()], trigger)
                    .await?;
                self.publish_profiles_updated().await?;
                let events = self.load_recent_activity_events().await?;
                self.publish_activity_events_updated(events).await?;
                self.clear_query_state(&key).await?;
                Ok(snapshot)
            }
            Err(error) => {
                self.set_query_state_error(key, trigger, &error).await?;
                Err(error)
            }
        }
    }

    async fn set_query_state_pending(
        &self,
        key: QueryStateKey,
        trigger: UsageUpdateTrigger,
    ) -> Result<(), RelayError> {
        self.hub.set_query_state(QueryStateItem {
            key,
            status: QueryStateStatus::Pending,
            trigger,
            error_code: None,
            message: None,
            updated_at: Utc::now(),
        });
        self.publish_query_state_updated().await
    }

    async fn set_query_state_error(
        &self,
        key: QueryStateKey,
        trigger: UsageUpdateTrigger,
        error: &RelayError,
    ) -> Result<(), RelayError> {
        self.hub.set_query_state(QueryStateItem {
            key,
            status: QueryStateStatus::Error,
            trigger,
            error_code: Some(error.code().as_str().to_string()),
            message: Some(error.to_string()),
            updated_at: Utc::now(),
        });
        self.publish_query_state_updated().await
    }

    async fn clear_query_state(&self, key: &QueryStateKey) -> Result<(), RelayError> {
        self.hub.clear_query_state(key);
        self.publish_query_state_updated().await
    }

    async fn run_login_task(
        &self,
        task_id: String,
        request: crate::AgentLoginRequest,
        cancel_requested: Arc<AtomicBool>,
    ) {
        let _guard = self.profile_write_lock.lock().await;
        let result = self
            .app
            .login_profile_cancellable(request, cancel_requested.clone())
            .await;

        let update_result = match result {
            Ok(result) => {
                if cancel_requested.load(std::sync::atomic::Ordering::SeqCst) {
                    self.tasks
                        .finish_cancelled(&task_id, Some("browser sign-in was cancelled".into()))
                } else {
                    match serialize(result.clone()) {
                        Ok(payload) => {
                            if let Err(error) = self.publish_profiles_updated().await {
                                self.tasks.finish_failed(
                                    &task_id,
                                    error.code().as_str().to_string(),
                                    error.to_string(),
                                )
                            } else {
                                self.tasks.finish_succeeded(
                                    &task_id,
                                    payload,
                                    Some("profile login completed".into()),
                                )
                            }
                        }
                        Err(error) => self.tasks.finish_failed(
                            &task_id,
                            crate::ErrorCode::Internal.as_str().to_string(),
                            error.message,
                        ),
                    }
                }
            }
            Err(error) => {
                if cancel_requested.load(std::sync::atomic::Ordering::SeqCst) {
                    self.tasks
                        .finish_cancelled(&task_id, Some("browser sign-in was cancelled".into()))
                } else {
                    self.tasks.finish_failed(
                        &task_id,
                        error.code().as_str().to_string(),
                        error.to_string(),
                    )
                }
            }
        };

        if let Some(update) = update_result {
            let _ = self.publish_task_updated(update).await;
        }
    }
}

fn usage_profile_query_key(profile_id: &str) -> QueryStateKey {
    QueryStateKey {
        kind: QueryStateKind::UsageProfile,
        profile_id: Some(profile_id.to_string()),
    }
}

fn query_state_storage_key(key: &QueryStateKey) -> String {
    match (&key.kind, key.profile_id.as_deref()) {
        (QueryStateKind::UsageProfile, Some(profile_id)) => format!("usage.profile:{profile_id}"),
        (QueryStateKind::UsageProfile, None) => "usage.profile:".into(),
    }
}

fn parse_params<T: DeserializeOwned>(params: Value) -> Result<T, crate::RpcErrorObject> {
    serde_json::from_value(params).map_err(|error| rpc_invalid_params(error.to_string()))
}

fn serialize<T: serde::Serialize>(value: T) -> Result<Value, crate::RpcErrorObject> {
    serde_json::to_value(value).map_err(|error| rpc_internal_error(error.to_string()))
}

fn parse_failure_reason(value: String) -> Result<FailureReason, crate::RpcErrorObject> {
    value.parse::<FailureReason>().map_err(rpc_invalid_params)
}
