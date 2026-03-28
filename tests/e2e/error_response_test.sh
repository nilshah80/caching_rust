#!/usr/bin/env bash
# =============================================================================
# E2E Error Response Tests
#
# Tests error handling and validates error response format.
#
# Usage:
#   BASE_URL=http://localhost:8080 ADMIN_KEY=dev-admin-key ./error_response_test.sh
# =============================================================================
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
ADMIN_KEY="${ADMIN_KEY:-dev-admin-key}"

PASS=0
FAIL=0
ERRORS=""

# Unique prefix to avoid key collisions
P="err_e2e_$$"

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

check_body_contains() {
    local description="$1"
    local expected="$2"
    local body="$3"

    if echo "$body" | grep -q "$expected"; then
        PASS=$((PASS + 1))
        echo "  PASS  $description (body contains '$expected')"
    else
        FAIL=$((FAIL + 1))
        ERRORS="$ERRORS\n  FAIL  $description: body does not contain '$expected'\n        body: ${body:0:200}"
        echo "  FAIL  $description (body does not contain '$expected')"
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

echo "============================================================"
echo " E2E Error Response Tests"
echo " Target: $BASE_URL"
echo "============================================================"
echo ""

# ==========================================================================
# 1. GET non-existent key -> 404 with KEY_NOT_FOUND
# ==========================================================================
echo "--- Key Not Found ---"

IFS='|' read -r status body < <(do_request GET "/api/v1/strings/${P}_nonexistent_key")
check "GET non-existent key returns 404" "404" "$status" "$body"
check_body_contains "Response contains KEY_NOT_FOUND" "KEY_NOT_FOUND" "$body"

echo ""

# ==========================================================================
# 2. Invalid JSON body -> 400
# ==========================================================================
echo "--- Invalid JSON Body ---"

IFS='|' read -r status body < <(do_request PUT "/api/v1/strings/${P}_badjson" "not valid json at all")
check "Invalid JSON body returns 400" "400" "$status" "$body"

echo ""

# ==========================================================================
# 3. Wrong type operation -> error with REDIS_ERROR
# ==========================================================================
echo "--- Wrong Type Operation ---"

# First, set a string key
IFS='|' read -r status body < <(do_request PUT "/api/v1/strings/${P}_strtype" '{"value":"hello"}')
check "SET string key" "200" "$status" "$body"

# Then try LPUSH on that string key (type mismatch)
IFS='|' read -r status body < <(do_request POST "/api/v1/lists/${P}_strtype/lpush" '{"values":["oops"]}')
check "LPUSH on string key returns error" "500" "$status" "$body"
check_body_contains "Response contains REDIS_ERROR" "REDIS_ERROR" "$body"

echo ""

# ==========================================================================
# 4. Invalid input -> 400 with INVALID_INPUT
# ==========================================================================
echo "--- Invalid Input ---"

IFS='|' read -r status body < <(do_request POST "/api/v1/lists/${P}_bad_list/lpush" '{"values":[]}')
check "Empty list push returns 400" "400" "$status" "$body"
check_body_contains "Response contains INVALID_INPUT" "INVALID_INPUT" "$body"

echo ""

# ==========================================================================
# 5. Validate error response format
# ==========================================================================
echo "--- Error Response Format Validation ---"

IFS='|' read -r status body < <(do_request GET "/api/v1/strings/${P}_format_check_nonexistent")

# Validate that error response has the expected structure:
# {"success":false, "timestamp":"...", "error":{"code":"...", "message":"..."}}
check "Error response status is 404" "404" "$status" "$body"
check_body_contains "Has success:false" '"success":false' "$body"
check_body_contains "Has timestamp field" '"timestamp"' "$body"
check_body_contains "Has error.code field" '"code"' "$body"
check_body_contains "Has error.message field" '"message"' "$body"

echo ""

# ==========================================================================
# Cleanup
# ==========================================================================
echo "--- Cleanup ---"
CLEANUP_KEYS=$(curl -s "${BASE_URL}/api/v1/keys/scan?pattern=${P}*&count=1000" | \
    sed -n 's/.*"keys":\[\([^]]*\)\].*/\1/p' | tr ',' '\n' | tr -d '"' | tr -d ' ')
if [[ -n "$CLEANUP_KEYS" ]]; then
    KEYS_JSON=$(echo "$CLEANUP_KEYS" | awk 'BEGIN{printf "["} NR>1{printf ","} {printf "\"%s\"",$0} END{printf "]"}')
    curl -s -X POST "${BASE_URL}/api/v1/keys/delete" \
        -H 'Content-Type: application/json' \
        -d "{\"keys\":$KEYS_JSON}" > /dev/null 2>&1 || true
fi
echo "  Cleanup attempted for prefix ${P}_*"

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
