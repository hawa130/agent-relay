pub mod adapters;
mod app;
pub mod models;
pub mod platform;
pub mod services;
pub mod store;

pub use app::{AddProfileRequest, EditProfileRequest, RelayApp};
pub use models::{
    ActiveState, AgentKind, AppSettings, AuthMode, DiagnosticsExport, DoctorReport, ErrorCode,
    FailureEvent, FailureReason, JsonResponse, LogTail, Profile, RelayError, StatusReport,
    SwitchCheckpoint, SwitchHistoryEntry, SwitchOutcome, SwitchReport,
};
