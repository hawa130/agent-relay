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
    Validation,
    Conflict,
    ExternalCommand,
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
            Self::Validation => "RELAY_VALIDATION",
            Self::Conflict => "RELAY_CONFLICT",
            Self::ExternalCommand => "RELAY_EXTERNAL_COMMAND",
            Self::Internal => "RELAY_INTERNAL",
        }
    }
}

#[derive(Debug, Error)]
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
    Validation(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    ExternalCommand(String),
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
            Self::Validation(_) => ErrorCode::Validation,
            Self::Conflict(_) => ErrorCode::Conflict,
            Self::ExternalCommand(_) => ErrorCode::ExternalCommand,
            Self::Internal(_) => ErrorCode::Internal,
        }
    }

    pub fn message(&self) -> Cow<'_, str> {
        Cow::Owned(self.to_string())
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
