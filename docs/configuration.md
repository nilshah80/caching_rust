# Configuration Guide

The Redis Caching Service is configured entirely through environment variables, using `__` (double underscore) as the section separator. A `.env` file in the project root is also supported (loaded via `dotenvy`).

## Quick Start

Create a `.env` file in the project root:

```env
SERVER__HOST=0.0.0.0
SERVER__PORT=8080
REDIS__URL=redis://localhost:6379
ADMIN__API_KEY=your-secret-key
LOG__LEVEL=info
LOG__FORMAT=pretty
```

Or pass environment variables directly:

```bash
SERVER__PORT=3000 REDIS__URL=redis://myhost:6379 cargo run
```

## Complete Reference

### Server Configuration

Controls the HTTP server behavior.

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `SERVER__HOST` | String | `0.0.0.0` | Network interface to bind to. Use `127.0.0.1` for local-only access. |
| `SERVER__PORT` | u16 | `8080` | TCP port to listen on. |
| `SERVER__REQUEST_TIMEOUT_MS` | u64 | `30000` | Maximum time (ms) for an HTTP request to complete. Returns HTTP 408 on timeout. |
| `SERVER__MAX_BODY_SIZE_BYTES` | usize | `10485760` | Maximum request body size (default 10 MiB). Returns HTTP 413 if exceeded. |
| `SERVER__MAX_BATCH_SIZE` | usize | `1000` | Maximum number of items in batch operations (MGET, MSET, pipeline commands). |
| `SERVER__MAX_VALUE_SIZE_BYTES` | usize | `524288` | Maximum size for a single value (default 512 KiB). |

**Tuning Notes:**
- `REQUEST_TIMEOUT_MS` should be greater than `BLOCKING__MAX_TIMEOUT_SECONDS * 1000` to avoid premature timeouts on blocking operations.
- `MAX_BODY_SIZE_BYTES` protects against oversized payloads. Increase if you store large JSON documents or binary data.
- `MAX_BATCH_SIZE` prevents runaway batch operations that could starve the connection pool.

### Redis Connection

Configures the connection to the Redis server.

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `REDIS__URL` | String | `redis://localhost:6379` | Redis connection URL. Supports `redis://`, `rediss://` (TLS), and `redis+unix://` schemes. |
| `REDIS__PASSWORD` | String | - | Redis password. Alternative to embedding in the URL. |
| `REDIS__DATABASE` | u8 | `0` | Database number (0-15). |
| `REDIS__TLS_ENABLED` | bool | `false` | Enable TLS for the connection. |
| `REDIS__TLS_CERT_PATH` | String | - | Path to client TLS certificate (mutual TLS). |
| `REDIS__TLS_KEY_PATH` | String | - | Path to client TLS private key (mutual TLS). |
| `REDIS__TLS_CA_PATH` | String | - | Path to custom CA certificate. |
| `REDIS__TLS_SKIP_VERIFY` | bool | `false` | Skip TLS certificate verification. **Not recommended for production.** |

**URL Format Examples:**

```env
# Standalone
REDIS__URL=redis://localhost:6379

# With password in URL
REDIS__URL=redis://:mypassword@redis-host:6379

# With password and database
REDIS__URL=redis://:mypassword@redis-host:6379/2

# TLS
REDIS__URL=rediss://redis-host:6380

# Unix socket
REDIS__URL=redis+unix:///var/run/redis/redis.sock
```

### Connection Pool

The service uses `deadpool-redis` for connection pooling. These settings control pool behavior.

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `POOL__MIN_SIZE` | u32 | `2` | Minimum number of idle connections maintained in the pool. |
| `POOL__MAX_SIZE` | u32 | `10` | Maximum number of connections the pool can create. |
| `POOL__CONNECT_TIMEOUT_MS` | u64 | `5000` | Timeout for establishing a new connection to Redis. |
| `POOL__COMMAND_TIMEOUT_MS` | u64 | `5000` | Timeout for a single Redis command execution. |
| `POOL__IDLE_TIMEOUT_MS` | u64 | `600000` | Time (ms) before an idle connection is closed (default 10 minutes). |

**Tuning Notes:**
- `MAX_SIZE` should match your expected concurrency level. Each concurrent HTTP request that hits Redis needs one connection.
- `MIN_SIZE` keeps warm connections ready. Set higher in high-throughput scenarios to avoid cold-start latency.
- For production with moderate load: `MIN_SIZE=5`, `MAX_SIZE=25-50`.
- For high-throughput: `MIN_SIZE=10`, `MAX_SIZE=50-100`. Monitor with the `/api/v1/admin/pool/stats` endpoint.
- If you see "pool exhausted" errors, increase `MAX_SIZE` or optimize Redis command latency.
- `IDLE_TIMEOUT_MS` should be less than Redis's `timeout` setting (if configured) to avoid using stale connections.

### Pub/Sub

Pub/Sub uses **dedicated connections** separate from the command pool. Each WebSocket subscription creates a new Redis connection.

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PUBSUB__MAX_SUBSCRIPTIONS` | usize | `100` | Maximum number of concurrent Pub/Sub subscriptions. Returns HTTP 503 when exceeded. |
| `PUBSUB__CONNECTION_TIMEOUT_MS` | u64 | `30000` | Timeout for establishing a Pub/Sub connection to Redis. |

**Tuning Notes:**
- Each subscription consumes one Redis connection and one OS file descriptor. Set `MAX_SUBSCRIPTIONS` based on your Redis `maxclients` and OS `ulimit`.
- Monitor subscription usage via `GET /api/v1/pubsub/stats`.
- Connections are automatically cleaned up when the WebSocket disconnects.

### Blocking Commands

Controls behavior for blocking Redis operations (BLPOP, BRPOP, BZPOPMIN, XREAD BLOCK, etc.) and SSE streaming.

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `BLOCKING__MAX_TIMEOUT_SECONDS` | u32 | `30` | Hard cap for blocking operation timeouts. Prevents indefinite blocking. |
| `BLOCKING__DEFAULT_TIMEOUT_SECONDS` | u32 | `5` | Default timeout when client doesn't specify one. |
| `BLOCKING__MAX_SSE_CONNECTIONS` | usize | `5` | Maximum concurrent SSE streaming connections. |
| `BLOCKING__DEFAULT_STREAM_READ_COUNT` | usize | `100` | Default number of entries to read from streams. |

**Tuning Notes:**
- `MAX_TIMEOUT_SECONDS` prevents HTTP worker starvation. Keep at or below 30s (the HTTP request timeout).
- `MAX_SSE_CONNECTIONS` limits resource usage for long-lived streaming connections. Each SSE connection holds a Redis connection and an Axum task for the duration.
- Blocking operations return HTTP 204 (No Content) when they time out without data, not an error.

### Admin Authentication

Protects administrative endpoints.

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `ADMIN__API_KEY` | String | `changeme-admin-key` | API key required in the `X-Admin-Api-Key` header for protected endpoints. |

**Security Notes:**
- **Always** change the default API key in production.
- The API key is transmitted in HTTP headers. Use TLS in production to protect it in transit.
- Public endpoints (`/api/v1/admin/pool/stats`, `/api/v1/admin/capabilities`) do not require the API key.

### Logging

Configures structured logging via `tracing`.

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `LOG__LEVEL` | String | `info` | Log level: `trace`, `debug`, `info`, `warn`, `error`. |
| `LOG__FORMAT` | String | `json` | Output format: `json` (structured, for production) or `pretty` (human-readable, for development). |
| `RUST_LOG` | String | - | Overrides `LOG__LEVEL` with full tracing filter syntax (e.g., `redis_caching_service=debug,tower_http=info`). |

**Examples:**

```env
# Development
LOG__LEVEL=debug
LOG__FORMAT=pretty

# Production
LOG__LEVEL=info
LOG__FORMAT=json

# Fine-grained control
RUST_LOG=redis_caching_service=debug,tower_http=warn,deadpool=info
```

## Configuration Profiles

### Development

```env
SERVER__HOST=0.0.0.0
SERVER__PORT=8080
REDIS__URL=redis://:devpassword@localhost:6379
POOL__MIN_SIZE=2
POOL__MAX_SIZE=10
ADMIN__API_KEY=dev-admin-key
LOG__LEVEL=debug
LOG__FORMAT=pretty
```

### Production

```env
SERVER__HOST=0.0.0.0
SERVER__PORT=8080
SERVER__REQUEST_TIMEOUT_MS=30000
SERVER__MAX_BODY_SIZE_BYTES=10485760
REDIS__URL=redis://:${REDIS_PASSWORD}@redis-primary:6379
REDIS__TLS_ENABLED=true
POOL__MIN_SIZE=10
POOL__MAX_SIZE=50
POOL__CONNECT_TIMEOUT_MS=3000
POOL__COMMAND_TIMEOUT_MS=3000
PUBSUB__MAX_SUBSCRIPTIONS=200
BLOCKING__MAX_TIMEOUT_SECONDS=30
BLOCKING__MAX_SSE_CONNECTIONS=20
ADMIN__API_KEY=${ADMIN_SECRET}
LOG__LEVEL=info
LOG__FORMAT=json
```

### Benchmarking

```env
SERVER__HOST=0.0.0.0
SERVER__PORT=8080
REDIS__URL=redis://:devpassword@redis:6379
POOL__MIN_SIZE=5
POOL__MAX_SIZE=50
ADMIN__API_KEY=dev-admin-key
LOG__LEVEL=warn
LOG__FORMAT=json
RUST_LOG=warn
```
