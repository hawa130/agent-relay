use crate::models::{
    ActiveStateUpdatedPayload, ActivityEventsParams, ActivityEventsResult, AddProfileParams,
    EditProfileParams, EngineConnectionState, EngineState, HealthUpdatedPayload,
    ImportProfileParams, InitialState, InitializeParams, InitializeResult, LoginProfileParams,
    LogsTailParams, LogsTailResult, ProfileIdParams, RefreshUsageParams, RefreshUsageResult,
    RelayRpcTopic, RpcNotification, RpcRequest, RpcServerCapabilities, RpcServerInfo,
    RpcSuccessResponse, SessionUpdate, SetProfileEnabledParams, SettingsResult,
    SettingsUpdateParams, SubscribeParams, SubscribeResult, SwitchCompletedPayload,
    SwitchFailedPayload, SwitchTrigger, UsageGetParams, UsageResult, UsageUpdateTrigger,
    UsageUpdatedPayload, rpc_from_error, rpc_internal_error, rpc_invalid_params,
    rpc_invalid_request, rpc_method_not_found,
};
use crate::{
    ActivityEventsQuery, CodexSettingsUpdateRequest, FailureReason, RelayApp, RelayError,
    SystemSettingsUpdateRequest,
};
use chrono::Utc;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashSet;
use tokio::sync::mpsc;

pub struct DaemonService {
    app: RelayApp,
    notifications: mpsc::UnboundedSender<RpcNotification>,
    started_at: chrono::DateTime<Utc>,
    seq: u64,
    subscribed_topics: HashSet<RelayRpcTopic>,
    connection_state: EngineConnectionState,
    shutdown_requested: bool,
}

impl DaemonService {
    pub fn new(app: RelayApp, notifications: mpsc::UnboundedSender<RpcNotification>) -> Self {
        Self {
            app,
            notifications,
            started_at: Utc::now(),
            seq: 0,
            subscribed_topics: HashSet::new(),
            connection_state: EngineConnectionState::Starting,
            shutdown_requested: false,
        }
    }

    pub fn shutdown_requested(&self) -> bool {
        self.shutdown_requested
    }

    pub async fn current_refresh_interval(&self) -> Result<i64, RelayError> {
        Ok(self.app.settings().await?.refresh_interval_seconds)
    }

    pub async fn startup_tick(&mut self) -> Result<(), RelayError> {
        self.refresh_and_maybe_auto_switch(UsageUpdateTrigger::Startup)
            .await
    }

    pub async fn interval_tick(&mut self) -> Result<(), RelayError> {
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
        if request.jsonrpc != "2.0" {
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
                        .map_err(|e| rpc_from_error(&e))?,
                })?
            }
            other => return Err(rpc_method_not_found(other)),
        };

        Ok(RpcSuccessResponse {
            jsonrpc: "2.0".into(),
            id: request.id,
            result,
        })
    }

    pub async fn handle_request(
        &mut self,
        request: RpcRequest,
    ) -> Result<RpcSuccessResponse, crate::RpcErrorObject> {
        if request.jsonrpc != "2.0" {
            return Err(rpc_invalid_request("jsonrpc must equal 2.0"));
        }

        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params).await?,
            "session/subscribe" => self.handle_subscribe(request.params).await?,
            "session/unsubscribe" => self.handle_unsubscribe(request.params).await?,
            "relay/doctor/get" => {
                serialize(self.app.doctor_report().map_err(|e| rpc_from_error(&e))?)?
            }
            "relay/status/get" => serialize(
                self.app
                    .system_status()
                    .await
                    .map_err(|e| rpc_from_error(&e))?,
            )?,
            "relay/profiles/list" => serialize(
                self.app
                    .list_profiles_with_usage()
                    .await
                    .map_err(|e| rpc_from_error(&e))?,
            )?,
            "relay/profiles/get" => {
                let params: ProfileIdParams = parse_params(request.params)?;
                serialize(
                    self.app
                        .profile_detail(&params.profile_id)
                        .await
                        .map_err(|e| rpc_from_error(&e))?,
                )?
            }
            "relay/profiles/add" => {
                let params: AddProfileParams = parse_params(request.params)?;
                serialize(
                    self.app
                        .add_profile(params.request)
                        .await
                        .map_err(|e| rpc_from_error(&e))?,
                )?
            }
            "relay/profiles/edit" => {
                let params: EditProfileParams = parse_params(request.params)?;
                serialize(
                    self.app
                        .edit_profile(&params.profile_id, params.request)
                        .await
                        .map_err(|e| rpc_from_error(&e))?,
                )?
            }
            "relay/profiles/import" => {
                let params: ImportProfileParams = parse_params(request.params)?;
                serialize(
                    self.app
                        .import_profile(params.request)
                        .await
                        .map_err(|e| rpc_from_error(&e))?,
                )?
            }
            "relay/profiles/login" => {
                let params: LoginProfileParams = parse_params(request.params)?;
                serialize(
                    self.app
                        .login_profile(params.request)
                        .await
                        .map_err(|e| rpc_from_error(&e))?,
                )?
            }
            "relay/profiles/remove" => {
                let params: ProfileIdParams = parse_params(request.params)?;
                serialize(
                    self.app
                        .remove_profile(&params.profile_id)
                        .await
                        .map_err(|e| rpc_from_error(&e))?,
                )?
            }
            "relay/profiles/set_enabled" => {
                let params: SetProfileEnabledParams = parse_params(request.params)?;
                serialize(
                    self.app
                        .set_profile_enabled(&params.profile_id, params.enabled)
                        .await
                        .map_err(|e| rpc_from_error(&e))?,
                )?
            }
            "relay/usage/get" => {
                let params: UsageGetParams = parse_params(request.params)?;
                let snapshot = if let Some(profile_id) = params.profile_id {
                    self.app
                        .profile_usage_report(&profile_id)
                        .await
                        .map_err(|e| rpc_from_error(&e))?
                } else {
                    self.app
                        .usage_report()
                        .await
                        .map_err(|e| rpc_from_error(&e))?
                };
                serialize(UsageResult { snapshot })?
            }
            "relay/usage/refresh" => serialize(self.handle_refresh(request.params).await?)?,
            "relay/switch/activate" => {
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
            "relay/switch/next" => serialize(
                self.handle_switch_result(
                    self.app.switch_next_profile().await,
                    SwitchTrigger::Manual,
                    None,
                )
                .await?,
            )?,
            "relay/settings/get" => serialize(SettingsResult {
                app: self.app.settings().await.map_err(|e| rpc_from_error(&e))?,
                codex: self
                    .app
                    .codex_settings()
                    .await
                    .map_err(|e| rpc_from_error(&e))?,
            })?,
            "relay/settings/update" => {
                serialize(self.handle_settings_update(request.params).await?)?
            }
            "relay/activity/events/list" => {
                let params: ActivityEventsParams = parse_params(request.params)?;
                serialize(ActivityEventsResult {
                    events: self
                        .app
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
                    logs: self
                        .app
                        .logs_tail(params.lines)
                        .map_err(|e| rpc_from_error(&e))?,
                })?
            }
            "relay/activity/diagnostics/export" => serialize(
                self.app
                    .diagnostics_export()
                    .await
                    .map_err(|e| rpc_from_error(&e))?,
            )?,
            "shutdown" => {
                self.shutdown_requested = true;
                serialize(serde_json::json!({ "accepted": true }))?
            }
            other => return Err(rpc_method_not_found(other)),
        };

        Ok(RpcSuccessResponse {
            jsonrpc: "2.0".into(),
            id: request.id,
            result,
        })
    }

    async fn handle_initialize(&mut self, params: Value) -> Result<Value, crate::RpcErrorObject> {
        let _: InitializeParams = parse_params(params)?;
        self.connection_state = EngineConnectionState::Ready;
        serialize(InitializeResult {
            protocol_version: "1".into(),
            server_info: RpcServerInfo {
                name: "relay".into(),
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
                    connection_state: self.connection_state,
                },
            },
        })
    }

    async fn handle_subscribe(&mut self, params: Value) -> Result<Value, crate::RpcErrorObject> {
        let params: SubscribeParams = parse_params(params)?;
        self.subscribed_topics.extend(params.topics.iter().copied());
        self.publish_health_update(EngineConnectionState::Ready, None)
            .await
            .map_err(|e| rpc_internal_error(e.to_string()))?;
        serialize(SubscribeResult {
            subscribed_topics: params.topics,
        })
    }

    async fn handle_unsubscribe(&mut self, params: Value) -> Result<Value, crate::RpcErrorObject> {
        let params: SubscribeParams = parse_params(params)?;
        for topic in params.topics {
            self.subscribed_topics.remove(&topic);
        }
        serialize(SubscribeResult {
            subscribed_topics: self.subscribed_topics.iter().copied().collect(),
        })
    }

    async fn handle_refresh(
        &mut self,
        params: Value,
    ) -> Result<RefreshUsageResult, crate::RpcErrorObject> {
        let params: RefreshUsageParams = parse_params(params)?;
        let snapshots = if let Some(profile_id) = params.profile_id {
            vec![
                self.app
                    .refresh_usage_profile(&profile_id)
                    .await
                    .map_err(|e| rpc_from_error(&e))?,
            ]
        } else if params.include_disabled {
            self.app
                .refresh_all_usage_reports()
                .await
                .map_err(|e| rpc_from_error(&e))?
        } else {
            self.app
                .refresh_enabled_usage_reports()
                .await
                .map_err(|e| rpc_from_error(&e))?
        };
        self.publish_usage_updated(snapshots.clone(), UsageUpdateTrigger::Manual)
            .await
            .map_err(|e| rpc_internal_error(e.to_string()))?;
        self.publish_active_state_updated()
            .await
            .map_err(|e| rpc_internal_error(e.to_string()))?;
        Ok(RefreshUsageResult { snapshots })
    }

    async fn handle_settings_update(
        &mut self,
        params: Value,
    ) -> Result<SettingsResult, crate::RpcErrorObject> {
        let params: SettingsUpdateParams = parse_params(params)?;
        let app = if let Some(app_patch) = params.app {
            self.app
                .update_system_settings(SystemSettingsUpdateRequest {
                    auto_switch_enabled: app_patch.auto_switch_enabled,
                    cooldown_seconds: app_patch.cooldown_seconds,
                    refresh_interval_seconds: app_patch.refresh_interval_seconds,
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
        Ok(SettingsResult { app, codex })
    }

    async fn handle_switch_result(
        &mut self,
        result: Result<crate::SwitchReport, RelayError>,
        trigger: SwitchTrigger,
        profile_id: Option<String>,
    ) -> Result<crate::SwitchReport, crate::RpcErrorObject> {
        match result {
            Ok(report) => {
                self.publish_switch_completed(report.clone(), trigger)
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
                Err(rpc_from_error(&error))
            }
        }
    }

    async fn refresh_and_maybe_auto_switch(
        &mut self,
        trigger: UsageUpdateTrigger,
    ) -> Result<(), RelayError> {
        let snapshots = self.app.refresh_enabled_usage_reports().await?;
        self.publish_usage_updated(snapshots.clone(), trigger)
            .await?;
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
        if !active_snapshot.can_auto_switch {
            return Ok(());
        }

        match self.app.switch_next_profile().await {
            Ok(report) => {
                self.publish_switch_completed(report, SwitchTrigger::Auto)
                    .await?;
                let post_switch = self.app.refresh_enabled_usage_reports().await?;
                self.publish_usage_updated(post_switch, UsageUpdateTrigger::PostSwitch)
                    .await?;
                self.publish_active_state_updated().await?;
            }
            Err(error) => {
                self.publish_switch_failed(&error, Some(active_profile_id), SwitchTrigger::Auto)
                    .await?;
            }
        }

        Ok(())
    }

    async fn publish_usage_updated(
        &mut self,
        snapshots: Vec<crate::UsageSnapshot>,
        trigger: UsageUpdateTrigger,
    ) -> Result<(), RelayError> {
        self.publish(
            RelayRpcTopic::UsageUpdated,
            UsageUpdatedPayload { snapshots, trigger },
        )
        .await
    }

    async fn publish_active_state_updated(&mut self) -> Result<(), RelayError> {
        let status = self.app.system_status().await?;
        let active_profile = self
            .app
            .list_profiles_with_usage()
            .await?
            .into_iter()
            .find(|item| {
                status.active_state.active_profile_id.as_deref() == Some(item.profile.id.as_str())
            });
        self.publish(
            RelayRpcTopic::ActiveStateUpdated,
            ActiveStateUpdatedPayload {
                active_state: status.active_state,
                active_profile,
            },
        )
        .await
    }

    async fn publish_switch_completed(
        &mut self,
        report: crate::SwitchReport,
        trigger: SwitchTrigger,
    ) -> Result<(), RelayError> {
        self.publish(
            RelayRpcTopic::SwitchCompleted,
            SwitchCompletedPayload { report, trigger },
        )
        .await
    }

    async fn publish_switch_failed(
        &mut self,
        error: &RelayError,
        profile_id: Option<String>,
        trigger: SwitchTrigger,
    ) -> Result<(), RelayError> {
        self.publish(
            RelayRpcTopic::SwitchFailed,
            SwitchFailedPayload {
                error_code: error.code().as_str().to_string(),
                message: error.to_string(),
                profile_id,
                trigger,
            },
        )
        .await
    }

    pub async fn publish_health_update(
        &mut self,
        state: EngineConnectionState,
        detail: Option<String>,
    ) -> Result<(), RelayError> {
        self.connection_state = state;
        self.publish(
            RelayRpcTopic::HealthUpdated,
            HealthUpdatedPayload { state, detail },
        )
        .await
    }

    async fn publish<T: serde::Serialize>(
        &mut self,
        topic: RelayRpcTopic,
        payload: T,
    ) -> Result<(), RelayError> {
        if !self.subscribed_topics.contains(&topic) {
            return Ok(());
        }
        self.seq += 1;
        self.notifications
            .send(RpcNotification {
                jsonrpc: "2.0".into(),
                method: "session/update".into(),
                params: SessionUpdate {
                    topic,
                    seq: self.seq,
                    timestamp: Utc::now(),
                    payload: serde_json::to_value(payload)
                        .map_err(|error| RelayError::Internal(error.to_string()))?,
                },
            })
            .map_err(|error| RelayError::Internal(error.to_string()))
    }
}

fn parse_params<T: DeserializeOwned>(params: Value) -> Result<T, crate::RpcErrorObject> {
    serde_json::from_value(params).map_err(|error| rpc_invalid_params(error.to_string()))
}

fn serialize<T: serde::Serialize>(value: T) -> Result<Value, crate::RpcErrorObject> {
    serde_json::to_value(value).map_err(|error| rpc_internal_error(error.to_string()))
}

fn parse_failure_reason(value: String) -> Result<FailureReason, crate::RpcErrorObject> {
    match value.as_str() {
        "SessionExhausted" => Ok(FailureReason::SessionExhausted),
        "WeeklyExhausted" => Ok(FailureReason::WeeklyExhausted),
        "AuthInvalid" => Ok(FailureReason::AuthInvalid),
        "QuotaExhausted" => Ok(FailureReason::QuotaExhausted),
        "RateLimited" => Ok(FailureReason::RateLimited),
        "CommandFailed" => Ok(FailureReason::CommandFailed),
        "ValidationFailed" => Ok(FailureReason::ValidationFailed),
        "Unknown" => Ok(FailureReason::Unknown),
        other => Err(rpc_invalid_params(format!(
            "unsupported failure reason: {other}"
        ))),
    }
}
