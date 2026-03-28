#!/usr/bin/env bash
# =============================================================================
# E2E SSE Streaming Tests
#
# Tests Server-Sent Events (SSE) streaming endpoints.
#
# Usage:
#   BASE_URL=http://localhost:8080 ADMIN_KEY=dev-admin-key ./sse_test.sh
# =============================================================================
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
ADMIN_KEY="${ADMIN_KEY:-dev-admin-key}"

PASS=0
FAIL=0
ERRORS=""

# Unique prefix to avoid key collisions
P="sse_e2e_$$"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
check() {
    local description="$1"
    local expected="$2"
    local actual="$3"

    if [[ "$actual" == "$expected" ]]; then
        PASS=$((PASS + 1))
        echo "  PASS  $description"
    else
        FAIL=$((FAIL + 1))
        ERRORS="$ERRORS\n  FAIL  $description: expected '$expected', got '$actual'"
        echo "  FAIL  $description (expected '$expected', got '$actual')"
    fi
}

check_contains() {
    local description="$1"
    local expected="$2"
    local actual="$3"

    if echo "$actual" | grep -q "$expected"; then
        PASS=$((PASS + 1))
        echo "  PASS  $description"
    else
        FAIL=$((FAIL + 1))
        ERRORS="$ERRORS\n  FAIL  $description: output does not contain '$expected'\n        got: ${actual:0:300}"
        echo "  FAIL  $description (output does not contain '$expected')"
    fi
}

do_request() {
    local method="$1"
    local path="$2"
    local data="${3:-}"

    local args=(-s -w '\n%{http_code}' -X "$method")
    if [[ -n "$data" ]]; then
        args+=(-H 'Content-Type: application/json' -d "$data")
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
echo " E2E SSE Streaming Tests"
echo " Target: $BASE_URL"
echo "============================================================"
echo ""

# ==========================================================================
# 1. BLPOP SSE stream receives data pushed during stream
# ==========================================================================
echo "--- BLPOP SSE Stream ---"

# Start SSE listener in background, capture output for up to 8 seconds
SSE_OUTPUT_FILE=$(mktemp)
curl -s --no-buffer --max-time 8 \
    -H "Accept: text/event-stream" \
    "${BASE_URL}/api/v1/lists/${P}_blpop_list/blpop/stream" \
    > "$SSE_OUTPUT_FILE" 2>/dev/null &
SSE_PID=$!

# Give the SSE connection time to establish
sleep 1

# Push data to the list
IFS='|' read -r status body < <(do_request POST "/api/v1/lists/${P}_blpop_list/rpush" \
    '{"values":["sse_blpop_value"]}')

# Wait for SSE to pick it up
sleep 3

# Kill SSE listener if still running
kill "$SSE_PID" 2>/dev/null || true
wait "$SSE_PID" 2>/dev/null || true

SSE_OUTPUT=$(cat "$SSE_OUTPUT_FILE")
rm -f "$SSE_OUTPUT_FILE"

check_contains "BLPOP SSE stream receives pushed data" "sse_blpop_value" "$SSE_OUTPUT"

echo ""

# ==========================================================================
# 2. BZPOPMIN SSE stream receives data
# ==========================================================================
echo "--- BZPOPMIN SSE Stream ---"

SSE_OUTPUT_FILE=$(mktemp)
curl -s --no-buffer --max-time 8 \
    -H "Accept: text/event-stream" \
    "${BASE_URL}/api/v1/sorted-sets/${P}_bzpop_zset/bzpopmin/stream" \
    > "$SSE_OUTPUT_FILE" 2>/dev/null &
SSE_PID=$!

sleep 1

# Add a member to the sorted set
IFS='|' read -r status body < <(do_request POST "/api/v1/sorted-sets/${P}_bzpop_zset/members" \
    '{"members":[{"member":"sse_zset_member","score":42.0}]}')

sleep 3

kill "$SSE_PID" 2>/dev/null || true
wait "$SSE_PID" 2>/dev/null || true

SSE_OUTPUT=$(cat "$SSE_OUTPUT_FILE")
rm -f "$SSE_OUTPUT_FILE"

check_contains "BZPOPMIN SSE stream receives sorted set data" "sse_zset_member" "$SSE_OUTPUT"

echo ""

# ==========================================================================
# 3. Stream SSE endpoint receives XADD data
# ==========================================================================
echo "--- Stream Subscribe SSE ---"

# First, add an initial entry so the stream exists
IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${P}_sse_stream/add" \
    '{"fields":{"init":"true"}}')

SSE_OUTPUT_FILE=$(mktemp)
curl -s --no-buffer --max-time 8 \
    -H "Accept: text/event-stream" \
    "${BASE_URL}/api/v1/streams/${P}_sse_stream/subscribe?last_id=0" \
    > "$SSE_OUTPUT_FILE" 2>/dev/null &
SSE_PID=$!

sleep 1

# Add data to the stream
IFS='|' read -r status body < <(do_request POST "/api/v1/streams/${P}_sse_stream/add" \
    '{"fields":{"sensor":"temp","value":"25.5"}}')

sleep 3

kill "$SSE_PID" 2>/dev/null || true
wait "$SSE_PID" 2>/dev/null || true

SSE_OUTPUT=$(cat "$SSE_OUTPUT_FILE")
rm -f "$SSE_OUTPUT_FILE"

# The SSE output should contain at least the initial entry or the new entry
check_contains "Stream SSE receives XADD data" "data:" "$SSE_OUTPUT"

echo ""

# ==========================================================================
# 4. SSE endpoints return proper text/event-stream content type
# ==========================================================================
echo "--- SSE Content-Type ---"

# Check content-type header using -D (dump headers)
HEADER_FILE=$(mktemp)
curl -s --no-buffer --max-time 3 \
    -D "$HEADER_FILE" \
    -H "Accept: text/event-stream" \
    "${BASE_URL}/api/v1/lists/${P}_ct_list/blpop/stream" \
    > /dev/null 2>&1 &
CT_PID=$!

sleep 2

kill "$CT_PID" 2>/dev/null || true
wait "$CT_PID" 2>/dev/null || true

HEADERS=$(cat "$HEADER_FILE")
rm -f "$HEADER_FILE"

check_contains "SSE Content-Type is text/event-stream" "text/event-stream" "$HEADERS"

echo ""

# ==========================================================================
# 5. SSE keepalive works (ping/comment received within 20s)
# ==========================================================================
echo "--- SSE Keepalive ---"

SSE_OUTPUT_FILE=$(mktemp)
curl -s --no-buffer --max-time 20 \
    -H "Accept: text/event-stream" \
    "${BASE_URL}/api/v1/lists/${P}_ka_list/blpop/stream" \
    > "$SSE_OUTPUT_FILE" 2>/dev/null &
SSE_PID=$!

# Wait up to 18 seconds for a keepalive comment (lines starting with ':')
KEEPALIVE_FOUND="false"
for i in $(seq 1 18); do
    sleep 1
    if grep -q "^:" "$SSE_OUTPUT_FILE" 2>/dev/null; then
        KEEPALIVE_FOUND="true"
        break
    fi
done

kill "$SSE_PID" 2>/dev/null || true
wait "$SSE_PID" 2>/dev/null || true

SSE_OUTPUT=$(cat "$SSE_OUTPUT_FILE")
rm -f "$SSE_OUTPUT_FILE"

check "SSE keepalive received within 20s" "true" "$KEEPALIVE_FOUND"

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
