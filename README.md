# rust-open-service-broker

A Rust implementation of the [Open Service Broker (OSB) API](https://github.com/openservicebrokerapi/servicebroker).

The OSB API is an open standard that lets cloud-native platforms (Kubernetes, OpenShift, Cloud Foundry) provision and bind to external services through a uniform set of REST endpoints.

## Concepts

- **Broker**: this server. Exposes the OSB API and offers services.
- **Catalog**: list of services and plans the broker advertises.
- **Service Instance**: a provisioned instance of a service (e.g., one database).
- **Service Binding**: credentials handed to an app so it can reach an instance.

### Lifecycle

1. Platform fetches the catalog (`GET /v2/catalog`).
2. Platform provisions an instance (`PUT /v2/service_instances/:id`).
3. Platform creates a binding (`PUT /v2/service_instances/:id/service_bindings/:bid`).
4. Platform unbinds (`DELETE`) and deprovisions (`DELETE`) when done.
5. For long-running operations, the platform polls `GET .../last_operation`.

## Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| GET    | `/v2/catalog` | List services and plans |
| PUT    | `/v2/service_instances/:id` | Provision |
| PATCH  | `/v2/service_instances/:id` | Update |
| DELETE | `/v2/service_instances/:id` | Deprovision |
| GET    | `/v2/service_instances/:id` | Fetch instance |
| GET    | `/v2/service_instances/:id/last_operation` | Poll async op |
| PUT    | `/v2/service_instances/:id/service_bindings/:bid` | Bind |
| DELETE | `/v2/service_instances/:id/service_bindings/:bid` | Unbind |
| GET    | `/v2/service_instances/:id/service_bindings/:bid` | Fetch binding |

All endpoints require:

- `X-Broker-API-Version` header (e.g., `2.17`)
- HTTP Basic Auth

## Running

```bash
cargo run                          # in-memory storage, sample catalog
cargo run --features postgres      # build with Postgres support compiled in
```

Defaults: `0.0.0.0:8080`, user `admin`, password `password`, in-memory storage.

### Configuration

| Variable                  | Default       | Notes                                          |
|---------------------------|---------------|------------------------------------------------|
| `BROKER_HOST` / `BROKER_PORT`         | `0.0.0.0` / `8080`           |                                       |
| `BROKER_USERNAME` / `BROKER_PASSWORD` | `admin` / `password`         | HTTP Basic Auth                       |
| `BROKER_CATALOG_PATH`     | unset         | Path to catalog JSON or YAML; sample built-in if unset |
| `BROKER_STORAGE`          | `memory`      | `memory` or `postgres`                         |
| `DATABASE_URL`            | unset         | Required when `BROKER_STORAGE=postgres`        |
| `BROKER_ASYNC_OP_MILLIS`  | `0`           | If >0 and `accepts_incomplete=true`, return `202` and finish in background |
| `RUST_LOG`                | `info`        | Logging filter                                 |

## Docker

```bash
# Builds the broker image and brings up Postgres + broker.
docker compose up --build

# Tail the broker logs
docker compose logs -f broker
```

The compose file sets `BROKER_STORAGE=postgres`, points `DATABASE_URL` at the bundled Postgres, and mounts the example catalog. Set `BROKER_STORAGE=memory` to skip Postgres.

## Try it

```bash
# Catalog
curl -u admin:password \
  -H "X-Broker-API-Version: 2.17" \
  http://localhost:8080/v2/catalog

# Provision
SERVICE_ID=4f6e6cf6-ffdd-425f-a2c7-3c9258ad2468
PLAN_ID=86064792-7ea2-467b-af93-ac9694d96d5b

curl -u admin:password -X PUT \
  -H "X-Broker-API-Version: 2.17" \
  -H "Content-Type: application/json" \
  -d "{\"service_id\":\"$SERVICE_ID\",\"plan_id\":\"$PLAN_ID\",\"organization_guid\":\"o\",\"space_guid\":\"s\"}" \
  http://localhost:8080/v2/service_instances/inst-1

# Bind
curl -u admin:password -X PUT \
  -H "X-Broker-API-Version: 2.17" \
  -H "Content-Type: application/json" \
  -d "{\"service_id\":\"$SERVICE_ID\",\"plan_id\":\"$PLAN_ID\"}" \
  http://localhost:8080/v2/service_instances/inst-1/service_bindings/bind-1

# Unbind
curl -u admin:password -X DELETE \
  -H "X-Broker-API-Version: 2.17" \
  "http://localhost:8080/v2/service_instances/inst-1/service_bindings/bind-1?service_id=$SERVICE_ID&plan_id=$PLAN_ID"

# Deprovision
curl -u admin:password -X DELETE \
  -H "X-Broker-API-Version: 2.17" \
  "http://localhost:8080/v2/service_instances/inst-1?service_id=$SERVICE_ID&plan_id=$PLAN_ID"
```

## Project Layout

```
src/
├── main.rs              entry point: picks storage + catalog source
├── lib.rs               router wiring
├── config.rs            env-driven config
├── error.rs             BrokerError -> HTTP mapping
├── auth.rs              basic-auth + version check middleware
├── broker.rs            catalog + storage + ops glue
├── catalog_loader.rs    JSON/YAML loader + built-in sample
├── operations.rs        async operation tracker + simulated tasks
├── validation.rs        JSON-Schema validation
├── models/              OSB request/response DTOs
├── handlers/            one module per OSB resource
└── storage/             Storage trait + in-memory + postgres backends
tests/                  integration tests for API, async ops, validation
catalog.example.json    sample catalog with JSON-Schema constraints
docker-compose.yml      broker + Postgres for local dev
Dockerfile              multi-stage build, ~80MB runtime image
docs/CLAUDE.md          architecture + conventions for AI assistants
```

## Tests

```bash
cargo test
```

## Status

Reference broker, ready to be specialized. Out of the box it provides:

- All OSB v2 endpoints (catalog, provision/update/deprovision, bind/unbind, last_operation)
- HTTP Basic Auth + `X-Broker-API-Version` enforcement
- Idempotent PUTs (200 / 201 / 409) and correct `410 Gone` on repeated deletes
- Catalog from JSON/YAML or a built-in sample
- JSON-Schema validation of `parameters` against `plan.schemas.*`
- Async operations (202 + `last_operation` polling) when `BROKER_ASYNC_OP_MILLIS>0`
- Pluggable storage: in-memory (default) or Postgres via the `postgres` feature
- Docker + docker-compose with bundled Postgres

To turn it into a real broker, replace `sample_credentials` and the no-op provisioning in the handlers with calls into your actual service (provision a DB, create a user, hand back real credentials).
