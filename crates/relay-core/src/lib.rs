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
    ActiveState, ActivityEventsUpdatedPayload, ActivityLogsUpdatedPayload, ActivityRefreshResult,
    AddProfileParams, AgentKind, AgentLinkResult, AppSettings, AuthMode, DiagnosticsExport,
    DoctorReport, DoctorUpdatedPayload, EditProfileParams, EngineConnectionState, EngineState,
    ErrorCode, FailureEvent, FailureReason, ImportProfileParams, InitializeParams,
    InitializeResult, JsonResponse, LogTail, LoginProfileParams, ProbeProvider, Profile,
    ProfileAccountState, ProfileDetail, ProfileIdParams, ProfileListItem, ProfileProbeIdentity,
    ProfileRecoveryReport, ProfilesUpdatedPayload, QueryStateItem, QueryStateKey, QueryStateKind,
    QueryStateStatus, QueryStateTrigger, QueryStateUpdatedPayload, RecoveredProfile,
    RefreshUsageParams, RefreshUsageResult, RelayError, RelayRpcTopic, RelayTaskKind,
    RelayTaskStatus, RpcClientCapabilities, RpcClientInfo, RpcErrorData, RpcErrorObject,
    RpcErrorResponse, RpcNotification, RpcRequest, RpcServerCapabilities, RpcServerInfo,
    RpcSuccessResponse, SessionUpdate, SetProfileEnabledParams, SettingsResult,
    SettingsUpdateParams, SettingsUpdatedPayload, SkippedRecoveredProfile, StatusReport,
    SubscribeParams, SubscribeResult, SwitchCheckpoint, SwitchCompletedPayload,
    SwitchFailedPayload, SwitchHistoryEntry, SwitchOutcome, SwitchReport, SwitchTrigger,
    SystemStatusReport, TaskCancelParams, TaskCancelResult, TaskStartResult, TaskUpdate,
    TaskUpdatedPayload, UsageCache, UsageConfidence, UsageGetParams, UsageRemoteError,
    UsageRemoteErrorKind, UsageResult, UsageSnapshot, UsageSource, UsageSourceMode, UsageStatus,
    UsageUpdateTrigger, UsageUpdatedPayload, UsageWindow,
};
pub use services::daemon_service::DaemonService;
