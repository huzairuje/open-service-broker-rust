//! DTOs for `/v2/service_instances/:id/service_bindings/:bid`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::common::{Context, Parameters};

/// Request body for `PUT .../service_bindings/:bid` (bind).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindRequest {
    pub service_id: String,
    pub plan_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_resource: Option<BindResource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Parameters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Context>,
}

/// `bind_resource` describes what the binding is for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindResource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
}

/// Response body for a successful bind.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BindResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<HashMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syslog_drain_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_service_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_mounts: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
}

/// Stored representation of a binding.
#[derive(Debug, Clone)]
pub struct ServiceBinding {
    pub id: String,
    pub instance_id: String,
    pub service_id: String,
    pub plan_id: String,
    pub credentials: HashMap<String, Value>,
    pub parameters: Option<Parameters>,
}
