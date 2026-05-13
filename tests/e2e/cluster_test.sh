#!/usr/bin/env sh
set -eu

BASE_URL="${1:-http://localhost:8080}"
API_KEY="${2:-dev-admin-key}"
PASSED=0
FAILED=0

assert_status() {
  name=$1; expected=$2; actual=$3
  if [ "$actual" = "$expected" ]; then
    echo "  PASS  $name (HTTP $actual)"
    PASSED=$((PASSED + 1))
  else
    echo "  FAIL  $name (expected $expected, got $actual)"
    FAILED=$((FAILED + 1))
  fi
}

assert_contains() {
  name=$1; body=$2; pattern=$3
  if echo "$body" | grep -q "$pattern"; then
    echo "  PASS  $name (contains '$pattern')"
    PASSED=$((PASSED + 1))
  else
    echo "  FAIL  $name (missing '$pattern')"
    FAILED=$((FAILED + 1))
  fi
}

assert_status_in() {
  name=$1; actual=$2; shift 2
  for s in "$@"; do
    if [ "$actual" = "$s" ]; then
      echo "  PASS  $name (HTTP $actual)"
      PASSED=$((PASSED + 1))
      return
    fi
  done
  echo "  FAIL  $name (expected one of [$*], got $actual)"
  FAILED=$((FAILED + 1))
}

echo "============================================================"
echo " E2E Cluster Tests"
echo " Target: $BASE_URL"
echo "============================================================"

echo ""
echo "--- Health (mode check) ---"
body=$(curl -sf "$BASE_URL/health/ready" || echo "{}")
assert_contains "Health mode is cluster" "$body" '"mode":"cluster"'
assert_contains "Health is ready" "$body" '"status":"ready"'
assert_contains "Redis connected" "$body" '"connected":true'

echo ""
echo "--- Cluster Info (auth required) ---"
status=$(curl -so /dev/null -w "%{http_code}" "$BASE_URL/api/v1/cluster/info")
assert_status "CLUSTER INFO without key (401)" "401" "$status"

status=$(curl -so /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/info")
assert_status "CLUSTER INFO with key (200)" "200" "$status"

body=$(curl -sf -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/info" || echo "{}")
assert_contains "cluster_state ok" "$body" '"cluster_state":"ok"'
assert_contains "slots assigned 16384" "$body" '"cluster_slots_assigned":16384'
assert_contains "3 known nodes" "$body" '"cluster_known_nodes":3'

echo ""
echo "--- Cluster Nodes ---"
status=$(curl -so /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/nodes")
assert_status "CLUSTER NODES (200)" "200" "$status"

echo ""
echo "--- Cluster Slots ---"
status=$(curl -so /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/slots")
assert_status "CLUSTER SLOTS (200)" "200" "$status"

echo ""
echo "--- Cluster Shards ---"
status=$(curl -so /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/shards")
assert_status "CLUSTER SHARDS (200)" "200" "$status"

echo ""
echo "--- Cluster Identity / Links ---"
body=$(curl -sf -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/myid" || echo "{}")
assert_contains "CLUSTER MYID has id" "$body" '"id":'

status=$(curl -so /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/myshardid")
assert_status "CLUSTER MYSHARDID (200)" "200" "$status"

status=$(curl -so /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/links")
assert_status "CLUSTER LINKS (200)" "200" "$status"

echo ""
echo "--- Cluster Keyslot ---"
body=$(curl -sf -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/keyslot/test" || echo "{}")
assert_contains "keyslot has slot field" "$body" '"slot":'

echo ""
echo "--- Cluster Slot Introspection ---"
status=$(curl -so /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/countkeysinslot/0")
assert_status "CLUSTER COUNTKEYSINSLOT (200)" "200" "$status"

status=$(curl -so /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/getkeysinslot/0?count=10")
assert_status "CLUSTER GETKEYSINSLOT (200)" "200" "$status"

status=$(curl -so /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/getkeysinslot/0?count=0")
assert_status "CLUSTER GETKEYSINSLOT rejects count=0" "400" "$status"

MASTER_ID=$(curl -sf -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/myid" \
  | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
if [ -n "$MASTER_ID" ]; then
  status=$(curl -so /dev/null -w "%{http_code}" \
    -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/replicas/$MASTER_ID")
  assert_status "CLUSTER REPLICAS (200)" "200" "$status"
else
  echo "  FAIL  CLUSTER REPLICAS (could not parse node id)"
  FAILED=$((FAILED + 1))
fi

echo ""
echo "--- Data Operations Through Cluster ---"
status=$(curl -so /dev/null -w "%{http_code}" -X PUT \
  "$BASE_URL/api/v1/strings/cluster-e2e-test" \
  -H "Content-Type: application/json" -d '{"value":"cluster-works"}')
assert_status "SET through cluster (200)" "200" "$status"

body=$(curl -sf "$BASE_URL/api/v1/strings/cluster-e2e-test" || echo "{}")
assert_contains "GET returns value" "$body" '"value":"cluster-works"'

echo ""
echo "--- Hash through cluster ---"
status=$(curl -so /dev/null -w "%{http_code}" -X PUT \
  "$BASE_URL/api/v1/hashes/cluster-e2e-hash" \
  -H "Content-Type: application/json" -d '{"items":{"name":"Alice","city":"NYC"}}')
assert_status "HSET through cluster (200)" "200" "$status"

body=$(curl -sf "$BASE_URL/api/v1/hashes/cluster-e2e-hash" || echo "{}")
assert_contains "HGETALL has name" "$body" '"name":"Alice"'

echo ""
echo "--- List through cluster ---"
status=$(curl -so /dev/null -w "%{http_code}" -X POST \
  "$BASE_URL/api/v1/lists/cluster-e2e-list/lpush" \
  -H "Content-Type: application/json" -d '{"values":["a","b","c"]}')
assert_status "LPUSH through cluster (200)" "200" "$status"

body=$(curl -sf "$BASE_URL/api/v1/lists/cluster-e2e-list/length" || echo "{}")
assert_contains "LLEN returns 3" "$body" '"length":3'

echo ""
echo "--- Scripting through cluster ---"
status=$(curl -so /dev/null -w "%{http_code}" -X POST \
  "$BASE_URL/api/v1/scripts/eval" \
  -H "Content-Type: application/json" \
  -H "X-Admin-Api-Key: $API_KEY" \
  -d '{"script":"return redis.call(\"get\", KEYS[1])","keys":["cluster-e2e-test"],"args":[]}')
assert_status "EVAL through cluster (200)" "200" "$status"

echo ""
echo "--- Admin uses standalone (not cluster-routed) ---"
body=$(curl -sf -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/admin/server/info" || echo "{}")
assert_contains "Admin INFO has redis_version" "$body" '"redis_version"'

echo ""
echo "--- Cluster Slot-Stats (Redis 8.2+; 501 on older builds) ---"
status=$(curl -so /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" \
  "$BASE_URL/api/v1/cluster/slot-stats?slot_start=0&slot_end=100")
assert_status_in "CLUSTER SLOT-STATS (range)" "$status" "200" "501"

# Auth check: missing key returns 401 regardless of capability
status=$(curl -so /dev/null -w "%{http_code}" \
  "$BASE_URL/api/v1/cluster/slot-stats?slot_start=0&slot_end=100")
assert_status "CLUSTER SLOT-STATS rejects no auth" "401" "$status"

# Empty filter is rejected with 400 (capability-on) or 501 (capability-off)
status=$(curl -so /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/slot-stats")
assert_status_in "CLUSTER SLOT-STATS rejects empty filter" "$status" "400" "501"

# ORDERBY with limit + desc — accepted form
status=$(curl -so /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" \
  "$BASE_URL/api/v1/cluster/slot-stats?order_by=key_count&limit=5&order=desc")
assert_status_in "CLUSTER SLOT-STATS (orderby)" "$status" "200" "501"

# Mixed mode is rejected with 400 (capability-on) or 501 (capability-off)
status=$(curl -so /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" \
  "$BASE_URL/api/v1/cluster/slot-stats?slot_start=0&slot_end=10&order_by=cpu_usec")
assert_status_in "CLUSTER SLOT-STATS rejects mixed mode" "$status" "400" "501"

echo ""
echo "--- Cleanup ---"
curl -sf -X DELETE "$BASE_URL/api/v1/keys/cluster-e2e-test" > /dev/null 2>&1 || true
curl -sf -X DELETE "$BASE_URL/api/v1/keys/cluster-e2e-hash" > /dev/null 2>&1 || true
curl -sf -X DELETE "$BASE_URL/api/v1/keys/cluster-e2e-list" > /dev/null 2>&1 || true
echo "  Cleanup done"

echo ""
echo "============================================================"
echo " Results: $PASSED passed, $FAILED failed"
echo "============================================================"
[ "$FAILED" -eq 0 ] || exit 1
