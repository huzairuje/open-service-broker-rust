//! Shared types used across multiple OSB requests/responses.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Free-form context object describing the platform making the request.
/// e.g., `{ "platform": "kubernetes", "namespace": "..." }`.
pub type Context = Value;

/// Free-form parameters supplied by the user when provisioning/binding.
pub type Parameters = Value;

/// Maintenance info on a plan or instance. The platform uses this to
/// detect whether an update is required to apply maintenance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MaintenanceInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Response body for async operations: `{"operation": "<token>"}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
}

/// State of an asynchronous operation as reported by `last_operation`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OperationState {
    #[serde(rename = "in progress")]
    InProgress,
    #[serde(rename = "succeeded")]
    Succeeded,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastOperationResponse {
    pub state: OperationState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
