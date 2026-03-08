use crate::models::RelayError;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct JsonResponse<T>
where
    T: Serialize,
{
    pub success: bool,
    pub error_code: Option<String>,
    pub message: String,
    pub data: Option<T>,
}

impl<T> JsonResponse<T>
where
    T: Serialize,
{
    pub fn success(message: impl Into<String>, data: T) -> Self {
        Self {
            success: true,
            error_code: None,
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn error(error: &RelayError) -> Self {
        Self {
            success: false,
            error_code: Some(error.code().as_str().to_string()),
            message: error.message().into_owned(),
            data: None,
        }
    }

    pub fn write_json(&self) -> Result<(), RelayError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|error| RelayError::Internal(error.to_string()))?;
        println!("{json}");
        Ok(())
    }
}
