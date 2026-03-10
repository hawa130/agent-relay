mod agent_link;
mod diagnostics;
mod doctor;
mod errors;
mod events;
mod overview;
mod probe_identity;
mod profile;
mod protocol;
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
pub use profile::{AgentKind, AuthMode, Profile};
pub use protocol::JsonResponse;
pub use settings::AppSettings;
pub use state::{ActiveState, StatusReport, SwitchCheckpoint, SwitchOutcome};
pub use switch::{SwitchHistoryEntry, SwitchReport};
pub use usage::{
    UsageCache, UsageConfidence, UsageSnapshot, UsageSource, UsageSourceMode, UsageStatus,
    UsageWindow,
};
