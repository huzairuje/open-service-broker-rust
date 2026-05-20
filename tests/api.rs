//! Integration tests covering the OSB endpoints end-to-end via the
//! axum router. We don't bind a real socket; we drive the router with
//! `tower::ServiceExt::oneshot`.

use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rust_open_service_broker::{
    broker::Broker, config::Config, storage::memory::MemoryStorage, AppState,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const SERVICE_ID: &str = "4f6e6cf6-ffdd-425f-a2c7-3c9258ad2468";
const PLAN_ID: &str = "86064792-7ea2-467b-af93-ac9694d96d5b";

fn test_app() -> axum::Router {
    let config = Arc::new(Config {
        host: "127.0.0.1".into(),
        port: 0,
        username: "admin".into(),
        password: "password".into(),
        min_api_version: "2.13".into(),
        catalog_path: None,
        storage: rust_open_service_broker::config::StorageBackend::Memory,
        database_url: None,
        async_op_millis: 0,
    });
    let storage = Arc::new(MemoryStorage::new());
    let broker = Arc::new(Broker::new(storage));
    let state = AppState { broker, config };
    rust_open_service_broker::build_router(state)
}

fn auth_header() -> String {
    format!("Basic {}", B64.encode("admin:password"))
}

fn req(method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("X-Broker-API-Version", "2.17")
        .header(header::AUTHORIZATION, auth_header());
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    let body = match body {
        Some(v) => Body::from(serde_json::to_vec(&v).unwrap()),
        None => Body::empty(),
    };
    builder.body(body).unwrap()
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    if bytes.is_empty() {
        return Value::Null;
    }
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn catalog_returns_services() {
    let app = test_app();
    let resp = app.oneshot(req("GET", "/v2/catalog", None)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(!body["services"].as_array().unwrap().is_empty());
    assert_eq!(body["services"][0]["id"], SERVICE_ID);
}

#[tokio::test]
async fn missing_api_version_is_rejected() {
    let app = test_app();
    let r = Request::builder()
        .method("GET")
        .uri("/v2/catalog")
        .header(header::AUTHORIZATION, auth_header())
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(r).await.unwrap();
    assert_eq!(resp.status(), StatusCode::PRECONDITION_FAILED);
}

#[tokio::test]
async fn missing_auth_is_rejected() {
    let app = test_app();
    let r = Request::builder()
        .method("GET")
        .uri("/v2/catalog")
        .header("X-Broker-API-Version", "2.17")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(r).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn provision_and_deprovision_flow() {
    let app = test_app();

    // Provision a fresh instance.
    let body = json!({
        "service_id": SERVICE_ID,
        "plan_id": PLAN_ID,
        "organization_guid": "org-1",
        "space_guid": "space-1"
    });
    let resp = app
        .clone()
        .oneshot(req(
            "PUT",
            "/v2/service_instances/inst-1",
            Some(body.clone()),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Idempotent: same params -> 200.
    let resp = app
        .clone()
        .oneshot(req("PUT", "/v2/service_instances/inst-1", Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Conflicting plan -> 409.
    let conflict = json!({
        "service_id": SERVICE_ID,
        "plan_id": "f52eabf8-e65c-4f5b-9e86-7f3c2c7b6f24",
        "organization_guid": "org-1",
        "space_guid": "space-1"
    });
    let resp = app
        .clone()
        .oneshot(req("PUT", "/v2/service_instances/inst-1", Some(conflict)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    // Deprovision.
    let uri = format!("/v2/service_instances/inst-1?service_id={SERVICE_ID}&plan_id={PLAN_ID}");
    let resp = app
        .clone()
        .oneshot(req("DELETE", &uri, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Second delete -> 410 Gone.
    let resp = app.oneshot(req("DELETE", &uri, None)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::GONE);
}

#[tokio::test]
async fn bind_and_unbind_flow() {
    let app = test_app();

    // Provision first.
    let body = json!({
        "service_id": SERVICE_ID,
        "plan_id": PLAN_ID,
        "organization_guid": "o",
        "space_guid": "s"
    });
    let resp = app
        .clone()
        .oneshot(req("PUT", "/v2/service_instances/inst-2", Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Bind.
    let bind_body = json!({ "service_id": SERVICE_ID, "plan_id": PLAN_ID });
    let resp = app
        .clone()
        .oneshot(req(
            "PUT",
            "/v2/service_instances/inst-2/service_bindings/bind-1",
            Some(bind_body),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert!(body["credentials"]["uri"].is_string());

    // Unbind.
    let uri = format!(
        "/v2/service_instances/inst-2/service_bindings/bind-1\
         ?service_id={SERVICE_ID}&plan_id={PLAN_ID}"
    );
    let resp = app
        .clone()
        .oneshot(req("DELETE", &uri, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Unbind again -> 410.
    let resp = app.oneshot(req("DELETE", &uri, None)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::GONE);
}
