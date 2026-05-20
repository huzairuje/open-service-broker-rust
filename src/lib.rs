//! rust-open-service-broker
//!
//! Library root. Exposes the broker building blocks so they can be
//! reused from `main.rs` and integration tests.

pub mod auth;
pub mod broker;
pub mod catalog_loader;
pub mod config;
pub mod error;
pub mod handlers;
pub mod models;
pub mod operations;
pub mod storage;
pub mod validation;

use std::sync::Arc;

use axum::{
    middleware,
    routing::{get, put},
    Router,
};
use tower_http::trace::TraceLayer;

use crate::broker::Broker;
use crate::config::Config;

/// Shared application state passed into every handler.
#[derive(Clone)]
pub struct AppState {
    pub broker: Arc<Broker>,
    pub config: Arc<Config>,
}

/// Build the axum router with all OSB v2 routes wired up.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/v2/catalog", get(handlers::catalog::get_catalog))
        .route(
            "/v2/service_instances/:instance_id",
            put(handlers::service_instance::provision)
                .patch(handlers::service_instance::update)
                .delete(handlers::service_instance::deprovision)
                .get(handlers::service_instance::get_instance),
        )
        .route(
            "/v2/service_instances/:instance_id/last_operation",
            get(handlers::service_instance::last_operation),
        )
        .route(
            "/v2/service_instances/:instance_id/service_bindings/:binding_id",
            put(handlers::service_binding::bind)
                .delete(handlers::service_binding::unbind)
                .get(handlers::service_binding::get_binding),
        )
        .route(
            "/v2/service_instances/:instance_id/service_bindings/:binding_id/last_operation",
            get(handlers::service_binding::last_operation),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::basic_auth,
        ))
        .layer(middleware::from_fn(auth::api_version_check))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
