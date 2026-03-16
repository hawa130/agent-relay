use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FailureReason {
    SessionExhausted,
    WeeklyExhausted,
    AccountUnavailable,
    AuthInvalid,
    QuotaExhausted,
    RateLimited,
    CommandFailed,
    ValidationFailed,
    Unknown,
}

impl FromStr for FailureReason {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "session-exhausted" | "SessionExhausted" => Ok(Self::SessionExhausted),
            "weekly-exhausted" | "WeeklyExhausted" => Ok(Self::WeeklyExhausted),
            "account-unavailable" | "AccountUnavailable" => Ok(Self::AccountUnavailable),
            "auth-invalid" | "AuthInvalid" => Ok(Self::AuthInvalid),
            "quota-exhausted" | "QuotaExhausted" => Ok(Self::QuotaExhausted),
            "rate-limited" | "RateLimited" => Ok(Self::RateLimited),
            "command-failed" | "CommandFailed" => Ok(Self::CommandFailed),
            "validation-failed" | "ValidationFailed" => Ok(Self::ValidationFailed),
            "unknown" | "Unknown" => Ok(Self::Unknown),
            other => Err(format!("unsupported failure reason: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureEvent {
    pub id: String,
    pub profile_id: Option<String>,
    pub reason: FailureReason,
    pub message: String,
    pub cooldown_until: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
