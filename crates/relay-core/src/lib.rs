pub mod adapters;
mod app;
mod internal;
pub mod models;
pub mod platform;
pub mod services;
pub mod store;

pub use adapters::codex::{CodexSettings, CodexSettingsUpdateRequest};
pub use app::BootstrapMode;
pub use app::{
    ActivityEventsQuery, AddProfileRequest, AgentLoginMode, AgentLoginRequest, EditProfileRequest,
    ImportProfileRequest, RelayApp, SystemSettingsUpdateRequest,
};
pub use models::{
    ActiveState, AddProfileParams, AgentKind, AgentLinkResult, AppSettings, AuthMode,
    DiagnosticsExport, DoctorReport, EditProfileParams, EngineConnectionState, EngineState,
    ErrorCode, FailureEvent, FailureReason, ImportProfileParams, InitializeParams,
    InitializeResult, JsonResponse, LogTail, LoginProfileParams, ProbeProvider, Profile,
    ProfileDetail, ProfileIdParams, ProfileListItem, ProfileProbeIdentity, ProfileRecoveryReport,
    RecoveredProfile, RefreshUsageParams, RefreshUsageResult, RelayError, RelayRpcTopic,
    RpcClientCapabilities, RpcClientInfo, RpcErrorData, RpcErrorObject, RpcErrorResponse,
    RpcNotification, RpcRequest, RpcServerCapabilities, RpcServerInfo, RpcSuccessResponse,
    SessionUpdate, SetProfileEnabledParams, SettingsResult, SettingsUpdateParams,
    SkippedRecoveredProfile, StatusReport, SubscribeParams, SubscribeResult, SwitchCheckpoint,
    SwitchCompletedPayload, SwitchFailedPayload, SwitchHistoryEntry, SwitchOutcome, SwitchReport,
    SwitchTrigger, SystemSettingsParams, SystemStatusReport, UsageCache, UsageConfidence,
    UsageGetParams, UsageResult, UsageSnapshot, UsageSource, UsageSourceMode, UsageStatus,
    UsageUpdateTrigger, UsageUpdatedPayload, UsageWindow,
};
pub use services::daemon_service::DaemonService;
