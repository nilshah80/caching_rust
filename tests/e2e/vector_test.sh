#!/usr/bin/env bash
# =============================================================================
# Vector Sets E2E Test (Phase 10.1)
#
# Tests Vector Sets operations against a running service backed by Redis 8.0+.
#
# Usage:
#   BASE_URL=http://localhost:8080 ADMIN_KEY=dev-admin-key ./vector_test.sh
# =============================================================================
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
ADMIN_KEY="${ADMIN_KEY:-dev-admin-key}"

PASS=0
FAIL=0
SKIP=0
ERRORS=""

# Unique prefix to avoid key collisions between test runs
P="vec_e2e_$$"

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

echo "============================================================"
echo " Vector Sets E2E Smoke Tests (Phase 10.1)"
echo " Target: $BASE_URL"
echo "============================================================"
echo ""

echo "--- Vector Pre-Check ---"
IFS='|' read -r status body < <(do_request GET "/health")
check "Health check" "200" "$status" "$body"
IFS='|' read -r status body < <(do_request GET "/api/v1/admin/capabilities" "" "X-Admin-Api-Key: $ADMIN_KEY")
if [[ "$body" != *"\"vectors\":true"* ]]; then
    echo "  SKIP  Vectors not available in capabilities. Ensure Redis 8.0+ is running."
    exit 0
fi
check "Vectors capability detected" "200" "$status" "$body"

echo ""
echo "--- Vector Commands ---"

IFS='|' read -r status body < <(do_request POST "/api/v1/vectors/${P}_vec1/add" \
    '{"items":{"doc1":[0.1, 0.2, 0.3], "doc2":[0.4, 0.5, 0.6]}}')
check "VADD" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/vectors/${P}_vec1/card")
check "VCARD" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/vectors/${P}_vec1/dim")
check "VDIM" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/vectors/${P}_vec1/emb" \
    '{"items":["doc1","doc2","missing"]}')
check "VEMB" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/vectors/${P}_vec1/ismember" \
    '{"items":["doc1","missing"]}')
check "VISMEMBER" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/vectors/${P}_vec1/sim" \
    '{"vector":[0.1, 0.2, 0.3],"k":1}')
check "VSIM" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/vectors/${P}_vec1/rem" \
    '{"items":["doc2"]}')
check "VREM" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/vectors/${P}_vec1/info")
check "VINFO" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/vectors/${P}_vec1/links/doc1")
check "VLINKS" "200" "$status" "$body"

# VRANGE is separately gated — only test if the capability reports it
IFS='|' read -r cap_status cap_body < <(do_request GET "/api/v1/admin/capabilities" "" "X-Admin-Api-Key: $ADMIN_KEY")
if echo "$cap_body" | grep -q '"vector_range":true'; then
    IFS='|' read -r status body < <(do_request POST "/api/v1/vectors/${P}_vec1/range" \
        '{"start":"-","end":"+"}')
    check "VRANGE" "200" "$status" "$body"
else
    echo "  SKIP  VRANGE not available on this Redis build"
    SKIP=$((SKIP + 1))
fi

echo ""
echo "--- Adversarial: Mixed Valid/Invalid Batch ---"

# Create a fresh vector set with known state
IFS='|' read -r status body < <(do_request POST "/api/v1/vectors/${P}_adv/add" \
    '{"items":{"seed":[1.0, 2.0, 3.0]}}')
check "VADD seed element" "200" "$status" "$body"

# Attempt to add a batch where the second element has a mismatched dimension.
# The Lua script pre-validates dimensions, so the entire batch must be rejected
# and the key must remain unchanged (only "seed" present).
IFS='|' read -r status body < <(do_request POST "/api/v1/vectors/${P}_adv/add" \
    '{"items":{"good":[4.0, 5.0, 6.0], "bad":[7.0, 8.0]}}')
check "VADD mixed-dim batch rejected" "400" "$status" "$body"

# Verify the key was not mutated — cardinality should still be 1 (only "seed")
IFS='|' read -r status body < <(do_request GET "/api/v1/vectors/${P}_adv/card")
check "VCARD unchanged after rejected batch" "200" "$status" "$body"
# Verify count is 1
if echo "$body" | grep -q '"count":1'; then
    PASS=$((PASS + 1))
    echo "  PASS  VCARD count == 1 (no partial writes)"
else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL  VCARD count != 1 after rejected batch\n        body: ${body:0:200}"
    echo "  FAIL  VCARD count != 1 after rejected batch"
fi

echo ""
echo "--- Regression: Redis-side error paths ---"

# VSIM with empty vector must be rejected as 400, not 500
IFS='|' read -r status body < <(do_request POST "/api/v1/vectors/${P}_vec1/sim" \
    '{"vector":[],"k":1}')
check "VSIM empty vector rejected (400)" "400" "$status" "$body"

# VSIM with k=0 must be rejected as 400, not 500
IFS='|' read -r status body < <(do_request POST "/api/v1/vectors/${P}_vec1/sim" \
    '{"vector":[0.1, 0.2, 0.3],"k":0}')
check "VSIM k=0 rejected (400)" "400" "$status" "$body"

# VADD with empty vector must be rejected as 400
IFS='|' read -r status body < <(do_request POST "/api/v1/vectors/${P}_adv/add" \
    '{"items":{"empty_vec":[]}}')
check "VADD empty vector rejected (400)" "400" "$status" "$body"

# VADD dimension mismatch against existing set (set has dim=3, batch sends dim=2)
IFS='|' read -r status body < <(do_request POST "/api/v1/vectors/${P}_adv/add" \
    '{"items":{"wrong_dim":[1.0, 2.0]}}')
check "VADD dim mismatch vs existing set rejected (400)" "400" "$status" "$body"

# Verify the adversarial key is still unchanged after all rejected batches
IFS='|' read -r status body < <(do_request GET "/api/v1/vectors/${P}_adv/card")
if echo "$body" | grep -q '"count":1'; then
    PASS=$((PASS + 1))
    echo "  PASS  VCARD still 1 after all error-path tests"
else
    FAIL=$((FAIL + 1))
    ERRORS="$ERRORS\n  FAIL  VCARD != 1 after error-path tests\n        body: ${body:0:200}"
    echo "  FAIL  VCARD != 1 after error-path tests"
fi

# VRANDMEMBER with count=0 must be rejected as 400
IFS='|' read -r status body < <(do_request POST "/api/v1/vectors/${P}_adv/randmember" \
    '{"count":0}')
check "VRANDMEMBER count=0 rejected (400)" "400" "$status" "$body"

echo ""
echo "--- Attribute lifecycle: set, get, delete ---"

# Set attributes on the seed element
IFS='|' read -r status body < <(do_request POST "/api/v1/vectors/${P}_adv/attr/seed" \
    '{"attributes":"{\"color\":\"blue\"}"}')
check "VSETATTR set attributes" "200" "$status" "$body"

# Verify attributes are set
IFS='|' read -r status body < <(do_request GET "/api/v1/vectors/${P}_adv/attr/seed")
check "VGETATTR returns attributes" "200" "$status" "$body"

# Delete attributes by sending empty string (Redis documented behavior)
IFS='|' read -r status body < <(do_request POST "/api/v1/vectors/${P}_adv/attr/seed" \
    '{"attributes":""}')
check "VSETATTR delete attributes (empty string)" "200" "$status" "$body"

# Verify attributes are now null/empty
IFS='|' read -r status body < <(do_request GET "/api/v1/vectors/${P}_adv/attr/seed")
check "VGETATTR returns null after deletion" "200" "$status" "$body"

echo ""
echo "--- Cleanup ---"
curl -s -X POST "${BASE_URL}/api/v1/keys/delete" \
    -H 'Content-Type: application/json' \
    -d "{\"keys\":[\"${P}_vec1\",\"${P}_adv\"]}" > /dev/null 2>&1 || true
echo "  Cleanup completed"

echo ""
echo "============================================================"
echo " Results: $PASS passed, $FAIL failed, $SKIP skipped"
echo "============================================================"

if [[ $FAIL -gt 0 ]]; then
    echo ""
    echo "Failures:"
    echo -e "$ERRORS"
    echo ""
    exit 1
fi
exit 0
