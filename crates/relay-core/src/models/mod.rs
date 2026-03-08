mod diagnostics;
mod doctor;
mod errors;
mod events;
mod profile;
mod protocol;
mod settings;
mod state;
mod switch;

pub use diagnostics::{DiagnosticsExport, LogTail};
pub use doctor::DoctorReport;
pub use errors::{ErrorCode, RelayError};
pub use events::{FailureEvent, FailureReason};
pub use profile::{AgentKind, AuthMode, Profile};
pub use protocol::JsonResponse;
pub use settings::AppSettings;
pub use state::{ActiveState, StatusReport, SwitchCheckpoint, SwitchOutcome};
pub use switch::{SwitchHistoryEntry, SwitchReport};
