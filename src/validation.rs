//! JSON-Schema validation for OSB `parameters`.
//!
//! Plans may declare schemas under `plan.schemas.service_instance.create`
//! / `update` and `plan.schemas.service_binding.create`. When present,
//! the broker validates the incoming `parameters` against them before
//! persisting anything.

use jsonschema::JSONSchema;
use serde_json::Value;

use crate::error::{BrokerError, BrokerResult};
use crate::models::catalog::Plan;

/// Which schema on the plan to use.
#[derive(Debug, Clone, Copy)]
pub enum SchemaKind {
    InstanceCreate,
    InstanceUpdate,
    BindingCreate,
}

/// Validate `parameters` (which may be `None`) against the relevant
/// schema declared on the plan. If no schema is declared, this is a
/// no-op.
pub fn validate(plan: &Plan, kind: SchemaKind, parameters: Option<&Value>) -> BrokerResult<()> {
    let schema = match (kind, plan.schemas.as_ref()) {
        (SchemaKind::InstanceCreate, Some(s)) => s
            .service_instance
            .as_ref()
            .and_then(|si| si.create.as_ref())
            .and_then(|c| c.parameters.as_ref()),
        (SchemaKind::InstanceUpdate, Some(s)) => s
            .service_instance
            .as_ref()
            .and_then(|si| si.update.as_ref())
            .and_then(|c| c.parameters.as_ref()),
        (SchemaKind::BindingCreate, Some(s)) => s
            .service_binding
            .as_ref()
            .and_then(|sb| sb.create.as_ref())
            .and_then(|c| c.parameters.as_ref()),
        _ => None,
    };

    let Some(schema) = schema else {
        return Ok(());
    };

    // OSB treats `parameters` as optional. If the caller sent nothing,
    // skip validation entirely; schemas only constrain what *was* sent.
    let Some(to_check) = parameters else {
        return Ok(());
    };

    let compiled = JSONSchema::compile(schema)
        .map_err(|e| BrokerError::Internal(format!("invalid plan schema: {e}")))?;

    if let Err(errors) = compiled.validate(to_check) {
        let messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        return Err(BrokerError::BadRequest(format!(
            "parameters failed schema validation: {}",
            messages.join("; ")
        )));
    }
    Ok(())
}
