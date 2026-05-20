//! Catalog loading: from JSON or YAML files, falling back to a built-in
//! sample catalog so the broker is usable out of the box.

use std::path::Path;

use crate::error::{BrokerError, BrokerResult};
use crate::models::catalog::{Catalog, Plan, Service};
use serde_json::json;

/// Load a catalog from `path`. Format is detected by the file extension
/// (`.json` or `.yaml`/`.yml`).
pub fn load_from_file<P: AsRef<Path>>(path: P) -> BrokerResult<Catalog> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)
        .map_err(|e| BrokerError::Internal(format!("read catalog {}: {e}", path.display())))?;

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("json")
        .to_ascii_lowercase();

    let catalog: Catalog = match ext.as_str() {
        "yaml" | "yml" => serde_yaml::from_slice(&bytes)
            .map_err(|e| BrokerError::Internal(format!("parse yaml catalog: {e}")))?,
        _ => serde_json::from_slice(&bytes)
            .map_err(|e| BrokerError::Internal(format!("parse json catalog: {e}")))?,
    };
    Ok(catalog)
}

/// Built-in sample catalog used when no file is configured.
pub fn default_catalog() -> Catalog {
    Catalog {
        services: vec![Service {
            id: "4f6e6cf6-ffdd-425f-a2c7-3c9258ad2468".into(),
            name: "example-db".into(),
            description: "An example database service offered by this broker.".into(),
            bindable: true,
            tags: vec!["example".into(), "database".into()],
            requires: vec![],
            metadata: Some(json!({
                "displayName": "Example DB",
                "longDescription": "Reference service used for testing the broker.",
                "providerDisplayName": "rust-open-service-broker"
            })),
            plans: vec![
                Plan {
                    id: "86064792-7ea2-467b-af93-ac9694d96d5b".into(),
                    name: "free".into(),
                    description: "Free shared plan.".into(),
                    metadata: Some(json!({ "bullets": ["Shared", "Free"] })),
                    free: Some(true),
                    bindable: Some(true),
                    plan_updateable: Some(true),
                    schemas: None,
                    maximum_polling_duration: None,
                    maintenance_info: None,
                },
                Plan {
                    id: "f52eabf8-e65c-4f5b-9e86-7f3c2c7b6f24".into(),
                    name: "pro".into(),
                    description: "Dedicated paid plan.".into(),
                    metadata: Some(json!({ "bullets": ["Dedicated", "Paid"] })),
                    free: Some(false),
                    bindable: Some(true),
                    plan_updateable: Some(true),
                    schemas: None,
                    maximum_polling_duration: None,
                    maintenance_info: None,
                },
            ],
            plan_updateable: Some(true),
            instances_retrievable: Some(true),
            bindings_retrievable: Some(true),
            allow_context_updates: None,
            dashboard_client: None,
        }],
    }
}
