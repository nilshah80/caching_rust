# Redis Caching Service

A high-performance Redis caching service built in Rust, providing a clean REST API for Redis operations.

## Features

- **String Operations**: GET, SET, MGET, MSET, INCR, DECR, APPEND, STRLEN, GETRANGE, SETRANGE, GETEX, GETDEL
- **Hash Operations**: HGET, HSET, HSETNX, HGETALL, HMGET, HDEL, HEXISTS, HKEYS, HVALS, HLEN, HINCRBY, HINCRBYFLOAT, HSTRLEN, HRANDFIELD, HSCAN
- **List Operations**: LPUSH, RPUSH, LPOP, RPOP, LRANGE, LLEN, LINDEX, LSET, LINSERT, LREM, LTRIM
- **Set Operations**: SADD, SREM, SMEMBERS, SISMEMBER, SCARD, SDIFF, SINTER, SUNION, SPOP, SRANDMEMBER
- **Sorted Set Operations**: ZADD, ZRANGE, ZREM, ZSCORE, ZRANK, ZCOUNT, ZCARD, ZINCRBY, ZRANGEBYSCORE
- **Stream Operations**: XADD, XLEN, XREAD, XRANGE, XREVRANGE, XDEL, XTRIM, XINFO + Consumer Groups (XGROUP, XREADGROUP, XACK, XCLAIM, XAUTOCLAIM, XPENDING)
- **Key Operations**: DEL, EXISTS, EXPIRE, TTL, PTTL, PERSIST, TYPE, RENAME, COPY, SCAN, KEYS, RANDOMKEY, TOUCH, UNLINK, DUMP, RESTORE, OBJECT
- **Admin Operations**: Server info, memory stats, client management, slowlog, latency monitoring, ACL
- **Real-time Streaming**: Server-Sent Events (SSE) for blocking stream reads
- **Connection Pooling**: Instrumented connection pool with metrics
- **OpenAPI Documentation**: Swagger UI at `/swagger-ui`
- **Health Checks**: Liveness and readiness endpoints
- **API Key Authentication**: Protected admin endpoints

## Architecture

The service follows a clean layered architecture:

```
src/
├── api/http/           # HTTP layer (routes, middleware, schemas)
├── application/        # Application services (business logic)
├── domain/             # Domain entities, errors, repository traits
├── infrastructure/     # Redis implementation, config, logging
└── shared/             # Shared utilities (app state, response types)
```

### Key Design Decisions

1. **Repository Pattern**: Domain layer defines traits, infrastructure implements them
2. **Service Layer**: Business logic and validation in application services
3. **Dependency Injection**: Services accept repository traits for testability
4. **Instrumented Pool**: Custom wrapper for connection pool metrics

## Quick Start

### Prerequisites

- Docker and Docker Compose
- Rust 1.75+ (for local development)

### Running with Docker

```bash
# Start the service with Redis
docker-compose up -d

# Check health
curl http://localhost:8080/health

# View logs
docker-compose logs -f caching-service
```

### Running Locally

```bash
# Start Redis
docker-compose up -d redis

# Run the service
cargo run

# Or with release optimizations
cargo run --release
```

## API Endpoints

### Health

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/health/ready` | Readiness probe |
| GET | `/health/live` | Liveness probe |

### String Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/strings/{key}` | Get string value |
| POST | `/api/v1/strings/{key}` | Set string value |
| DELETE | `/api/v1/strings/{key}` | Get and delete (GETDEL) |
| POST | `/api/v1/strings/mget` | Get multiple keys |
| POST | `/api/v1/strings/mset` | Set multiple keys |
| PATCH | `/api/v1/strings/{key}/incr` | Increment value |
| PATCH | `/api/v1/strings/{key}/decr` | Decrement value |
| PATCH | `/api/v1/strings/{key}/append` | Append to value |
| GET | `/api/v1/strings/{key}/length` | Get string length |
| GET | `/api/v1/strings/{key}/range` | Get substring |
| PATCH | `/api/v1/strings/{key}/range` | Set substring |
| GET | `/api/v1/strings/{key}/getex` | Get with TTL update |

### Key Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/keys/delete` | Delete multiple keys (DEL/UNLINK) |
| POST | `/api/v1/keys/exists` | Check if keys exist (EXISTS) |
| POST | `/api/v1/keys/touch` | Update access time (TOUCH) |
| GET | `/api/v1/keys/scan` | Scan keys with pattern (SCAN) |
| GET | `/api/v1/keys` | List keys matching pattern (KEYS) |
| GET | `/api/v1/keys/random` | Get random key (RANDOMKEY) |
| GET | `/api/v1/keys/{key}` | Get key info (TYPE, TTL, OBJECT) |
| DELETE | `/api/v1/keys/{key}` | Delete single key (UNLINK) |
| GET | `/api/v1/keys/{key}/ttl` | Get TTL in seconds/milliseconds |
| PATCH | `/api/v1/keys/{key}/expire` | Set expiration (EXPIRE/PEXPIRE) |
| PATCH | `/api/v1/keys/{key}/persist` | Remove expiration (PERSIST) |
| GET | `/api/v1/keys/{key}/type` | Get key type (TYPE) |
| PATCH | `/api/v1/keys/{key}/rename` | Rename key (RENAME/RENAMENX) |
| POST | `/api/v1/keys/{key}/copy` | Copy key (COPY) |
| GET | `/api/v1/keys/{key}/dump` | Serialize key (DUMP) |
| POST | `/api/v1/keys/{key}/restore` | Deserialize key (RESTORE) |
| GET | `/api/v1/keys/{key}/object` | Object encoding/refcount/idletime |

### Hash Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/hashes/{key}` | Get all fields (HGETALL) |
| GET | `/api/v1/hashes/{key}/{field}` | Get field value (HGET) |
| POST | `/api/v1/hashes/{key}` | Set fields (HSET/HSETNX) |
| DELETE | `/api/v1/hashes/{key}` | Delete fields (HDEL) |
| POST | `/api/v1/hashes/{key}/mget` | Get multiple fields (HMGET) |
| GET | `/api/v1/hashes/{key}/exists/{field}` | Check field exists (HEXISTS) |
| GET | `/api/v1/hashes/{key}/keys` | Get field names (HKEYS) |
| GET | `/api/v1/hashes/{key}/vals` | Get field values (HVALS) |
| GET | `/api/v1/hashes/{key}/len` | Get field count (HLEN) |
| POST | `/api/v1/hashes/{key}/incrby` | Increment field (HINCRBY) |
| POST | `/api/v1/hashes/{key}/incrbyfloat` | Increment float (HINCRBYFLOAT) |
| GET | `/api/v1/hashes/{key}/strlen/{field}` | Get field string length (HSTRLEN) |
| GET | `/api/v1/hashes/{key}/randfield` | Get random field (HRANDFIELD) |
| GET | `/api/v1/hashes/{key}/scan` | Scan fields (HSCAN) |

### List Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/lists/{key}/push` | Push elements (LPUSH/RPUSH) |
| POST | `/api/v1/lists/{key}/pop` | Pop elements (LPOP/RPOP) |
| GET | `/api/v1/lists/{key}/range` | Get range (LRANGE) |
| GET | `/api/v1/lists/{key}/len` | Get length (LLEN) |
| GET | `/api/v1/lists/{key}/index/{index}` | Get by index (LINDEX) |
| PUT | `/api/v1/lists/{key}/index/{index}` | Set by index (LSET) |
| POST | `/api/v1/lists/{key}/insert` | Insert element (LINSERT) |
| POST | `/api/v1/lists/{key}/rem` | Remove elements (LREM) |
| POST | `/api/v1/lists/{key}/trim` | Trim list (LTRIM) |

### Set Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/sets/{key}/add` | Add members (SADD) |
| POST | `/api/v1/sets/{key}/rem` | Remove members (SREM) |
| GET | `/api/v1/sets/{key}/members` | Get all members (SMEMBERS) |
| POST | `/api/v1/sets/{key}/ismember` | Check membership (SISMEMBER) |
| GET | `/api/v1/sets/{key}/card` | Get cardinality (SCARD) |
| POST | `/api/v1/sets/diff` | Difference (SDIFF) |
| POST | `/api/v1/sets/inter` | Intersection (SINTER) |
| POST | `/api/v1/sets/union` | Union (SUNION) |
| POST | `/api/v1/sets/{key}/pop` | Pop random members (SPOP) |
| GET | `/api/v1/sets/{key}/randmember` | Random members (SRANDMEMBER) |

### Sorted Set Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/sortedsets/{key}/add` | Add members with scores (ZADD) |
| GET | `/api/v1/sortedsets/{key}/range` | Get range (ZRANGE) |
| POST | `/api/v1/sortedsets/{key}/rem` | Remove members (ZREM) |
| GET | `/api/v1/sortedsets/{key}/score/{member}` | Get score (ZSCORE) |
| GET | `/api/v1/sortedsets/{key}/rank/{member}` | Get rank (ZRANK) |
| GET | `/api/v1/sortedsets/{key}/count` | Count in score range (ZCOUNT) |
| GET | `/api/v1/sortedsets/{key}/card` | Get cardinality (ZCARD) |
| POST | `/api/v1/sortedsets/{key}/incrby` | Increment score (ZINCRBY) |
| GET | `/api/v1/sortedsets/{key}/rangebyscore` | Get by score range (ZRANGEBYSCORE) |

### Stream Operations

Streams provide append-only log data structures. See [Stream Operations](#stream-operations-1) for detailed documentation.

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/streams/{key}` | Add entry (XADD) |
| GET | `/api/v1/streams/{key}/length` | Get length (XLEN) |
| POST | `/api/v1/streams/read` | Read entries (XREAD) |
| GET | `/api/v1/streams/{key}/range` | Get range (XRANGE/XREVRANGE) |
| POST | `/api/v1/streams/{key}/delete` | Delete entries (XDEL) |
| POST | `/api/v1/streams/{key}/trim` | Trim stream (XTRIM) |
| GET | `/api/v1/streams/{key}/info` | Stream info (XINFO STREAM) |
| GET | `/api/v1/streams/subscribe` | **SSE** Subscribe to streams |
| POST | `/api/v1/streams/{key}/groups` | Create group (XGROUP CREATE) |
| DELETE | `/api/v1/streams/{key}/groups/{group}` | Delete group (XGROUP DESTROY) |
| POST | `/api/v1/streams/{key}/groups/{group}/read` | Read group (XREADGROUP) |
| POST | `/api/v1/streams/{key}/groups/{group}/ack` | Acknowledge (XACK) |
| GET | `/api/v1/streams/{key}/groups/{group}/pending` | Pending entries (XPENDING) |
| POST | `/api/v1/streams/{key}/groups/{group}/claim` | Claim entries (XCLAIM) |
| POST | `/api/v1/streams/{key}/groups/{group}/autoclaim` | Auto-claim (XAUTOCLAIM) |
| GET | `/api/v1/streams/{key}/groups/{group}/subscribe` | **SSE** Subscribe with group |
| GET | `/api/v1/streams/{key}/groups` | List groups (XINFO GROUPS) |
| GET | `/api/v1/streams/{key}/groups/{group}/consumers` | List consumers (XINFO CONSUMERS) |

### Admin Operations (API Key Required)

Admin endpoints require the `X-Admin-Api-Key` header for authentication.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/admin/pool/stats` | Connection pool stats (public) |
| GET | `/api/v1/admin/capabilities` | Redis capabilities (public) |
| GET | `/api/v1/admin/server/info` | Server information |
| GET | `/api/v1/admin/server/time` | Server time |
| GET | `/api/v1/admin/server/dbsize` | Database size |
| GET | `/api/v1/admin/server/lastsave` | Last save timestamp |
| GET | `/api/v1/admin/server/memory/stats` | Memory statistics |
| POST | `/api/v1/admin/server/memory/usage` | Memory usage for key |
| GET | `/api/v1/admin/server/memory/doctor` | Memory doctor report |
| POST | `/api/v1/admin/server/memory/purge` | Purge memory |
| DELETE | `/api/v1/admin/db/flush` | Flush current database |
| DELETE | `/api/v1/admin/db/flushall` | Flush all databases |
| POST | `/api/v1/admin/db/copy` | Copy key |
| POST | `/api/v1/admin/db/move` | Move key to another DB |
| POST | `/api/v1/admin/db/swapdb` | Swap databases |
| POST | `/api/v1/admin/config/get` | Get configuration |
| POST | `/api/v1/admin/config/set` | Set configuration |
| POST | `/api/v1/admin/config/rewrite` | Rewrite config file |
| POST | `/api/v1/admin/config/resetstat` | Reset statistics |
| POST | `/api/v1/admin/persistence/save` | Synchronous save |
| POST | `/api/v1/admin/persistence/bgsave` | Background save |
| POST | `/api/v1/admin/persistence/bgrewriteaof` | Rewrite AOF |
| GET | `/api/v1/admin/client/list` | List clients |
| POST | `/api/v1/admin/client/kill` | Kill client |
| POST | `/api/v1/admin/client/pause` | Pause clients |
| POST | `/api/v1/admin/client/unpause` | Unpause clients |
| POST | `/api/v1/admin/client/setname` | Set client name |
| GET | `/api/v1/admin/client/getname` | Get client name |
| GET | `/api/v1/admin/client/id` | Get client ID |
| POST | `/api/v1/admin/slowlog/get` | Get slowlog entries |
| GET | `/api/v1/admin/slowlog/len` | Slowlog length |
| POST | `/api/v1/admin/slowlog/reset` | Reset slowlog |
| GET | `/api/v1/admin/latency/latest` | Latest latency events |
| POST | `/api/v1/admin/latency/history` | Latency history |
| GET | `/api/v1/admin/latency/doctor` | Latency doctor |
| POST | `/api/v1/admin/latency/reset` | Reset latency data |
| GET | `/api/v1/admin/acl/list` | List ACL rules |
| GET | `/api/v1/admin/acl/users` | List ACL users |
| GET | `/api/v1/admin/acl/whoami` | Current user |
| POST | `/api/v1/admin/acl/cat` | ACL categories |
| POST | `/api/v1/admin/acl/genpass` | Generate password |
| POST | `/api/v1/admin/acl/log` | ACL log |

## Configuration

Configuration is loaded from environment variables:

### Server

| Variable | Default | Description |
|----------|---------|-------------|
| `SERVER__HOST` | `0.0.0.0` | Bind address |
| `SERVER__PORT` | `8080` | Bind port |

### Redis

| Variable | Default | Description |
|----------|---------|-------------|
| `REDIS__URL` | `redis://localhost:6379` | Redis URL |
| `REDIS__PASSWORD` | - | Redis password |
| `REDIS__DATABASE` | `0` | Database number |
| `REDIS__TLS_ENABLED` | `false` | Enable TLS |
| `REDIS__TLS_INSECURE` | `false` | Skip TLS verification |

### Connection Pool

| Variable | Default | Description |
|----------|---------|-------------|
| `POOL__MIN_SIZE` | `2` | Minimum connections |
| `POOL__MAX_SIZE` | `10` | Maximum connections |
| `POOL__TIMEOUT_MS` | `5000` | Connection timeout |

### Admin

| Variable | Default | Description |
|----------|---------|-------------|
| `ADMIN__API_KEY` | `changeme` | Admin API key |

## Examples

### Set a String Value

```bash
curl -X POST http://localhost:8080/api/v1/strings/mykey \
  -H "Content-Type: application/json" \
  -d '{"value": "hello world", "ttl": 3600}'
```

### Get a String Value

```bash
curl http://localhost:8080/api/v1/strings/mykey
```

### Get Server Info (Admin)

```bash
curl -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/server/info
```

### Multiple Get

```bash
curl -X POST http://localhost:8080/api/v1/strings/mget \
  -H "Content-Type: application/json" \
  -d '{"keys": ["key1", "key2", "key3"]}'
```

### Check Key Existence

```bash
curl -X POST http://localhost:8080/api/v1/keys/exists \
  -H "Content-Type: application/json" \
  -d '{"keys": ["key1", "key2", "nonexistent"]}'
```

### Scan Keys with Pattern

```bash
curl "http://localhost:8080/api/v1/keys/scan?pattern=key*&count=10"
```

### Get Key Info

```bash
curl http://localhost:8080/api/v1/keys/mykey
```

### Set Key Expiration

```bash
curl -X PATCH http://localhost:8080/api/v1/keys/mykey/expire \
  -H "Content-Type: application/json" \
  -d '{"seconds": 3600}'
```

### Rename Key

```bash
curl -X PATCH http://localhost:8080/api/v1/keys/oldkey/rename \
  -H "Content-Type: application/json" \
  -d '{"new_key": "newkey", "nx": false}'
```

### Copy Key

```bash
curl -X POST http://localhost:8080/api/v1/keys/sourcekey/copy \
  -H "Content-Type: application/json" \
  -d '{"destination": "destkey", "replace": false}'
```

### Delete Multiple Keys

```bash
curl -X POST http://localhost:8080/api/v1/keys/delete \
  -H "Content-Type: application/json" \
  -d '{"keys": ["key1", "key2"], "async_delete": false}'
```

## Development

### Build

```bash
cargo build
cargo build --release
```

### Test

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Lint

```bash
cargo clippy
cargo fmt --check
```

### Documentation

```bash
# Generate and open docs
cargo doc --open
```

## API Documentation

Swagger UI is available at `/swagger-ui` when the service is running.

OpenAPI spec is available at `/api-docs/openapi.json`.

---

## Stream Operations

Redis Streams are append-only log data structures that provide powerful message queue functionality. This service exposes a complete REST API for stream operations, including consumer groups for distributed processing.

### Basic Stream Operations

#### Add Entry to Stream (XADD)

```bash
curl -X POST http://localhost:8080/api/v1/streams/mystream \
  -H "Content-Type: application/json" \
  -d '{
    "fields": {"user": "alice", "action": "login", "ip": "192.168.1.1"}
  }'
```

With trimming options:
```bash
curl -X POST http://localhost:8080/api/v1/streams/mystream \
  -H "Content-Type: application/json" \
  -d '{
    "fields": {"event": "click"},
    "maxlen": 1000,
    "approximate": true
  }'
```

#### Get Stream Length (XLEN)

```bash
curl http://localhost:8080/api/v1/streams/mystream/length
```

#### Read Entries (XREAD)

Read from one or more streams:
```bash
curl -X POST http://localhost:8080/api/v1/streams/read \
  -H "Content-Type: application/json" \
  -d '{
    "streams": {"mystream": "0"},
    "count": 10
  }'
```

Read new entries only (from last ID):
```bash
curl -X POST http://localhost:8080/api/v1/streams/read \
  -H "Content-Type: application/json" \
  -d '{
    "streams": {"mystream": "$"},
    "block_ms": 5000
  }'
```

#### Get Range (XRANGE/XREVRANGE)

```bash
# Forward range (oldest to newest)
curl "http://localhost:8080/api/v1/streams/mystream/range?start=-&end=+"

# Reverse range (newest to oldest)
curl "http://localhost:8080/api/v1/streams/mystream/range?start=+&end=-&reverse=true"

# With count limit
curl "http://localhost:8080/api/v1/streams/mystream/range?start=-&end=+&count=10"
```

#### Delete Entries (XDEL)

```bash
curl -X POST http://localhost:8080/api/v1/streams/mystream/delete \
  -H "Content-Type: application/json" \
  -d '{"ids": ["1704000001234-0", "1704000001234-1"]}'
```

#### Trim Stream (XTRIM)

```bash
# Trim by MAXLEN
curl -X POST http://localhost:8080/api/v1/streams/mystream/trim \
  -H "Content-Type: application/json" \
  -d '{"strategy": "maxlen", "threshold": 1000, "approximate": true}'

# Trim by MINID
curl -X POST http://localhost:8080/api/v1/streams/mystream/trim \
  -H "Content-Type: application/json" \
  -d '{"strategy": "minid", "threshold": "1704000000000-0"}'
```

#### Stream Info (XINFO STREAM)

```bash
curl "http://localhost:8080/api/v1/streams/mystream/info"

# With full details
curl "http://localhost:8080/api/v1/streams/mystream/info?full=true&count=10"
```

### Consumer Groups

Consumer groups enable distributed processing where multiple consumers can read from a stream, with each message delivered to only one consumer in the group.

#### Create Consumer Group

```bash
curl -X POST http://localhost:8080/api/v1/streams/mystream/groups \
  -H "Content-Type: application/json" \
  -d '{
    "group": "mygroup",
    "id": "$",
    "mkstream": true
  }'
```

- `id`: Start position. Use `$` for new messages only, `0` for all existing messages
- `mkstream`: Create stream if it doesn't exist

#### Read with Consumer Group (XREADGROUP)

```bash
curl -X POST http://localhost:8080/api/v1/streams/mystream/groups/mygroup/read \
  -H "Content-Type: application/json" \
  -d '{
    "consumer": "worker-1",
    "id": ">",
    "count": 10
  }'
```

- `id`: Use `>` for new messages, or specific ID to re-read pending messages

#### Acknowledge Messages (XACK)

```bash
curl -X POST http://localhost:8080/api/v1/streams/mystream/groups/mygroup/ack \
  -H "Content-Type: application/json" \
  -d '{"ids": ["1704000001234-0", "1704000001234-1"]}'
```

#### View Pending Messages (XPENDING)

```bash
# Summary
curl "http://localhost:8080/api/v1/streams/mystream/groups/mygroup/pending"

# Detailed list
curl "http://localhost:8080/api/v1/streams/mystream/groups/mygroup/pending?start=-&end=+&count=10"

# Filter by consumer
curl "http://localhost:8080/api/v1/streams/mystream/groups/mygroup/pending?consumer=worker-1"
```

#### Claim Pending Messages (XCLAIM)

Transfer ownership of pending messages to another consumer:

```bash
curl -X POST http://localhost:8080/api/v1/streams/mystream/groups/mygroup/claim \
  -H "Content-Type: application/json" \
  -d '{
    "consumer": "worker-2",
    "min_idle_time_ms": 60000,
    "ids": ["1704000001234-0"]
  }'
```

#### Auto-Claim (XAUTOCLAIM)

Automatically claim idle pending messages:

```bash
curl -X POST http://localhost:8080/api/v1/streams/mystream/groups/mygroup/autoclaim \
  -H "Content-Type: application/json" \
  -d '{
    "consumer": "worker-2",
    "min_idle_time_ms": 60000,
    "start": "0-0",
    "count": 10
  }'
```

#### List Consumer Groups

```bash
curl http://localhost:8080/api/v1/streams/mystream/groups
```

#### List Consumers in Group

```bash
curl http://localhost:8080/api/v1/streams/mystream/groups/mygroup/consumers
```

#### Delete Consumer Group

```bash
curl -X DELETE http://localhost:8080/api/v1/streams/mystream/groups/mygroup
```

### Consumer Management (Admin)

These endpoints require the `X-Admin-Api-Key` header:

```bash
# Create consumer
curl -X POST http://localhost:8080/api/v1/streams/mystream/groups/mygroup/consumers \
  -H "X-Admin-Api-Key: dev-admin-key" \
  -H "Content-Type: application/json" \
  -d '{"consumer": "worker-3"}'

# Delete consumer
curl -X DELETE http://localhost:8080/api/v1/streams/mystream/groups/mygroup/consumers/worker-3 \
  -H "X-Admin-Api-Key: dev-admin-key"

# Set group ID
curl -X PUT http://localhost:8080/api/v1/streams/mystream/groups/mygroup/setid \
  -H "X-Admin-Api-Key: dev-admin-key" \
  -H "Content-Type: application/json" \
  -d '{"id": "0"}'
```

---

## Server-Sent Events (SSE)

The service provides real-time streaming capabilities using Server-Sent Events (SSE). This allows clients to receive stream entries as they are added, without polling.

### What is SSE?

Server-Sent Events is a standard that allows servers to push real-time updates to clients over HTTP. Unlike WebSockets, SSE:
- Uses standard HTTP (works through proxies and firewalls)
- Is unidirectional (server to client only)
- Automatically reconnects on connection loss
- Supports event types and message IDs

### Subscribe to Streams

Subscribe to one or more streams and receive entries in real-time:

```bash
curl -N "http://localhost:8080/api/v1/streams/subscribe?streams=mystream:$&block_ms=30000"
```

Parameters:
- `streams`: Comma-separated list of `stream:id` pairs
  - Use `$` to receive only new messages
  - Use `0` to receive all existing and new messages
- `block_ms`: Maximum wait time per poll (capped at 30000ms)
- `count`: Maximum entries per response

**Response Format:**
```
event: entries
data: {"key":"mystream","entries":[{"id":"1704000001234-0","fields":{"user":"alice"}}]}

event: entries
data: {"key":"mystream","entries":[{"id":"1704000001235-0","fields":{"user":"bob"}}]}
```

### Subscribe with Consumer Group

Subscribe as a consumer within a group for distributed processing:

```bash
curl -N "http://localhost:8080/api/v1/streams/mystream/groups/mygroup/subscribe?consumer=worker-1&block_ms=30000"
```

Parameters:
- `consumer`: Consumer name (required)
- `id`: Start position (default: `>` for new messages)
- `block_ms`: Maximum wait time per poll
- `count`: Maximum entries per response
- `noack`: Don't add to pending list (default: false)

**Response Format:**
```
event: entries
data: {"key":"mystream","entries":[{"id":"1704000001234-0","fields":{"event":"click"}}]}
```

### SSE Implementation Notes

1. **Timeout Enforcement**: Block time is capped at 30 seconds to prevent indefinite blocking
2. **Connection Keep-alive**: The connection stays open, receiving entries as they arrive
3. **Error Handling**: Errors are sent as SSE events with JSON error details
4. **Client Compatibility**: Works with any SSE-compatible client (browsers, curl with `-N`, EventSource API)

### JavaScript Example

```javascript
const eventSource = new EventSource(
  'http://localhost:8080/api/v1/streams/subscribe?streams=mystream:$'
);

eventSource.addEventListener('entries', (event) => {
  const data = JSON.parse(event.data);
  console.log(`Stream: ${data.key}`);
  data.entries.forEach(entry => {
    console.log(`  ID: ${entry.id}`, entry.fields);
  });
});

eventSource.onerror = (error) => {
  console.error('SSE connection error:', error);
};
```

---

## Admin Operations

Admin endpoints provide server management capabilities and require authentication.

### Authentication

Protected admin endpoints require the `X-Admin-Api-Key` header:

```bash
curl -H "X-Admin-Api-Key: your-api-key" \
  http://localhost:8080/api/v1/admin/server/info
```

The API key is configured via the `ADMIN__API_KEY` environment variable (default: `changeme`).

### Public Admin Endpoints

These endpoints don't require authentication:

| Endpoint | Description |
|----------|-------------|
| `GET /api/v1/admin/pool/stats` | Connection pool statistics |
| `GET /api/v1/admin/capabilities` | Redis server capabilities |

### Protected Endpoints

#### Server Information

```bash
# Full server info
curl -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/server/info

# Specific section
curl -H "X-Admin-Api-Key: dev-admin-key" \
  "http://localhost:8080/api/v1/admin/server/info?section=memory"

# Database size
curl -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/server/dbsize

# Server time
curl -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/server/time
```

#### Memory Management

```bash
# Memory statistics
curl -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/server/memory/stats

# Memory usage for specific key
curl -X POST -H "X-Admin-Api-Key: dev-admin-key" \
  -H "Content-Type: application/json" \
  -d '{"key": "mykey"}' \
  http://localhost:8080/api/v1/admin/server/memory/usage

# Memory doctor analysis
curl -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/server/memory/doctor

# Purge memory
curl -X POST -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/server/memory/purge
```

#### Database Operations

```bash
# Flush current database
curl -X DELETE -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/db/flush

# Flush all databases (DANGEROUS)
curl -X DELETE -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/db/flushall

# Swap databases
curl -X POST -H "X-Admin-Api-Key: dev-admin-key" \
  -H "Content-Type: application/json" \
  -d '{"db1": 0, "db2": 1}' \
  http://localhost:8080/api/v1/admin/db/swapdb
```

#### Client Management

```bash
# List connected clients
curl -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/client/list

# Kill client by ID
curl -X POST -H "X-Admin-Api-Key: dev-admin-key" \
  -H "Content-Type: application/json" \
  -d '{"id": 123}' \
  http://localhost:8080/api/v1/admin/client/kill

# Pause all clients
curl -X POST -H "X-Admin-Api-Key: dev-admin-key" \
  -H "Content-Type: application/json" \
  -d '{"timeout_ms": 5000}' \
  http://localhost:8080/api/v1/admin/client/pause
```

#### Slowlog & Latency

```bash
# Get slowlog entries
curl -X POST -H "X-Admin-Api-Key: dev-admin-key" \
  -H "Content-Type: application/json" \
  -d '{"count": 10}' \
  http://localhost:8080/api/v1/admin/slowlog/get

# Latest latency events
curl -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/latency/latest

# Latency doctor analysis
curl -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/latency/doctor
```

#### Persistence

```bash
# Synchronous save (blocks)
curl -X POST -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/persistence/save

# Background save
curl -X POST -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/persistence/bgsave

# Rewrite AOF
curl -X POST -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/persistence/bgrewriteaof
```

#### ACL (Access Control)

```bash
# List ACL rules
curl -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/acl/list

# List users
curl -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/acl/users

# Current user
curl -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/acl/whoami

# Generate secure password
curl -X POST -H "X-Admin-Api-Key: dev-admin-key" \
  -H "Content-Type: application/json" \
  -d '{"bits": 256}' \
  http://localhost:8080/api/v1/admin/acl/genpass
```

---

## Redis Requirements

| Feature | Minimum Redis Version |
|---------|----------------------|
| Strings, Hashes, Lists, Sets, Sorted Sets | 2.0+ |
| Key operations (SCAN, OBJECT, etc.) | 2.8+ |
| Streams (XADD, XREAD, etc.) | 5.0+ |
| Consumer Groups (XGROUP, XREADGROUP) | 5.0+ |
| XAUTOCLAIM | 6.2+ |
| XINFO extended fields | 7.0+ |

This service is tested with Redis 7.x and Redis 8.x (with built-in modules).

---

## License

MIT
