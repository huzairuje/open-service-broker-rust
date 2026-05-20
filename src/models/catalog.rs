//! Catalog DTOs: `GET /v2/catalog` response.
//!
//! Mirrors the OSB spec field-for-field via serde rename attributes.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::common::MaintenanceInfo;

/// Top-level catalog response: a list of services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub services: Vec<Service>,
}

/// A service offered by the broker (e.g., "postgresql").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    /// Stable, unique identifier of the service (UUID recommended).
    pub id: String,
    /// CLI-friendly name (e.g., "postgresql").
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Whether instances of this service can be bound to apps.
    pub bindable: bool,
    /// Tags to help search/categorize the service.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Permissions the service requires (e.g., "syslog_drain").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    /// Free-form display metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    /// Available plans for this service.
    pub plans: Vec<Plan>,
    /// Whether the platform may update existing instances to a new plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_updateable: Option<bool>,
    /// Whether instance deletion can be retried after a failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instances_retrievable: Option<bool>,
    /// Whether bindings can be retrieved via GET.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bindings_retrievable: Option<bool>,
    /// Whether the broker accepts the `context` object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_context_updates: Option<bool>,
    /// Identity provider configuration (rare).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dashboard_client: Option<DashboardClient>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardClient {
    pub id: String,
    pub secret: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
}

/// A plan is a tier of a service (e.g., "free", "pro").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bindable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_updateable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemas: Option<Schemas>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_polling_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintenance_info: Option<MaintenanceInfo>,
}

/// JSON-Schema definitions for parameter validation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Schemas {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_instance: Option<ServiceInstanceSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_binding: Option<ServiceBindingSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceInstanceSchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create: Option<SchemaParameters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<SchemaParameters>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceBindingSchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create: Option<SchemaParameters>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SchemaParameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}
