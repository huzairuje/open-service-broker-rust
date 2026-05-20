//! Middleware: HTTP Basic Auth + OSB API version check.

use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

use crate::{error::BrokerError, AppState};

/// Reject requests missing the `X-Broker-API-Version` header. The OSB
/// spec requires this header on every call.
pub async fn api_version_check(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, BrokerError> {
    if headers.get("X-Broker-API-Version").is_none() {
        return Err(BrokerError::UnsupportedApiVersion);
    }
    Ok(next.run(request).await)
}

/// Validate HTTP Basic credentials against the configured username/password.
pub async fn basic_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, BrokerError> {
    let header_value = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(BrokerError::Unauthorized)?;

    let token = header_value
        .strip_prefix("Basic ")
        .ok_or(BrokerError::Unauthorized)?;

    let decoded = B64
        .decode(token.trim())
        .map_err(|_| BrokerError::Unauthorized)?;
    let decoded = String::from_utf8(decoded).map_err(|_| BrokerError::Unauthorized)?;

    let (user, pass) = decoded.split_once(':').ok_or(BrokerError::Unauthorized)?;

    if user == state.config.username && pass == state.config.password {
        Ok(next.run(request).await)
    } else {
        Err(BrokerError::Unauthorized)
    }
}

// Keep a reference to StatusCode so it isn't pruned if we add direct
// status responses later.
#[allow(dead_code)]
const _SC: StatusCode = StatusCode::OK;
