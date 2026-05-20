//! Handlers for `/v2/service_instances/:instance_id`.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use std::time::Duration;

use crate::{
    error::{BrokerError, BrokerResult},
    models::{
        common::{LastOperationResponse, OperationState},
        service_instance::{
            GetInstanceResponse, ProvisionRequest, ProvisionResponse, ServiceInstance,
            UpdateRequest, UpdateResponse,
        },
    },
    operations::{spawn_simulated, OperationKind, ResourceKind},
    validation::{validate, SchemaKind},
    AppState,
};

#[derive(Debug, Deserialize, Default)]
pub struct ProvisionQuery {
    #[serde(default)]
    pub accepts_incomplete: Option<bool>,
}

/// `PUT /v2/service_instances/:instance_id`
///
/// Idempotent: if an instance with the same id and parameters already
/// exists, returns `200`. If different parameters, returns `409`.
pub async fn provision(
    State(state): State<AppState>,
    Path(instance_id): Path<String>,
    Query(q): Query<ProvisionQuery>,
    Json(req): Json<ProvisionRequest>,
) -> BrokerResult<(StatusCode, Json<ProvisionResponse>)> {
    let service = state.broker.find_service(&req.service_id)?;
    let plan = state.broker.find_plan(service, &req.plan_id)?;

    // Validate user-supplied parameters against plan schema (if any).
    validate(plan, SchemaKind::InstanceCreate, req.parameters.as_ref())?;

    if let Some(existing) = state.broker.storage().get_instance(&instance_id).await? {
        return if existing.service_id == req.service_id && existing.plan_id == req.plan_id {
            Ok((StatusCode::OK, Json(ProvisionResponse::default())))
        } else {
            Err(BrokerError::Conflict(format!(
                "instance {instance_id} already exists with different params"
            )))
        };
    }

    let instance = ServiceInstance {
        id: instance_id.clone(),
        service_id: req.service_id,
        plan_id: req.plan_id,
        parameters: req.parameters,
        context: req.context,
        dashboard_url: None,
    };

    let async_ms = state.config.async_op_millis;
    if async_ms > 0 && q.accepts_incomplete.unwrap_or(false) {
        // Defer the actual write so the operation observably transitions
        // through `in progress`.
        let op_id = state.broker.operations().start(
            ResourceKind::Instance,
            &instance_id,
            OperationKind::Provision,
        );
        let storage = state.broker.storage().clone();
        let tracker = state.broker.operations().clone();
        spawn_simulated(
            tracker,
            op_id.clone(),
            Duration::from_millis(async_ms),
            move || {
                // Best-effort write; errors are logged but not surfaced.
                let storage = storage.clone();
                tokio::spawn(async move {
                    if let Err(e) = storage.put_instance(instance).await {
                        tracing::error!(error = %e, "async provision failed");
                    }
                });
            },
        );
        return Ok((
            StatusCode::ACCEPTED,
            Json(ProvisionResponse {
                operation: Some(op_id),
                ..Default::default()
            }),
        ));
    }

    state.broker.storage().put_instance(instance).await?;
    Ok((StatusCode::CREATED, Json(ProvisionResponse::default())))
}

/// `PATCH /v2/service_instances/:instance_id`
pub async fn update(
    State(state): State<AppState>,
    Path(instance_id): Path<String>,
    Query(q): Query<ProvisionQuery>,
    Json(req): Json<UpdateRequest>,
) -> BrokerResult<(StatusCode, Json<UpdateResponse>)> {
    let mut instance = state
        .broker
        .storage()
        .get_instance(&instance_id)
        .await?
        .ok_or_else(|| BrokerError::NotFound(format!("instance {instance_id}")))?;

    if instance.service_id != req.service_id {
        return Err(BrokerError::BadRequest(
            "service_id does not match existing instance".into(),
        ));
    }

    // Validate update parameters against the target plan's schema.
    let target_plan_id = req
        .plan_id
        .clone()
        .unwrap_or_else(|| instance.plan_id.clone());
    let service = state.broker.find_service(&instance.service_id)?;
    let plan = state.broker.find_plan(service, &target_plan_id)?;
    validate(plan, SchemaKind::InstanceUpdate, req.parameters.as_ref())?;

    if let Some(plan_id) = req.plan_id {
        instance.plan_id = plan_id;
    }
    if let Some(p) = req.parameters {
        instance.parameters = Some(p);
    }
    if let Some(c) = req.context {
        instance.context = Some(c);
    }

    let async_ms = state.config.async_op_millis;
    if async_ms > 0 && q.accepts_incomplete.unwrap_or(false) {
        let op_id = state.broker.operations().start(
            ResourceKind::Instance,
            &instance_id,
            OperationKind::Update,
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
                    if let Err(e) = storage.put_instance(instance).await {
                        tracing::error!(error = %e, "async update failed");
                    }
                });
            },
        );
        return Ok((
            StatusCode::ACCEPTED,
            Json(UpdateResponse {
                operation: Some(op_id),
                ..Default::default()
            }),
        ));
    }

    state.broker.storage().put_instance(instance).await?;
    Ok((StatusCode::OK, Json(UpdateResponse::default())))
}

#[derive(Debug, Deserialize)]
pub struct DeprovisionQuery {
    pub service_id: String,
    pub plan_id: String,
    #[serde(default)]
    pub accepts_incomplete: Option<bool>,
}

/// `DELETE /v2/service_instances/:instance_id`
pub async fn deprovision(
    State(state): State<AppState>,
    Path(instance_id): Path<String>,
    Query(q): Query<DeprovisionQuery>,
) -> BrokerResult<(StatusCode, Json<serde_json::Value>)> {
    // Resource must exist; if not, OSB says 410.
    if state
        .broker
        .storage()
        .get_instance(&instance_id)
        .await?
        .is_none()
    {
        return Err(BrokerError::Gone(format!("instance {instance_id}")));
    }

    let async_ms = state.config.async_op_millis;
    if async_ms > 0 && q.accepts_incomplete.unwrap_or(false) {
        let op_id = state.broker.operations().start(
            ResourceKind::Instance,
            &instance_id,
            OperationKind::Deprovision,
        );
        let storage = state.broker.storage().clone();
        let tracker = state.broker.operations().clone();
        let id = instance_id.clone();
        spawn_simulated(
            tracker,
            op_id.clone(),
            Duration::from_millis(async_ms),
            move || {
                let storage = storage.clone();
                tokio::spawn(async move {
                    let _ = storage.delete_instance(&id).await;
                });
            },
        );
        return Ok((
            StatusCode::ACCEPTED,
            Json(serde_json::json!({ "operation": op_id })),
        ));
    }

    state.broker.storage().delete_instance(&instance_id).await?;
    Ok((StatusCode::OK, Json(serde_json::json!({}))))
}

/// `GET /v2/service_instances/:instance_id`
pub async fn get_instance(
    State(state): State<AppState>,
    Path(instance_id): Path<String>,
) -> BrokerResult<Json<GetInstanceResponse>> {
    let instance = state
        .broker
        .storage()
        .get_instance(&instance_id)
        .await?
        .ok_or_else(|| BrokerError::NotFound(format!("instance {instance_id}")))?;

    Ok(Json(GetInstanceResponse {
        service_id: Some(instance.service_id),
        plan_id: Some(instance.plan_id),
        dashboard_url: instance.dashboard_url,
        parameters: instance.parameters,
        maintenance_info: None,
    }))
}

#[derive(Debug, Deserialize)]
pub struct LastOperationQuery {
    #[serde(default)]
    pub service_id: Option<String>,
    #[serde(default)]
    pub plan_id: Option<String>,
    #[serde(default)]
    pub operation: Option<String>,
}

/// `GET /v2/service_instances/:instance_id/last_operation`
///
/// Looks up the operation token in the tracker. If absent (e.g., the
/// platform polls without a token, or the broker restarted), falls
/// back to checking storage existence.
pub async fn last_operation(
    State(state): State<AppState>,
    Path(instance_id): Path<String>,
    Query(q): Query<LastOperationQuery>,
) -> BrokerResult<Json<LastOperationResponse>> {
    if let Some(op_id) = &q.operation {
        if let Some(op) = state.broker.operations().get(op_id) {
            return Ok(Json(LastOperationResponse {
                state: op.state,
                description: Some(op.description),
            }));
        }
    }
    if let Some(op) = state
        .broker
        .operations()
        .latest_for(ResourceKind::Instance, &instance_id)
    {
        return Ok(Json(LastOperationResponse {
            state: op.state,
            description: Some(op.description),
        }));
    }

    let exists = state
        .broker
        .storage()
        .get_instance(&instance_id)
        .await?
        .is_some();
    Ok(Json(LastOperationResponse {
        state: OperationState::Succeeded,
        description: Some(if exists {
            "operation completed".into()
        } else {
            "instance has been removed".into()
        }),
    }))
}
