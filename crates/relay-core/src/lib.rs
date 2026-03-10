pub mod adapters;
mod app;
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
    ActiveState, AgentKind, AgentLinkResult, AppSettings, AuthMode, DiagnosticsExport,
    DoctorReport, ErrorCode, FailureEvent, FailureReason, JsonResponse, LogTail, ProbeProvider,
    Profile, ProfileDetail, ProfileListItem, ProfileProbeIdentity, RelayError, StatusReport,
    SwitchCheckpoint, SwitchHistoryEntry, SwitchOutcome, SwitchReport, SystemStatusReport,
    UsageCache, UsageConfidence, UsageSnapshot, UsageSource, UsageSourceMode, UsageStatus,
    UsageWindow,
};
