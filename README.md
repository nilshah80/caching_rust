# Redis Caching Service

A high-performance Redis caching service built in Rust, providing a comprehensive REST API for 300+ Redis commands. Features include connection pooling, real-time streaming (SSE), WebSocket-based Pub/Sub, OpenAPI documentation, and capability-aware module support.

## Features

### Core Data Types
- **Strings**: GET, SET, MGET, MSET, INCR, DECR, APPEND, STRLEN, GETRANGE, SETRANGE, GETEX, GETDEL
- **Hashes**: HGET, HSET, HSETNX, HGETALL, HMGET, HDEL, HEXISTS, HKEYS, HVALS, HLEN, HINCRBY, HINCRBYFLOAT, HSTRLEN, HRANDFIELD, HSCAN
- **Lists**: LPUSH, RPUSH, LPOP, RPOP, LRANGE, LLEN, LINDEX, LSET, LINSERT, LREM, LTRIM, LPOS, LMOVE, LMPOP + blocking variants (BLPOP, BRPOP, BLMOVE, BLMPOP) with SSE streaming
- **Sets**: SADD, SREM, SMEMBERS, SISMEMBER, SCARD, SDIFF, SINTER, SUNION, SPOP, SRANDMEMBER
- **Sorted Sets**: ZADD, ZRANGE, ZREM, ZSCORE, ZMSCORE, ZRANK, ZREVRANK, ZCOUNT, ZCARD, ZINCRBY, ZRANGEBYSCORE, ZRANGEBYLEX, ZRANGESTORE, ZUNION, ZINTER, ZDIFF, ZPOPMIN, ZPOPMAX + blocking variants (BZPOPMIN, BZPOPMAX, BZMPOP) with SSE streaming
- **Bitmaps**: GETBIT, SETBIT, BITCOUNT, BITPOS, BITOP, BITFIELD, BITFIELD_RO
- **Geospatial**: GEOADD, GEOPOS, GEODIST, GEOHASH, GEOSEARCH, GEOSEARCHSTORE, GEORADIUS, GEORADIUSBYMEMBER
- **HyperLogLog**: PFADD, PFCOUNT, PFMERGE

### Key Management
- DEL, EXISTS, EXPIRE, TTL, PTTL, PERSIST, TYPE, RENAME, COPY, SCAN, KEYS, RANDOMKEY, TOUCH, UNLINK, DUMP, RESTORE, OBJECT

### Advanced Features
- **Streams**: Full XADD/XREAD/XRANGE support + consumer groups (XGROUP, XREADGROUP, XACK, XCLAIM, XAUTOCLAIM, XPENDING) with SSE real-time subscriptions
- **Pub/Sub**: Publish, subscribe via WebSocket, pattern subscriptions, sharded channels (Redis 7+), connection stats
- **Transactions**: MULTI/EXEC with optional WATCH, string compare-and-set (CAS), hash compare-and-set (HCAS)
- **Lua Scripting**: EVAL, EVALSHA, SCRIPT LOAD/EXISTS/FLUSH/KILL/DEBUG
- **Redis Functions**: FUNCTION LOAD/LIST/DELETE/FLUSH/CALL/DUMP/RESTORE/STATS/KILL (Redis 7+)

### Redis Modules (auto-detected)
- **RedisJSON**: JSON.SET, JSON.GET, JSON.DEL, JSON.MGET, JSON.MSET, array/object operations, numeric operations
- **RediSearch**: FT.CREATE, FT.SEARCH, FT.AGGREGATE, FT.PROFILE, aliases, suggestions, synonyms, dictionaries, spellcheck
- **Bloom Filters**: BF.RESERVE, BF.ADD, BF.EXISTS, BF.INSERT, BF.INFO, BF.CARD, BF.SCANDUMP, BF.LOADCHUNK
- **Cuckoo Filters**: CF.RESERVE, CF.ADD, CF.ADDNX, CF.EXISTS, CF.INSERT, CF.INSERTNX, CF.DEL, CF.COUNT, CF.INFO
- **Count-Min Sketch**: CMS.INITBYDIM, CMS.INITBYPROB, CMS.INCRBY, CMS.QUERY, CMS.MERGE, CMS.INFO
- **Top-K**: TOPK.RESERVE, TOPK.ADD, TOPK.INCRBY, TOPK.QUERY, TOPK.COUNT, TOPK.LIST, TOPK.INFO
- **RedisTimeSeries**: TS.CREATE, TS.ADD, TS.MADD, TS.GET, TS.MGET, TS.RANGE, TS.MRANGE, TS.INCRBY, TS.DECRBY, TS.ALTER, TS.CREATERULE, TS.DELETERULE, TS.QUERYINDEX

### Operations & Infrastructure
- **Admin**: Server info, memory stats, client management, slowlog, latency monitoring, ACL, persistence, config management
- **Connection Pooling**: Instrumented deadpool-redis pool with configurable sizes and timeouts
- **OpenAPI Documentation**: Swagger UI at `/swagger-ui`
- **Health Checks**: Liveness (`/health/live`) and readiness (`/health/ready`) probes
- **API Key Authentication**: Protected admin and stream management endpoints
- **Capability Detection**: Auto-detects Redis version and loaded modules at startup

## Architecture

The service follows a clean layered architecture:

```
src/
├── api/http/           # HTTP layer (Axum routes, middleware, schemas, OpenAPI)
├── application/        # Application services (business logic, validation)
├── domain/             # Domain entities, errors, repository traits
├── infrastructure/     # Redis implementation, config, logging, connection pool
└── shared/             # Shared utilities (app state, response types, blocking)
```

**Key Design Principles:**
1. **Repository Pattern** — domain layer defines traits, infrastructure implements them against Redis
2. **Service Layer** — business logic, validation, and timeout enforcement in application services
3. **Capability-Driven Routing** — modules (JSON, Search, Bloom, TimeSeries) and features (Streams, Functions) are conditionally mounted based on detected Redis capabilities
4. **Dedicated Pub/Sub Connections** — subscriptions use separate connections, not the command pool
5. **Bounded Blocking** — all blocking commands (BLPOP, BRPOP, XREAD BLOCK, etc.) enforce a configurable max timeout

For detailed architectural decisions, see [docs/architecture.md](docs/architecture.md).

## Quick Start

### Prerequisites

- Docker and Docker Compose
- Rust 1.95+ (for local development)

### Running with Docker

```bash
# Start the service with Redis 8
docker-compose up -d

# Check health
curl http://localhost:8080/health

# View Swagger UI
open http://localhost:8080/swagger-ui

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

## API Reference

### Health Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/health/ready` | Readiness probe |
| GET | `/health/live` | Liveness probe |

### String Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/strings/{key}` | Get string value |
| PUT | `/api/v1/strings/{key}` | Set string value |
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
| POST | `/api/v1/strings/lcs` | Longest common subsequence (Redis 7+) |

### Hash Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/hashes/{key}` | Get all fields (HGETALL) |
| PUT | `/api/v1/hashes/{key}` | Set fields (HSET) |
| GET | `/api/v1/hashes/{key}/fields/{field}` | Get field value (HGET) |
| POST | `/api/v1/hashes/{key}/set-nx` | Set field if not exists (HSETNX) |
| POST | `/api/v1/hashes/{key}/fields/get` | Get multiple fields (HMGET) |
| DELETE | `/api/v1/hashes/{key}/fields` | Delete fields (HDEL) |
| GET | `/api/v1/hashes/{key}/fields/{field}/exists` | Check field exists (HEXISTS) |
| GET | `/api/v1/hashes/{key}/keys` | Get field names (HKEYS) |
| GET | `/api/v1/hashes/{key}/values` | Get field values (HVALS) |
| GET | `/api/v1/hashes/{key}/length` | Get field count (HLEN) |
| PATCH | `/api/v1/hashes/{key}/fields/{field}/incr` | Increment field (HINCRBY) |
| PATCH | `/api/v1/hashes/{key}/fields/{field}/incr-float` | Increment float (HINCRBYFLOAT) |
| GET | `/api/v1/hashes/{key}/fields/{field}/length` | Get field string length (HSTRLEN) |
| GET | `/api/v1/hashes/{key}/random` | Get random field (HRANDFIELD) |
| GET | `/api/v1/hashes/{key}/scan` | Scan fields (HSCAN) |

**Hash Field Expiration** (Redis 7.4+):

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/hashes/{key}/fields/expire` | Set field expiry in seconds (HEXPIRE) |
| POST | `/api/v1/hashes/{key}/fields/pexpire` | Set field expiry in ms (HPEXPIRE) |
| POST | `/api/v1/hashes/{key}/fields/expireat` | Set field expiry as unix timestamp (HEXPIREAT) |
| POST | `/api/v1/hashes/{key}/fields/pexpireat` | Set field expiry as unix ms timestamp (HPEXPIREAT) |
| POST | `/api/v1/hashes/{key}/fields/expiretime` | Get field expiry time (HEXPIRETIME) |
| POST | `/api/v1/hashes/{key}/fields/pexpiretime` | Get field expiry ms time (HPEXPIRETIME) |
| POST | `/api/v1/hashes/{key}/fields/ttl` | Get field TTL in seconds (HTTL) |
| POST | `/api/v1/hashes/{key}/fields/pttl` | Get field TTL in ms (HPTTL) |
| POST | `/api/v1/hashes/{key}/fields/persist` | Remove field expiry (HPERSIST) |

**Redis 8.0+ Hash Commands**:

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/hashes/{key}/getex` | Get fields with expiry update (HGETEX) |
| POST | `/api/v1/hashes/{key}/setex` | Set fields with expiry (HSETEX) |
| POST | `/api/v1/hashes/{key}/getdel` | Get and delete fields (HGETDEL) |

### List Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/lists/{key}/lpush` | Push to head (LPUSH) |
| POST | `/api/v1/lists/{key}/rpush` | Push to tail (RPUSH) |
| POST | `/api/v1/lists/{key}/lpushx` | Push to head if exists (LPUSHX) |
| POST | `/api/v1/lists/{key}/rpushx` | Push to tail if exists (RPUSHX) |
| POST | `/api/v1/lists/{key}/lpop` | Pop from head (LPOP) |
| POST | `/api/v1/lists/{key}/rpop` | Pop from tail (RPOP) |
| GET | `/api/v1/lists/{key}/range` | Get range (LRANGE) |
| GET | `/api/v1/lists/{key}/length` | Get length (LLEN) |
| GET | `/api/v1/lists/{key}/index` | Get by index (LINDEX) |
| PATCH | `/api/v1/lists/{key}/set` | Set by index (LSET) |
| POST | `/api/v1/lists/{key}/insert` | Insert element (LINSERT) |
| DELETE | `/api/v1/lists/{key}/remove` | Remove elements (LREM) |
| POST | `/api/v1/lists/{key}/trim` | Trim list (LTRIM) |
| GET | `/api/v1/lists/{key}/pos` | Find element position (LPOS) |
| POST | `/api/v1/lists/move` | Move element between lists (LMOVE) |
| POST | `/api/v1/lists/rpoplpush` | Pop and push (RPOPLPUSH) |
| POST | `/api/v1/lists/mpop` | Pop from multiple lists (LMPOP) |

**Blocking Operations** (return 204 on timeout):

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/lists/blpop` | Blocking left pop (BLPOP) |
| POST | `/api/v1/lists/brpop` | Blocking right pop (BRPOP) |
| POST | `/api/v1/lists/blmove` | Blocking move (BLMOVE) |
| POST | `/api/v1/lists/brpoplpush` | Blocking pop and push (BRPOPLPUSH) |
| POST | `/api/v1/lists/blmpop` | Blocking multi-pop (BLMPOP) |

**SSE Streaming** (real-time blocking via Server-Sent Events):

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/lists/{key}/blpop/stream` | Stream repeated BLPOP results |
| GET | `/api/v1/lists/{key}/brpop/stream` | Stream repeated BRPOP results |
| GET | `/api/v1/lists/blmpop/stream` | Stream repeated BLMPOP results |

### Set Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/sets/{key}/members` | Add members (SADD) |
| DELETE | `/api/v1/sets/{key}/members` | Remove members (SREM) |
| GET | `/api/v1/sets/{key}/members` | Get all members (SMEMBERS) |
| POST | `/api/v1/sets/{key}/ismember` | Check membership (SISMEMBER) |
| POST | `/api/v1/sets/{key}/mismember` | Check multiple members (SMISMEMBER) |
| GET | `/api/v1/sets/{key}/card` | Get cardinality (SCARD) |
| GET | `/api/v1/sets/{key}/random` | Random members (SRANDMEMBER) |
| POST | `/api/v1/sets/{key}/pop` | Pop random members (SPOP) |
| POST | `/api/v1/sets/move` | Move member between sets (SMOVE) |
| POST | `/api/v1/sets/inter` | Intersection (SINTER) |
| POST | `/api/v1/sets/interstore` | Intersection and store (SINTERSTORE) |
| POST | `/api/v1/sets/intercard` | Intersection cardinality (SINTERCARD) |
| POST | `/api/v1/sets/union` | Union (SUNION) |
| POST | `/api/v1/sets/unionstore` | Union and store (SUNIONSTORE) |
| POST | `/api/v1/sets/diff` | Difference (SDIFF) |
| POST | `/api/v1/sets/diffstore` | Difference and store (SDIFFSTORE) |
| GET | `/api/v1/sets/{key}/scan` | Scan members (SSCAN) |

### Sorted Set Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/sorted-sets/{key}/members` | Add members with scores (ZADD) |
| DELETE | `/api/v1/sorted-sets/{key}/members` | Remove members (ZREM) |
| POST | `/api/v1/sorted-sets/{key}/incr` | Add with increment (ZADD INCR) |
| GET | `/api/v1/sorted-sets/{key}/score/{member}` | Get score (ZSCORE) |
| POST | `/api/v1/sorted-sets/{key}/mscore` | Get multiple scores (ZMSCORE) |
| POST | `/api/v1/sorted-sets/{key}/incrby` | Increment score (ZINCRBY) |
| GET | `/api/v1/sorted-sets/{key}/card` | Get cardinality (ZCARD) |
| POST | `/api/v1/sorted-sets/{key}/count` | Count in score range (ZCOUNT) |
| POST | `/api/v1/sorted-sets/{key}/lexcount` | Count in lex range (ZLEXCOUNT) |
| GET | `/api/v1/sorted-sets/{key}/rank/{member}` | Get rank (ZRANK) |
| GET | `/api/v1/sorted-sets/{key}/revrank/{member}` | Get reverse rank (ZREVRANK) |
| GET | `/api/v1/sorted-sets/{key}/range` | Get range (ZRANGE) |
| POST | `/api/v1/sorted-sets/{key}/rangebyscore` | Get by score range (ZRANGEBYSCORE) |
| POST | `/api/v1/sorted-sets/{key}/rangebylex` | Get by lex range (ZRANGEBYLEX) |
| POST | `/api/v1/sorted-sets/{key}/rangestore` | Store range result (ZRANGESTORE) |
| POST | `/api/v1/sorted-sets/{key}/remrangebyrank` | Remove by rank range (ZREMRANGEBYRANK) |
| POST | `/api/v1/sorted-sets/{key}/remrangebyscore` | Remove by score range (ZREMRANGEBYSCORE) |
| POST | `/api/v1/sorted-sets/{key}/remrangebylex` | Remove by lex range (ZREMRANGEBYLEX) |
| POST | `/api/v1/sorted-sets/{key}/popmin` | Pop min score members (ZPOPMIN) |
| POST | `/api/v1/sorted-sets/{key}/popmax` | Pop max score members (ZPOPMAX) |
| GET | `/api/v1/sorted-sets/{key}/random` | Random members (ZRANDMEMBER) |
| POST | `/api/v1/sorted-sets/union` | Union (ZUNION) |
| POST | `/api/v1/sorted-sets/unionstore` | Union and store (ZUNIONSTORE) |
| POST | `/api/v1/sorted-sets/inter` | Intersection (ZINTER) |
| POST | `/api/v1/sorted-sets/interstore` | Intersect and store (ZINTERSTORE) |
| POST | `/api/v1/sorted-sets/intercard` | Intersection cardinality (ZINTERCARD) |
| POST | `/api/v1/sorted-sets/diff` | Difference (ZDIFF) |
| POST | `/api/v1/sorted-sets/diffstore` | Diff and store (ZDIFFSTORE) |
| GET | `/api/v1/sorted-sets/{key}/scan` | Scan members (ZSCAN) |
| POST | `/api/v1/sorted-sets/zmpop` | Pop from multiple sets (ZMPOP) |

**Blocking Operations** (return 204 on timeout):

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/sorted-sets/bzpopmin` | Blocking pop min (BZPOPMIN) |
| POST | `/api/v1/sorted-sets/bzpopmax` | Blocking pop max (BZPOPMAX) |
| POST | `/api/v1/sorted-sets/bzmpop` | Blocking multi-pop (BZMPOP) |

**SSE Streaming**:

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/sorted-sets/{key}/bzpopmin/stream` | Stream BZPOPMIN results |
| GET | `/api/v1/sorted-sets/{key}/bzpopmax/stream` | Stream BZPOPMAX results |

### Bitmap Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/bitmaps/{key}/bit/{offset}` | Get bit value (GETBIT) |
| PUT | `/api/v1/bitmaps/{key}/bit/{offset}` | Set bit value (SETBIT) |
| GET | `/api/v1/bitmaps/{key}/count` | Count set bits (BITCOUNT) |
| GET | `/api/v1/bitmaps/{key}/pos` | Find first bit (BITPOS) |
| POST | `/api/v1/bitmaps/operations` | Bitwise operations (BITOP) |
| POST | `/api/v1/bitmaps/{key}/bitfield` | Bitfield operations (BITFIELD) |
| POST | `/api/v1/bitmaps/{key}/bitfield/ro` | Read-only bitfield (BITFIELD_RO) |

### Geospatial Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/geo/{key}` | Add members (GEOADD) |
| POST | `/api/v1/geo/{key}/pos` | Get positions (GEOPOS) |
| GET | `/api/v1/geo/{key}/dist/{member1}/{member2}` | Get distance (GEODIST) |
| POST | `/api/v1/geo/{key}/hash` | Get geohashes (GEOHASH) |
| POST | `/api/v1/geo/{key}/search` | Search area (GEOSEARCH) |
| POST | `/api/v1/geo/{dest_key}/searchstore` | Search and store (GEOSEARCHSTORE) |
| GET | `/api/v1/geo/{key}/radius` | Search by radius (GEORADIUS, legacy) |
| GET | `/api/v1/geo/{key}/radius/{member}` | Search around member (GEORADIUSBYMEMBER, legacy) |

### HyperLogLog Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/hll/{key}/add` | Add elements (PFADD) |
| POST | `/api/v1/hll/count` | Estimate cardinality (PFCOUNT) |
| POST | `/api/v1/hll/{key}/merge` | Merge sets (PFMERGE) |

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
| GET | `/api/v1/keys/{key}/ttl` | Get TTL |
| PATCH | `/api/v1/keys/{key}/expire` | Set expiration (EXPIRE/PEXPIRE) |
| PATCH | `/api/v1/keys/{key}/persist` | Remove expiration (PERSIST) |
| GET | `/api/v1/keys/{key}/type` | Get key type (TYPE) |
| PATCH | `/api/v1/keys/{key}/rename` | Rename key (RENAME/RENAMENX) |
| POST | `/api/v1/keys/{key}/copy` | Copy key (COPY) |
| GET | `/api/v1/keys/{key}/dump` | Serialize key (DUMP) |
| POST | `/api/v1/keys/{key}/restore` | Deserialize key (RESTORE) |
| GET | `/api/v1/keys/{key}/object` | Object encoding/refcount/idletime |
| POST | `/api/v1/keys/{key}/sort` | Sort key (SORT) |
| POST | `/api/v1/keys/{key}/sort/store` | Sort and store (SORT ... STORE) |
| POST | `/api/v1/keys/{key}/sort/readonly` | Read-only sort (SORT_RO) |

### Stream Operations

Requires Redis 5.0+. Conditionally enabled based on detected capabilities.

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/streams/{key}/add` | Add entry (XADD) |
| GET | `/api/v1/streams/{key}/length` | Get length (XLEN) |
| GET | `/api/v1/streams/{key}/range` | Get range (XRANGE) |
| GET | `/api/v1/streams/{key}/revrange` | Get reverse range (XREVRANGE) |
| DELETE | `/api/v1/streams/{key}/entries` | Delete entries (XDEL) |
| POST | `/api/v1/streams/{key}/trim` | Trim stream (XTRIM) |
| GET | `/api/v1/streams/{key}/info` | Stream info (XINFO STREAM) |
| POST | `/api/v1/streams/read` | Read entries (XREAD) |
| POST | `/api/v1/streams/read/blocking` | Blocking read (XREAD BLOCK) |
| GET | `/api/v1/streams/{key}/subscribe` | **SSE** Subscribe to stream |

**Consumer Groups:**

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/streams/{key}/groups` | List groups (XINFO GROUPS) |
| GET | `/api/v1/streams/{key}/groups/{group}/consumers` | List consumers (XINFO CONSUMERS) |
| POST | `/api/v1/streams/{key}/groups/{group}/read` | Read with group (XREADGROUP) |
| POST | `/api/v1/streams/{key}/groups/{group}/read/blocking` | Blocking group read |
| POST | `/api/v1/streams/{key}/groups/{group}/ack` | Acknowledge (XACK) |
| GET | `/api/v1/streams/{key}/groups/{group}/pending` | Pending summary (XPENDING) |
| GET | `/api/v1/streams/{key}/groups/{group}/pending/detail` | Pending detail (XPENDING) |
| POST | `/api/v1/streams/{key}/groups/{group}/claim` | Claim entries (XCLAIM) |
| POST | `/api/v1/streams/{key}/groups/{group}/autoclaim` | Auto-claim idle (XAUTOCLAIM) |
| GET | `/api/v1/streams/{key}/groups/{group}/subscribe` | **SSE** Subscribe with group |

**Stream Admin** (requires `X-Admin-Api-Key`):

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/streams/{key}/groups` | Create group (XGROUP CREATE) |
| DELETE | `/api/v1/streams/{key}/groups/{group}` | Delete group (XGROUP DESTROY) |
| POST | `/api/v1/streams/{key}/groups/{group}/setid` | Set group ID (XGROUP SETID) |
| POST | `/api/v1/streams/{key}/groups/{group}/consumers` | Create consumer (XGROUP CREATECONSUMER) |
| DELETE | `/api/v1/streams/{key}/groups/{group}/consumers/{consumer}` | Delete consumer (XGROUP DELCONSUMER) |
| POST | `/api/v1/streams/{key}/setid` | Set stream last ID (XSETID) |

### Pub/Sub Operations

WebSocket-based subscriptions with dedicated connections (not from the command pool).

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/pubsub/publish` | Publish message to channel |
| GET | `/api/v1/pubsub/channels` | List active channels |
| POST | `/api/v1/pubsub/numsub` | Subscriber counts for channels |
| GET | `/api/v1/pubsub/numpat` | Total pattern subscriptions |
| GET | `/api/v1/pubsub/stats` | Pub/Sub connection statistics |
| POST | `/api/v1/pubsub/spublish` | Publish to sharded channel (Redis 7+) |
| GET | `/api/v1/pubsub/shardchannels` | List active sharded channels |
| POST | `/api/v1/pubsub/shardnumsub` | Sharded subscriber counts |
| GET | `/api/v1/pubsub/subscribe` | **WebSocket** Subscribe to channels |
| GET | `/api/v1/pubsub/psubscribe` | **WebSocket** Pattern subscribe |
| GET | `/api/v1/pubsub/ssubscribe` | **WebSocket** Sharded channel subscribe (Redis 7+) |

### Transaction Operations

All commands in a transaction are bundled in a single HTTP request (stateless, no session required).

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/transactions/execute` | Execute MULTI/EXEC (optional WATCH) |
| POST | `/api/v1/transactions/cas` | String compare-and-set (Lua-based) |
| POST | `/api/v1/transactions/hcas` | Hash field compare-and-set (Lua-based) |

### Scripting Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/scripts/eval` | Evaluate Lua script (EVAL/EVAL_RO) |
| POST | `/api/v1/scripts/evalsha` | Evaluate by SHA (EVALSHA) |
| POST | `/api/v1/scripts/load` | Load script (SCRIPT LOAD) |
| POST | `/api/v1/scripts/exists` | Check scripts exist (SCRIPT EXISTS) |
| POST | `/api/v1/scripts/flush` | Flush script cache (SCRIPT FLUSH) |
| POST | `/api/v1/scripts/kill` | Kill running script (SCRIPT KILL) |
| POST | `/api/v1/scripts/debug` | Set debug mode (SCRIPT DEBUG) |

### Redis Functions (Redis 7+)

Conditionally enabled based on detected Redis version.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/functions` | List functions (FUNCTION LIST) |
| POST | `/api/v1/functions/load` | Load library (FUNCTION LOAD) |
| DELETE | `/api/v1/functions/{name}` | Delete library |
| POST | `/api/v1/functions/flush` | Flush all functions (FUNCTION FLUSH) |
| POST | `/api/v1/functions/call` | Call function (FCALL/FCALL_RO) |
| GET | `/api/v1/functions/dump` | Dump functions (FUNCTION DUMP) |
| POST | `/api/v1/functions/restore` | Restore functions (FUNCTION RESTORE) |
| GET | `/api/v1/functions/stats` | Function stats (FUNCTION STATS) |
| POST | `/api/v1/functions/kill` | Kill running function (FUNCTION KILL) |

### RedisJSON Operations

Requires the RedisJSON module. Conditionally enabled.

| Method | Endpoint | Description |
|--------|----------|-------------|
| PUT | `/api/v1/json/{key}` | Set JSON value (JSON.SET) |
| GET | `/api/v1/json/{key}` | Get JSON value (JSON.GET) |
| DELETE | `/api/v1/json/{key}` | Delete JSON path (JSON.DEL) |
| POST | `/api/v1/json/mget` | Multi-get (JSON.MGET) |
| POST | `/api/v1/json/mset` | Multi-set (JSON.MSET) |
| GET | `/api/v1/json/{key}/type` | Get value type (JSON.TYPE) |
| GET | `/api/v1/json/{key}/strlen` | String length (JSON.STRLEN) |
| PATCH | `/api/v1/json/{key}/strappend` | Append string (JSON.STRAPPEND) |
| PATCH | `/api/v1/json/{key}/numincrby` | Increment number (JSON.NUMINCRBY) |
| PATCH | `/api/v1/json/{key}/nummultby` | Multiply number (JSON.NUMMULTBY) |
| PATCH | `/api/v1/json/{key}/toggle` | Toggle boolean (JSON.TOGGLE) |
| POST | `/api/v1/json/{key}/clear` | Clear value (JSON.CLEAR) |
| GET | `/api/v1/json/{key}/arrlen` | Array length (JSON.ARRLEN) |
| POST | `/api/v1/json/{key}/arrappend` | Array append (JSON.ARRAPPEND) |
| POST | `/api/v1/json/{key}/arrindex` | Array search (JSON.ARRINDEX) |
| POST | `/api/v1/json/{key}/arrinsert` | Array insert (JSON.ARRINSERT) |
| DELETE | `/api/v1/json/{key}/arrpop` | Array pop (JSON.ARRPOP) |
| POST | `/api/v1/json/{key}/arrtrim` | Array trim (JSON.ARRTRIM) |
| GET | `/api/v1/json/{key}/objlen` | Object length (JSON.OBJLEN) |
| GET | `/api/v1/json/{key}/objkeys` | Object keys (JSON.OBJKEYS) |
| GET | `/api/v1/json/{key}/debug/memory` | Memory usage (JSON.DEBUG MEMORY) |
| GET | `/api/v1/json/{key}/resp` | RESP format (JSON.RESP) |

### RediSearch Operations

Requires the RediSearch module. Conditionally enabled.

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/search/indices` | Create index (FT.CREATE) |
| GET | `/api/v1/search/indices` | List indices (FT._LIST) |
| GET | `/api/v1/search/indices/{index}` | Index info (FT.INFO) |
| DELETE | `/api/v1/search/indices/{index}` | Drop index (FT.DROPINDEX) |
| POST | `/api/v1/search/indices/{index}/fields` | Alter index (FT.ALTER) |
| POST | `/api/v1/search/indices/{index}/search` | Search (FT.SEARCH) |
| POST | `/api/v1/search/indices/{index}/aggregate` | Aggregate (FT.AGGREGATE) |
| POST | `/api/v1/search/indices/{index}/explain` | Explain query (FT.EXPLAIN) |
| POST | `/api/v1/search/indices/{index}/profile` | Profile query (FT.PROFILE) |
| POST | `/api/v1/search/aliases` | Add alias (FT.ALIASADD) |
| DELETE | `/api/v1/search/aliases/{alias}` | Delete alias (FT.ALIASDEL) |
| PUT | `/api/v1/search/aliases/{alias}` | Update alias (FT.ALIASUPDATE) |
| POST | `/api/v1/search/suggest/{key}` | Add suggestion (FT.SUGADD) |
| GET | `/api/v1/search/suggest/{key}` | Get suggestions (FT.SUGGET) |
| DELETE | `/api/v1/search/suggest/{key}` | Delete suggestion (FT.SUGDEL) |
| GET | `/api/v1/search/suggest/{key}/len` | Suggestion count (FT.SUGLEN) |
| GET | `/api/v1/search/indices/{index}/synonyms` | Dump synonyms (FT.SYNDUMP) |
| PUT | `/api/v1/search/indices/{index}/synonyms/{group}` | Update synonyms (FT.SYNUPDATE) |
| POST | `/api/v1/search/indices/{index}/spellcheck` | Spellcheck (FT.SPELLCHECK) |
| POST | `/api/v1/search/dicts/{dict}/terms` | Add to dictionary (FT.DICTADD) |
| DELETE | `/api/v1/search/dicts/{dict}/terms` | Remove from dictionary (FT.DICTDEL) |
| GET | `/api/v1/search/dicts/{dict}/terms` | Dump dictionary (FT.DICTDUMP) |

### Bloom Filter Operations

Requires the RedisBloom module. Conditionally enabled.

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/bloom/{key}` | Reserve filter (BF.RESERVE) |
| GET | `/api/v1/bloom/{key}` | Filter info (BF.INFO) |
| POST | `/api/v1/bloom/{key}/add` | Add items (BF.ADD/BF.MADD) |
| POST | `/api/v1/bloom/{key}/exists` | Check items (BF.EXISTS/BF.MEXISTS) |
| POST | `/api/v1/bloom/{key}/insert` | Insert with auto-create (BF.INSERT) |
| GET | `/api/v1/bloom/{key}/card` | Cardinality (BF.CARD) |
| GET | `/api/v1/bloom/{key}/scandump` | Incremental save (BF.SCANDUMP) |
| POST | `/api/v1/bloom/{key}/loadchunk` | Incremental restore (BF.LOADCHUNK) |

### Cuckoo Filter Operations

Requires the RedisBloom module. Conditionally enabled.

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/cuckoo/{key}` | Reserve filter (CF.RESERVE) |
| GET | `/api/v1/cuckoo/{key}` | Filter info (CF.INFO) |
| POST | `/api/v1/cuckoo/{key}/add` | Add item (CF.ADD) |
| POST | `/api/v1/cuckoo/{key}/addnx` | Add if not exists (CF.ADDNX) |
| POST | `/api/v1/cuckoo/{key}/exists` | Check items (CF.EXISTS/CF.MEXISTS) |
| POST | `/api/v1/cuckoo/{key}/insert` | Insert items (CF.INSERT) |
| POST | `/api/v1/cuckoo/{key}/insertnx` | Insert if not exists (CF.INSERTNX) |
| DELETE | `/api/v1/cuckoo/{key}/del` | Delete item (CF.DEL) |
| POST | `/api/v1/cuckoo/{key}/count` | Count item (CF.COUNT) |
| GET | `/api/v1/cuckoo/{key}/scandump` | Incremental save (CF.SCANDUMP) |
| POST | `/api/v1/cuckoo/{key}/loadchunk` | Incremental restore (CF.LOADCHUNK) |

### Count-Min Sketch Operations

Requires the RedisBloom module. Conditionally enabled.

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/cms/{key}/initbydim` | Init by dimensions (CMS.INITBYDIM) |
| POST | `/api/v1/cms/{key}/initbyprob` | Init by probability (CMS.INITBYPROB) |
| POST | `/api/v1/cms/{key}/incrby` | Increment counts (CMS.INCRBY) |
| POST | `/api/v1/cms/{key}/query` | Query counts (CMS.QUERY) |
| POST | `/api/v1/cms/{key}/merge` | Merge sketches (CMS.MERGE) |
| GET | `/api/v1/cms/{key}` | Sketch info (CMS.INFO) |

### Top-K Operations

Requires the RedisBloom module. Conditionally enabled.

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/topk/{key}` | Reserve (TOPK.RESERVE) |
| GET | `/api/v1/topk/{key}` | Info (TOPK.INFO) |
| POST | `/api/v1/topk/{key}/add` | Add items (TOPK.ADD) |
| POST | `/api/v1/topk/{key}/incrby` | Increment items (TOPK.INCRBY) |
| POST | `/api/v1/topk/{key}/query` | Query items (TOPK.QUERY) |
| POST | `/api/v1/topk/{key}/count` | Count items (TOPK.COUNT) |
| GET | `/api/v1/topk/{key}/list` | List top items (TOPK.LIST) |

### RedisTimeSeries Operations

Requires the RedisTimeSeries module. Conditionally enabled.

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/timeseries` | Create time series (TS.CREATE) |
| GET | `/api/v1/timeseries/{key}` | Get latest sample (TS.GET) |
| PATCH | `/api/v1/timeseries/{key}` | Alter time series (TS.ALTER) |
| POST | `/api/v1/timeseries/{key}/samples` | Add sample (TS.ADD) |
| DELETE | `/api/v1/timeseries/{key}/samples` | Delete samples in range (TS.DEL) |
| GET | `/api/v1/timeseries/{key}/range` | Get range (TS.RANGE) |
| GET | `/api/v1/timeseries/{key}/revrange` | Get reverse range (TS.REVRANGE) |
| POST | `/api/v1/timeseries/{key}/incrby` | Increment (TS.INCRBY) |
| POST | `/api/v1/timeseries/{key}/decrby` | Decrement (TS.DECRBY) |
| GET | `/api/v1/timeseries/{key}/info` | Series info (TS.INFO) |
| POST | `/api/v1/timeseries/{key}/rules` | Create compaction rule (TS.CREATERULE) |
| DELETE | `/api/v1/timeseries/{key}/rules/{dest_key}` | Delete compaction rule (TS.DELETERULE) |
| POST | `/api/v1/timeseries/madd` | Multi-add (TS.MADD) |
| POST | `/api/v1/timeseries/mget` | Multi-get (TS.MGET) |
| POST | `/api/v1/timeseries/mrange` | Multi-range (TS.MRANGE) |
| POST | `/api/v1/timeseries/mrevrange` | Multi-reverse range (TS.MREVRANGE) |
| POST | `/api/v1/timeseries/queryindex` | Query by labels (TS.QUERYINDEX) |

### Admin Operations

Admin endpoints require the `X-Admin-Api-Key` header.

**Public** (no auth required):

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/admin/pool/stats` | Connection pool statistics |
| GET | `/api/v1/admin/capabilities` | Redis server capabilities |

**Server:**

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/admin/server/info` | Server information |
| GET | `/api/v1/admin/server/time` | Server time |
| GET | `/api/v1/admin/server/dbsize` | Database size |
| GET | `/api/v1/admin/server/lastsave` | Last save timestamp |

**Memory:**

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/admin/server/memory/stats` | Memory statistics |
| POST | `/api/v1/admin/server/memory/usage` | Memory usage for key |
| GET | `/api/v1/admin/server/memory/doctor` | Memory doctor report |
| POST | `/api/v1/admin/server/memory/purge` | Purge memory |

**Database:**

| Method | Endpoint | Description |
|--------|----------|-------------|
| DELETE | `/api/v1/admin/db/flush` | Flush current database |
| DELETE | `/api/v1/admin/db/flushall` | Flush all databases |
| POST | `/api/v1/admin/db/copy` | Copy key |
| POST | `/api/v1/admin/db/move` | Move key to another DB |
| POST | `/api/v1/admin/db/swapdb` | Swap databases |

**Configuration:**

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/admin/config/get` | Get configuration |
| POST | `/api/v1/admin/config/set` | Set configuration |
| POST | `/api/v1/admin/config/rewrite` | Rewrite config file |
| POST | `/api/v1/admin/config/resetstat` | Reset statistics |

**Persistence:**

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/admin/persistence/save` | Synchronous save |
| POST | `/api/v1/admin/persistence/bgsave` | Background save |
| POST | `/api/v1/admin/persistence/bgrewriteaof` | Rewrite AOF |

**Client Management:**

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/admin/client/list` | List clients |
| POST | `/api/v1/admin/client/kill` | Kill client |
| POST | `/api/v1/admin/client/pause` | Pause clients |
| POST | `/api/v1/admin/client/unpause` | Unpause clients |
| POST | `/api/v1/admin/client/setname` | Set client name |
| GET | `/api/v1/admin/client/getname` | Get client name |
| GET | `/api/v1/admin/client/id` | Get client ID |

**Slowlog & Latency:**

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/admin/slowlog/get` | Get slowlog entries |
| GET | `/api/v1/admin/slowlog/len` | Slowlog length |
| POST | `/api/v1/admin/slowlog/reset` | Reset slowlog |
| GET | `/api/v1/admin/latency/latest` | Latest latency events |
| POST | `/api/v1/admin/latency/history` | Latency history |
| GET | `/api/v1/admin/latency/doctor` | Latency doctor |
| POST | `/api/v1/admin/latency/reset` | Reset latency data |

**ACL (Access Control):**

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/admin/acl/list` | List ACL rules |
| GET | `/api/v1/admin/acl/users` | List ACL users |
| GET | `/api/v1/admin/acl/whoami` | Current user |
| POST | `/api/v1/admin/acl/cat` | ACL categories |
| POST | `/api/v1/admin/acl/genpass` | Generate password |
| POST | `/api/v1/admin/acl/log` | ACL log |

## Configuration

All configuration is loaded from environment variables using `__` (double underscore) as the section separator. A `.env` file is also supported.

### Server

| Variable | Default | Description |
|----------|---------|-------------|
| `SERVER__HOST` | `0.0.0.0` | Bind address |
| `SERVER__PORT` | `8080` | Bind port |
| `SERVER__REQUEST_TIMEOUT_MS` | `30000` | HTTP request timeout |
| `SERVER__MAX_BODY_SIZE_BYTES` | `10485760` | Max request body size (10 MiB) |
| `SERVER__MAX_BATCH_SIZE` | `1000` | Max items in batch operations |
| `SERVER__MAX_VALUE_SIZE_BYTES` | `524288` | Max value size (512 KiB) |

### Redis

| Variable | Default | Description |
|----------|---------|-------------|
| `REDIS__URL` | `redis://localhost:6379` | Redis connection URL |
| `REDIS__PASSWORD` | - | Redis password (optional) |
| `REDIS__DATABASE` | `0` | Database number |
| `REDIS__TLS_ENABLED` | `false` | Enable TLS |
| `REDIS__TLS_CERT_PATH` | - | Client certificate path |
| `REDIS__TLS_KEY_PATH` | - | Client key path |
| `REDIS__TLS_CA_PATH` | - | CA certificate path |
| `REDIS__TLS_SKIP_VERIFY` | `false` | Skip TLS certificate verification |

### Connection Pool

| Variable | Default | Description |
|----------|---------|-------------|
| `POOL__MIN_SIZE` | `2` | Minimum idle connections |
| `POOL__MAX_SIZE` | `10` | Maximum connections |
| `POOL__CONNECT_TIMEOUT_MS` | `5000` | Connection timeout |
| `POOL__COMMAND_TIMEOUT_MS` | `5000` | Command execution timeout |
| `POOL__IDLE_TIMEOUT_MS` | `600000` | Idle connection timeout (10 min) |

### Pub/Sub

| Variable | Default | Description |
|----------|---------|-------------|
| `PUBSUB__MAX_SUBSCRIPTIONS` | `100` | Max concurrent subscriptions |
| `PUBSUB__CONNECTION_TIMEOUT_MS` | `30000` | Subscription connection timeout |

### Blocking Commands

| Variable | Default | Description |
|----------|---------|-------------|
| `BLOCKING__MAX_TIMEOUT_SECONDS` | `30` | Hard cap for blocking operations |
| `BLOCKING__DEFAULT_TIMEOUT_SECONDS` | `5` | Default blocking timeout |
| `BLOCKING__MAX_SSE_CONNECTIONS` | `5` | Max concurrent SSE connections |
| `BLOCKING__DEFAULT_STREAM_READ_COUNT` | `100` | Default count for stream reads |

### Admin

| Variable | Default | Description |
|----------|---------|-------------|
| `ADMIN__API_KEY` | `changeme-admin-key` | API key for protected endpoints |

### Logging

| Variable | Default | Description |
|----------|---------|-------------|
| `LOG__LEVEL` | `info` | Log level (trace, debug, info, warn, error) |
| `LOG__FORMAT` | `json` | Log format (`json` or `pretty`) |
| `RUST_LOG` | - | Overrides `LOG__LEVEL` if set (tracing filter syntax) |

For detailed configuration guidance, see [docs/configuration.md](docs/configuration.md).

## Usage Examples

### Set and Get a String

```bash
curl -X PUT http://localhost:8080/api/v1/strings/mykey \
  -H "Content-Type: application/json" \
  -d '{"value": "hello world", "ttl_seconds": 3600}'

curl http://localhost:8080/api/v1/strings/mykey
```

### Working with Hashes

```bash
curl -X PUT http://localhost:8080/api/v1/hashes/user:1 \
  -H "Content-Type: application/json" \
  -d '{"items": {"name": "Alice", "email": "alice@example.com", "role": "admin"}}'

curl http://localhost:8080/api/v1/hashes/user:1
```

### Pub/Sub via WebSocket

```javascript
const ws = new WebSocket('ws://localhost:8080/api/v1/pubsub/subscribe?channels=news,alerts');
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  console.log(`[${msg.channel}] ${msg.message}`);
};
```

```bash
curl -X POST http://localhost:8080/api/v1/pubsub/publish \
  -H "Content-Type: application/json" \
  -d '{"channel": "news", "message": "Breaking news!"}'
```

### SSE Stream Subscription

```bash
curl -N "http://localhost:8080/api/v1/streams/events/subscribe?last_id=0"
```

Each entry is emitted as a separate `event: message` with the entry serialized as JSON:

```javascript
const es = new EventSource('http://localhost:8080/api/v1/streams/events/subscribe?last_id=0');
es.addEventListener('message', (event) => {
  const entry = JSON.parse(event.data);
  console.log(`ID: ${entry.id}`, entry.fields);
});
es.addEventListener('error', (event) => {
  console.error('Stream error:', event.data);
});
```

### Transactions

Commands use tagged variants with a `type` field in `SCREAMING_SNAKE_CASE`:

```bash
curl -X POST http://localhost:8080/api/v1/transactions/execute \
  -H "Content-Type: application/json" \
  -d '{
    "watch_keys": ["counter"],
    "commands": [
      {"type": "GET", "key": "counter"},
      {"type": "SET", "key": "counter", "value": "42"}
    ]
  }'
```

### Compare-and-Set

```bash
curl -X POST http://localhost:8080/api/v1/transactions/cas \
  -H "Content-Type: application/json" \
  -d '{"key": "version", "expected_value": "1", "new_value": "2"}'
```

### Admin Operations

```bash
curl -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/server/info

curl -H "X-Admin-Api-Key: dev-admin-key" \
  http://localhost:8080/api/v1/admin/server/memory/stats
```

### Bloom Filter

```bash
curl -X POST http://localhost:8080/api/v1/bloom/emails \
  -H "Content-Type: application/json" \
  -d '{"error_rate": 0.01, "capacity": 10000}'

curl -X POST http://localhost:8080/api/v1/bloom/emails/add \
  -H "Content-Type: application/json" \
  -d '{"items": ["user@example.com"]}'

curl -X POST http://localhost:8080/api/v1/bloom/emails/exists \
  -H "Content-Type: application/json" \
  -d '{"items": ["user@example.com", "unknown@example.com"]}'
```

### TimeSeries

```bash
curl -X POST http://localhost:8080/api/v1/timeseries \
  -H "Content-Type: application/json" \
  -d '{"key": "temperature:office", "labels": {"location": "office", "sensor": "temp1"}}'

curl -X POST http://localhost:8080/api/v1/timeseries/temperature:office/samples \
  -H "Content-Type: application/json" \
  -d '{"timestamp": 1711627200000, "value": 22.5}'

curl "http://localhost:8080/api/v1/timeseries/temperature:office/range?from=0&to=4102444800000"
```

For more examples including Python, JavaScript, and shell clients, see [examples/](examples/).

## Development

### Build

```bash
cargo build
cargo build --release
```

### Test

```bash
cargo test
cargo test -- --nocapture
cargo test test_name
```

### Integration Tests

```bash
# Start Redis first
docker-compose up -d redis

# Run integration tests
cargo test --test integration
```

### Lint

```bash
cargo clippy
cargo fmt --check
```

### Documentation

```bash
cargo doc --open
```

### Benchmarks

```bash
# Start Redis first
docker-compose up -d redis

cargo bench
```

## API Documentation

Interactive Swagger UI is available at `/swagger-ui` when the service is running.

OpenAPI spec (JSON) is available at `/api-docs/openapi.json`.

## Redis Requirements

| Feature | Minimum Redis Version |
|---------|----------------------|
| Core types (Strings, Hashes, Lists, Sets, Sorted Sets) | 2.0+ |
| Key operations (SCAN, OBJECT, etc.) | 2.8+ |
| Streams (XADD, XREAD, Consumer Groups) | 5.0+ |
| ACL support | 6.0+ |
| XAUTOCLAIM, LMPOP, ZMPOP | 6.2+ |
| Redis Functions, LCS, COMMAND DOCS | 7.0+ |
| Sharded Pub/Sub | 7.0+ |
| Hash field expiration | 7.4+ |
| RedisJSON, RediSearch, RedisBloom, TimeSeries | Redis module or Redis 8+ |

This service is tested with Redis 7.x and Redis 8.x (which includes all modules built-in).

## Documentation

- [Configuration Guide](docs/configuration.md) — detailed configuration reference with tuning advice
- [Deployment Guide](docs/deployment.md) — Docker, Docker Compose, and production deployment
- [Architecture](docs/architecture.md) — architectural decisions and design rationale
- [Examples](examples/) — client code in Python, JavaScript, and shell

## License

MIT
