# Redis Caching Service — Task Registry

> Production-ready Redis caching service in Rust (Axum + redis-rs + deadpool).
> **300+ Redis commands** covered across all core types, modules, and admin operations.

---

## Architectural Decisions

1. **Transactions**: Single-request bundled model (WATCH+MULTI+EXEC in one HTTP call, CAS via Lua)
2. **Pub/Sub**: Dedicated connections per WebSocket (not from command pool), hard subscription limit
3. **Blocking Commands**: Enforced max timeout (30s), 204 on timeout, SSE for streaming XREAD
4. **Module Detection**: Capabilities detected at startup via MODULE LIST; unavailable modules return 501
5. **Pool Instrumentation**: Custom `InstrumentedPool` wrapper with wait_count/duration metrics

---

## Phase 1: Foundation & Project Setup ✅

- [x] **1.1.1**: Initialize Cargo project with dependencies and feature flags
- [x] **1.1.2**: Set up project directory structure (domain/application/infrastructure/api layers)
- [x] **1.1.3**: Configure dev environment (.env, rustfmt, clippy)
- [x] **1.1.4**: Create Docker dev environment (multi-stage Dockerfile, docker-compose with Redis Stack)
- [x] **1.2.1**: Implement configuration module (Settings, env vars, .env loading)
- [x] **1.2.2**: Create configuration validation
- [x] **1.3.1**: Implement structured logging (tracing + tracing-subscriber, JSON format)
- [x] **1.3.2**: Add request ID middleware
- [x] **1.4.1**: Implement error type hierarchy (CacheError enum, HTTP status mapping)
- [x] **1.4.2**: Create standardized API response types (ApiResponse wrapper)

---

## Phase 2: Redis Connection & Core Infrastructure ✅

- [x] **2.1.1**: Implement connection pool with deadpool-redis
- [x] **2.1.2**: Add pool instrumentation and metrics (wait count, duration, circuit breaker)
- [x] **2.1.3**: Implement connection health checking and warm-up
- [x] **2.1.4**: Add TLS/SSL support (rustls)
- [x] **2.2.1**: Implement PubSubManager with dedicated connections
- [x] **2.2.2**: Add subscription lifecycle management (connect, subscribe, cleanup)
- [x] **2.2.3**: Add subscription limits and backpressure (503 when limit reached)
- [x] **2.3.1**: Implement Redis capability detection (MODULE LIST, version parsing)
- [x] **2.3.2**: Create capabilities endpoint (GET /api/v1/capabilities)
- [x] **2.3.3**: Implement conditional route registration based on capabilities
- [x] **2.4.1**: Set up Axum HTTP server with graceful shutdown
- [x] **2.4.2**: Add middleware stack (CORS, body limits, timeout, security headers, tracing)
- [x] **2.4.3**: Add Prometheus metrics endpoint (/metrics)
- [x] **2.4.4**: Add health/readiness/liveness endpoints
- [x] **2.5.1**: Set up utoipa OpenAPI generation
- [x] **2.5.2**: Add Swagger UI (utoipa-swagger-ui)

---

## Phase 3: Core Data Types ✅

### 3.1 String Operations ✅
- [x] **3.1.1–3.1.4**: Repository trait, implementation, routes, schemas
- Commands: GET, SET, MGET, MSET, GETSET, SETNX, SETEX, PSETEX, MSETNX, INCR, INCRBY, INCRBYFLOAT, DECR, DECRBY, APPEND, STRLEN, GETRANGE, SETRANGE, GETDEL, GETEX

### 3.2 Hash Operations ✅
- [x] **3.2.1–3.2.4**: Repository trait, implementation, routes, schemas
- Commands: HGET, HSET, HMGET, HMSET, HDEL, HGETALL, HINCRBY, HINCRBYFLOAT, HKEYS, HVALS, HLEN, HEXISTS, HSCAN, HRANDFIELD, HSETNX, HSTRLEN

### 3.3 List Operations ✅
- [x] **3.3.1–3.3.6**: Repository, implementation, routes, schemas, blocking support, SSE streaming
- Commands: LPUSH, RPUSH, LPOP, RPOP, LRANGE, LLEN, LINDEX, LSET, LINSERT, LREM, LTRIM, LPOS, LMOVE, BLPOP, BRPOP, BLMOVE, LPUSHX, RPUSHX

### 3.4 Set Operations ✅
- [x] **3.4.1–3.4.4**: Repository, implementation, routes, schemas
- Commands: SADD, SREM, SMEMBERS, SISMEMBER, SMISMEMBER, SCARD, SPOP, SRANDMEMBER, SUNION, SINTER, SDIFF, SUNIONSTORE, SINTERSTORE, SDIFFSTORE, SINTERCARD, SSCAN, SMOVE

### 3.5 Sorted Set Operations ✅
- [x] **3.5.1–3.5.6**: Repository, implementation, routes, schemas, blocking support, OpenAPI
- Commands: ZADD, ZREM, ZRANGE, ZRANGEBYSCORE, ZRANGEBYLEX, ZREVRANGE, ZSCORE, ZMSCORE, ZRANK, ZREVRANK, ZCARD, ZCOUNT, ZLEXCOUNT, ZINCRBY, ZPOPMIN, ZPOPMAX, BZPOPMIN, BZPOPMAX, ZRANDMEMBER, ZSCAN, ZUNION, ZINTER, ZDIFF, ZUNIONSTORE, ZINTERSTORE, ZDIFFSTORE, ZRANGESTORE, ZINTERCARD, ZMPOP, BZMPOP, ZREMRANGEBYLEX, ZREMRANGEBYRANK, ZREMRANGEBYSCORE

### 3.6 Stream Operations ✅
- [x] **3.6.1–3.6.8**: Repository, implementation, routes, schemas, consumer groups, blocking, SSE, OpenAPI
- Commands: XADD, XREAD, XRANGE, XREVRANGE, XLEN, XINFO, XTRIM, XDEL, XGROUP (CREATE/DESTROY/SETID/DELCONSUMER/CREATECONSUMER), XREADGROUP, XACK, XPENDING, XCLAIM, XAUTOCLAIM

### 3.7 Key Operations ✅
- [x] **3.7.1–3.7.4**: Repository, implementation, routes, schemas
- Commands: DEL, UNLINK, EXISTS, TYPE, TTL, PTTL, EXPIRE, PEXPIRE, EXPIREAT, PEXPIREAT, PERSIST, RENAME, RENAMENX, KEYS, SCAN, RANDOMKEY, OBJECT (ENCODING/REFCOUNT/IDLETIME/FREQ/HELP), DUMP, RESTORE, TOUCH, COPY, MOVE, WAIT, EXPIRETIME, PEXPIRETIME

---

## Phase 4: Redis Modules ✅

### 4.1 RedisJSON ✅
- [x] **4.1.1–4.1.4**: Repository, implementation, routes, schemas
- Commands: JSON.SET, JSON.GET, JSON.DEL, JSON.MGET, JSON.TYPE, JSON.NUMINCRBY, JSON.NUMMULTBY, JSON.STRAPPEND, JSON.STRLEN, JSON.ARRAPPEND, JSON.ARRINDEX, JSON.ARRINSERT, JSON.ARRLEN, JSON.ARRPOP, JSON.ARRTRIM, JSON.OBJKEYS, JSON.OBJLEN, JSON.DEBUG MEMORY, JSON.RESP, JSON.TOGGLE, JSON.CLEAR, JSON.MERGE

### 4.2 RediSearch ✅
- [x] **4.2.1–4.2.4**: Repository, implementation, routes, schemas
- Commands: FT.CREATE, FT.SEARCH, FT.AGGREGATE, FT.INFO, FT.DROPINDEX, FT._LIST, FT.ALTER, FT.ALIASADD, FT.ALIASDEL, FT.ALIASUPDATE, FT.TAGVALS, FT.SUGADD, FT.SUGGET, FT.SUGDEL, FT.SUGLEN, FT.DICTADD, FT.DICTDEL, FT.DICTDUMP, FT.SYNUPDATE, FT.SYNDUMP, FT.SPELLCHECK, FT.EXPLAIN, FT.EXPLAINCLI, FT.PROFILE

### 4.3 RedisBloom ✅
- [x] **4.3.1–4.3.4**: Repository, implementation, routes, schemas
- Bloom: BF.ADD, BF.EXISTS, BF.MADD, BF.MEXISTS, BF.INFO, BF.RESERVE, BF.INSERT, BF.CARD, BF.SCANDUMP, BF.LOADCHUNK
- Cuckoo: CF.ADD, CF.ADDNX, CF.EXISTS, CF.DEL, CF.COUNT, CF.INFO, CF.RESERVE, CF.INSERT, CF.INSERTNX, CF.MEXISTS, CF.SCANDUMP, CF.LOADCHUNK

### 4.4 Probabilistic Data Structures ✅
- [x] **4.4.1–4.4.4**: Repository, implementation, routes, schemas
- CMS: CMS.INITBYDIM, CMS.INITBYPROB, CMS.INCRBY, CMS.QUERY, CMS.INFO, CMS.MERGE
- Top-K: TOPK.RESERVE, TOPK.ADD, TOPK.INCRBY, TOPK.QUERY, TOPK.COUNT, TOPK.LIST, TOPK.INFO

---

## Phase 5: New Features (Beyond Go/Node ports) ✅

### 5.1 Bitmap Operations ✅
- [x] **5.1.1–5.1.4**: Repository, implementation, routes, schemas
- Commands: SETBIT, GETBIT, BITCOUNT, BITPOS, BITOP, BITFIELD, BITFIELD_RO

### 5.2 Geospatial Operations ✅
- [x] **5.2.1–5.2.4**: Repository, implementation, routes, schemas
- Commands: GEOADD, GEODIST, GEOHASH, GEOPOS, GEOSEARCH, GEOSEARCHSTORE

### 5.3 Pub/Sub Operations ✅
- [x] **5.3.1–5.3.6**: Repository, WebSocket subscribe/psubscribe, HTTP publish, routes, schemas, OpenAPI
- Commands: PUBLISH, SUBSCRIBE, PSUBSCRIBE, UNSUBSCRIBE, PUNSUBSCRIBE, PUBSUB (CHANNELS/NUMSUB/NUMPAT/SHARDCHANNELS/SHARDNUMSUB), SPUBLISH

### 5.4 Transaction Operations ✅
- [x] **5.4.1–5.4.4**: Service, routes, schemas, OpenAPI
- Supports 60+ command types in MULTI/EXEC, CAS via Lua, HCAS for hashes

### 5.5 Lua Scripting ✅
- [x] **5.5.1–5.5.4**: Repository, implementation, routes, schemas
- Commands: EVAL, EVALSHA, EVAL_RO, EVALSHA_RO, SCRIPT LOAD/EXISTS/FLUSH/KILL/DEBUG

### 5.6 Redis Functions ✅
- [x] **5.6.1–5.6.4**: Repository, implementation, routes, schemas
- Commands: FUNCTION LOAD/LIST/DELETE/FLUSH/DUMP/RESTORE/STATS, FCALL, FCALL_RO

### 5.7 RedisTimeSeries ✅
- [x] **5.7.1–5.7.4**: Repository, implementation, routes, schemas
- Commands: TS.CREATE, TS.ADD, TS.MADD, TS.GET, TS.MGET, TS.INFO, TS.RANGE, TS.REVRANGE, TS.MRANGE, TS.MREVRANGE, TS.QUERYINDEX, TS.ALTER, TS.CREATERULE, TS.DELETERULE, TS.DEL, TS.INCRBY, TS.DECRBY

### 5.8 Redis 7.0+ List Operations ✅
- [x] **5.8.1–5.8.2**: LMPOP/BLMPOP implementation and routes

### 5.9 Command Introspection ✅
- [x] **5.9.1–5.9.2**: COMMAND COUNT/LIST/DOCS/INFO/GETKEYS implementation and routes

### 5.10 SORT / SORT_RO ✅
- [x] **5.10.1** (merged into 5.15): SORT and SORT_RO with BY, GET, LIMIT, ASC/DESC, ALPHA, STORE

### 5.11 ACL Enhancements ✅
- [x] **5.11.1–5.11.2**: ACL DRYRUN implementation and routes

### 5.12 Hash Field Expiration (Redis 7.4+) ✅
- [x] **5.12.1–5.12.4**: Repository, implementation, routes, schemas
- Commands: HEXPIRE, HPEXPIRE, HTTL, HPTTL, HPERSIST, HEXPIREAT, HPEXPIREAT, HEXPIRETIME, HPEXPIRETIME

### 5.13 Redis 8.0+ Hash Commands ✅
- [x] **5.13.1–5.13.4**: HRANDFIELD and extended hash operations

### 5.14 LCS (Longest Common Subsequence) ✅
- [x] **5.14.1–5.14.4**: Repository, implementation, routes, schemas

### 5.15 SORT / SORT_RO Operations ✅
- [x] **5.15.1–5.15.4**: Repository, implementation, routes, schemas

### 5.16 Blocking Command Policy Enforcement ✅
- [x] **5.16.1–5.16.2**: Timeout enforcer, SSE connection limits

---

## Phase 6: Admin & Server Operations ✅

### 6.1 Database Operations ✅
- [x] **6.1.1–6.1.3**: FLUSHDB, FLUSHALL, DBSIZE, SWAPDB, SELECT, MOVE, COPY + admin auth

### 6.2 Server Operations ✅
- [x] **6.2.1–6.2.2**: INFO, SERVER TIME, LASTSAVE, DEBUG OBJECT, MEMORY (USAGE/STATS/DOCTOR)

### 6.3 Configuration Operations ✅
- [x] **6.3.1–6.3.2**: CONFIG GET/SET/REWRITE/RESETSTAT

### 6.4 Persistence Operations ✅
- [x] **6.4.1–6.4.2**: SAVE, BGSAVE, BGREWRITEAOF

### 6.5 Client Operations ✅
- [x] **6.5.1–6.5.2**: CLIENT LIST/KILL/SETNAME/GETNAME/PAUSE/UNPAUSE/ID/INFO/NO-EVICT

### 6.6 Monitoring Operations ✅
- [x] **6.6.1–6.6.2**: SLOWLOG GET/LEN/RESET, LATENCY LATEST/GRAPH/HISTORY/DOCTOR

### 6.7 ACL Operations ✅
- [x] **6.7.1–6.7.2**: ACL LIST/USERS/WHOAMI/CAT/GENPASS/LOG/SETUSER/DELUSER/GETUSER/LOAD/SAVE/DRYRUN

---

## Phase 7: Cluster & Sentinel Support ✅

### 7.1 Cluster Operations ✅
- [x] **7.1.1–7.1.4**: ClusterPool, cluster-aware routing, CLUSTER INFO/NODES/SLOTS/SHARDS/KEYSLOT, E2E tests

### 7.2 Sentinel Support ✅
- [x] **7.2.1–7.2.4**: Sentinel master resolution, failover watcher, pool hot-swap, E2E tests

---

## Testing & Deployment ✅

- [x] Unit tests: 1078 tests across all services, repositories, routes, schemas, middleware
- [x] Integration tests: Docker-based Redis tests via testcontainers
- [x] E2E tests: Standalone (167 tests), Sentinel (11), Cluster (20) — all passing
- [x] CI: GitHub Actions (fmt, clippy, test, build)
- [x] Docker: Multi-stage build, distroless debian13 runtime (64MB image)
- [x] Production: Release build with LTO, graceful shutdown, Prometheus metrics, rate limiting

---

## Grand Total: ~300+ Redis commands implemented across all phases

---

## Phase 10: Redis 8.0+ Feature Support (Planned)

> Future enhancements identified from gap analysis against Redis 8.0/8.2/8.4 official documentation.

### 10.1 Vector Sets (Redis 8.0) 🔴 HIGH PRIORITY

> Headline Redis 8.0 feature — native vector similarity search for AI/ML workloads. 13 commands.

- [x] **10.1.1**: Implement VectorSet repository trait (gated by capability detection)

  | Command | Method | Description |
  |---------|--------|-------------|
  | VADD | `vadd` | Add element with vector |
  | VREM | `vrem` | Remove element |
  | VSIM | `vsim` | Query by vector similarity |
  | VCARD | `vcard` | Count elements |
  | VDIM | `vdim` | Get vector dimensionality |
  | VEMB | `vemb` | Get element's embedding vector |
  | VISMEMBER | `vismember` | Check membership |
  | VLINKS | `vlinks` | Get HNSW graph neighbors |
  | VRANDMEMBER | `vrandmember` | Random member(s) |
  | VRANGE | `vrange` | Range query |
  | VINFO | `vinfo` | Vector set metadata |
  | VGETATTR | `vgetattr` | Get JSON attributes |
  | VSETATTR | `vsetattr` | Set JSON attributes |

- [x] **10.1.2**: Create VectorSet request/response schemas with OpenAPI annotations
- [x] **10.1.3**: Create VectorSet API routes
- [x] **10.1.4**: Add E2E tests for vector set operations
- [x] **10.1.5**: Add OpenAPI documentation and capability-based route filtering

### 10.2 Hash Atomic Operations (Redis 8.0+) 🟢 COMPLETED

> Atomic hash field operations: get+delete and get/set+expire in a single round-trip.

- [x] **10.2.1**: Add HGETDEL, HGETEX, HSETEX to HashRepository
- [x] **10.2.2**: Create request/response schemas and API routes
- [x] **10.2.3**: Add unit and E2E tests

### 10.3 Search Enhancements (Redis 8.4+) 🟢 COMPLETED

- [x] **10.3.1**: Implement FT.CONFIG GET/SET
- [x] **10.3.2**: Implement FT.HYBRID (hybrid text + vector search with RRF)
- [x] **10.3.3**: Implement FT.CURSOR READ/DEL (aggregation result pagination)

### 10.4 T-Digest (Probabilistic Module) 🟡 COMPLETED

> Completes probabilistic coverage (Bloom, Cuckoo, CMS, Top-K done). Quantile estimation for latency monitoring. 14 commands. Gated by the RedisBloom module alongside CMS/TopK.

- [x] **10.4.1**: Implement T-Digest repository trait

  | Command | Method |
  |---------|--------|
  | TDIGEST.CREATE | `tdigest_create` |
  | TDIGEST.ADD | `tdigest_add` |
  | TDIGEST.QUANTILE | `tdigest_quantile` |
  | TDIGEST.CDF | `tdigest_cdf` |
  | TDIGEST.RANK | `tdigest_rank` |
  | TDIGEST.REVRANK | `tdigest_revrank` |
  | TDIGEST.BYRANK | `tdigest_byrank` |
  | TDIGEST.BYREVRANK | `tdigest_byrevrank` |
  | TDIGEST.MIN | `tdigest_min` |
  | TDIGEST.MAX | `tdigest_max` |
  | TDIGEST.INFO | `tdigest_info` |
  | TDIGEST.MERGE | `tdigest_merge` |
  | TDIGEST.RESET | `tdigest_reset` |
  | TDIGEST.TRIMMED_MEAN | `tdigest_trimmed_mean` |

- [x] **10.4.2**: Create request/response schemas and API routes
- [x] **10.4.3**: Add unit and E2E tests
- [x] **10.4.4**: Add OpenAPI documentation and capability-based route filtering

### 10.5 New String/Key Commands (Redis 8.4+) 🟢 COMPLETED

- [x] **10.5.1**: Implement MSETEX (atomic multi-key SET with shared TTL)
- [x] **10.5.2**: Implement DELEX (conditional delete by value/digest) and DIGEST (XXH3 hash)
- [x] **10.5.3**: Add unit and E2E tests

### 10.6 JSON.MSET (JSON v2.6+) 🟢 COMPLETED

- [x] **10.6.1**: Implement JSON.MSET (atomic multi-key JSON set)
- [x] **10.6.2**: Add unit and E2E tests

### 10.7 Admin/Monitoring Enhancements 🟢 COMPLETED

- [x] **10.7.1**: Implement LATENCY HISTOGRAM, MEMORY MALLOC-STATS, COMMAND GETKEYSANDFLAGS
- [x] **10.7.2**: Implement CLUSTER SLOT-STATS (Redis 8.2)

### 10.8 TimeSeries Missing Options 🟢 COMPLETED

- [x] **10.8.1**: Add IGNORE (ignoreMaxTimediff/ignoreMaxValDiff) to TS.CREATE, TS.ALTER, TS.ADD
- [x] **10.8.2**: Add alignTimestamp to TS.CREATERULE
- [x] **10.8.3**: Add ON_DUPLICATE per-sample policy to TS.ADD

### 10.9 Stream Enhancements (Redis 8.2+) 🟢 COMPLETED

- [x] **10.9.1**: Implement XACKDEL (acknowledge + delete atomically)

---

### Phase 10 Summary

| Task | Feature | Commands | Priority | Status |
|------|---------|----------|----------|--------|
| 10.1 | Vector Sets | 13 | 🔴 High | Completed |
| 10.2 | Hash Atomic Ops (HGETDEL/HGETEX/HSETEX) | 3 | 🟡 Medium-High | Completed |
| 10.3 | Search (FT.CONFIG, FT.HYBRID, FT.CURSOR) | 5 | 🟡 Medium | Completed |
| 10.4 | T-Digest | 14 | 🟡 Medium | Completed |
| 10.5 | MSETEX / DELEX / DIGEST | 3 | 🟡 Medium | Completed |
| 10.6 | JSON.MSET | 1 | 🟢 Low | Completed |
| 10.7 | Admin Monitoring Enhancements | 4 | 🟢 Low | Completed |
| 10.8 | TimeSeries Missing Options | 3 opts | 🟢 Low | Completed |
| 10.9 | Stream XACKDEL | 1 | 🟢 Low | Completed |
| **Total** | | **~47 commands** | | |

### Not Planned (Intentionally Out of Scope)

| Feature | Reason |
|---------|--------|
| Client tracking (CLIENT CACHING/GETREDIR/NO-TOUCH/TRACKING) | Server-assisted client-side caching; not REST-appropriate |
| Replica management (FAILOVER, REPLICAOF, ROLE) | Not relevant for a caching service API |
| Module management (MODULE LOAD/UNLOAD) | Admin-level server management |
| Connection-level (ECHO, HELLO, RESET, QUIT, AUTH, PING) | Handled by Redis driver |
| Internal replication (PSYNC, SYNC, REPLCONF) | Never exposed via API |
| SSUBSCRIBE (WebSocket) | Redis crate limitation |
| LOLWUT | Easter egg |
