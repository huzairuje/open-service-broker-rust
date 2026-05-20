//! Error type for the broker. Each variant maps to a specific HTTP
//! status code and OSB-spec error body.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// OSB-spec error body. Brokers SHOULD return `error` (a machine-readable
/// code) and `description` (human-readable).
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_usable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_repeatable: Option<bool>,
}

#[derive(Debug, Error)]
pub enum BrokerError {
    #[error("missing or unsupported X-Broker-API-Version header")]
    UnsupportedApiVersion,

    #[error("authentication failed")]
    Unauthorized,

    #[error("invalid request: {0}")]
    BadRequest(String),

    #[error("resource not found: {0}")]
    NotFound(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("gone: {0}")]
    Gone(String),

    #[error("unprocessable: {code} - {description}")]
    Unprocessable { code: String, description: String },

    #[error("async required for this operation")]
    AsyncRequired,

    #[error("concurrency error: {0}")]
    ConcurrencyError(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for BrokerError {
    fn into_response(self) -> Response {
        let (status, code, description) = match &self {
            BrokerError::UnsupportedApiVersion => (
                StatusCode::PRECONDITION_FAILED,
                Some("PreconditionFailed".to_string()),
                self.to_string(),
            ),
            BrokerError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                Some("Unauthorized".to_string()),
                self.to_string(),
            ),
            BrokerError::BadRequest(_) => (StatusCode::BAD_REQUEST, None, self.to_string()),
            BrokerError::NotFound(_) => (StatusCode::NOT_FOUND, None, self.to_string()),
            BrokerError::Conflict(_) => (StatusCode::CONFLICT, None, self.to_string()),
            BrokerError::Gone(_) => (StatusCode::GONE, None, self.to_string()),
            BrokerError::Unprocessable { code, description } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Some(code.clone()),
                description.clone(),
            ),
            BrokerError::AsyncRequired => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Some("AsyncRequired".to_string()),
                "This service plan requires client support for asynchronous \
                 service operations."
                    .to_string(),
            ),
            BrokerError::ConcurrencyError(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Some("ConcurrencyError".to_string()),
                self.to_string(),
            ),
            BrokerError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, None, self.to_string()),
        };

        let body = ErrorBody {
            error: code,
            description,
            instance_usable: None,
            update_repeatable: None,
        };
        (status, Json(body)).into_response()
    }
}

pub type BrokerResult<T> = Result<T, BrokerError>;
