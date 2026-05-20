//! DTOs for `/v2/service_instances/:id` provisioning lifecycle.

use serde::{Deserialize, Serialize};

use crate::models::common::{Context, MaintenanceInfo, Parameters};

/// Request body for `PUT /v2/service_instances/:id` (provision).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionRequest {
    pub service_id: String,
    pub plan_id: String,
    /// Required pre-2.13; replaced by `context` in newer versions but
    /// still sent by many platforms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Context>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Parameters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintenance_info: Option<MaintenanceInfo>,
}

/// Response body for a successful provision.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvisionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dashboard_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Request body for `PATCH /v2/service_instances/:id` (update).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRequest {
    pub service_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Parameters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Context>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_values: Option<PreviousValues>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintenance_info: Option<MaintenanceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousValues {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintenance_info: Option<MaintenanceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dashboard_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
}

/// Response body for `GET /v2/service_instances/:id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInstanceResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dashboard_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Parameters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintenance_info: Option<MaintenanceInfo>,
}

/// Stored representation of a provisioned instance.
#[derive(Debug, Clone)]
pub struct ServiceInstance {
    pub id: String,
    pub service_id: String,
    pub plan_id: String,
    pub parameters: Option<Parameters>,
    pub context: Option<Context>,
    pub dashboard_url: Option<String>,
}
