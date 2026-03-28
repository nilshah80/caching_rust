#!/usr/bin/env bash
# =============================================================================
# E2E Auth Tests
#
# Tests authentication and authorization for admin endpoints.
#
# Usage:
#   BASE_URL=http://localhost:8080 ADMIN_KEY=dev-admin-key ./auth_test.sh
# =============================================================================
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
ADMIN_KEY="${ADMIN_KEY:-dev-admin-key}"

PASS=0
FAIL=0
ERRORS=""

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
check() {
    local description="$1"
    local expected_status="$2"
    local actual_status="$3"
    local body="$4"

    if [[ "$actual_status" == "$expected_status" ]]; then
        PASS=$((PASS + 1))
        echo "  PASS  $description (HTTP $actual_status)"
    else
        FAIL=$((FAIL + 1))
        ERRORS="$ERRORS\n  FAIL  $description: expected $expected_status, got $actual_status\n        body: ${body:0:200}"
        echo "  FAIL  $description (expected $expected_status, got $actual_status)"
    fi
}

do_request() {
    local method="$1"
    local path="$2"
    local data="${3:-}"
    local extra_headers="${4:-}"

    local args=(-s -w '\n%{http_code}' -X "$method")
    if [[ -n "$data" ]]; then
        args+=(-H 'Content-Type: application/json' -d "$data")
    fi
    if [[ -n "$extra_headers" ]]; then
        args+=(-H "$extra_headers")
    fi

    local response
    response=$(curl "${args[@]}" "${BASE_URL}${path}")
    local status
    status=$(echo "$response" | tail -1)
    local body
    body=$(echo "$response" | sed '$d')
    echo "$status|$body"
}

admin_request() {
    local method="$1"
    local path="$2"
    local data="${3:-}"
    do_request "$method" "$path" "$data" "X-Admin-Api-Key: $ADMIN_KEY"
}

echo "============================================================"
echo " E2E Auth Tests"
echo " Target: $BASE_URL"
echo "============================================================"
echo ""

# ==========================================================================
# Admin endpoints WITHOUT API key should return 401
# ==========================================================================
echo "--- Admin endpoints without API key (expect 401) ---"

IFS='|' read -r status body < <(do_request GET "/api/v1/admin/server/info")
check "Server info without key" "401" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/admin/config/get" '{"pattern":"maxmemory"}')
check "Config get without key" "401" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/admin/persistence/bgsave" '')
check "Persistence bgsave without key" "401" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/admin/client/list")
check "Client list without key" "401" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/admin/slowlog/get" '{"count":5}')
check "Slowlog get without key" "401" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/admin/acl/list")
check "ACL list without key" "401" "$status" "$body"

echo ""

# ==========================================================================
# Admin endpoints with WRONG API key should return 401
# ==========================================================================
echo "--- Admin endpoints with wrong API key (expect 401) ---"

IFS='|' read -r status body < <(do_request GET "/api/v1/admin/server/info" "" "X-Admin-Api-Key: wrong-key-12345")
check "Server info with wrong key" "401" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/admin/config/get" '{"pattern":"maxmemory"}' "X-Admin-Api-Key: wrong-key-12345")
check "Config get with wrong key" "401" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/admin/acl/list" "" "X-Admin-Api-Key: wrong-key-12345")
check "ACL list with wrong key" "401" "$status" "$body"

echo ""

# ==========================================================================
# Admin endpoints with CORRECT API key should return 200
# ==========================================================================
echo "--- Admin endpoints with correct API key (expect 200) ---"

# Server info group
IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/server/info")
check "Server info with correct key" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/server/time")
check "Server time with correct key" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/server/dbsize")
check "DB size with correct key" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/server/lastsave")
check "Last save with correct key" "200" "$status" "$body"

# Config group
IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/config/get" '{"pattern":"maxmemory"}')
check "Config get with correct key" "200" "$status" "$body"

# Persistence group
IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/persistence/bgsave" '')
check "Persistence bgsave with correct key" "200" "$status" "$body"

# Client group
IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/client/list")
check "Client list with correct key" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/client/id")
check "Client id with correct key" "200" "$status" "$body"

# Monitoring group (slowlog)
IFS='|' read -r status body < <(admin_request POST "/api/v1/admin/slowlog/get" '{"count":5}')
check "Slowlog get with correct key" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/slowlog/len")
check "Slowlog len with correct key" "200" "$status" "$body"

# ACL group
IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/acl/list")
check "ACL list with correct key" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/acl/users")
check "ACL users with correct key" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/acl/whoami")
check "ACL whoami with correct key" "200" "$status" "$body"

echo ""

# ==========================================================================
# Non-admin endpoints work without key (expect 200)
# ==========================================================================
echo "--- Non-admin endpoints without key (expect 200) ---"

IFS='|' read -r status body < <(do_request GET "/health")
check "Health check without key" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/admin/pool/stats")
check "Pool stats without key (public)" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/admin/capabilities")
check "Capabilities without key (public)" "200" "$status" "$body"

echo ""
echo "============================================================"
echo " Results: $PASS passed, $FAIL failed"
echo "============================================================"

if [[ $FAIL -gt 0 ]]; then
    echo ""
    echo "Failures:"
    echo -e "$ERRORS"
    echo ""
    exit 1
fi

exit 0
