use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailureReason {
    AuthInvalid,
    QuotaExhausted,
    RateLimited,
    CommandFailed,
    ValidationFailed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureEvent {
    pub id: String,
    pub profile_id: Option<String>,
    pub reason: FailureReason,
    pub message: String,
    pub cooldown_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
