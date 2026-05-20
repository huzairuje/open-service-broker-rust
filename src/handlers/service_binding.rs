//! Handlers for `/v2/service_instances/:id/service_bindings/:bid`.

use std::collections::HashMap;
use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    error::{BrokerError, BrokerResult},
    models::{
        common::{LastOperationResponse, OperationState},
        service_binding::{BindRequest, BindResponse, ServiceBinding},
    },
    operations::{spawn_simulated, OperationKind, ResourceKind},
    validation::{validate, SchemaKind},
    AppState,
};

#[derive(Debug, Deserialize, Default)]
pub struct BindQuery {
    #[serde(default)]
    pub accepts_incomplete: Option<bool>,
}

/// `PUT /v2/service_instances/:instance_id/service_bindings/:binding_id`
pub async fn bind(
    State(state): State<AppState>,
    Path((instance_id, binding_id)): Path<(String, String)>,
    Query(q): Query<BindQuery>,
    Json(req): Json<BindRequest>,
) -> BrokerResult<(StatusCode, Json<BindResponse>)> {
    // The instance must exist before we can bind to it.
    let instance = state
        .broker
        .storage()
        .get_instance(&instance_id)
        .await?
        .ok_or_else(|| BrokerError::BadRequest(format!("instance {instance_id} not found")))?;

    let service = state.broker.find_service(&req.service_id)?;
    if !service.bindable {
        return Err(BrokerError::BadRequest(format!(
            "service {} is not bindable",
            service.name
        )));
    }
    let plan = state.broker.find_plan(service, &req.plan_id)?;
    validate(plan, SchemaKind::BindingCreate, req.parameters.as_ref())?;

    if let Some(existing) = state.broker.storage().get_binding(&binding_id).await? {
        return if existing.instance_id == instance_id
            && existing.service_id == req.service_id
            && existing.plan_id == req.plan_id
        {
            Ok((
                StatusCode::OK,
                Json(BindResponse {
                    credentials: Some(existing.credentials),
                    ..Default::default()
                }),
            ))
        } else {
            Err(BrokerError::Conflict(format!(
                "binding {binding_id} already exists with different params"
            )))
        };
    }

    let credentials = sample_credentials(&instance.id);
    let binding = ServiceBinding {
        id: binding_id.clone(),
        instance_id,
        service_id: req.service_id,
        plan_id: req.plan_id,
        credentials: credentials.clone(),
        parameters: req.parameters,
    };

    let async_ms = state.config.async_op_millis;
    if async_ms > 0 && q.accepts_incomplete.unwrap_or(false) {
        let op_id = state.broker.operations().start(
            ResourceKind::Binding,
            &binding_id,
            OperationKind::Bind,
        );
        let storage = state.broker.storage().clone();
        let tracker = state.broker.operations().clone();
        spawn_simulated(
            tracker,
            op_id.clone(),
            Duration::from_millis(async_ms),
            move || {
                let storage = storage.clone();
                tokio::spawn(async move {
                    if let Err(e) = storage.put_binding(binding).await {
                        tracing::error!(error = %e, "async bind failed");
                    }
                });
            },
        );
        return Ok((
            StatusCode::ACCEPTED,
            Json(BindResponse {
                operation: Some(op_id),
                ..Default::default()
            }),
        ));
    }

    state.broker.storage().put_binding(binding).await?;
    Ok((
        StatusCode::CREATED,
        Json(BindResponse {
            credentials: Some(credentials),
            ..Default::default()
        }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct UnbindQuery {
    pub service_id: String,
    pub plan_id: String,
    #[serde(default)]
    pub accepts_incomplete: Option<bool>,
}

/// `DELETE /v2/service_instances/:instance_id/service_bindings/:binding_id`
pub async fn unbind(
    State(state): State<AppState>,
    Path((_instance_id, binding_id)): Path<(String, String)>,
    Query(q): Query<UnbindQuery>,
) -> BrokerResult<(StatusCode, Json<Value>)> {
    if state
        .broker
        .storage()
        .get_binding(&binding_id)
        .await?
        .is_none()
    {
        return Err(BrokerError::Gone(format!("binding {binding_id}")));
    }

    let async_ms = state.config.async_op_millis;
    if async_ms > 0 && q.accepts_incomplete.unwrap_or(false) {
        let op_id = state.broker.operations().start(
            ResourceKind::Binding,
            &binding_id,
            OperationKind::Unbind,
        );
        let storage = state.broker.storage().clone();
        let tracker = state.broker.operations().clone();
        let id = binding_id.clone();
        spawn_simulated(
            tracker,
            op_id.clone(),
            Duration::from_millis(async_ms),
            move || {
                let storage = storage.clone();
                tokio::spawn(async move {
                    let _ = storage.delete_binding(&id).await;
                });
            },
        );
        return Ok((StatusCode::ACCEPTED, Json(json!({ "operation": op_id }))));
    }

    state.broker.storage().delete_binding(&binding_id).await?;
    Ok((StatusCode::OK, Json(json!({}))))
}

/// `GET /v2/service_instances/:instance_id/service_bindings/:binding_id`
pub async fn get_binding(
    State(state): State<AppState>,
    Path((_instance_id, binding_id)): Path<(String, String)>,
) -> BrokerResult<Json<BindResponse>> {
    let binding = state
        .broker
        .storage()
        .get_binding(&binding_id)
        .await?
        .ok_or_else(|| BrokerError::NotFound(format!("binding {binding_id}")))?;

    Ok(Json(BindResponse {
        credentials: Some(binding.credentials),
        ..Default::default()
    }))
}

/// `GET .../service_bindings/:binding_id/last_operation`
pub async fn last_operation(
    State(state): State<AppState>,
    Path((_instance_id, binding_id)): Path<(String, String)>,
) -> BrokerResult<Json<LastOperationResponse>> {
    if let Some(op) = state
        .broker
        .operations()
        .latest_for(ResourceKind::Binding, &binding_id)
    {
        return Ok(Json(LastOperationResponse {
            state: op.state,
            description: Some(op.description),
        }));
    }

    let exists = state
        .broker
        .storage()
        .get_binding(&binding_id)
        .await?
        .is_some();
    Ok(Json(LastOperationResponse {
        state: OperationState::Succeeded,
        description: Some(if exists {
            "binding completed".into()
        } else {
            "binding has been removed".into()
        }),
    }))
}

/// Generate sample credentials for an instance. A real broker would
/// provision an actual user/password against the underlying service.
fn sample_credentials(instance_id: &str) -> HashMap<String, Value> {
    let mut creds = HashMap::new();
    creds.insert("uri".into(), json!(format!("example://{}", instance_id)));
    creds.insert("username".into(), json!("user"));
    creds.insert("password".into(), json!("secret"));
    creds
}
