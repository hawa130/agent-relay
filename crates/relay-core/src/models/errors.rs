use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidInput,
    NotFound,
    NotImplemented,
    Io,
    Store,
    SchemaIncompatible,
    Validation,
    Conflict,
    ExternalCommand,
    Auth,
    QuotaExhausted,
    RateLimited,
    Internal,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "RELAY_INVALID_INPUT",
            Self::NotFound => "RELAY_NOT_FOUND",
            Self::NotImplemented => "RELAY_NOT_IMPLEMENTED",
            Self::Io => "RELAY_IO",
            Self::Store => "RELAY_STORE",
            Self::SchemaIncompatible => "RELAY_SCHEMA_INCOMPATIBLE",
            Self::Validation => "RELAY_VALIDATION",
            Self::Conflict => "RELAY_CONFLICT",
            Self::ExternalCommand => "RELAY_EXTERNAL_COMMAND",
            Self::Auth => "RELAY_AUTH",
            Self::QuotaExhausted => "RELAY_QUOTA_EXHAUSTED",
            Self::RateLimited => "RELAY_RATE_LIMITED",
            Self::Internal => "RELAY_INTERNAL",
        }
    }
}

#[derive(Debug, Clone, Error)]
pub enum RelayError {
    #[error("{0}")]
    InvalidInput(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    NotImplemented(&'static str),
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Store(String),
    #[error("{0}")]
    SchemaIncompatible(String),
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    ExternalCommand(String),
    #[error("{0}")]
    Auth(String),
    #[error("{0}")]
    QuotaExhausted(String),
    #[error("{0}")]
    RateLimited(String),
    #[error("{0}")]
    Internal(String),
}

impl RelayError {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidInput(_) => ErrorCode::InvalidInput,
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::NotImplemented(_) => ErrorCode::NotImplemented,
            Self::Io(_) => ErrorCode::Io,
            Self::Store(_) => ErrorCode::Store,
            Self::SchemaIncompatible(_) => ErrorCode::SchemaIncompatible,
            Self::Validation(_) => ErrorCode::Validation,
            Self::Conflict(_) => ErrorCode::Conflict,
            Self::ExternalCommand(_) => ErrorCode::ExternalCommand,
            Self::Auth(_) => ErrorCode::Auth,
            Self::QuotaExhausted(_) => ErrorCode::QuotaExhausted,
            Self::RateLimited(_) => ErrorCode::RateLimited,
            Self::Internal(_) => ErrorCode::Internal,
        }
    }

    pub fn message(&self) -> Cow<'_, str> {
        match self {
            Self::InvalidInput(s)
            | Self::Io(s)
            | Self::Store(s)
            | Self::SchemaIncompatible(s)
            | Self::Validation(s)
            | Self::Conflict(s)
            | Self::ExternalCommand(s)
            | Self::Auth(s)
            | Self::QuotaExhausted(s)
            | Self::RateLimited(s)
            | Self::Internal(s)
            | Self::NotFound(s) => Cow::Borrowed(s.as_str()),
            Self::NotImplemented(s) => Cow::Borrowed(s),
        }
    }
}

impl From<std::io::Error> for RelayError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<sea_orm::DbErr> for RelayError {
    fn from(value: sea_orm::DbErr) -> Self {
        Self::Store(value.to_string())
    }
}
