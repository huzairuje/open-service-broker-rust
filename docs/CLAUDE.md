# CLAUDE.md

This file provides guidance to Claude Code (and other AI assistants) when working with this repository.

## Project Overview

**rust-open-service-broker** is a Rust implementation of the [Open Service Broker (OSB) API](https://github.com/openservicebrokerapi/servicebroker) specification. The OSB API is an open standard that allows cloud-native platforms (Kubernetes, OpenShift, Cloud Foundry) to integrate with external services (databases, message queues, etc.) through a standardized set of RESTful endpoints.

### Core Concepts

- **Broker**: The component that offers services and exposes the OSB API endpoints. This project implements the broker side.
- **Catalog**: The list of services and plans the broker provides to the platform.
- **Service Instance**: A provisioned instance of a service (e.g., a specific PostgreSQL database).
- **Service Binding**: Credentials/connection info that allows an application to access a service instance.
- **Provisioning / Deprovisioning**: Creating and tearing down service instances.
- **Binding / Unbinding**: Creating and removing access credentials.

### Service Lifecycle

1. Platform fetches the catalog from the broker (`GET /v2/catalog`).
2. Platform asks the broker to provision an instance (`PUT /v2/service_instances/:id`).
3. When an app needs access, platform creates a binding (`PUT /v2/service_instances/:id/service_bindings/:bid`).
4. When done, platform unbinds (`DELETE`) and deprovisions (`DELETE`).
5. For async operations, platform polls `GET /v2/service_instances/:id/last_operation`.

## Architecture

```
src/
├── main.rs              # Entry point: loads config, picks storage, builds broker
├── lib.rs               # Library root, re-exports public API + router
├── config.rs            # Config + StorageBackend enum
├── error.rs             # Error types and HTTP response mapping
├── auth.rs              # Basic Auth + API version middleware
├── catalog_loader.rs    # JSON/YAML catalog loader + built-in sample
├── operations.rs        # Async operation tracker + simulated background tasks
├── validation.rs        # JSON-Schema validation against plan.schemas
├── models/              # OSB request/response DTOs
│   ├── catalog.rs       # Service, Plan, Schemas
│   ├── service_instance.rs
│   ├── service_binding.rs
│   └── common.rs        # Context, Metadata, OperationState ("in progress")
├── handlers/            # Axum HTTP handlers per OSB endpoint
│   ├── catalog.rs
│   ├── service_instance.rs
│   └── service_binding.rs
├── storage/             # Storage trait + backends
│   ├── mod.rs           # Storage trait
│   ├── memory.rs        # In-memory (DashMap) implementation
│   └── postgres.rs      # Postgres backend (feature = "postgres")
└── broker.rs            # Catalog + storage + operation tracker glue
```

## OSB API Conformance

- **API version header**: All endpoints require `X-Broker-API-Version` (e.g., `2.17`).
- **Authentication**: HTTP Basic Auth on all endpoints.
- **Idempotency**: PUT for provision/bind must be idempotent. Repeated calls with same params return `200 OK`; conflicting params return `409 Conflict`.
- **Async operations**: Provision/deprovision/update can return `202 Accepted` with an `operation` field; platform polls `last_operation`.
- **Status codes**:
  - `200 OK` — already exists with same params
  - `201 Created` — newly created
  - `202 Accepted` — async operation started
  - `400 Bad Request` — invalid request body
  - `409 Conflict` — exists with different params
  - `410 Gone` — already deleted (deprovision/unbind)
  - `422 Unprocessable Entity` — async required, plan not updatable, etc.

## Development Commands

```bash
# Build (default features = in-memory storage only)
cargo build

# Build with Postgres backend support
cargo build --features postgres

# Run
cargo run

# Run all tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Format & lint
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features postgres -- -D warnings
```

## Docker

```bash
# Build broker image and start broker + Postgres
docker compose up --build

# Hit the broker (defaults to memory; set BROKER_STORAGE=postgres in compose)
curl -u admin:password -H "X-Broker-API-Version: 2.17" http://localhost:8080/v2/catalog
```

## Configuration

The broker reads configuration from environment variables:

| Variable                  | Default       | Description                                                |
|---------------------------|---------------|------------------------------------------------------------|
| `BROKER_HOST`             | `0.0.0.0`     | Bind address                                               |
| `BROKER_PORT`             | `8080`        | Bind port                                                  |
| `BROKER_USERNAME`         | `admin`       | Basic auth username                                        |
| `BROKER_PASSWORD`         | `password`    | Basic auth password                                        |
| `BROKER_MIN_API_VERSION`  | `2.13`        | Minimum OSB API version accepted                           |
| `BROKER_CATALOG_PATH`     | unset         | Path to a catalog JSON/YAML file (built-in sample if unset) |
| `BROKER_STORAGE`          | `memory`      | `memory` or `postgres`                                     |
| `DATABASE_URL`            | unset         | Required when `BROKER_STORAGE=postgres`                    |
| `BROKER_ASYNC_OP_MILLIS`  | `0`           | Simulated async work duration. `0` = sync provisioning.    |
| `RUST_LOG`                | `info`        | Logging level                                              |

## Testing the Broker

```bash
# Get catalog
curl -u admin:password \
  -H "X-Broker-API-Version: 2.17" \
  http://localhost:8080/v2/catalog

# Provision an instance
curl -u admin:password -X PUT \
  -H "X-Broker-API-Version: 2.17" \
  -H "Content-Type: application/json" \
  -d '{"service_id":"...","plan_id":"...","organization_guid":"org","space_guid":"space"}' \
  http://localhost:8080/v2/service_instances/my-instance-id
```

## Key Design Decisions

- **axum** as the web framework: modern, async, tower-compatible middleware.
- **Storage trait**: backend is abstracted so memory/Postgres/Redis impls can be swapped at runtime via `BROKER_STORAGE`.
- **Catalog source**: load from JSON/YAML file (`BROKER_CATALOG_PATH`) or fall back to a built-in sample.
- **Async operations**: a `Tokio` background task simulates work; `last_operation` looks up the live state from `OperationTracker`.
- **JSON-Schema validation**: `parameters` are validated against `plan.schemas.*` before persisting; failures return `400`.
- **Strong typing for OSB payloads**: every request/response struct mirrors the OSB spec field names exactly via `serde` rename attributes (note: `OperationState` serializes `"in progress"` with the space, per spec).

## Conventions

- Use `thiserror` for error types and map them to HTTP responses in one place (`error.rs`).
- Prefer `Arc<dyn Storage>` for sharing state across handlers.
- All handler functions return `Result<impl IntoResponse, BrokerError>`.
- Field names in JSON match the OSB spec (snake_case, e.g., `service_id`, `plan_id`).
- Tests live alongside modules (`#[cfg(test)] mod tests`) and as integration tests in `tests/`.

## References

- [OSB API spec](https://github.com/openservicebrokerapi/servicebroker/blob/master/spec.md)
- [axum docs](https://docs.rs/axum)
- [serde docs](https://serde.rs)
