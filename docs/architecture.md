# Architecture & Design Decisions

This document explains the key architectural decisions made for the Redis Caching Service, particularly around areas where Redis semantics interact with HTTP constraints.

## Layered Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                      api/http                                │
│  Routes (Axum handlers) → Schemas (request/response types)  │
│  Middleware (auth, tracing, timeout, CORS, body limit)       │
│  OpenAPI (utoipa + Swagger UI)                               │
├──────────────────────────────────────────────────────────────┤
│                    application                               │
│  Services (business logic, validation, timeout enforcement)  │
├──────────────────────────────────────────────────────────────┤
│                      domain                                  │
│  Entities (value types) │ Repository traits │ Errors         │
├──────────────────────────────────────────────────────────────┤
│                   infrastructure                             │
│  Redis repositories │ Connection pool │ Config │ Logging     │
│  PubSubManager │ Capability detection                        │
└──────────────────────────────────────────────────────────────┘
```

**Flow:** HTTP request → Route handler → Application service → Repository trait → Redis implementation → Redis server

The repository pattern allows the domain and application layers to be independent of the Redis implementation. Repository traits define the contract; the infrastructure layer provides Redis-specific implementations.

## Capability-Driven Routing

At startup, the service:

1. Connects to Redis and runs `INFO server` to detect the Redis version
2. Runs `MODULE LIST` to detect loaded modules (JSON, Search, Bloom, TimeSeries)
3. Runs `CLUSTER INFO` to detect cluster mode

Based on detected capabilities, route groups are conditionally mounted:

| Capability | Detection | Routes Enabled |
|-----------|-----------|---------------|
| Streams | Redis ≥ 5.0 | `/api/v1/streams/*` |
| ACL | Redis ≥ 6.0 | ACL admin endpoints |
| Functions | Redis ≥ 7.0 | `/api/v1/functions/*` |
| RedisJSON | `rejson` / `redisjson` module | `/api/v1/json/*` |
| RediSearch | `search` / `ft` module | `/api/v1/search/*` |
| RedisBloom | `bf` / `bloom` module | `/api/v1/bloom/*`, `/api/v1/cuckoo/*`, `/api/v1/cms/*`, `/api/v1/topk/*` |
| TimeSeries | `timeseries` module | `/api/v1/timeseries/*` |
| Cluster | `cluster_enabled:1` in CLUSTER INFO | Cluster management |

Route groups for unavailable features are not mounted at all, so requests to those paths return HTTP 404 (Not Found). This allows the same binary to work against minimal Redis 5.x or a fully-loaded Redis 8.x. Individual feature checks within a mounted route group (e.g., LCS requiring Redis 7.0+, hash field expiration requiring 7.4+) return HTTP 501 (Not Implemented) via `CacheError::ModuleNotAvailable`.

## Decision 1: Transaction Model (Single-Request Bundled)

### Problem

Redis `WATCH` → `MULTI` → `EXEC` requires a single connection held across multiple commands, but HTTP is stateless — there's no way to "hold" a connection across multiple HTTP requests.

### Solution

All transaction operations are bundled in a single HTTP request. The service executes WATCH (if specified), MULTI, all commands, and EXEC atomically on one pooled connection.

```
POST /api/v1/transactions/execute
{
    "watch_keys": ["counter"],       // Optional: WATCH before MULTI
    "commands": [
        {"type": "GET", "key": "counter"},
        {"type": "SET", "key": "counter", "value": "42"}
    ]
}
```

Commands use tagged enum variants with a `type` field in `SCREAMING_SNAKE_CASE` (e.g., `GET`, `SET`, `INCR_BY`, `H_SET`, `L_PUSH`, `Z_ADD`). Each variant has its own fields — see the OpenAPI spec or `schemas/transactions.rs` for the full list.

For optimistic locking patterns (compare-and-set), dedicated Lua-script-based endpoints provide equivalent functionality without WATCH:

```
POST /api/v1/transactions/cas
{
    "key": "version",
    "expected_value": "1",
    "new_value": "2"
}
```

The Lua-based CAS is preferred over WATCH because it's atomic in a single round-trip and doesn't require retry logic.

### Rationale

- Covers 95%+ of real-world transaction use cases
- No connection pinning or session management needed
- Stateless and horizontally scalable
- Lua-based CAS provides equivalent functionality to WATCH for optimistic locking

### Trade-offs

- Cannot express multi-request interactive transactions (rare in practice)
- Client must know all commands upfront (standard for HTTP APIs)
- WATCH retries must be handled client-side (same as native Redis)

## Decision 2: Pub/Sub Connection Architecture

### Problem

Redis subscriptions require dedicated long-lived connections — once a connection enters subscription mode, it can only receive messages and manage subscriptions. Using pooled connections would permanently remove them from the pool, causing pool exhaustion.

### Solution

Pub/Sub uses a completely separate connection management layer (`PubSubManager`) with its own limits, independent of the command pool.

```
┌─────────────┐     ┌───────────────────┐     ┌───────────┐
│   HTTP API   │────▶│  Command Pool      │────▶│           │
│  (commands)  │     │  (deadpool-redis)  │     │           │
└─────────────┘     └───────────────────┘     │   Redis   │
                                               │   Server  │
┌─────────────┐     ┌───────────────────┐     │           │
│  WebSocket   │────▶│  PubSubManager     │────▶│           │
│  (subscribe) │     │  (dedicated conns) │     │           │
└─────────────┘     └───────────────────┘     └───────────┘
```

**Key behaviors:**
- WebSocket subscription requests create **new dedicated Redis connections**, NOT from the command pool
- Hard limit on concurrent subscriptions (`PUBSUB__MAX_SUBSCRIPTIONS`, default 100)
- Returns HTTP 503 when the subscription limit is exceeded
- Automatic cleanup when the WebSocket disconnects (connection is released)
- Statistics tracked independently (active count, total created, messages, errors)

### Rationale

- Command pool is never starved by subscriptions
- Subscription lifecycle is tied to the WebSocket lifecycle (automatic cleanup)
- Independent scaling — pool size and subscription limit can be tuned separately
- Clear failure mode (503) when subscription capacity is reached

## Decision 3: Blocking Commands with Bounded Timeouts

### Problem

Redis blocking commands (`BLPOP`, `BRPOP`, `BZPOPMIN`, `XREAD BLOCK`, etc.) can block indefinitely if no timeout is specified. In an HTTP API context, this causes:
- HTTP request timeouts (clients and proxies typically have 30-60s limits)
- Worker/connection starvation (blocked connections can't serve other requests)
- Hard-to-debug hangs when timeouts cascade

### Solution

All blocking operations enforce a bounded timeout through `BlockingTimeoutEnforcer`:

- **Minimum timeout**: 1 second (prevents spin-loops)
- **Maximum timeout**: configurable, defaults to 30 seconds (`BLOCKING__MAX_TIMEOUT_SECONDS`)
- **Default timeout**: 5 seconds when client doesn't specify one

```
Client requests BLPOP with timeout=60s
  → Enforcer clamps to max(30s)
  → Redis BLPOP executes with 30s timeout
  → If data arrives: HTTP 200 with data
  → If timeout expires: HTTP 204 No Content (not an error)
```

**Response semantics:**
- **200 OK** — data was available within the timeout
- **204 No Content** — timeout expired, no data available (client should retry if desired)
- **504 Gateway Timeout** — internal timeout (should not occur under normal conditions)

### SSE for Long-Running Consumers

For use cases that need continuous consumption (stream processing, event listening), **Server-Sent Events (SSE)** endpoints provide persistent connections that repeatedly poll with bounded timeouts:

```
GET /api/v1/streams/{key}/subscribe          # Stream subscription
GET /api/v1/lists/{key}/blpop/stream         # Repeated BLPOP
GET /api/v1/sorted-sets/{key}/bzpopmin/stream # Repeated BZPOPMIN
```

SSE connections are limited by `BLOCKING__MAX_SSE_CONNECTIONS` (default 5) to prevent resource exhaustion. Each SSE connection holds a Redis connection and an async task for its lifetime.

### Rationale

- Prevents indefinite blocking and HTTP timeouts
- Workers/connections always return to the pool within a bounded time
- 204 response clearly communicates "no data yet" vs. an error
- SSE provides the long-running consumer pattern without breaking HTTP semantics

## Middleware Architecture

Global middleware is applied in this order (outermost first):

1. **DefaultBodyLimit** — rejects oversized request bodies
2. **TraceLayer** — structured request/response logging via `tower_http`
3. **TimeoutLayer** — enforces `SERVER__REQUEST_TIMEOUT_MS` (returns HTTP 408)
4. **CorsLayer** — permissive CORS (allow all origins, methods, headers)

Admin authentication is handled at the handler level, not as global middleware. Protected handlers validate the `X-Admin-Api-Key` header before proceeding.

## Connection Pool Instrumentation

The `InstrumentedPool` wraps `deadpool-redis` to provide:
- Connection acquisition timing
- Pool utilization metrics (available via `/api/v1/admin/pool/stats`)
- Capability detection at startup

This wrapper is transparent to the application and domain layers — they interact with standard `redis` crate types.
