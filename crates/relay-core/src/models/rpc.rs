use crate::CodexSettings;
use crate::models::{
    ActiveState, AppSettings, ErrorCode, FailureEvent, LogTail, ProfileListItem, SwitchReport,
    SwitchTrigger, SystemStatusReport, UsageSnapshot,
};
use crate::{AddProfileRequest, AgentLoginRequest, EditProfileRequest, ImportProfileRequest};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const JSONRPC_VERSION: &str = "2.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcSuccessResponse {
    pub jsonrpc: String,
    pub id: String,
    pub result: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcErrorResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub error: RpcErrorObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcErrorObject {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<RpcErrorData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcErrorData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relay_error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: SessionUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    pub client_info: RpcClientInfo,
    pub capabilities: RpcClientCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcClientCapabilities {
    pub supports_subscriptions: bool,
    pub supports_health_updates: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub server_info: RpcServerInfo,
    pub capabilities: RpcServerCapabilities,
    pub initial_state: InitialState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcServerCapabilities {
    pub supports_subscriptions: bool,
    pub supports_health_updates: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialState {
    pub status: SystemStatusReport,
    pub profiles: Vec<ProfileListItem>,
    pub codex_settings: CodexSettings,
    pub engine: EngineState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineState {
    pub started_at: DateTime<Utc>,
    pub connection_state: EngineConnectionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineConnectionState {
    Starting,
    Ready,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeParams {
    pub topics: Vec<RelayRpcTopic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeResult {
    pub subscribed_topics: Vec<RelayRpcTopic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelayRpcTopic {
    #[serde(rename = "usage.updated")]
    UsageUpdated,
    #[serde(rename = "query_state.updated")]
    QueryStateUpdated,
    #[serde(rename = "active_state.updated")]
    ActiveStateUpdated,
    #[serde(rename = "settings.updated")]
    SettingsUpdated,
    #[serde(rename = "profiles.updated")]
    ProfilesUpdated,
    #[serde(rename = "activity.events.updated")]
    ActivityEventsUpdated,
    #[serde(rename = "activity.logs.updated")]
    ActivityLogsUpdated,
    #[serde(rename = "doctor.updated")]
    DoctorUpdated,
    #[serde(rename = "switch.completed")]
    SwitchCompleted,
    #[serde(rename = "switch.failed")]
    SwitchFailed,
    #[serde(rename = "task.updated")]
    TaskUpdated,
    #[serde(rename = "health.updated")]
    HealthUpdated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUpdate {
    pub topic: RelayRpcTopic,
    pub seq: u64,
    pub timestamp: DateTime<Utc>,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageGetParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshUsageParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub include_disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshUsageResult {
    pub snapshots: Vec<UsageSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileIdParams {
    pub profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetProfileEnabledParams {
    pub profile_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditProfileParams {
    pub profile_id: String,
    pub request: EditProfileRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProfileParams {
    pub request: AddProfileRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProfileParams {
    pub request: ImportProfileRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginProfileParams {
    pub request: AgentLoginRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCancelParams {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStartResult {
    pub task_id: String,
    pub kind: RelayTaskKind,
    pub accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCancelResult {
    pub accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEventsParams {
    pub limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEventsResult {
    pub events: Vec<FailureEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityRefreshResult {
    pub events: Vec<FailureEvent>,
    pub logs: LogTail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsTailParams {
    pub lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsResult {
    pub app: AppSettings,
    pub codex: CodexSettings,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppSettingsPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_switch_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown_seconds: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_interval_seconds: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_query_concurrency: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodexSettingsPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_source_mode: Option<crate::models::UsageSourceMode>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SettingsUpdateParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<AppSettingsPatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codex: Option<CodexSettingsPatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsUpdatedPayload {
    pub settings: SettingsResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesUpdatedPayload {
    pub profiles: Vec<ProfileListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEventsUpdatedPayload {
    pub events: Vec<FailureEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLogsUpdatedPayload {
    pub logs: LogTail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorUpdatedPayload {
    pub report: crate::DoctorReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageUpdatedPayload {
    pub snapshots: Vec<UsageSnapshot>,
    pub trigger: UsageUpdateTrigger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsageUpdateTrigger {
    Startup,
    Interval,
    Manual,
    PostSwitch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryStateTrigger {
    Startup,
    Interval,
    Manual,
    PostSwitch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryStateStatus {
    Pending,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueryStateKey {
    pub kind: QueryStateKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QueryStateKind {
    UsageProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStateItem {
    pub key: QueryStateKey,
    pub status: QueryStateStatus,
    pub trigger: QueryStateTrigger,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStateUpdatedPayload {
    pub states: Vec<QueryStateItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveStateUpdatedPayload {
    pub active_state: ActiveState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_profile: Option<ProfileListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchCompletedPayload {
    pub report: SwitchReport,
    pub trigger: SwitchTrigger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchFailedPayload {
    pub error_code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    pub trigger: SwitchTrigger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayTaskKind {
    #[serde(rename = "ProfileLogin")]
    ProfileLogin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayTaskStatus {
    Pending,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskUpdate {
    pub task_id: String,
    pub kind: RelayTaskKind,
    pub status: RelayTaskStatus,
    pub started_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskUpdatedPayload {
    pub task: TaskUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthUpdatedPayload {
    pub state: EngineConnectionState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageResult {
    pub snapshot: UsageSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsTailResult {
    pub logs: LogTail,
}

pub fn rpc_invalid_request(message: impl Into<String>) -> RpcErrorObject {
    RpcErrorObject {
        code: -32600,
        message: message.into(),
        data: None,
    }
}

pub fn rpc_method_not_found(method: &str) -> RpcErrorObject {
    RpcErrorObject {
        code: -32601,
        message: format!("unknown method: {method}"),
        data: None,
    }
}

pub fn rpc_invalid_params(message: impl Into<String>) -> RpcErrorObject {
    RpcErrorObject {
        code: -32602,
        message: message.into(),
        data: None,
    }
}

pub fn rpc_from_error(error: &crate::models::RelayError) -> RpcErrorObject {
    RpcErrorObject {
        code: -32010,
        message: error.to_string(),
        data: Some(RpcErrorData {
            relay_error_code: Some(error.code().as_str().to_string()),
        }),
    }
}

pub fn rpc_internal_error(message: impl Into<String>) -> RpcErrorObject {
    RpcErrorObject {
        code: -32603,
        message: message.into(),
        data: Some(RpcErrorData {
            relay_error_code: Some(ErrorCode::Internal.as_str().to_string()),
        }),
    }
}
