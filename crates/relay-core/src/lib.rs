pub mod adapters;
mod app;
pub mod models;
pub mod platform;
pub mod services;
pub mod store;

pub use app::BootstrapMode;
pub use app::{
    AddProfileRequest, CodexLoginRequest, EditProfileRequest, RelayApp, UsageSettingsUpdateRequest,
};
pub use models::{
    ActiveState, AgentKind, AppSettings, AuthMode, CodexLinkResult, DiagnosticsExport,
    DoctorReport, ErrorCode, FailureEvent, FailureReason, JsonResponse, LogTail, ProbeProvider,
    Profile, ProfileProbeIdentity, RelayError, StatusReport, SwitchCheckpoint, SwitchHistoryEntry,
    SwitchOutcome, SwitchReport, UsageCache, UsageConfidence, UsageSnapshot, UsageSource,
    UsageSourceMode, UsageStatus, UsageWindow,
};
