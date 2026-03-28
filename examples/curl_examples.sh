#!/usr/bin/env bash
#
# Redis Caching Service - curl Examples
#
# Usage:
#   chmod +x curl_examples.sh
#   ./curl_examples.sh
#
# Prerequisites:
#   - The service must be running (docker-compose up -d)
#   - curl and jq must be installed

set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
ADMIN_KEY="${ADMIN_KEY:-dev-admin-key}"

header() {
  echo ""
  echo "================================================================"
  echo "  $1"
  echo "================================================================"
}

run() {
  echo ""
  echo "→ $1"
  shift
  "$@" | jq . 2>/dev/null || true
}

# ===========================================================================
# Health
# ===========================================================================

header "Health Checks"

run "GET /health" \
  curl -s "$BASE_URL/health"

run "GET /health/ready" \
  curl -s "$BASE_URL/health/ready"

# ===========================================================================
# Strings
# ===========================================================================

header "String Operations"

run "SET greeting" \
  curl -s -X PUT "$BASE_URL/api/v1/strings/greeting" \
    -H "Content-Type: application/json" \
    -d '{"value": "Hello, World!", "ttl_seconds": 3600}'

run "GET greeting" \
  curl -s "$BASE_URL/api/v1/strings/greeting"

run "MSET multiple keys" \
  curl -s -X POST "$BASE_URL/api/v1/strings/mset" \
    -H "Content-Type: application/json" \
    -d '{"pairs": {"key1": "value1", "key2": "value2", "counter": "0"}}'

run "MGET multiple keys" \
  curl -s -X POST "$BASE_URL/api/v1/strings/mget" \
    -H "Content-Type: application/json" \
    -d '{"keys": ["key1", "key2", "counter", "missing"]}'

run "INCR counter by 5" \
  curl -s -X PATCH "$BASE_URL/api/v1/strings/counter/incr" \
    -H "Content-Type: application/json" \
    -d '{"delta": 5}'

run "APPEND to greeting" \
  curl -s -X PATCH "$BASE_URL/api/v1/strings/greeting/append" \
    -H "Content-Type: application/json" \
    -d '{"value": " How are you?"}'

run "STRLEN greeting" \
  curl -s "$BASE_URL/api/v1/strings/greeting/length"

run "GETRANGE greeting (0..4)" \
  curl -s "$BASE_URL/api/v1/strings/greeting/range?start=0&end=4"

# ===========================================================================
# Hashes
# ===========================================================================

header "Hash Operations"

run "HSET user:1" \
  curl -s -X PUT "$BASE_URL/api/v1/hashes/user:1" \
    -H "Content-Type: application/json" \
    -d '{"items": {"name": "Alice", "email": "alice@example.com", "age": "30"}}'

run "HGETALL user:1" \
  curl -s "$BASE_URL/api/v1/hashes/user:1"

run "HGET user:1 name" \
  curl -s "$BASE_URL/api/v1/hashes/user:1/fields/name"

run "HMGET user:1 [name, email]" \
  curl -s -X POST "$BASE_URL/api/v1/hashes/user:1/fields/get" \
    -H "Content-Type: application/json" \
    -d '{"fields": ["name", "email"]}'

run "HINCRBY user:1 age +1" \
  curl -s -X PATCH "$BASE_URL/api/v1/hashes/user:1/fields/age/incr" \
    -H "Content-Type: application/json" \
    -d '{"delta": 1}'

run "HKEYS user:1" \
  curl -s "$BASE_URL/api/v1/hashes/user:1/keys"

run "HEXISTS user:1 email" \
  curl -s "$BASE_URL/api/v1/hashes/user:1/fields/email/exists"

# ===========================================================================
# Lists
# ===========================================================================

header "List Operations"

run "RPUSH tasks" \
  curl -s -X POST "$BASE_URL/api/v1/lists/tasks/rpush" \
    -H "Content-Type: application/json" \
    -d '{"values": ["task-a", "task-b", "task-c"]}'

run "LRANGE tasks 0..-1" \
  curl -s "$BASE_URL/api/v1/lists/tasks/range?start=0&stop=-1"

run "LLEN tasks" \
  curl -s "$BASE_URL/api/v1/lists/tasks/length"

run "LPOP tasks" \
  curl -s -X POST "$BASE_URL/api/v1/lists/tasks/lpop" \
    -H "Content-Type: application/json" \
    -d '{"count": 1}'

run "LINSERT tasks BEFORE task-c" \
  curl -s -X POST "$BASE_URL/api/v1/lists/tasks/insert" \
    -H "Content-Type: application/json" \
    -d '{"pivot": "task-c", "value": "task-b2", "position": "before"}'

# ===========================================================================
# Sets
# ===========================================================================

header "Set Operations"

run "SADD tags" \
  curl -s -X POST "$BASE_URL/api/v1/sets/tags/members" \
    -H "Content-Type: application/json" \
    -d '{"members": ["rust", "redis", "docker"]}'

run "SMEMBERS tags" \
  curl -s "$BASE_URL/api/v1/sets/tags/members"

run "SCARD tags" \
  curl -s "$BASE_URL/api/v1/sets/tags/card"

run "SISMEMBER tags rust" \
  curl -s -X POST "$BASE_URL/api/v1/sets/tags/ismember" \
    -H "Content-Type: application/json" \
    -d '{"member": "rust"}'

# ===========================================================================
# Sorted Sets
# ===========================================================================

header "Sorted Set Operations"

run "ZADD leaderboard" \
  curl -s -X POST "$BASE_URL/api/v1/sorted-sets/leaderboard/members" \
    -H "Content-Type: application/json" \
    -d '{"members": [{"member": "alice", "score": 100}, {"member": "bob", "score": 85}, {"member": "charlie", "score": 92}]}'

run "ZRANGE leaderboard (rev, with scores)" \
  curl -s "$BASE_URL/api/v1/sorted-sets/leaderboard/range?start=0&stop=-1&rev=true&with_scores=true"

run "ZSCORE leaderboard alice" \
  curl -s "$BASE_URL/api/v1/sorted-sets/leaderboard/score/alice"

run "ZRANK leaderboard bob" \
  curl -s "$BASE_URL/api/v1/sorted-sets/leaderboard/rank/bob"

run "ZINCRBY leaderboard bob +20" \
  curl -s -X POST "$BASE_URL/api/v1/sorted-sets/leaderboard/incrby" \
    -H "Content-Type: application/json" \
    -d '{"member": "bob", "increment": 20}'

# ===========================================================================
# Bitmaps
# ===========================================================================

header "Bitmap Operations"

run "SETBIT online:2024-01-15 user=42 -> 1" \
  curl -s -X PUT "$BASE_URL/api/v1/bitmaps/online:2024-01-15/bit/42" \
    -H "Content-Type: application/json" \
    -d '{"value": true}'

run "GETBIT online:2024-01-15 user=42" \
  curl -s "$BASE_URL/api/v1/bitmaps/online:2024-01-15/bit/42"

run "BITCOUNT online:2024-01-15" \
  curl -s "$BASE_URL/api/v1/bitmaps/online:2024-01-15/count"

# ===========================================================================
# Key Operations
# ===========================================================================

header "Key Operations"

run "EXISTS [greeting, user:1, nonexistent]" \
  curl -s -X POST "$BASE_URL/api/v1/keys/exists" \
    -H "Content-Type: application/json" \
    -d '{"keys": ["greeting", "user:1", "nonexistent"]}'

run "SCAN pattern=*" \
  curl -s "$BASE_URL/api/v1/keys/scan?pattern=*&count=20"

run "TTL greeting" \
  curl -s "$BASE_URL/api/v1/keys/greeting/ttl"

run "TYPE greeting" \
  curl -s "$BASE_URL/api/v1/keys/greeting/type"

run "EXPIRE greeting 120s" \
  curl -s -X PATCH "$BASE_URL/api/v1/keys/greeting/expire" \
    -H "Content-Type: application/json" \
    -d '{"seconds": 120}'

# ===========================================================================
# Transactions
# ===========================================================================

header "Transactions"

curl -s -X PUT "$BASE_URL/api/v1/strings/tx-counter" \
  -H "Content-Type: application/json" \
  -d '{"value": "10"}' > /dev/null

run "MULTI/EXEC (GET + SET)" \
  curl -s -X POST "$BASE_URL/api/v1/transactions/execute" \
    -H "Content-Type: application/json" \
    -d '{
      "commands": [
        {"type": "GET", "key": "tx-counter"},
        {"type": "SET", "key": "tx-counter", "value": "20"}
      ]
    }'

run "Compare-and-Set (CAS)" \
  curl -s -X POST "$BASE_URL/api/v1/transactions/cas" \
    -H "Content-Type: application/json" \
    -d '{"key": "tx-counter", "expected_value": "20", "new_value": "30"}'

# ===========================================================================
# Pub/Sub (publish)
# ===========================================================================

header "Pub/Sub"

run "PUBLISH to notifications channel" \
  curl -s -X POST "$BASE_URL/api/v1/pubsub/publish" \
    -H "Content-Type: application/json" \
    -d '{"channel": "notifications", "message": "Hello from curl!"}'

run "PUBSUB CHANNELS" \
  curl -s "$BASE_URL/api/v1/pubsub/channels"

run "PUBSUB STATS" \
  curl -s "$BASE_URL/api/v1/pubsub/stats"

echo ""
echo "  TIP: To subscribe via WebSocket, use wscat:"
echo "    wscat -c 'ws://localhost:8080/api/v1/pubsub/subscribe?channels=notifications'"

# ===========================================================================
# Scripting
# ===========================================================================

header "Lua Scripting"

run "EVAL (return argument)" \
  curl -s -X POST "$BASE_URL/api/v1/scripts/eval" \
    -H "Content-Type: application/json" \
    -d '{"script": "return ARGV[1]", "keys": [], "args": ["hello from lua"]}'

run "SCRIPT LOAD" \
  curl -s -X POST "$BASE_URL/api/v1/scripts/load" \
    -H "Content-Type: application/json" \
    -d '{"script": "return redis.call(\"GET\", KEYS[1])"}'

# ===========================================================================
# Admin Operations
# ===========================================================================

header "Admin Operations"

run "Pool Stats (public)" \
  curl -s "$BASE_URL/api/v1/admin/pool/stats"

run "Capabilities (public)" \
  curl -s "$BASE_URL/api/v1/admin/capabilities"

run "Server Info (memory section)" \
  curl -s "$BASE_URL/api/v1/admin/server/info?section=memory" \
    -H "X-Admin-Api-Key: $ADMIN_KEY"

run "DB Size" \
  curl -s "$BASE_URL/api/v1/admin/server/dbsize" \
    -H "X-Admin-Api-Key: $ADMIN_KEY"

run "Server Time" \
  curl -s "$BASE_URL/api/v1/admin/server/time" \
    -H "X-Admin-Api-Key: $ADMIN_KEY"

run "Slowlog (last 5)" \
  curl -s -X POST "$BASE_URL/api/v1/admin/slowlog/get" \
    -H "X-Admin-Api-Key: $ADMIN_KEY" \
    -H "Content-Type: application/json" \
    -d '{"count": 5}'

# ===========================================================================
# SSE Streaming Example
# ===========================================================================

header "SSE Streaming"

echo ""
echo "  To subscribe to a stream via SSE (entries arrive as event: message):"
echo "    curl -N '$BASE_URL/api/v1/streams/mystream/subscribe?last_id=0'"
echo ""
echo "  To subscribe to repeated BLPOP via SSE:"
echo "    curl -N '$BASE_URL/api/v1/lists/mylist/blpop/stream'"

# ===========================================================================
# Cleanup
# ===========================================================================

header "Cleanup"

run "Delete all demo keys" \
  curl -s -X POST "$BASE_URL/api/v1/keys/delete" \
    -H "Content-Type: application/json" \
    -d '{"keys": ["greeting", "key1", "key2", "counter", "user:1", "tasks", "tags", "leaderboard", "online:2024-01-15", "tx-counter"]}'

echo ""
echo "All examples completed!"
