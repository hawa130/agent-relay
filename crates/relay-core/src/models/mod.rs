mod agent_link;
mod diagnostics;
mod doctor;
mod errors;
mod events;
mod overview;
mod probe_identity;
mod profile;
mod protocol;
mod rpc;
mod settings;
mod state;
mod switch;
mod usage;

pub use agent_link::AgentLinkResult;
pub use diagnostics::{DiagnosticsExport, LogTail};
pub use doctor::DoctorReport;
pub use errors::{ErrorCode, RelayError};
pub use events::{FailureEvent, FailureReason};
pub use overview::{
    ProfileDetail, ProfileListItem, ProfileRecoveryReport, RecoveredProfile,
    SkippedRecoveredProfile, SystemStatusReport,
};
pub use probe_identity::{ProbeProvider, ProfileProbeIdentity};
pub use profile::{AgentKind, AuthMode, Profile, ProfileAccountState};
pub use protocol::JsonResponse;
pub use rpc::{
    ActiveStateUpdatedPayload, ActivityEventsParams, ActivityEventsResult,
    ActivityEventsUpdatedPayload, ActivityLogsUpdatedPayload, ActivityRefreshResult,
    AddProfileParams, AppSettingsPatch, CodexSettingsParams, CodexSettingsPatch,
    DoctorUpdatedPayload, EditProfileParams, EngineConnectionState, EngineState,
    HealthUpdatedPayload, ImportProfileParams, InitialState, InitializeParams, InitializeResult,
    LoginProfileParams, LogsTailParams, LogsTailResult, ProfileIdParams, ProfileResult,
    ProfilesUpdatedPayload, QueryStateItem, QueryStateKey, QueryStateKind, QueryStateStatus,
    QueryStateTrigger, QueryStateUpdatedPayload, RefreshUsageParams, RefreshUsageResult,
    RelayRpcTopic, RelayTaskKind, RelayTaskStatus, RpcClientCapabilities, RpcClientInfo,
    RpcErrorData, RpcErrorObject, RpcErrorResponse, RpcNotification, RpcRequest,
    RpcServerCapabilities, RpcServerInfo, RpcSuccessResponse, SessionUpdate,
    SetProfileEnabledParams, SettingsResult, SettingsUpdateParams, SettingsUpdatedPayload,
    SubscribeParams, SubscribeResult, SwitchCompletedPayload, SwitchFailedPayload,
    SystemSettingsParams, TaskCancelParams, TaskCancelResult, TaskStartResult, TaskUpdate,
    TaskUpdatedPayload, UsageGetParams, UsageResult, UsageUpdateTrigger, UsageUpdatedPayload,
    rpc_from_error, rpc_internal_error, rpc_invalid_params, rpc_invalid_request,
    rpc_method_not_found,
};
pub use settings::AppSettings;
pub use state::{ActiveState, StatusReport, SwitchCheckpoint, SwitchOutcome};
pub use switch::{SwitchHistoryEntry, SwitchReport, SwitchTrigger};
pub use usage::{
    UsageCache, UsageConfidence, UsageRemoteError, UsageRemoteErrorKind, UsageSnapshot,
    UsageSource, UsageSourceMode, UsageStatus, UsageWindow,
};
