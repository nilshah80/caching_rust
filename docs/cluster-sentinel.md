# Phase 7: Cluster & Sentinel Support

## Overview

Phase 7 adds optional Redis Cluster and Sentinel support to the caching service. Both modes are **additive** -- the existing standalone connection mode remains the default. Cluster and Sentinel are mutually exclusive; enabling both is a configuration error.

## Architecture

### Connection Modes

```
                    +-------------------+
                    |   Settings::load  |
                    +--------+----------+
                             |
              +--------------+--------------+
              |              |              |
        standalone      cluster        sentinel
     (default mode)   (multi-node)   (HA failover)
              |              |              |
     deadpool-redis   ClusterClient   SentinelClient
         Pool          (redis crate)   (redis crate)
              |              |              |
              +--------------+--------------+
                             |
                      +------+------+
                      |  AppState   |
                      |  .pool      |  (standalone or sentinel)
                      |  .cluster   |  (Option<ClusterConnection>)
                      +-------------+
```

### Design Decisions

1. **Cluster uses the `redis` crate's `ClusterClient`** -- handles MOVED/ASK redirects, slot mapping, and node discovery automatically. We do NOT wrap it in `deadpool` because the cluster client already manages multiple connections internally.

2. **Sentinel uses the `redis` crate's `SentinelClient`** to resolve the current master, then creates a standard `deadpool-redis` pool pointing at that master. This means all existing code works unchanged -- only the connection URL resolution changes.

3. **Cluster info endpoints are admin-protected** -- CLUSTER INFO/NODES/SLOTS expose topology details that should not be public.

4. **Capability gating** -- Cluster routes only register when `capabilities.features.cluster == true` (detected at startup via `CLUSTER INFO`).

## Configuration

### Cluster Mode

```env
# Enable cluster mode (mutually exclusive with sentinel)
REDIS__CLUSTER_ENABLED=true

# Comma-separated seed node URLs (at least 3 recommended)
REDIS__CLUSTER_NODES=redis://node1:7001,redis://node2:7002,redis://node3:7003

# Optional: cluster-specific settings
REDIS__CLUSTER_READ_FROM_REPLICAS=false
REDIS__PASSWORD=shared-cluster-password
```

### Sentinel Mode

```env
# Enable sentinel mode (mutually exclusive with cluster)
REDIS__SENTINEL_ENABLED=true

# Comma-separated sentinel node URLs (at least 3 recommended)
REDIS__SENTINEL_NODES=redis://sentinel1:26379,redis://sentinel2:26379,redis://sentinel3:26379

# Master group name as configured in sentinel.conf
REDIS__SENTINEL_MASTER_NAME=mymaster

# Optional: separate sentinel authentication
REDIS__SENTINEL_PASSWORD=sentinel-secret
```

### Validation Rules

- `cluster_enabled` and `sentinel_enabled` cannot both be `true`
- `cluster_nodes` must be non-empty when `cluster_enabled=true`
- `sentinel_nodes` must be non-empty when `sentinel_enabled=true`
- `sentinel_master_name` must be non-empty when `sentinel_enabled=true`

## API Endpoints

### Cluster Info (admin-protected, gated by `capabilities.features.cluster`)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/cluster/info` | Cluster state, slots, epoch, known nodes count |
| GET | `/api/v1/cluster/nodes` | Full node list with roles, slots, flags |
| GET | `/api/v1/cluster/slots` | Slot-to-node mapping |
| GET | `/api/v1/cluster/shards` | Redis 7.0+ shard topology (replaces SLOTS) |
| GET | `/api/v1/cluster/keyslot/{key}` | Hash slot for a given key (0-16383) |

All cluster endpoints require the `X-Admin-Api-Key` header.

### Health Endpoint Changes

The `/health/ready` response includes a `mode` field indicating the connection topology:

```json
{
  "status": "ready",
  "mode": "sentinel",
  "redis": {
    "connected": true,
    "pool": {
      "size": 1,
      "available": 1,
      "max_size": 10,
      "total_connections_created": 0,
      "total_wait_count": 1,
      "avg_wait_ms": 0.0,
      "current_waiting": 0,
      "failed_checkouts": 0
    }
  },
  "capabilities": { ... }
}
```

The `mode` field is one of `"standalone"`, `"cluster"`, or `"sentinel"`.
The `redis` object always contains `connected` and `pool` stats — no topology-specific fields like `cluster_state` are included (use the dedicated `/api/v1/cluster/info` endpoint for that).

## Implementation Files

### Implemented (exists in repo)

```
src/infrastructure/config/settings.rs              -- cluster/sentinel config fields, validation
src/infrastructure/redis/cluster_connection.rs     -- ClusterPool (per-request connections, no mutex)
src/infrastructure/redis/sentinel_watcher.rs       -- background failover watcher (polls sentinel, swaps pool)
src/infrastructure/redis/connection.rs             -- InstrumentedPool with RwLock<PoolState> for atomic swap,
                                                      sentinel master resolution at startup
src/domain/repositories/cluster_repository.rs      -- trait
src/infrastructure/redis/repositories/cluster_repo.rs -- impl (uses standalone pool for CLUSTER commands)
src/application/services/cluster_service.rs        -- service
src/api/http/routes/cluster.rs                     -- 5 admin-protected endpoints, gated by capabilities
src/api/http/schemas/cluster.rs                    -- request/response types
src/shared/app_state.rs                            -- cluster_service, cluster_pool, resolved_url propagation
src/api/http/routes/mod.rs                         -- cluster routes registered
src/api/http/routes/health.rs                      -- reports connection mode (standalone/cluster/sentinel)
src/main.rs                                        -- startup mode branching, cluster pool creation,
                                                      sentinel watcher spawn
docker-compose.cluster-test.yml                    -- 3-node cluster test infrastructure
docker-compose.sentinel-test.yml                   -- master + replica + 3 sentinels
tests/fixtures/sentinel.conf                       -- sentinel config fixture
```

### E2E and CI

```
tests/e2e/cluster_test.sh                          -- cluster E2E test script (20 assertions)
tests/e2e/sentinel_test.sh                         -- sentinel E2E test script (11 assertions)
docker-compose.cluster-test.yml                    -- all-in-Docker cluster test (service + 3 nodes + test runner)
docker-compose.sentinel-test.yml                   -- all-in-Docker sentinel test (service + master + replica + 3 sentinels + test runner)
.github/workflows/ci.yml                           -- cluster-test and sentinel-test CI jobs
```

## Testing

### Unit Tests

**Config validation:**
- Reject `cluster_enabled=true` with empty `cluster_nodes`
- Reject `sentinel_enabled=true` with empty `sentinel_nodes`
- Reject both `cluster_enabled=true` and `sentinel_enabled=true`
- Accept valid cluster config
- Accept valid sentinel config
- Default config (both disabled) passes validation

**Cluster service:**
- Mock cluster repository, test `cluster_info()` returns parsed struct
- Test `cluster_keyslot()` returns u16 in range 0-16383
- Test route registration gating (cluster routes absent when `capabilities.features.cluster=false`)

**Schemas:**
- Test `ClusterInfoResponse` serialization
- Test `ClusterNodeResponse` parsing
- Test `KeySlotResponse` serialization

### Integration Tests

Integration tests require a multi-node Redis setup. These run in a dedicated CI job with a custom docker-compose file.

**docker-compose.cluster-test.yml:**

```yaml
services:
  redis-node-1:
    image: redis:8.0-M04
    command: >
      redis-server
      --cluster-enabled yes
      --cluster-config-file nodes.conf
      --cluster-node-timeout 5000
      --port 7001
    ports: ["7001:7001"]

  redis-node-2:
    image: redis:8.0-M04
    command: >
      redis-server
      --cluster-enabled yes
      --cluster-config-file nodes.conf
      --cluster-node-timeout 5000
      --port 7002
    ports: ["7002:7002"]

  redis-node-3:
    image: redis:8.0-M04
    command: >
      redis-server
      --cluster-enabled yes
      --cluster-config-file nodes.conf
      --cluster-node-timeout 5000
      --port 7003
    ports: ["7003:7003"]

  cluster-init:
    image: redis:8.0-M04
    depends_on: [redis-node-1, redis-node-2, redis-node-3]
    entrypoint: >
      sh -c "sleep 3 &&
      redis-cli --cluster create
        redis-node-1:7001 redis-node-2:7002 redis-node-3:7003
        --cluster-replicas 0 --cluster-yes"
```

**docker-compose.sentinel-test.yml:**

```yaml
services:
  redis-master:
    image: redis:8.0-M04
    command: redis-server --port 6380
    ports: ["6380:6380"]

  redis-replica:
    image: redis:8.0-M04
    command: redis-server --port 6381 --replicaof redis-master 6380
    ports: ["6381:6381"]
    depends_on: [redis-master]

  sentinel-1:
    image: redis:8.0-M04
    command: redis-sentinel /etc/sentinel.conf
    volumes: [./tests/fixtures/sentinel.conf:/etc/sentinel.conf]
    ports: ["26379:26379"]
    depends_on: [redis-master, redis-replica]

  sentinel-2:
    image: redis:8.0-M04
    command: redis-sentinel /etc/sentinel.conf
    volumes: [./tests/fixtures/sentinel.conf:/etc/sentinel.conf]
    ports: ["26380:26379"]
    depends_on: [redis-master, redis-replica]

  sentinel-3:
    image: redis:8.0-M04
    command: redis-sentinel /etc/sentinel.conf
    volumes: [./tests/fixtures/sentinel.conf:/etc/sentinel.conf]
    ports: ["26381:26379"]
    depends_on: [redis-master, redis-replica]
```

**tests/fixtures/sentinel.conf:**

```
port 26379
sentinel monitor mymaster redis-master 6380 2
sentinel down-after-milliseconds mymaster 5000
sentinel failover-timeout mymaster 10000
sentinel parallel-syncs mymaster 1
```

**Cluster integration test cases:**

```
tests/e2e/cluster_test.sh
```

1. `CLUSTER INFO` returns `cluster_state:ok` and `cluster_slots_assigned:16384`
2. `CLUSTER NODES` returns 3 nodes, all marked as master
3. `CLUSTER KEYSLOT test` returns a valid slot (0-16383)
4. `CLUSTER SHARDS` returns 3 shards (Redis 7.0+)
5. Regular SET/GET works through the cluster connection (key routing)
6. Health endpoint shows `mode: cluster` and `cluster_state: ok`
7. Auth required for all cluster endpoints (401 without key)

**Sentinel integration test cases:**

```
tests/e2e/sentinel_test.sh
```

1. Service connects through sentinel and health shows `mode: sentinel`
2. SET/GET works normally
3. Kill master container -> wait for failover (5-10s) -> verify service reconnects
4. Read data written before failover (confirms new master has the data)
5. Health endpoint transitions from not_ready back to ready after failover

### E2E Test Scripts

**tests/e2e/cluster_test.sh:**

```bash
#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${1:-http://localhost:8080}"
API_KEY="${2:-dev-admin-key}"
PASSED=0 FAILED=0

assert_status() {
  local name=$1 expected=$2 actual=$3
  if [ "$actual" = "$expected" ]; then
    echo "  PASS  $name (HTTP $actual)"
    PASSED=$((PASSED + 1))
  else
    echo "  FAIL  $name (expected $expected, got $actual)"
    FAILED=$((FAILED + 1))
  fi
}

echo "--- Cluster Info ---"
status=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/info")
assert_status "CLUSTER INFO" 200 "$status"

echo "--- Cluster Nodes ---"
status=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/nodes")
assert_status "CLUSTER NODES" 200 "$status"

echo "--- Cluster Slots ---"
status=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/slots")
assert_status "CLUSTER SLOTS" 200 "$status"

echo "--- Cluster Shards ---"
status=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/shards")
assert_status "CLUSTER SHARDS" 200 "$status"

echo "--- Cluster Keyslot ---"
resp=$(curl -s -H "X-Admin-Api-Key: $API_KEY" "$BASE_URL/api/v1/cluster/keyslot/test")
slot=$(echo "$resp" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['slot'])")
if [ "$slot" -ge 0 ] && [ "$slot" -le 16383 ]; then
  echo "  PASS  CLUSTER KEYSLOT (slot=$slot)"
  PASSED=$((PASSED + 1))
else
  echo "  FAIL  CLUSTER KEYSLOT (invalid slot=$slot)"
  FAILED=$((FAILED + 1))
fi

echo "--- Auth Required ---"
status=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/v1/cluster/info")
assert_status "No auth returns 401" 401 "$status"

echo "--- Data Operations Through Cluster ---"
curl -s -X PUT "$BASE_URL/api/v1/strings/cluster-test" \
  -H "Content-Type: application/json" -d '{"value":"cluster-works"}' > /dev/null
resp=$(curl -s "$BASE_URL/api/v1/strings/cluster-test")
val=$(echo "$resp" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['value'])")
if [ "$val" = "cluster-works" ]; then
  echo "  PASS  SET/GET through cluster"
  PASSED=$((PASSED + 1))
else
  echo "  FAIL  SET/GET through cluster (got $val)"
  FAILED=$((FAILED + 1))
fi

echo ""
echo "Results: $PASSED passed, $FAILED failed"
[ "$FAILED" -eq 0 ] || exit 1
```

**tests/e2e/sentinel_test.sh:**

```bash
#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${1:-http://localhost:8080}"
PASSED=0 FAILED=0

assert_status() {
  local name=$1 expected=$2 actual=$3
  if [ "$actual" = "$expected" ]; then
    echo "  PASS  $name (HTTP $actual)"
    PASSED=$((PASSED + 1))
  else
    echo "  FAIL  $name (expected $expected, got $actual)"
    FAILED=$((FAILED + 1))
  fi
}

echo "--- Health through Sentinel ---"
status=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/health/ready")
assert_status "Health ready" 200 "$status"

echo "--- Write data ---"
status=$(curl -s -o /dev/null -w "%{http_code}" -X PUT \
  "$BASE_URL/api/v1/strings/sentinel-test" \
  -H "Content-Type: application/json" -d '{"value":"before-failover"}')
assert_status "SET before failover" 200 "$status"

echo "--- Simulate failover (stop master) ---"
docker stop redis-master 2>/dev/null || true
echo "  Waiting 15s for sentinel failover..."
sleep 15

echo "--- Verify service recovered ---"
status=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/health/ready")
assert_status "Health after failover" 200 "$status"

echo "--- Read data after failover ---"
resp=$(curl -s "$BASE_URL/api/v1/strings/sentinel-test")
val=$(echo "$resp" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['value'])" 2>/dev/null || echo "FAILED")
if [ "$val" = "before-failover" ]; then
  echo "  PASS  GET after failover (data preserved)"
  PASSED=$((PASSED + 1))
else
  echo "  FAIL  GET after failover (got: $val)"
  FAILED=$((FAILED + 1))
fi

echo "--- Restart master ---"
docker start redis-master 2>/dev/null || true

echo ""
echo "Results: $PASSED passed, $FAILED failed"
[ "$FAILED" -eq 0 ] || exit 1
```

### CI Integration

Cluster and Sentinel tests run in a **separate CI job** from the main test suite because they need custom docker-compose setups:

```yaml
# In .github/workflows/ci.yml (new job)
cluster-test:
  name: Cluster & Sentinel Tests
  needs: test
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Start cluster
      run: docker compose -f docker-compose.cluster-test.yml up -d --wait
    - name: Build and start service
      run: |
        cargo build --release --locked
        REDIS__CLUSTER_ENABLED=true \
        REDIS__CLUSTER_NODES=redis://localhost:7001,redis://localhost:7002,redis://localhost:7003 \
        ./target/release/redis-caching-service &
        sleep 3
    - name: Run cluster E2E tests
      run: bash tests/e2e/cluster_test.sh
    - name: Teardown cluster
      run: docker compose -f docker-compose.cluster-test.yml down -v

sentinel-test:
  name: Sentinel Failover Tests
  needs: test
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Start sentinel setup
      run: docker compose -f docker-compose.sentinel-test.yml up -d --wait
    - name: Build and start service
      run: |
        cargo build --release --locked
        REDIS__SENTINEL_ENABLED=true \
        REDIS__SENTINEL_NODES=redis://localhost:26379,redis://localhost:26380,redis://localhost:26381 \
        REDIS__SENTINEL_MASTER_NAME=mymaster \
        ./target/release/redis-caching-service &
        sleep 3
    - name: Run sentinel E2E tests
      run: bash tests/e2e/sentinel_test.sh
    - name: Teardown sentinel
      run: docker compose -f docker-compose.sentinel-test.yml down -v
```

## Risks and Considerations

### Cluster limitations

- **Multi-key operations across slots**: Commands like MSET/MGET, SUNION, etc. will fail if keys hash to different slots. The service should return a clear error ("CROSSSLOT keys in request don't hash to the same slot") rather than an opaque Redis error.
- **Lua scripts**: EVAL requires all keys to be in the same slot. The existing scripting endpoint already accepts a `keys` array, so the cluster client can route correctly.
- **Transactions**: MULTI/EXEC only works on a single node. The existing transaction endpoint needs a note that all keys must be in the same hash slot when in cluster mode.
- **Pub/Sub**: Regular pub/sub broadcasts to all nodes. Sharded pub/sub (SPUBLISH/SSUBSCRIBE) routes by channel hash slot -- already stubbed in the codebase.

### Sentinel limitations

- **Write availability during failover**: There is a window (typically 5-15 seconds) where writes will fail. The service will return 503 from health/ready during this period.
- **Split-brain**: If sentinels disagree, two masters can exist briefly. The `redis` crate handles this by reconnecting to the consensus master.

### Performance implications

- **Cluster**: Each command has ~1 extra RTT for slot lookup on first access (cached afterward). Cross-slot operations are rejected, not silently degraded.
- **Sentinel**: No performance impact during normal operation. During failover, there's a brief connection storm as the pool reconnects.
