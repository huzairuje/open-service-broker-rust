//! Tests for async operations: when `BROKER_ASYNC_OP_MILLIS > 0` and
//! the platform sends `accepts_incomplete=true`, the broker returns
//! `202` with an `operation` token and finishes the work in the
//! background.

use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rust_open_service_broker::{
    broker::Broker,
    config::{Config, StorageBackend},
    storage::memory::MemoryStorage,
    AppState,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const SERVICE_ID: &str = "4f6e6cf6-ffdd-425f-a2c7-3c9258ad2468";
const PLAN_ID: &str = "86064792-7ea2-467b-af93-ac9694d96d5b";

fn app_with_async(delay_ms: u64) -> axum::Router {
    let config = Arc::new(Config {
        host: "127.0.0.1".into(),
        port: 0,
        username: "admin".into(),
        password: "password".into(),
        min_api_version: "2.13".into(),
        catalog_path: None,
        storage: StorageBackend::Memory,
        database_url: None,
        async_op_millis: delay_ms,
    });
    let storage = Arc::new(MemoryStorage::new());
    let broker = Arc::new(Broker::new(storage));
    rust_open_service_broker::build_router(AppState { broker, config })
}

fn auth() -> String {
    format!("Basic {}", B64.encode("admin:password"))
}

fn req(method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header("X-Broker-API-Version", "2.17")
        .header(header::AUTHORIZATION, auth());
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let body = match body {
        Some(v) => Body::from(serde_json::to_vec(&v).unwrap()),
        None => Body::empty(),
    };
    b.body(body).unwrap()
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

#[tokio::test]
async fn async_provision_returns_202_then_succeeds() {
    let app = app_with_async(50);

    let body = json!({
        "service_id": SERVICE_ID,
        "plan_id": PLAN_ID,
        "organization_guid": "o",
        "space_guid": "s"
    });
    let resp = app
        .clone()
        .oneshot(req(
            "PUT",
            "/v2/service_instances/inst-async?accepts_incomplete=true",
            Some(body),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body = body_json(resp).await;
    let op_id = body["operation"].as_str().unwrap().to_string();

    // Poll once before the simulated work finishes -> in progress.
    let uri = format!("/v2/service_instances/inst-async/last_operation?operation={op_id}");
    let resp = app.clone().oneshot(req("GET", &uri, None)).await.unwrap();
    let body = body_json(resp).await;
    assert_eq!(body["state"], "in progress");

    // Wait for the background task to complete and re-poll.
    tokio::time::sleep(Duration::from_millis(150)).await;
    let resp = app.oneshot(req("GET", &uri, None)).await.unwrap();
    let body = body_json(resp).await;
    assert_eq!(body["state"], "succeeded");
}
