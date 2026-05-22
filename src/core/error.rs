use std::time::Duration;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("OPENAI_API_KEY is not set")]
    MissingApiKey,
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("I/O operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to serialize or deserialize JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("OpenAI API error ({status}): {message}")]
    Api {
        status: StatusCode,
        message: String,
        error: Option<ApiError>,
    },
    #[error("stream error: {0}")]
    Stream(String),
    #[error("failed to parse chat completion: {0}")]
    Parse(String),
    #[error("operation timed out after {0:?}")]
    Timeout(Duration),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub param: Option<String>,
    pub code: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ApiErrorEnvelope {
    pub error: Option<ApiError>,
}
