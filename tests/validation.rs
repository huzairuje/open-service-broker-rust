//! Tests for JSON-Schema parameter validation against `plan.schemas`.

use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rust_open_service_broker::{
    broker::Broker,
    catalog_loader,
    config::{Config, StorageBackend},
    storage::memory::MemoryStorage,
    AppState,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const SERVICE_ID: &str = "4f6e6cf6-ffdd-425f-a2c7-3c9258ad2468";
const PLAN_ID: &str = "86064792-7ea2-467b-af93-ac9694d96d5b";

fn app_with_catalog_file() -> axum::Router {
    let catalog =
        catalog_loader::load_from_file("catalog.example.json").expect("read catalog.example.json");
    let config = Arc::new(Config {
        host: "127.0.0.1".into(),
        port: 0,
        username: "admin".into(),
        password: "password".into(),
        min_api_version: "2.13".into(),
        catalog_path: Some("catalog.example.json".into()),
        storage: StorageBackend::Memory,
        database_url: None,
        async_op_millis: 0,
    });
    let storage = Arc::new(MemoryStorage::new());
    let broker = Arc::new(Broker::with_catalog(storage, catalog));
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
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

#[tokio::test]
async fn provision_with_valid_parameters_succeeds() {
    let app = app_with_catalog_file();

    let body = json!({
        "service_id": SERVICE_ID,
        "plan_id": PLAN_ID,
        "organization_guid": "o",
        "space_guid": "s",
        "parameters": { "db_name": "mydb" }
    });
    let resp = app
        .oneshot(req("PUT", "/v2/service_instances/v1", Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn provision_with_invalid_parameters_is_rejected() {
    let app = app_with_catalog_file();

    // db_name violates minLength=1 (empty string).
    let body = json!({
        "service_id": SERVICE_ID,
        "plan_id": PLAN_ID,
        "organization_guid": "o",
        "space_guid": "s",
        "parameters": { "db_name": "" }
    });
    let resp = app
        .oneshot(req("PUT", "/v2/service_instances/v2", Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    assert!(body["description"]
        .as_str()
        .unwrap()
        .contains("schema validation"));
}

#[tokio::test]
async fn provision_with_wrong_type_is_rejected() {
    let app = app_with_catalog_file();

    // db_name should be a string.
    let body = json!({
        "service_id": SERVICE_ID,
        "plan_id": PLAN_ID,
        "organization_guid": "o",
        "space_guid": "s",
        "parameters": { "db_name": 42 }
    });
    let resp = app
        .oneshot(req("PUT", "/v2/service_instances/v3", Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
