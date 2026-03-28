#!/usr/bin/env bash
# =============================================================================
# Core E2E Smoke Test (Phases 1-4, 6)
#
# Tests core Redis operations against a running service.
#
# Usage:
#   BASE_URL=http://localhost:8080 ADMIN_KEY=dev-admin-key ./core_smoke_test.sh
# =============================================================================
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
ADMIN_KEY="${ADMIN_KEY:-dev-admin-key}"

PASS=0
FAIL=0
SKIP=0
ERRORS=""

# Unique prefix to avoid key collisions between test runs
P="core_e2e_$$"

# ---------------------------------------------------------------------------
# Helper functions
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

check_range() {
    local description="$1"
    local min_status="$2"
    local max_status="$3"
    local actual_status="$4"
    local body="$5"

    if [[ "$actual_status" -ge "$min_status" && "$actual_status" -le "$max_status" ]]; then
        PASS=$((PASS + 1))
        echo "  PASS  $description (HTTP $actual_status)"
    else
        FAIL=$((FAIL + 1))
        ERRORS="$ERRORS\n  FAIL  $description: expected ${min_status}-${max_status}, got $actual_status\n        body: ${body:0:200}"
        echo "  FAIL  $description (expected ${min_status}-${max_status}, got $actual_status)"
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
echo " Core E2E Smoke Tests (Phases 1-4, 6)"
echo " Target: $BASE_URL"
echo "============================================================"
echo ""

# ==========================================================================
# Health
# ==========================================================================
echo "--- Health ---"

IFS='|' read -r status body < <(do_request GET "/health")
check "Health check" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/health/ready")
check "Readiness check" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/health/live")
check "Liveness check" "200" "$status" "$body"

echo ""

# ==========================================================================
# Strings
# ==========================================================================
echo "--- Strings ---"

IFS='|' read -r status body < <(do_request PUT "/api/v1/strings/${P}_str1" \
    '{"value":"hello"}')
check "SET" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/strings/${P}_str1")
check "GET" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request PUT "/api/v1/strings/${P}_str2" \
    '{"value":"world"}')

IFS='|' read -r status body < <(do_request POST "/api/v1/strings/mget" \
    "{\"keys\":[\"${P}_str1\",\"${P}_str2\"]}")
check "MGET" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/strings/mset" \
    "{\"pairs\":{\"${P}_ms1\":\"a\",\"${P}_ms2\":\"b\"}}")
check "MSET" "200" "$status" "$body"

# INCR: set a numeric value first
IFS='|' read -r status body < <(do_request PUT "/api/v1/strings/${P}_counter" \
    '{"value":"10"}')

IFS='|' read -r status body < <(do_request PATCH "/api/v1/strings/${P}_counter/incr" \
    '{"delta":5}')
check "INCR" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request PATCH "/api/v1/strings/${P}_str1/append" \
    '{"value":" world"}')
check "APPEND" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request DELETE "/api/v1/strings/${P}_str2")
check "GETDEL" "200" "$status" "$body"

echo ""

# ==========================================================================
# Hashes
# ==========================================================================
echo "--- Hashes ---"

IFS='|' read -r status body < <(do_request PUT "/api/v1/hashes/${P}_hash" \
    '{"items":{"name":"Alice","age":"30","city":"NYC"}}')
check "HSET" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/hashes/${P}_hash")
check "HGETALL" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/hashes/${P}_hash/fields/name")
check "HGET" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/hashes/${P}_hash/length")
check "HLEN" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/hashes/${P}_hash/keys")
check "HKEYS" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/hashes/${P}_hash/values")
check "HVALS" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request PATCH "/api/v1/hashes/${P}_hash/fields/age/incr" \
    '{"delta":1}')
check "HINCRBY" "200" "$status" "$body"

echo ""

# ==========================================================================
# Lists
# ==========================================================================
echo "--- Lists ---"

IFS='|' read -r status body < <(do_request POST "/api/v1/lists/${P}_list/rpush" \
    '{"values":["a","b","c"]}')
check "RPUSH" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/lists/${P}_list/lpush" \
    '{"values":["z"]}')
check "LPUSH" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/lists/${P}_list/length")
check "LLEN" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/lists/${P}_list/range?start=0&stop=-1")
check "LRANGE" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/lists/${P}_list/lpop" '{}')
check "LPOP" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/lists/${P}_list/rpop" '{}')
check "RPOP" "200" "$status" "$body"

echo ""

# ==========================================================================
# Sets
# ==========================================================================
echo "--- Sets ---"

IFS='|' read -r status body < <(do_request POST "/api/v1/sets/${P}_set/members" \
    '{"members":["x","y","z","x"]}')
check "SADD" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/sets/${P}_set/members")
check "SMEMBERS" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/sets/${P}_set/card")
check "SCARD" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/sets/${P}_set/ismember" \
    '{"member":"x"}')
check "SISMEMBER" "200" "$status" "$body"

echo ""

# ==========================================================================
# Sorted Sets
# ==========================================================================
echo "--- Sorted Sets ---"

IFS='|' read -r status body < <(do_request POST "/api/v1/sorted-sets/${P}_zset/members" \
    '{"members":[{"member":"alice","score":100},{"member":"bob","score":200},{"member":"carol","score":150}]}')
check "ZADD" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/sorted-sets/${P}_zset/range?start=0&stop=-1")
check "ZRANGE" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/sorted-sets/${P}_zset/score/alice")
check "ZSCORE" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/sorted-sets/${P}_zset/card")
check "ZCARD" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/sorted-sets/${P}_zset/rank/bob")
check "ZRANK" "200" "$status" "$body"

echo ""

# ==========================================================================
# Streams
# ==========================================================================
echo "--- Streams ---"

IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${P}_stream/add" \
    '{"fields":{"sensor":"temp","value":"21.5"}}')
check "XADD" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${P}_stream/add" \
    '{"fields":{"sensor":"temp","value":"22.0"}}')

IFS='|' read -r status body < <(do_request GET "/api/v1/streams/${P}_stream/length")
check "XLEN" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/streams/${P}_stream/range?start=-&end=%2B")
check "XRANGE" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/streams/${P}_stream/info")
check "XINFO" "200" "$status" "$body"

echo ""

# ==========================================================================
# Keys
# ==========================================================================
echo "--- Keys ---"

IFS='|' read -r status body < <(do_request POST "/api/v1/keys/exists" \
    "{\"keys\":[\"${P}_str1\",\"${P}_nonexistent\"]}")
check "EXISTS" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/keys/${P}_str1/ttl")
check "TTL" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/keys/${P}_str1/type")
check "TYPE" "200" "$status" "$body"

IFS='|' read -r status body < <(do_request GET "/api/v1/keys/scan?pattern=${P}*&count=100")
check "SCAN" "200" "$status" "$body"

echo ""

# ==========================================================================
# Admin
# ==========================================================================
echo "--- Admin ---"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/server/dbsize")
check "DBSIZE" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/server/info")
check "INFO" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/acl/list")
check "ACL LIST" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/pool/stats")
check "POOL STATS" "200" "$status" "$body"

IFS='|' read -r status body < <(admin_request GET "/api/v1/admin/capabilities")
check "CAPABILITIES" "200" "$status" "$body"

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
