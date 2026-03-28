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
echo "--- Cluster Keyslot ---"
body=$(curl -sf -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/keyslot/test" || echo "{}")
assert_contains "keyslot has slot field" "$body" '"slot":'

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
  -d '{"script":"return redis.call(\"get\", KEYS[1])","keys":["cluster-e2e-test"],"args":[]}')
assert_status "EVAL through cluster (200)" "200" "$status"

echo ""
echo "--- Admin uses standalone (not cluster-routed) ---"
body=$(curl -sf -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/admin/server/info" || echo "{}")
assert_contains "Admin INFO has redis_version" "$body" '"redis_version"'

echo ""
echo "--- Cleanup ---"
curl -sf -X DELETE "$BASE_URL/api/v1/strings/cluster-e2e-test" > /dev/null 2>&1 || true
curl -sf -X DELETE "$BASE_URL/api/v1/strings/cluster-e2e-hash" > /dev/null 2>&1 || true
echo "  Cleanup done"

echo ""
echo "============================================================"
echo " Results: $PASSED passed, $FAILED failed"
echo "============================================================"
[ "$FAILED" -eq 0 ] || exit 1
