# Redis Caching Service - Rust Implementation Plan

## Overview

A high-performance, production-ready Redis caching service built with Rust, providing comprehensive Redis operations through a clean REST API interface. This implementation draws from the existing Go (`caching-service`) and Node.js (`caching_node`) implementations while leveraging Rust's performance, safety, and concurrency guarantees.

## Project Goals

1. **Complete Redis Coverage** - Support ALL Redis data types, modules, and features
2. **High Performance** - Leverage Rust's zero-cost abstractions and async runtime
3. **Type Safety** - Strong compile-time guarantees with Rust's type system
4. **Production Ready** - Connection pooling, health checks, metrics, graceful shutdown
5. **Clean Architecture** - Maintainable, testable, and extensible codebase
6. **API Compatibility** - RESTful API similar to Go/Node implementations for easy migration

---

## Technology Stack

### Core Framework & Libraries

| Component | Library | Rationale |
|-----------|---------|-----------|
| **HTTP Framework** | `axum` | Async, ergonomic, tower-based, excellent performance |
| **Async Runtime** | `tokio` | Industry standard, mature ecosystem |
| **Redis Client** | `redis-rs` with `deadpool-redis` | Async support, connection pooling, comprehensive Redis support |
| **Serialization** | `serde` + `serde_json` | De-facto standard, excellent performance |
| **Validation** | `validator` | Derive-based validation, similar to Go's validator |
| **Configuration** | `config` + `dotenvy` | Flexible config from env/files |
| **Logging** | `tracing` + `tracing-subscriber` | Structured logging, async-aware, spans |
| **Error Handling** | `thiserror` + `anyhow` | Ergonomic error types |
| **OpenAPI/Swagger** | `utoipa` + `utoipa-swagger-ui` | Compile-time OpenAPI generation |
| **Testing** | `tokio-test` + `testcontainers` | Async testing with real Redis |

### Additional Dependencies

```toml
[dependencies]
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
redis = { version = "0.25", features = ["tokio-comp", "connection-manager", "json", "cluster-async"] }
deadpool-redis = "0.15"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
validator = { version = "0.18", features = ["derive"] }
config = "0.14"
dotenvy = "0.15"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
thiserror = "1"
anyhow = "1"
utoipa = { version = "4", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "7", features = ["axum"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace", "request-id", "timeout"] }

[dev-dependencies]
tokio-test = "0.4"
testcontainers = "0.15"
testcontainers-modules = { version = "0.3", features = ["redis"] }
```

---

## Architecture

### Clean Architecture Layers

```
┌─────────────────────────────────────────────────────────────────────┐
│                         API Layer (HTTP)                            │
│  Routes → Controllers → Request/Response DTOs → OpenAPI Schemas     │
├─────────────────────────────────────────────────────────────────────┤
│                      Application Layer                              │
│  Use Cases → Services → Business Logic → Orchestration              │
├─────────────────────────────────────────────────────────────────────┤
│                        Domain Layer                                 │
│  Entities → Value Objects → Repository Traits → Domain Errors       │
├─────────────────────────────────────────────────────────────────────┤
│                    Infrastructure Layer                             │
│  Redis Repository → Connection Pool → Logging → Config              │
└─────────────────────────────────────────────────────────────────────┘
```

### Project Structure

```
caching_rust/
├── Cargo.toml
├── Cargo.lock
├── .env.example
├── .gitignore
├── README.md
├── Dockerfile
├── docker-compose.yml
├── plan.md
├── tasks.md
│
├── src/
│   ├── main.rs                      # Application entry point
│   ├── lib.rs                       # Library root
│   │
│   ├── api/                         # API Layer
│   │   ├── mod.rs
│   │   ├── http/
│   │   │   ├── mod.rs
│   │   │   ├── server.rs            # Axum server setup
│   │   │   ├── router.rs            # Route aggregation
│   │   │   ├── middleware/          # Custom middleware
│   │   │   │   ├── mod.rs
│   │   │   │   ├── request_id.rs
│   │   │   │   ├── logging.rs
│   │   │   │   └── error_handler.rs
│   │   │   ├── routes/              # Route definitions
│   │   │   │   ├── mod.rs
│   │   │   │   ├── health.rs
│   │   │   │   ├── strings.rs
│   │   │   │   ├── hashes.rs
│   │   │   │   ├── lists.rs
│   │   │   │   ├── sets.rs
│   │   │   │   ├── sorted_sets.rs
│   │   │   │   ├── streams.rs
│   │   │   │   ├── keys.rs
│   │   │   │   ├── json.rs
│   │   │   │   ├── search.rs
│   │   │   │   ├── bloom.rs
│   │   │   │   ├── probabilistic.rs
│   │   │   │   ├── geo.rs
│   │   │   │   ├── bitmap.rs
│   │   │   │   ├── timeseries.rs
│   │   │   │   ├── pubsub.rs
│   │   │   │   ├── transactions.rs
│   │   │   │   ├── scripting.rs
│   │   │   │   ├── functions.rs
│   │   │   │   └── admin.rs
│   │   │   └── schemas/             # Request/Response schemas
│   │   │       ├── mod.rs
│   │   │       ├── common.rs
│   │   │       ├── strings.rs
│   │   │       ├── hashes.rs
│   │   │       ├── lists.rs
│   │   │       ├── sets.rs
│   │   │       ├── sorted_sets.rs
│   │   │       ├── streams.rs
│   │   │       ├── keys.rs
│   │   │       ├── json.rs
│   │   │       ├── search.rs
│   │   │       ├── bloom.rs
│   │   │       ├── probabilistic.rs
│   │   │       ├── geo.rs
│   │   │       ├── bitmap.rs
│   │   │       ├── timeseries.rs
│   │   │       ├── pubsub.rs
│   │   │       ├── transactions.rs
│   │   │       ├── scripting.rs
│   │   │       ├── functions.rs
│   │   │       └── admin.rs
│   │
│   ├── application/                 # Application Layer
│   │   ├── mod.rs
│   │   ├── services/
│   │   │   ├── mod.rs
│   │   │   ├── string_service.rs
│   │   │   ├── hash_service.rs
│   │   │   ├── list_service.rs
│   │   │   ├── set_service.rs
│   │   │   ├── sorted_set_service.rs
│   │   │   ├── stream_service.rs
│   │   │   ├── key_service.rs
│   │   │   ├── json_service.rs
│   │   │   ├── search_service.rs
│   │   │   ├── bloom_service.rs
│   │   │   ├── probabilistic_service.rs
│   │   │   ├── geo_service.rs
│   │   │   ├── bitmap_service.rs
│   │   │   ├── timeseries_service.rs
│   │   │   ├── pubsub_service.rs
│   │   │   ├── transaction_service.rs
│   │   │   ├── scripting_service.rs
│   │   │   ├── function_service.rs
│   │   │   └── admin_service.rs
│   │
│   ├── domain/                      # Domain Layer
│   │   ├── mod.rs
│   │   ├── entities/
│   │   │   ├── mod.rs
│   │   │   └── ... (domain entities)
│   │   ├── repositories/            # Repository traits
│   │   │   ├── mod.rs
│   │   │   ├── string_repository.rs
│   │   │   ├── hash_repository.rs
│   │   │   ├── list_repository.rs
│   │   │   ├── set_repository.rs
│   │   │   ├── sorted_set_repository.rs
│   │   │   ├── stream_repository.rs
│   │   │   ├── key_repository.rs
│   │   │   ├── json_repository.rs
│   │   │   ├── search_repository.rs
│   │   │   ├── bloom_repository.rs
│   │   │   ├── probabilistic_repository.rs
│   │   │   ├── geo_repository.rs
│   │   │   ├── bitmap_repository.rs
│   │   │   ├── timeseries_repository.rs
│   │   │   ├── pubsub_repository.rs
│   │   │   ├── transaction_repository.rs
│   │   │   ├── scripting_repository.rs
│   │   │   ├── function_repository.rs
│   │   │   └── admin_repository.rs
│   │   └── errors/
│   │       ├── mod.rs
│   │       └── cache_error.rs
│   │
│   ├── infrastructure/              # Infrastructure Layer
│   │   ├── mod.rs
│   │   ├── redis/
│   │   │   ├── mod.rs
│   │   │   ├── connection.rs        # Connection pool setup
│   │   │   ├── client.rs            # Redis client wrapper
│   │   │   └── repositories/        # Repository implementations
│   │   │       ├── mod.rs
│   │   │       ├── string_repo.rs
│   │   │       ├── hash_repo.rs
│   │   │       ├── list_repo.rs
│   │   │       ├── set_repo.rs
│   │   │       ├── sorted_set_repo.rs
│   │   │       ├── stream_repo.rs
│   │   │       ├── key_repo.rs
│   │   │       ├── json_repo.rs
│   │   │       ├── search_repo.rs
│   │   │       ├── bloom_repo.rs
│   │   │       ├── probabilistic_repo.rs
│   │   │       ├── geo_repo.rs
│   │   │       ├── bitmap_repo.rs
│   │   │       ├── timeseries_repo.rs
│   │   │       ├── pubsub_repo.rs
│   │   │       ├── transaction_repo.rs
│   │   │       ├── scripting_repo.rs
│   │   │       ├── function_repo.rs
│   │   │       └── admin_repo.rs
│   │   ├── config/
│   │   │   ├── mod.rs
│   │   │   └── settings.rs
│   │   └── logging/
│   │       ├── mod.rs
│   │       └── tracing_setup.rs
│   │
│   └── shared/                      # Shared utilities
│       ├── mod.rs
│       ├── app_state.rs             # Application state
│       ├── response.rs              # Common response types
│       └── utils.rs                 # Helper functions
│
├── tests/                           # Integration & E2E tests
│   ├── common/
│   │   └── mod.rs
│   ├── integration/
│   │   ├── mod.rs
│   │   ├── strings_test.rs
│   │   ├── hashes_test.rs
│   │   └── ...
│   └── e2e/
│       └── ...
│
├── benches/                         # Benchmarks
│   └── redis_operations.rs
│
└── scripts/
    ├── run_tests.sh
    └── seed_data.sh
```

---

## Redis Features Coverage

### Complete Feature Matrix

This implementation will cover **100%** of Redis features, organized by category:

#### 1. Core Data Types (Fully Implemented in Go/Node)

| Data Type | Commands | Status |
|-----------|----------|--------|
| **Strings** | GET, SET, SETNX, SETEX, MGET, MSET, MSETNX, INCR, INCRBY, INCRBYFLOAT, DECR, DECRBY, APPEND, STRLEN, GETRANGE, SETRANGE, GETEX, GETDEL | ✅ Port from Go/Node |
| **Hashes** | HGET, HSET, HSETNX, HGETALL, HMGET, HMSET, HDEL, HEXISTS, HKEYS, HVALS, HLEN, HINCRBY, HINCRBYFLOAT, HSTRLEN, HRANDFIELD, HSCAN | ✅ Port from Go/Node |
| **Lists** | LPUSH, RPUSH, LPUSHX, RPUSHX, LPOP, RPOP, RPOPLPUSH, LMOVE, LMPOP, LLEN, LRANGE, LINDEX, LSET, LINSERT, LREM, LTRIM, BLPOP, BRPOP, BLMOVE, LPOS | ✅ Port from Go/Node |
| **Sets** | SADD, SREM, SMEMBERS, SISMEMBER, SMISMEMBER, SCARD, SRANDMEMBER, SPOP, SMOVE, SINTER, SINTERSTORE, SINTERCARD, SUNION, SUNIONSTORE, SDIFF, SDIFFSTORE, SSCAN | ✅ Port from Go/Node |
| **Sorted Sets** | ZADD, ZREM, ZRANGE, ZREVRANGE, ZRANGEBYSCORE, ZREVRANGEBYSCORE, ZRANGEBYLEX, ZREVRANGEBYLEX, ZSCORE, ZMSCORE, ZRANK, ZREVRANK, ZCARD, ZCOUNT, ZLEXCOUNT, ZINCRBY, ZPOPMIN, ZPOPMAX, BZPOPMIN, BZPOPMAX, ZMPOP, BZMPOP, ZRANGESTORE, ZUNION, ZUNIONSTORE, ZINTER, ZINTERSTORE, ZINTERCARD, ZDIFF, ZDIFFSTORE, ZRANDMEMBER, ZSCAN | ✅ Port from Go/Node |
| **Streams** | XADD, XREAD, XREADGROUP, XRANGE, XREVRANGE, XLEN, XTRIM, XDEL, XGROUP CREATE/DESTROY/SETID/DELCONSUMER/CREATECONSUMER, XACK, XCLAIM, XAUTOCLAIM, XPENDING, XINFO STREAM/GROUPS/CONSUMERS, XSETID | ✅ Port from Go/Node |

#### 2. Advanced Redis Modules

| Module | Commands | Status |
|--------|----------|--------|
| **RedisJSON** | JSON.SET, JSON.GET, JSON.MGET, JSON.DEL, JSON.TYPE, JSON.STRLEN, JSON.ARRLEN, JSON.ARRAPPEND, JSON.ARRINDEX, JSON.ARRINSERT, JSON.ARRPOP, JSON.ARRTRIM, JSON.OBJKEYS, JSON.OBJLEN, JSON.NUMINCRBY, JSON.NUMMULTBY, JSON.TOGGLE, JSON.CLEAR, JSON.RESP, JSON.DEBUG MEMORY | ✅ Port from Go/Node |
| **RediSearch** | FT.CREATE, FT.DROPINDEX, FT.INFO, FT.SEARCH, FT.AGGREGATE, FT.EXPLAIN, FT.PROFILE, FT.ALTER, FT.ALIASADD, FT.ALIASDEL, FT.ALIASUPDATE, FT.TAGVALS, FT.SUGADD, FT.SUGGET, FT.SUGDEL, FT.SUGLEN, FT.SYNDUMP, FT.SYNUPDATE, FT.SPELLCHECK, FT.DICTADD, FT.DICTDEL, FT.DICTDUMP, FT._LIST | ✅ Port from Go/Node |
| **RedisBloom** | BF.RESERVE, BF.ADD, BF.MADD, BF.EXISTS, BF.MEXISTS, BF.INSERT, BF.INFO, BF.SCANDUMP, BF.LOADCHUNK, BF.CARD | ✅ Port from Go |
| **Cuckoo Filter** | CF.RESERVE, CF.ADD, CF.ADDNX, CF.INSERT, CF.INSERTNX, CF.EXISTS, CF.MEXISTS, CF.DEL, CF.COUNT, CF.SCANDUMP, CF.LOADCHUNK, CF.INFO | ✅ Port from Go |
| **Count-Min Sketch** | CMS.INITBYDIM, CMS.INITBYPROB, CMS.INCRBY, CMS.QUERY, CMS.MERGE, CMS.INFO | ✅ Port from Go |
| **Top-K** | TOPK.RESERVE, TOPK.ADD, TOPK.INCRBY, TOPK.QUERY, TOPK.COUNT, TOPK.LIST, TOPK.INFO | ✅ Port from Go |
| **HyperLogLog** | PFADD, PFCOUNT, PFMERGE | ✅ Port from Go |
| **RedisTimeSeries** | TS.CREATE, TS.ALTER, TS.ADD, TS.MADD, TS.INCRBY, TS.DECRBY, TS.DEL, TS.GET, TS.MGET, TS.RANGE, TS.REVRANGE, TS.MRANGE, TS.MREVRANGE, TS.QUERYINDEX, TS.INFO, TS.CREATERULE, TS.DELETERULE | 🆕 NEW |

#### 3. Missing Features to Add (Not in Go/Node)

| Feature | Commands | Priority |
|---------|----------|----------|
| **Bitmaps** | SETBIT, GETBIT, BITCOUNT, BITPOS, BITOP, BITFIELD, BITFIELD_RO | 🔴 High |
| **Geospatial** | GEOADD, GEODIST, GEOHASH, GEOPOS, GEORADIUS, GEORADIUSBYMEMBER, GEOSEARCH, GEOSEARCHSTORE | 🔴 High |
| **Pub/Sub** | PUBLISH, SUBSCRIBE, PSUBSCRIBE, UNSUBSCRIBE, PUNSUBSCRIBE, PUBSUB CHANNELS/NUMSUB/NUMPAT, SSUBSCRIBE, SUNSUBSCRIBE, SPUBLISH | 🔴 High |
| **Transactions** | MULTI, EXEC, DISCARD, WATCH, UNWATCH | 🔴 High |
| **Lua Scripting** | EVAL, EVALSHA, EVALSHA_RO, EVAL_RO, SCRIPT LOAD/EXISTS/FLUSH/KILL/DEBUG | 🔴 High |
| **Redis Functions** | FUNCTION LOAD/DELETE/FLUSH/DUMP/RESTORE/LIST/STATS/KILL, FCALL, FCALL_RO | 🟡 Medium |
| **RedisGraph** | GRAPH.QUERY, GRAPH.RO_QUERY, GRAPH.DELETE, GRAPH.EXPLAIN, GRAPH.PROFILE, GRAPH.SLOWLOG, GRAPH.CONFIG | 🟡 Medium |
| **Cluster** | CLUSTER INFO/NODES/SLOTS/KEYSLOT/SHARDS, etc. | 🟢 Low |
| **Sentinel** | SENTINEL commands | 🟢 Low |

#### 4. Admin & Server Operations

| Category | Commands | Status |
|----------|----------|--------|
| **Database** | FLUSHDB, FLUSHALL, DBSIZE, SWAPDB, SELECT, MOVE, COPY | ✅ Port |
| **Server** | INFO, TIME, LASTSAVE, DEBUG, MEMORY DOCTOR/MALLOC-SIZE/PURGE/STATS/USAGE | ✅ Port |
| **Config** | CONFIG GET/SET/REWRITE/RESETSTAT, ACL CAT/DELUSER/GENPASS/GETUSER/LIST/LOAD/LOG/SAVE/SETUSER/USERS/WHOAMI | ✅ Extend |
| **Persistence** | SAVE, BGSAVE, BGREWRITEAOF, SHUTDOWN | ✅ Port |
| **Client** | CLIENT LIST/KILL/SETNAME/GETNAME/PAUSE/UNPAUSE/ID/INFO/NO-EVICT/REPLY/TRACKINGINFO | ✅ Port |
| **Scan** | SCAN, HSCAN, SSCAN, ZSCAN | ✅ Port |
| **Monitoring** | SLOWLOG GET/LEN/RESET, LATENCY DOCTOR/GRAPH/HISTORY/LATEST/RESET, DEBUG SLEEP/OBJECT/SEGFAULT | ✅ Extend |
| **Keys** | DEL, EXISTS, EXPIRE, EXPIREAT, EXPIRETIME, TTL, PTTL, PERSIST, KEYS, SCAN, RANDOMKEY, RENAME, RENAMENX, TYPE, OBJECT ENCODING/FREQ/IDLETIME/REFCOUNT/HELP, TOUCH, UNLINK, WAIT, DUMP, RESTORE, MIGRATE, SORT, SORT_RO | ✅ Port |

---

## API Design

### RESTful Conventions

```
Base URL: /api/v1

# Resource-based endpoints
GET    /strings/:key           # Get string value
PUT    /strings/:key           # Set string value
POST   /strings/mget           # Multi-get
POST   /strings/mset           # Multi-set
PATCH  /strings/:key/incr      # Increment
PATCH  /strings/:key/append    # Append

# Similar patterns for all data types...
```

### Response Format

```json
{
  "success": true,
  "timestamp": "2025-01-03T12:00:00.000Z",
  "request_id": "uuid-v4",
  "data": { ... },
  "meta": {
    "ttl": 300,
    "encoding": "embstr"
  }
}
```

### Error Response Format

```json
{
  "success": false,
  "timestamp": "2025-01-03T12:00:00.000Z",
  "request_id": "uuid-v4",
  "error": {
    "code": "KEY_NOT_FOUND",
    "message": "Key 'user:123' does not exist",
    "details": { ... }
  }
}
```

---

## Configuration

### Environment Variables

```env
# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
RUST_LOG=info

# Redis
REDIS_URL=redis://localhost:6379
REDIS_PASSWORD=
REDIS_DATABASE=0
REDIS_TLS_ENABLED=false
REDIS_TLS_CERT_PATH=
REDIS_TLS_KEY_PATH=
REDIS_TLS_CA_PATH=
REDIS_TLS_SKIP_VERIFY=false

# Connection Pool
REDIS_POOL_MIN_SIZE=2
REDIS_POOL_MAX_SIZE=10
REDIS_CONNECT_TIMEOUT_MS=5000
REDIS_COMMAND_TIMEOUT_MS=5000

# Admin
ADMIN_API_KEY=changeme-admin-key
```

---

## Testing Strategy

### Test Levels

1. **Unit Tests** - Domain logic, validators, transformers
2. **Integration Tests** - Repository implementations with testcontainers
3. **E2E Tests** - Full API testing with HTTP client
4. **Benchmark Tests** - Performance testing with criterion

### Coverage Target

- Lines: 90%+
- Branches: 85%+
- Functions: 95%+

---

## Performance Considerations

1. **Connection Pooling** - Use deadpool-redis for efficient connection reuse
2. **Async I/O** - Fully async with Tokio runtime
3. **Zero-Copy** - Minimize allocations with bytes crate where possible
4. **Pipelining** - Support Redis pipelining for batch operations
5. **Backpressure** - Tower middleware for rate limiting and load shedding

---

## Security Considerations

1. **Input Validation** - Strict validation on all inputs
2. **TLS Support** - Optional TLS for Redis connections
3. **Authentication** - Redis AUTH support
4. **Admin Protection** - API key for admin endpoints
5. **Rate Limiting** - Tower-based rate limiting
6. **CORS** - Configurable CORS policies

---

## Deployment

### Docker Support

- Multi-stage Dockerfile for minimal image size
- Docker Compose with Redis Stack
- Health check endpoints

### Kubernetes Ready

- Readiness and liveness probes
- Graceful shutdown handling
- ConfigMap/Secret support

---

## Roadmap

### Phase 1: Foundation (Week 1-2)
- Project setup with all dependencies
- Configuration and logging infrastructure
- Redis connection pool with health checks
- Core data types (Strings, Hashes, Lists, Sets, Sorted Sets)

### Phase 2: Advanced Data Types (Week 3-4)
- Streams
- Key management operations
- RedisJSON module
- RediSearch module

### Phase 3: Probabilistic & Missing Features (Week 5-6)
- Bloom filters, Cuckoo filters
- Count-Min Sketch, Top-K, HyperLogLog
- Bitmaps (NEW)
- Geospatial (NEW)
- Pub/Sub (NEW)

### Phase 4: Advanced Features (Week 7-8)
- Transactions (MULTI/EXEC)
- Lua Scripting (EVAL/EVALSHA)
- Redis Functions (FCALL)
- RedisTimeSeries (NEW)
- RedisGraph (optional)

### Phase 5: Admin & Production (Week 9-10)
- Admin operations
- Monitoring and metrics
- ACL support
- Complete test coverage
- Documentation and examples

---

## References

- [Go Implementation](../caching-service/) - Primary reference for API design
- [Node.js Implementation](../caching_node/) - Reference for clean architecture
- [Redis Documentation](https://redis.io/docs/) - Official Redis docs
- [redis-rs Documentation](https://docs.rs/redis/) - Rust Redis client docs
