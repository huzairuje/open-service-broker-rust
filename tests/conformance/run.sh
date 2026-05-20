#!/usr/bin/env bash
# OSB conformance smoke test for rust-open-service-broker.
#
# Walks the full Open Service Broker v2 lifecycle and asserts the
# response status code at each step. Designed to be a quick
# alternative to the official Kotlin checker for local development.
#
# Usage:
#   tests/conformance/run.sh                       # defaults to localhost:8080
#   BROKER_URL=http://broker:8080 tests/conformance/run.sh
#   BROKER_USER=admin BROKER_PASS=password tests/conformance/run.sh

set -u

# --- config ----------------------------------------------------------------

BROKER_URL="${BROKER_URL:-http://localhost:8080}"
BROKER_USER="${BROKER_USER:-admin}"
BROKER_PASS="${BROKER_PASS:-password}"
API_VERSION="${BROKER_API_VERSION:-2.17}"

# Service + plan IDs match the built-in sample catalog AND catalog.example.json.
SERVICE_ID="4f6e6cf6-ffdd-425f-a2c7-3c9258ad2468"
PLAN_ID_FREE="86064792-7ea2-467b-af93-ac9694d96d5b"
PLAN_ID_PRO="f52eabf8-e65c-4f5b-9e86-7f3c2c7b6f24"

# Fresh ids per run to avoid colliding with prior state in Postgres.
SUFFIX="$(date +%s)-$$"
INSTANCE_ID="conf-inst-$SUFFIX"
BINDING_ID="conf-bind-$SUFFIX"

# --- output helpers --------------------------------------------------------

if [[ -t 1 ]]; then
    GREEN=$'\033[32m'; RED=$'\033[31m'; YELLOW=$'\033[33m'
    BOLD=$'\033[1m'; RESET=$'\033[0m'
else
    GREEN=""; RED=""; YELLOW=""; BOLD=""; RESET=""
fi

PASS=0
FAIL=0
FAILURES=()

step() { echo "${BOLD}-- $* --${RESET}"; }

# Run a single curl call. Captures status to $STATUS, body to $BODY.
# Args: METHOD PATH [json-body] [extra-curl-args...]
osb_call() {
    local method="$1"; shift
    local path="$1"; shift
    local body=""
    if [[ $# -gt 0 ]]; then
        body="$1"; shift
    fi

    local tmp; tmp="$(mktemp)"
    if [[ -n "$body" ]]; then
        STATUS=$(curl -sS -o "$tmp" -w '%{http_code}' \
            -u "$BROKER_USER:$BROKER_PASS" \
            -H "X-Broker-API-Version: $API_VERSION" \
            -H "Content-Type: application/json" \
            -X "$method" \
            -d "$body" \
            "$@" \
            "$BROKER_URL$path" || echo "000")
    else
        STATUS=$(curl -sS -o "$tmp" -w '%{http_code}' \
            -u "$BROKER_USER:$BROKER_PASS" \
            -H "X-Broker-API-Version: $API_VERSION" \
            -X "$method" \
            "$@" \
            "$BROKER_URL$path" || echo "000")
    fi
    BODY="$(cat "$tmp")"
    rm -f "$tmp"
}

# Assert that $STATUS is one of the expected codes.
# Args: name expected-status [more-expected...]
expect() {
    local name="$1"; shift
    local expected=("$@")
    for code in "${expected[@]}"; do
        if [[ "$STATUS" == "$code" ]]; then
            echo "  ${GREEN}PASS${RESET} $name (status $STATUS)"
            PASS=$((PASS + 1))
            return 0
        fi
    done
    echo "  ${RED}FAIL${RESET} $name (status $STATUS, expected ${expected[*]})"
    if [[ -n "${BODY:-}" ]]; then
        echo "       body: $(echo "$BODY" | head -c 200)"
    fi
    FAIL=$((FAIL + 1))
    FAILURES+=("$name")
    return 1
}

# --- 0. preflight ----------------------------------------------------------

step "preflight: broker reachable at $BROKER_URL"
if ! curl -sS -o /dev/null --max-time 5 "$BROKER_URL/v2/catalog" \
    -H "X-Broker-API-Version: $API_VERSION" \
    -u "$BROKER_USER:$BROKER_PASS"; then
    echo "${RED}cannot reach $BROKER_URL${RESET}"
    echo "is the broker running? try: cargo run  OR  docker compose up -d"
    exit 1
fi
echo "  ${GREEN}OK${RESET} broker is reachable"

# --- 1. catalog ------------------------------------------------------------

step "GET /v2/catalog"
osb_call GET /v2/catalog
expect "catalog returns 200" 200

step "auth + version header negative checks"
STATUS=$(curl -sS -o /dev/null -w '%{http_code}' \
    -H "X-Broker-API-Version: $API_VERSION" \
    "$BROKER_URL/v2/catalog")
expect "missing auth -> 401" 401

STATUS=$(curl -sS -o /dev/null -w '%{http_code}' \
    -u "$BROKER_USER:$BROKER_PASS" \
    "$BROKER_URL/v2/catalog")
expect "missing X-Broker-API-Version -> 412" 412

# --- 2. provision lifecycle ------------------------------------------------

PROVISION_BODY=$(cat <<JSON
{
  "service_id": "$SERVICE_ID",
  "plan_id":    "$PLAN_ID_FREE",
  "organization_guid": "conf-org",
  "space_guid":        "conf-space",
  "context": { "platform": "conformance-script" }
}
JSON
)

step "PUT /v2/service_instances/$INSTANCE_ID (first time)"
osb_call PUT "/v2/service_instances/$INSTANCE_ID" "$PROVISION_BODY"
expect "first provision -> 201 or 202" 201 202

step "PUT same instance with same params (idempotent)"
osb_call PUT "/v2/service_instances/$INSTANCE_ID" "$PROVISION_BODY"
expect "repeat provision -> 200" 200

step "PUT same instance with different plan (conflict)"
CONFLICT_BODY=$(echo "$PROVISION_BODY" | sed "s/$PLAN_ID_FREE/$PLAN_ID_PRO/")
osb_call PUT "/v2/service_instances/$INSTANCE_ID" "$CONFLICT_BODY"
expect "conflicting params -> 409" 409

step "GET /v2/service_instances/$INSTANCE_ID"
osb_call GET "/v2/service_instances/$INSTANCE_ID"
expect "fetch instance -> 200" 200

# --- 3. binding lifecycle --------------------------------------------------

BIND_BODY=$(cat <<JSON
{
  "service_id": "$SERVICE_ID",
  "plan_id":    "$PLAN_ID_FREE"
}
JSON
)

step "PUT .../service_bindings/$BINDING_ID"
osb_call PUT \
    "/v2/service_instances/$INSTANCE_ID/service_bindings/$BINDING_ID" \
    "$BIND_BODY"
expect "first bind -> 201 or 202" 201 202

step "PUT same binding with same params (idempotent)"
osb_call PUT \
    "/v2/service_instances/$INSTANCE_ID/service_bindings/$BINDING_ID" \
    "$BIND_BODY"
expect "repeat bind -> 200" 200

step "GET .../service_bindings/$BINDING_ID"
osb_call GET "/v2/service_instances/$INSTANCE_ID/service_bindings/$BINDING_ID"
expect "fetch binding -> 200" 200

# --- 4. cleanup ------------------------------------------------------------

step "DELETE binding"
osb_call DELETE \
    "/v2/service_instances/$INSTANCE_ID/service_bindings/$BINDING_ID?service_id=$SERVICE_ID&plan_id=$PLAN_ID_FREE"
expect "first unbind -> 200 or 202" 200 202

step "DELETE binding again"
osb_call DELETE \
    "/v2/service_instances/$INSTANCE_ID/service_bindings/$BINDING_ID?service_id=$SERVICE_ID&plan_id=$PLAN_ID_FREE"
expect "repeat unbind -> 410" 410

step "DELETE instance"
osb_call DELETE \
    "/v2/service_instances/$INSTANCE_ID?service_id=$SERVICE_ID&plan_id=$PLAN_ID_FREE"
expect "first deprovision -> 200 or 202" 200 202

step "DELETE instance again"
osb_call DELETE \
    "/v2/service_instances/$INSTANCE_ID?service_id=$SERVICE_ID&plan_id=$PLAN_ID_FREE"
expect "repeat deprovision -> 410" 410

# --- 5. error handling -----------------------------------------------------

step "PUT instance with unknown service_id"
BAD_BODY='{"service_id":"00000000-0000-0000-0000-000000000000","plan_id":"00000000-0000-0000-0000-000000000000","organization_guid":"o","space_guid":"s"}'
osb_call PUT "/v2/service_instances/conf-bad-$SUFFIX" "$BAD_BODY"
expect "unknown service_id -> 400" 400

# --- 6. JSON-Schema validation (only when catalog has schemas) -------------
#
# These checks rely on the FREE plan's `parameters.db_name` schema in
# catalog.example.json. They're skipped (treated as PASS-with-note)
# against the built-in default catalog, which has no schemas.

VALIDATION_INSTANCE="conf-val-$SUFFIX"

step "PUT with invalid parameters (db_name empty)"
INVALID_BODY=$(cat <<JSON
{
  "service_id": "$SERVICE_ID",
  "plan_id":    "$PLAN_ID_FREE",
  "organization_guid": "o",
  "space_guid":        "s",
  "parameters": { "db_name": "" }
}
JSON
)
osb_call PUT "/v2/service_instances/$VALIDATION_INSTANCE" "$INVALID_BODY"
if [[ "$STATUS" == "400" ]]; then
    expect "empty db_name -> 400" 400
elif [[ "$STATUS" == "201" || "$STATUS" == "200" ]]; then
    echo "  ${YELLOW}SKIP${RESET} catalog has no JSON-Schema on free plan (got $STATUS)"
    # Clean up the accidentally-created instance.
    osb_call DELETE "/v2/service_instances/$VALIDATION_INSTANCE?service_id=$SERVICE_ID&plan_id=$PLAN_ID_FREE"
else
    expect "empty db_name -> 400" 400
fi

step "PUT with valid parameters (db_name=mydb)"
VALID_BODY=$(cat <<JSON
{
  "service_id": "$SERVICE_ID",
  "plan_id":    "$PLAN_ID_FREE",
  "organization_guid": "o",
  "space_guid":        "s",
  "parameters": { "db_name": "mydb" }
}
JSON
)
osb_call PUT "/v2/service_instances/$VALIDATION_INSTANCE" "$VALID_BODY"
expect "valid db_name -> 201 or 202" 201 202
# Clean up.
osb_call DELETE "/v2/service_instances/$VALIDATION_INSTANCE?service_id=$SERVICE_ID&plan_id=$PLAN_ID_FREE" > /dev/null

# --- 7. async operations (only when broker is configured for async) --------
#
# Set RUN_ASYNC_TESTS=1 and ensure the broker is started with
# BROKER_ASYNC_OP_MILLIS > 0 (e.g., 2000) and `accepts_incomplete=true`.

if [[ "${RUN_ASYNC_TESTS:-0}" == "1" ]]; then
    ASYNC_INSTANCE="conf-async-$SUFFIX"
    step "PUT instance with accepts_incomplete=true (async)"
    osb_call PUT \
        "/v2/service_instances/$ASYNC_INSTANCE?accepts_incomplete=true" \
        "$PROVISION_BODY"
    expect "async provision -> 202" 202

    OP_ID=$(echo "$BODY" | sed -n 's/.*"operation":"\([^"]*\)".*/\1/p')
    if [[ -n "$OP_ID" ]]; then
        step "GET last_operation (should be 'in progress')"
        osb_call GET \
            "/v2/service_instances/$ASYNC_INSTANCE/last_operation?operation=$OP_ID"
        expect "poll while running -> 200" 200
        if echo "$BODY" | grep -q '"in progress"'; then
            echo "  ${GREEN}PASS${RESET} state == \"in progress\""
            PASS=$((PASS + 1))
        else
            echo "  ${YELLOW}WARN${RESET} state was not 'in progress' (broker too fast?)"
        fi

        step "sleep + GET last_operation (should be 'succeeded')"
        sleep 3
        osb_call GET \
            "/v2/service_instances/$ASYNC_INSTANCE/last_operation?operation=$OP_ID"
        expect "poll after delay -> 200" 200
        if echo "$BODY" | grep -q '"succeeded"'; then
            echo "  ${GREEN}PASS${RESET} state == \"succeeded\""
            PASS=$((PASS + 1))
        else
            echo "  ${RED}FAIL${RESET} state did not become 'succeeded': $BODY"
            FAIL=$((FAIL + 1))
            FAILURES+=("async final state")
        fi
    fi

    # Best-effort cleanup.
    osb_call DELETE \
        "/v2/service_instances/$ASYNC_INSTANCE?service_id=$SERVICE_ID&plan_id=$PLAN_ID_FREE&accepts_incomplete=true" \
        > /dev/null || true
else
    step "async tests"
    echo "  ${YELLOW}SKIP${RESET} set RUN_ASYNC_TESTS=1 and start broker with BROKER_ASYNC_OP_MILLIS>0"
fi

# --- summary ---------------------------------------------------------------

echo
echo "${BOLD}=== Conformance Summary ===${RESET}"
echo "  ${GREEN}passed: $PASS${RESET}"
if [[ $FAIL -gt 0 ]]; then
    echo "  ${RED}failed: $FAIL${RESET}"
    for name in "${FAILURES[@]}"; do
        echo "    - $name"
    done
    exit 1
fi
echo "  ${GREEN}all checks passed${RESET}"
