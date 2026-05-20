//! `GET /v2/catalog` handler.

use axum::{extract::State, Json};

use crate::{error::BrokerResult, models::catalog::Catalog, AppState};

/// Return the static catalog the broker advertises.
pub async fn get_catalog(State(state): State<AppState>) -> BrokerResult<Json<Catalog>> {
    Ok(Json(state.broker.catalog().clone()))
}
