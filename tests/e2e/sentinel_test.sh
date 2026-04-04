#!/usr/bin/env sh
set -eu

BASE_URL="${1:-http://localhost:8080}"
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
echo " E2E Sentinel Tests"
echo " Target: $BASE_URL"
echo "============================================================"

echo ""
echo "--- Health (mode check) ---"
body=$(curl -sf "$BASE_URL/health/ready" || echo "{}")
assert_contains "Health mode is sentinel" "$body" '"mode":"sentinel"'
assert_contains "Health is ready" "$body" '"status":"ready"'
assert_contains "Redis connected" "$body" '"connected":true'

echo ""
echo "--- Write data before failover ---"
status=$(curl -so /dev/null -w "%{http_code}" -X PUT \
  "$BASE_URL/api/v1/strings/sentinel-e2e-test" \
  -H "Content-Type: application/json" -d '{"value":"before-failover"}')
assert_status "SET before failover (200)" "200" "$status"

body=$(curl -sf "$BASE_URL/api/v1/strings/sentinel-e2e-test" || echo "{}")
assert_contains "GET returns pre-failover value" "$body" '"value":"before-failover"'

echo ""
echo "--- Hash operations through sentinel ---"
status=$(curl -so /dev/null -w "%{http_code}" -X PUT \
  "$BASE_URL/api/v1/hashes/sentinel-e2e-hash" \
  -H "Content-Type: application/json" -d '{"items":{"name":"Bob","role":"sentinel-test"}}')
assert_status "HSET through sentinel (200)" "200" "$status"

body=$(curl -sf "$BASE_URL/api/v1/hashes/sentinel-e2e-hash" || echo "{}")
assert_contains "HGETALL has name" "$body" '"name":"Bob"'

echo ""
echo "--- List operations through sentinel ---"
status=$(curl -so /dev/null -w "%{http_code}" -X POST \
  "$BASE_URL/api/v1/lists/sentinel-e2e-list/lpush" \
  -H "Content-Type: application/json" -d '{"values":["x","y","z"]}')
assert_status "LPUSH through sentinel (200)" "200" "$status"

body=$(curl -sf "$BASE_URL/api/v1/lists/sentinel-e2e-list/length" || echo "{}")
assert_contains "LLEN returns 3" "$body" '"length":3'

echo ""
echo "--- Admin operations (standalone connection) ---"
body=$(curl -sf -H "X-Admin-Api-Key: dev-admin-key" "$BASE_URL/api/v1/admin/server/info" || echo "{}")
assert_contains "Admin INFO has redis_version" "$body" '"redis_version"'

echo ""
echo "--- Pool stats ---"
body=$(curl -sf "$BASE_URL/api/v1/admin/pool/stats" || echo "{}")
assert_contains "Pool stats has max_size" "$body" '"max_size"'

echo ""
echo "--- Cleanup ---"
curl -sf -X DELETE "$BASE_URL/api/v1/keys/sentinel-e2e-test" > /dev/null 2>&1 || true
curl -sf -X DELETE "$BASE_URL/api/v1/keys/sentinel-e2e-hash" > /dev/null 2>&1 || true
curl -sf -X DELETE "$BASE_URL/api/v1/keys/sentinel-e2e-list" > /dev/null 2>&1 || true
echo "  Cleanup done"

echo ""
echo "============================================================"
echo " Results: $PASSED passed, $FAILED failed"
echo "============================================================"
[ "$FAILED" -eq 0 ] || exit 1
