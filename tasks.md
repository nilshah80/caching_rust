# Redis Caching Service - Rust Implementation Tasks

## Overview

This document contains the complete task breakdown for implementing a production-ready Redis caching service in Rust. Tasks are organized by phase and priority, with clear acceptance criteria for each.

---

## Architectural Decisions

> **Important**: This section documents key architectural decisions made to address Redis-specific challenges in an HTTP API context.

### Decision 1: Transaction Model (Single-Request Bundled)

**Problem**: Redis WATCH→MULTI→EXEC requires a single connection, but HTTP is stateless.

**Solution**: All transaction operations are bundled in a single HTTP request. No session state required.

```rust
// Single-request transaction - all commands executed atomically
struct TransactionRequest {
    commands: Vec<RedisCommand>,  // Executed within MULTI/EXEC
}

// For optimistic locking patterns, use Lua scripts instead of WATCH
struct OptimisticUpdateRequest {
    key: String,
    expected_value: serde_json::Value,
    new_value: serde_json::Value,
    // Implemented via Lua: if GET == expected then SET new else return nil
}
```

**Rationale**:
- Covers 95%+ of real-world transaction use cases
- No connection pinning or session management needed
- Stateless and horizontally scalable
- For WATCH-style CAS operations, Lua scripts provide equivalent functionality

---

### Decision 2: Pub/Sub Connection Architecture

**Problem**: Redis subscriptions require dedicated long-lived connections. Using pooled connections would exhaust the pool.

**Solution**: Separate connection management for Pub/Sub with explicit limits.

```rust
struct AppState {
    command_pool: Pool<RedisConnectionManager>,  // For normal commands (pooled)
    pubsub_manager: Arc<PubSubManager>,          // Dedicated subscription manager
}

struct PubSubManager {
    redis_url: String,
    max_subscriptions: usize,                    // Hard limit (e.g., 100)
    active_subscriptions: AtomicUsize,
    // Each WebSocket gets a NEW dedicated connection, NOT from the pool
}
```

**Configuration**:
```env
PUBSUB_MAX_SUBSCRIPTIONS=100
PUBSUB_CONNECTION_TIMEOUT_MS=30000
PUBSUB_IDLE_TIMEOUT_MS=300000
```

**Behavior**:
- WebSocket subscription requests create NEW dedicated Redis connections
- These connections are NOT taken from the command pool
- Hard limit on concurrent subscriptions (returns 503 when exceeded)
- Automatic cleanup when WebSocket disconnects

---

### Decision 3: Blocking Commands with Bounded Timeouts

**Problem**: BLPOP/BRPOP/BZPOPMIN/XREAD BLOCK can block indefinitely, causing HTTP timeouts and worker starvation.

**Solution**: Enforce maximum timeout + appropriate response semantics.

```rust
const MAX_BLOCKING_TIMEOUT_SECONDS: u32 = 30;

struct BlockingPopRequest {
    timeout_seconds: u32,  // Required, max 30s enforced
    // ... other fields
}

// Response codes:
// 200 OK + data: Item was available
// 204 No Content: Timeout reached, no data available
// 504 Gateway Timeout: Internal timeout (should not happen normally)
```

**For streaming use cases (XREAD)**: Use Server-Sent Events (SSE) endpoints.

---

### Decision 4: Redis Module Capability Detection

**Problem**: RedisJSON/Search/Bloom/TimeSeries may not be installed on all Redis instances.

**Solution**: Detect capabilities at startup and conditionally register routes.

```rust
struct RedisCapabilities {
    redis_version: String,
    has_json: bool,        // ReJSON module
    has_search: bool,      // RediSearch module
    has_bloom: bool,       // RedisBloom module
    has_timeseries: bool,  // RedisTimeSeries module
    has_graph: bool,       // RedisGraph module (deprecated)
}

// Detected via: MODULE LIST command at startup
// Routes for unavailable modules return 501 Not Implemented
```

**Endpoint**: `GET /api/v1/capabilities` exposes detected modules.

---

### Decision 5: Connection Pool Metrics with Custom Instrumentation

**Problem**: `deadpool-redis` doesn't expose wait_count/wait_duration directly.

**Solution**: Wrap pool with custom instrumentation layer.

```rust
struct InstrumentedPool {
    inner: Pool<RedisConnectionManager>,
    metrics: Arc<PoolMetrics>,
}

struct PoolMetrics {
    // From deadpool Status
    size: AtomicUsize,
    available: AtomicUsize,

    // Custom instrumentation
    total_connections_created: AtomicU64,
    total_wait_count: AtomicU64,
    total_wait_duration_ms: AtomicU64,
    current_waiting: AtomicUsize,
}

impl InstrumentedPool {
    async fn get(&self) -> Result<PooledConnection> {
        self.metrics.current_waiting.fetch_add(1, Ordering::Relaxed);
        let start = Instant::now();

        let result = self.inner.get().await;

        let wait_ms = start.elapsed().as_millis() as u64;
        self.metrics.total_wait_count.fetch_add(1, Ordering::Relaxed);
        self.metrics.total_wait_duration_ms.fetch_add(wait_ms, Ordering::Relaxed);
        self.metrics.current_waiting.fetch_sub(1, Ordering::Relaxed);

        result
    }
}
```

---

## Phase 1: Foundation & Project Setup

### 1.1 Project Initialization
- [ ] **Task 1.1.1**: Initialize Cargo project with workspace structure
  - Create `Cargo.toml` with all required dependencies
  - Set up proper feature flags for redis-rs
  - Configure Tokio runtime features
  - **Acceptance**: `cargo build` succeeds with no warnings

- [ ] **Task 1.1.2**: Set up project directory structure
  - Create all directories as per plan.md architecture
  - Create empty `mod.rs` files for each module
  - Set up `lib.rs` and `main.rs`
  - **Acceptance**: All modules are importable

- [ ] **Task 1.1.3**: Configure development environment
  - Create `.env.example` with all configuration variables
  - Create `.gitignore` for Rust projects
  - Set up `rustfmt.toml` and `clippy.toml`
  - **Acceptance**: `cargo clippy` and `cargo fmt --check` pass

- [ ] **Task 1.1.4**: Create Docker development environment
  - Create `Dockerfile` with multi-stage build
  - Create `docker-compose.yml` with Redis Stack (includes all modules)
  - Include Redis Insight for debugging
  - **Acceptance**: `docker-compose up` starts Redis and app

### 1.2 Configuration System
- [ ] **Task 1.2.1**: Implement configuration module
  - Create `Settings` struct with all config fields
  - Implement loading from environment variables
  - Implement loading from config files (optional)
  - Add validation for all config values
  - **Acceptance**: Config loads from `.env` and validates

- [ ] **Task 1.2.2**: Implement configuration types
  ```rust
  #[derive(Debug, Clone, Deserialize)]
  pub struct Settings {
      pub server: ServerConfig,
      pub redis: RedisConfig,
      pub pool: PoolConfig,
      pub pubsub: PubSubConfig,
      pub blocking: BlockingConfig,
      pub admin: AdminConfig,
  }

  #[derive(Debug, Clone, Deserialize)]
  pub struct ServerConfig {
      pub host: String,           // default: "0.0.0.0"
      pub port: u16,              // default: 8080
      pub request_timeout_ms: u64, // default: 30000
  }

  #[derive(Debug, Clone, Deserialize)]
  pub struct RedisConfig {
      pub url: String,            // default: "redis://localhost:6379"
      pub password: Option<String>,
      pub database: u8,           // default: 0
      pub tls_enabled: bool,
      pub tls_cert_path: Option<String>,
      pub tls_key_path: Option<String>,
      pub tls_ca_path: Option<String>,
      pub tls_skip_verify: bool,
  }

  #[derive(Debug, Clone, Deserialize)]
  pub struct PoolConfig {
      pub min_size: u32,          // default: 2
      pub max_size: u32,          // default: 10
      pub connect_timeout_ms: u64, // default: 5000
      pub command_timeout_ms: u64, // default: 5000
      pub idle_timeout_ms: u64,   // default: 600000
  }

  #[derive(Debug, Clone, Deserialize)]
  pub struct PubSubConfig {
      pub max_subscriptions: usize,     // default: 100
      pub connection_timeout_ms: u64,   // default: 30000
      pub idle_timeout_ms: u64,         // default: 300000
  }

  #[derive(Debug, Clone, Deserialize)]
  pub struct BlockingConfig {
      pub max_timeout_seconds: u32,     // default: 30, max enforced
      pub default_timeout_seconds: u32, // default: 5
  }

  #[derive(Debug, Clone, Deserialize)]
  pub struct AdminConfig {
      pub api_key: String,        // required for admin endpoints
  }
  ```
  - **Acceptance**: All config types serialize/deserialize correctly

### 1.3 Logging & Tracing
- [ ] **Task 1.3.1**: Set up tracing infrastructure
  - Configure `tracing-subscriber` with JSON output
  - Implement env-filter for log levels
  - Add request ID propagation
  - **Acceptance**: Structured logs appear in console

- [ ] **Task 1.3.2**: Create custom logging middleware
  - Log request method, path, status, duration
  - Include request ID in all log entries
  - Mask sensitive data (passwords, keys)
  - **Acceptance**: Each request generates structured log entry

### 1.4 Error Handling
- [ ] **Task 1.4.1**: Define domain error types
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum CacheError {
      #[error("Key not found: {0}")]
      KeyNotFound(String),

      #[error("Connection failed: {0}")]
      ConnectionFailed(String),

      #[error("Operation timeout")]
      Timeout,

      #[error("Invalid input: {0}")]
      InvalidInput(String),

      #[error("Redis error: {0}")]
      RedisError(#[from] redis::RedisError),

      #[error("Module not available: {0}")]
      ModuleNotAvailable(String),

      #[error("Subscription limit reached")]
      SubscriptionLimitReached,

      #[error("Blocking timeout - no data available")]
      BlockingTimeout,

      #[error("Transaction failed: {0}")]
      TransactionFailed(String),

      #[error("Script error: {0}")]
      ScriptError(String),

      #[error("Unauthorized")]
      Unauthorized,

      #[error("Internal error: {0}")]
      Internal(String),
  }

  impl CacheError {
      pub fn status_code(&self) -> StatusCode {
          match self {
              Self::KeyNotFound(_) => StatusCode::NOT_FOUND,
              Self::InvalidInput(_) => StatusCode::BAD_REQUEST,
              Self::Timeout | Self::BlockingTimeout => StatusCode::GATEWAY_TIMEOUT,
              Self::ModuleNotAvailable(_) => StatusCode::NOT_IMPLEMENTED,
              Self::SubscriptionLimitReached => StatusCode::SERVICE_UNAVAILABLE,
              Self::Unauthorized => StatusCode::UNAUTHORIZED,
              _ => StatusCode::INTERNAL_SERVER_ERROR,
          }
      }

      pub fn error_code(&self) -> &'static str {
          match self {
              Self::KeyNotFound(_) => "KEY_NOT_FOUND",
              Self::ConnectionFailed(_) => "CONNECTION_FAILED",
              Self::Timeout => "TIMEOUT",
              Self::InvalidInput(_) => "INVALID_INPUT",
              Self::ModuleNotAvailable(_) => "MODULE_NOT_AVAILABLE",
              Self::SubscriptionLimitReached => "SUBSCRIPTION_LIMIT_REACHED",
              Self::BlockingTimeout => "BLOCKING_TIMEOUT",
              Self::TransactionFailed(_) => "TRANSACTION_FAILED",
              Self::Unauthorized => "UNAUTHORIZED",
              _ => "INTERNAL_ERROR",
          }
      }
  }
  ```
  - **Acceptance**: All error types map to HTTP status codes

- [ ] **Task 1.4.2**: Create error response middleware
  - Convert domain errors to HTTP responses
  - Include error code, message, request_id
  - Log errors appropriately
  ```rust
  #[derive(Serialize)]
  pub struct ErrorResponse {
      pub success: bool,
      pub timestamp: DateTime<Utc>,
      pub request_id: String,
      pub error: ErrorDetail,
  }

  #[derive(Serialize)]
  pub struct ErrorDetail {
      pub code: String,
      pub message: String,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub details: Option<serde_json::Value>,
  }
  ```
  - **Acceptance**: Errors return consistent JSON format

---

## Phase 2: Redis Connection & Core Infrastructure

### 2.1 Redis Connection Pool (Command Pool)
- [ ] **Task 2.1.1**: Implement connection pool with deadpool-redis
  - Configure pool size from settings
  - Implement health check on connections
  - Handle connection failures gracefully
  - **Acceptance**: Pool creates min_size connections on startup

- [ ] **Task 2.1.2**: Implement connection manager with instrumentation
  ```rust
  pub struct InstrumentedPool {
      inner: Pool<RedisConnectionManager>,
      metrics: Arc<PoolMetrics>,
  }

  pub struct PoolMetrics {
      pub size: AtomicUsize,
      pub available: AtomicUsize,
      pub max_size: usize,
      pub total_connections_created: AtomicU64,
      pub total_wait_count: AtomicU64,
      pub total_wait_duration_ms: AtomicU64,
      pub current_waiting: AtomicUsize,
      pub failed_checkouts: AtomicU64,
  }

  impl InstrumentedPool {
      pub async fn get(&self) -> Result<PooledConnection, CacheError> {
          self.metrics.current_waiting.fetch_add(1, Ordering::Relaxed);
          self.metrics.total_wait_count.fetch_add(1, Ordering::Relaxed);
          let start = Instant::now();

          let result = self.inner.get().await;

          let wait_ms = start.elapsed().as_millis() as u64;
          self.metrics.total_wait_duration_ms.fetch_add(wait_ms, Ordering::Relaxed);
          self.metrics.current_waiting.fetch_sub(1, Ordering::Relaxed);

          match result {
              Ok(conn) => {
                  self.update_status();
                  Ok(conn)
              }
              Err(e) => {
                  self.metrics.failed_checkouts.fetch_add(1, Ordering::Relaxed);
                  Err(CacheError::ConnectionFailed(e.to_string()))
              }
          }
      }

      pub fn get_stats(&self) -> PoolStatsResponse {
          let status = self.inner.status();
          PoolStatsResponse {
              size: status.size,
              available: status.available,
              max_size: status.max_size,
              total_connections_created: self.metrics.total_connections_created.load(Ordering::Relaxed),
              total_wait_count: self.metrics.total_wait_count.load(Ordering::Relaxed),
              avg_wait_ms: self.calculate_avg_wait(),
              current_waiting: self.metrics.current_waiting.load(Ordering::Relaxed),
              failed_checkouts: self.metrics.failed_checkouts.load(Ordering::Relaxed),
          }
      }
  }
  ```
  - **Acceptance**: Pool tracks all metrics including wait times

- [ ] **Task 2.1.3**: Implement connection manager with reconnection
  - Create wrapper for redis connection
  - Add reconnection logic with exponential backoff
  - Implement connection timeouts
  - **Acceptance**: Connections recover after Redis restart

- [ ] **Task 2.1.4**: Add TLS support
  - Configure TLS from environment
  - Support custom CA certificates
  - Support skip-verify option
  - **Acceptance**: Connects to TLS-enabled Redis

### 2.2 Pub/Sub Connection Manager (Separate from Pool)
- [ ] **Task 2.2.1**: Implement dedicated Pub/Sub connection manager
  ```rust
  pub struct PubSubManager {
      redis_url: String,
      tls_config: Option<TlsConfig>,
      max_subscriptions: usize,
      connection_timeout: Duration,
      idle_timeout: Duration,
      active_subscriptions: AtomicUsize,
      metrics: Arc<PubSubMetrics>,
  }

  pub struct PubSubMetrics {
      pub active_subscriptions: AtomicUsize,
      pub total_subscriptions_created: AtomicU64,
      pub total_messages_delivered: AtomicU64,
      pub subscription_errors: AtomicU64,
  }

  impl PubSubManager {
      /// Creates a NEW dedicated connection for subscription (not from pool)
      pub async fn create_subscription(
          &self,
          channels: Vec<String>,
      ) -> Result<PubSubConnection, CacheError> {
          let current = self.active_subscriptions.load(Ordering::Relaxed);
          if current >= self.max_subscriptions {
              return Err(CacheError::SubscriptionLimitReached);
          }

          // Atomically increment, check again
          let prev = self.active_subscriptions.fetch_add(1, Ordering::SeqCst);
          if prev >= self.max_subscriptions {
              self.active_subscriptions.fetch_sub(1, Ordering::SeqCst);
              return Err(CacheError::SubscriptionLimitReached);
          }

          // Create NEW dedicated connection
          let client = redis::Client::open(self.redis_url.clone())?;
          let conn = tokio::time::timeout(
              self.connection_timeout,
              client.get_async_pubsub()
          ).await
              .map_err(|_| CacheError::Timeout)?
              .map_err(|e| CacheError::ConnectionFailed(e.to_string()))?;

          self.metrics.total_subscriptions_created.fetch_add(1, Ordering::Relaxed);

          Ok(PubSubConnection::new(
              conn,
              channels,
              self.active_subscriptions.clone(),
              self.metrics.clone(),
          ))
      }

      pub fn get_stats(&self) -> PubSubStats {
          PubSubStats {
              active_subscriptions: self.active_subscriptions.load(Ordering::Relaxed),
              max_subscriptions: self.max_subscriptions,
              total_created: self.metrics.total_subscriptions_created.load(Ordering::Relaxed),
              total_messages: self.metrics.total_messages_delivered.load(Ordering::Relaxed),
              errors: self.metrics.subscription_errors.load(Ordering::Relaxed),
          }
      }
  }

  pub struct PubSubConnection {
      inner: redis::aio::PubSub,
      channels: Vec<String>,
      active_count: Arc<AtomicUsize>,
      metrics: Arc<PubSubMetrics>,
  }

  impl Drop for PubSubConnection {
      fn drop(&mut self) {
          // Decrement active count when connection is dropped
          self.active_count.fetch_sub(1, Ordering::SeqCst);
      }
  }
  ```
  - **Acceptance**: Pub/Sub uses dedicated connections with hard limits

### 2.3 Redis Capability Detection
- [ ] **Task 2.3.1**: Implement capability detection at startup
  ```rust
  #[derive(Debug, Clone, Serialize)]
  pub struct RedisCapabilities {
      pub redis_version: String,
      pub modules: ModuleCapabilities,
      pub features: FeatureCapabilities,
      pub detected_at: DateTime<Utc>,
  }

  #[derive(Debug, Clone, Serialize)]
  pub struct ModuleCapabilities {
      pub json: bool,        // ReJSON / RedisJSON
      pub search: bool,      // RediSearch
      pub bloom: bool,       // RedisBloom (BF, CF, CMS, TopK)
      pub timeseries: bool,  // RedisTimeSeries
      pub graph: bool,       // RedisGraph (deprecated but may exist)
  }

  #[derive(Debug, Clone, Serialize)]
  pub struct FeatureCapabilities {
      pub streams: bool,     // Redis 5.0+
      pub acl: bool,         // Redis 6.0+
      pub functions: bool,   // Redis 7.0+
      pub cluster: bool,     // Cluster mode enabled
  }

  impl RedisCapabilities {
      pub async fn detect(conn: &mut Connection) -> Result<Self, CacheError> {
          // Get Redis version
          let info: String = redis::cmd("INFO").arg("server").query_async(conn).await?;
          let redis_version = Self::parse_version(&info);

          // Get loaded modules
          let modules: Vec<Vec<String>> = redis::cmd("MODULE").arg("LIST")
              .query_async(conn).await
              .unwrap_or_default();

          let module_names: Vec<String> = modules.iter()
              .filter_map(|m| m.get(1).cloned())
              .collect();

          Ok(Self {
              redis_version: redis_version.clone(),
              modules: ModuleCapabilities {
                  json: module_names.iter().any(|n| n.to_lowercase().contains("rejson")),
                  search: module_names.iter().any(|n| n.to_lowercase().contains("search")),
                  bloom: module_names.iter().any(|n| n.to_lowercase().contains("bf")),
                  timeseries: module_names.iter().any(|n| n.to_lowercase().contains("timeseries")),
                  graph: module_names.iter().any(|n| n.to_lowercase().contains("graph")),
              },
              features: FeatureCapabilities {
                  streams: Self::version_gte(&redis_version, "5.0.0"),
                  acl: Self::version_gte(&redis_version, "6.0.0"),
                  functions: Self::version_gte(&redis_version, "7.0.0"),
                  cluster: Self::detect_cluster(conn).await,
              },
              detected_at: Utc::now(),
          })
      }
  }
  ```
  - **Acceptance**: Capabilities detected at startup and cached

- [ ] **Task 2.3.2**: Implement capability-gated route registration
  ```rust
  pub fn build_router(state: AppState) -> Router {
      let capabilities = &state.capabilities;

      let mut router = Router::new()
          // Always available - core Redis types
          .merge(health_routes())
          .merge(string_routes())
          .merge(hash_routes())
          .merge(list_routes())
          .merge(set_routes())
          .merge(sorted_set_routes())
          .merge(key_routes())
          .merge(admin_routes());

      // Conditionally add stream routes (Redis 5.0+)
      if capabilities.features.streams {
          router = router.merge(stream_routes());
      }

      // Conditionally add module routes
      if capabilities.modules.json {
          router = router.merge(json_routes());
      }
      if capabilities.modules.search {
          router = router.merge(search_routes());
      }
      if capabilities.modules.bloom {
          router = router.merge(bloom_routes());
          router = router.merge(cuckoo_routes());
          router = router.merge(cms_routes());
          router = router.merge(topk_routes());
      }
      if capabilities.modules.timeseries {
          router = router.merge(timeseries_routes());
      }

      // Redis 7.0+ features
      if capabilities.features.functions {
          router = router.merge(functions_routes());
      }

      router.with_state(state)
  }
  ```
  - **Acceptance**: Unavailable module routes return 501 Not Implemented

- [ ] **Task 2.3.3**: Create capabilities endpoint
  ```rust
  // GET /api/v1/capabilities
  async fn get_capabilities(State(state): State<AppState>) -> Json<RedisCapabilities> {
      Json(state.capabilities.clone())
  }
  ```
  - **Acceptance**: Endpoint returns detected capabilities

### 2.4 HTTP Server Setup
- [ ] **Task 2.4.1**: Set up Axum HTTP server
  - Configure server with graceful shutdown
  - Add CORS middleware
  - Add request timeout middleware
  - **Acceptance**: Server starts and responds to requests

- [ ] **Task 2.4.2**: Implement router structure
  - Create router factory for all routes
  - Organize routes by feature (strings, hashes, etc.)
  - Set up nested routers for `/api/v1`
  - **Acceptance**: All route groups are accessible

- [ ] **Task 2.4.3**: Create application state
  ```rust
  #[derive(Clone)]
  pub struct AppState {
      pub command_pool: Arc<InstrumentedPool>,
      pub pubsub_manager: Arc<PubSubManager>,
      pub capabilities: Arc<RedisCapabilities>,
      pub config: Arc<Settings>,
  }
  ```
  - **Acceptance**: Handlers can access pool, pubsub manager, and config

- [ ] **Task 2.4.4**: Implement health check endpoints
  - `GET /health` - Basic health check
  - `GET /health/ready` - Readiness (Redis connected + capabilities loaded)
  - `GET /health/live` - Liveness probe
  - **Acceptance**: K8s probes pass when healthy

### 2.5 OpenAPI/Swagger Documentation
- [ ] **Task 2.5.1**: Set up utoipa for OpenAPI generation
  - Configure OpenAPI metadata
  - Add server information
  - Set up security schemes
  - **Acceptance**: OpenAPI spec generated at compile time

- [ ] **Task 2.5.2**: Integrate Swagger UI
  - Serve Swagger UI at `/swagger-ui`
  - Serve OpenAPI JSON at `/api-docs/openapi.json`
  - **Acceptance**: Interactive docs available in browser

---

## Phase 3: Core Data Types (Port from Go/Node)

### 3.1 String Operations
- [ ] **Task 3.1.1**: Implement String repository trait
  ```rust
  #[async_trait]
  pub trait StringRepository: Send + Sync {
      async fn get(&self, key: &str) -> Result<Option<StringValue>, CacheError>;
      async fn set(&self, key: &str, value: &str, opts: SetOptions) -> Result<SetResult, CacheError>;
      async fn set_nx(&self, key: &str, value: &str, ttl: Option<Duration>) -> Result<bool, CacheError>;
      async fn set_ex(&self, key: &str, value: &str, ttl: Duration) -> Result<(), CacheError>;
      async fn mget(&self, keys: &[String]) -> Result<MGetResult, CacheError>;
      async fn mset(&self, pairs: &[(String, String)]) -> Result<(), CacheError>;
      async fn mset_nx(&self, pairs: &[(String, String)]) -> Result<bool, CacheError>;
      async fn incr(&self, key: &str) -> Result<i64, CacheError>;
      async fn incr_by(&self, key: &str, delta: i64) -> Result<i64, CacheError>;
      async fn incr_by_float(&self, key: &str, delta: f64) -> Result<f64, CacheError>;
      async fn decr(&self, key: &str) -> Result<i64, CacheError>;
      async fn decr_by(&self, key: &str, delta: i64) -> Result<i64, CacheError>;
      async fn append(&self, key: &str, value: &str) -> Result<i64, CacheError>;
      async fn str_len(&self, key: &str) -> Result<i64, CacheError>;
      async fn get_range(&self, key: &str, start: i64, end: i64) -> Result<String, CacheError>;
      async fn set_range(&self, key: &str, offset: i64, value: &str) -> Result<i64, CacheError>;
      async fn get_ex(&self, key: &str, opts: GetExOptions) -> Result<Option<String>, CacheError>;
      async fn get_del(&self, key: &str) -> Result<Option<String>, CacheError>;
  }
  ```

- [ ] **Task 3.1.2**: Implement String operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | GET | `get` | High |
  | SET | `set` with options (NX, XX, EX, PX, KEEPTTL, GET) | High |
  | SETNX | `set_nx` | High |
  | SETEX | `set_ex` | High |
  | MGET | `mget` | High |
  | MSET | `mset` | High |
  | MSETNX | `mset_nx` | Medium |
  | INCR | `incr` | High |
  | INCRBY | `incr_by` | High |
  | INCRBYFLOAT | `incr_by_float` | High |
  | DECR | `decr` | High |
  | DECRBY | `decr_by` | High |
  | APPEND | `append` | Medium |
  | STRLEN | `str_len` | Medium |
  | GETRANGE | `get_range` | Medium |
  | SETRANGE | `set_range` | Medium |
  | GETEX | `get_ex` | Medium |
  | GETDEL | `get_del` | Medium |
  - **Acceptance**: All string operations pass unit tests

- [ ] **Task 3.1.3**: Create String API routes
  - `GET /api/v1/strings/:key`
  - `PUT /api/v1/strings/:key`
  - `DELETE /api/v1/strings/:key` (GETDEL)
  - `POST /api/v1/strings/mget`
  - `POST /api/v1/strings/mset`
  - `PATCH /api/v1/strings/:key/incr`
  - `PATCH /api/v1/strings/:key/append`
  - `GET /api/v1/strings/:key/range`
  - `PATCH /api/v1/strings/:key/range`
  - **Acceptance**: All routes return correct responses

- [ ] **Task 3.1.4**: Create String request/response schemas
  ```rust
  #[derive(Debug, Serialize, Deserialize, ToSchema)]
  pub struct SetStringRequest {
      pub value: String,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub ttl_seconds: Option<u64>,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub ttl_ms: Option<u64>,
      #[serde(default)]
      pub nx: bool,  // Only set if not exists
      #[serde(default)]
      pub xx: bool,  // Only set if exists
      #[serde(default)]
      pub get: bool, // Return previous value
      #[serde(default)]
      pub keep_ttl: bool,
  }

  #[derive(Debug, Serialize, ToSchema)]
  pub struct StringValue {
      pub key: String,
      pub value: String,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub ttl: Option<i64>,
      pub length: usize,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub encoding: Option<String>,
  }
  ```
  - **Acceptance**: Schemas validate inputs correctly

### 3.2 Hash Operations
- [ ] **Task 3.2.1**: Implement Hash repository trait
- [ ] **Task 3.2.2**: Implement Hash operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | HGET | `hget` | High |
  | HSET | `hset` | High |
  | HSETNX | `hset_nx` | High |
  | HGETALL | `hget_all` | High |
  | HMGET | `hmget` | High |
  | HMSET | `hmset` | High |
  | HDEL | `hdel` | High |
  | HEXISTS | `hexists` | High |
  | HKEYS | `hkeys` | Medium |
  | HVALS | `hvals` | Medium |
  | HLEN | `hlen` | Medium |
  | HINCRBY | `hincr_by` | High |
  | HINCRBYFLOAT | `hincr_by_float` | Medium |
  | HSTRLEN | `hstr_len` | Low |
  | HRANDFIELD | `hrand_field` | Low |
  | HSCAN | `hscan` | Medium |

- [ ] **Task 3.2.3**: Create Hash API routes
- [ ] **Task 3.2.4**: Create Hash request/response schemas

### 3.3 List Operations (with Blocking Command Support)
- [ ] **Task 3.3.1**: Implement List repository trait
  ```rust
  #[async_trait]
  pub trait ListRepository: Send + Sync {
      // Non-blocking operations
      async fn lpush(&self, key: &str, values: &[String]) -> Result<i64, CacheError>;
      async fn rpush(&self, key: &str, values: &[String]) -> Result<i64, CacheError>;
      async fn lpop(&self, key: &str, count: Option<u32>) -> Result<Vec<String>, CacheError>;
      async fn rpop(&self, key: &str, count: Option<u32>) -> Result<Vec<String>, CacheError>;
      async fn lrange(&self, key: &str, start: i64, stop: i64) -> Result<Vec<String>, CacheError>;
      async fn llen(&self, key: &str) -> Result<i64, CacheError>;
      // ... other non-blocking operations

      // Blocking operations with bounded timeout
      async fn blpop(&self, keys: &[String], timeout: Duration) -> Result<Option<(String, String)>, CacheError>;
      async fn brpop(&self, keys: &[String], timeout: Duration) -> Result<Option<(String, String)>, CacheError>;
      async fn blmove(&self, source: &str, dest: &str, src_dir: Direction, dst_dir: Direction, timeout: Duration) -> Result<Option<String>, CacheError>;
  }
  ```

- [ ] **Task 3.3.2**: Implement List operations (non-blocking)
  | Command | Method | Priority |
  |---------|--------|----------|
  | LPUSH | `lpush` | High |
  | RPUSH | `rpush` | High |
  | LPUSHX | `lpush_x` | Medium |
  | RPUSHX | `rpush_x` | Medium |
  | LPOP | `lpop` | High |
  | RPOP | `rpop` | High |
  | RPOPLPUSH | `rpop_lpush` | Medium |
  | LMOVE | `lmove` | Medium |
  | LMPOP | `lmpop` | Low |
  | LLEN | `llen` | High |
  | LRANGE | `lrange` | High |
  | LINDEX | `lindex` | Medium |
  | LSET | `lset` | Medium |
  | LINSERT | `linsert` | Medium |
  | LREM | `lrem` | Medium |
  | LTRIM | `ltrim` | Medium |
  | LPOS | `lpos` | Low |

- [ ] **Task 3.3.3**: Implement blocking List operations with timeout enforcement
  ```rust
  // Blocking operations - enforced max timeout
  const MAX_BLOCKING_TIMEOUT: Duration = Duration::from_secs(30);

  pub async fn blpop(
      State(state): State<AppState>,
      Path(key): Path<String>,
      Json(req): Json<BlockingPopRequest>,
  ) -> Result<Response, CacheError> {
      // Enforce maximum timeout
      let timeout = Duration::from_secs(
          req.timeout_seconds.min(state.config.blocking.max_timeout_seconds) as u64
      );

      let mut conn = state.command_pool.get().await?;

      // Execute with slightly longer HTTP timeout to account for network
      let result = tokio::time::timeout(
          timeout + Duration::from_secs(5),
          redis::cmd("BLPOP")
              .arg(&key)
              .arg(timeout.as_secs())
              .query_async::<Option<(String, String)>>(&mut *conn)
      ).await;

      match result {
          Ok(Ok(Some((key, value)))) => {
              Ok(Json(BlockingPopResponse { key, value }).into_response())
          }
          Ok(Ok(None)) => {
              // Redis timeout - no data available
              Ok(StatusCode::NO_CONTENT.into_response())
          }
          Ok(Err(e)) => Err(CacheError::RedisError(e)),
          Err(_) => {
              // HTTP timeout (should rarely happen)
              Err(CacheError::Timeout)
          }
      }
  }

  #[derive(Debug, Deserialize, ToSchema)]
  pub struct BlockingPopRequest {
      /// Timeout in seconds (max 30, required)
      #[validate(range(min = 1, max = 30))]
      pub timeout_seconds: u32,
  }
  ```
  | Command | Method | Priority | Notes |
  |---------|--------|----------|-------|
  | BLPOP | `blpop` | Medium | Max 30s timeout, returns 204 on timeout |
  | BRPOP | `brpop` | Medium | Max 30s timeout, returns 204 on timeout |
  | BLMOVE | `blmove` | Low | Max 30s timeout |

- [ ] **Task 3.3.4**: Create List API routes
  ```
  # Non-blocking
  POST   /api/v1/lists/:key/lpush
  POST   /api/v1/lists/:key/rpush
  POST   /api/v1/lists/:key/lpop
  POST   /api/v1/lists/:key/rpop
  GET    /api/v1/lists/:key/range
  GET    /api/v1/lists/:key/length

  # Blocking (long-poll style)
  POST   /api/v1/lists/:key/blpop   # Returns 204 on timeout
  POST   /api/v1/lists/:key/brpop   # Returns 204 on timeout
  ```

- [ ] **Task 3.3.5**: Create List request/response schemas

### 3.4 Set Operations
- [ ] **Task 3.4.1**: Implement Set repository trait
- [ ] **Task 3.4.2**: Implement Set operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | SADD | `sadd` | High |
  | SREM | `srem` | High |
  | SMEMBERS | `smembers` | High |
  | SISMEMBER | `sismember` | High |
  | SMISMEMBER | `smismember` | Medium |
  | SCARD | `scard` | High |
  | SRANDMEMBER | `srand_member` | Medium |
  | SPOP | `spop` | Medium |
  | SMOVE | `smove` | Medium |
  | SINTER | `sinter` | Medium |
  | SINTERSTORE | `sinter_store` | Medium |
  | SINTERCARD | `sinter_card` | Low |
  | SUNION | `sunion` | Medium |
  | SUNIONSTORE | `sunion_store` | Medium |
  | SDIFF | `sdiff` | Medium |
  | SDIFFSTORE | `sdiff_store` | Medium |
  | SSCAN | `sscan` | Medium |

- [ ] **Task 3.4.3**: Create Set API routes
- [ ] **Task 3.4.4**: Create Set request/response schemas

### 3.5 Sorted Set Operations (with Blocking Command Support)
- [ ] **Task 3.5.1**: Implement Sorted Set repository trait
- [ ] **Task 3.5.2**: Implement Sorted Set operations (non-blocking)
  | Command | Method | Priority |
  |---------|--------|----------|
  | ZADD | `zadd` with options (NX, XX, GT, LT, CH, INCR) | High |
  | ZREM | `zrem` | High |
  | ZRANGE | `zrange` with WITHSCORES | High |
  | ZREVRANGE | `zrev_range` | High |
  | ZRANGEBYSCORE | `zrange_by_score` | High |
  | ZREVRANGEBYSCORE | `zrev_range_by_score` | High |
  | ZRANGEBYLEX | `zrange_by_lex` | Medium |
  | ZREVRANGEBYLEX | `zrev_range_by_lex` | Medium |
  | ZSCORE | `zscore` | High |
  | ZMSCORE | `zmscore` | Medium |
  | ZRANK | `zrank` | High |
  | ZREVRANK | `zrev_rank` | High |
  | ZCARD | `zcard` | High |
  | ZCOUNT | `zcount` | Medium |
  | ZLEXCOUNT | `zlex_count` | Low |
  | ZINCRBY | `zincr_by` | High |
  | ZPOPMIN | `zpop_min` | Medium |
  | ZPOPMAX | `zpop_max` | Medium |
  | ZRANGESTORE | `zrange_store` | Low |
  | ZUNION | `zunion` | Medium |
  | ZUNIONSTORE | `zunion_store` | Medium |
  | ZINTER | `zinter` | Medium |
  | ZINTERSTORE | `zinter_store` | Medium |
  | ZINTERCARD | `zinter_card` | Low |
  | ZDIFF | `zdiff` | Medium |
  | ZDIFFSTORE | `zdiff_store` | Medium |
  | ZRANDMEMBER | `zrand_member` | Low |
  | ZSCAN | `zscan` | Medium |
  | ZREMRANGEBYRANK | `zrem_range_by_rank` | Medium |
  | ZREMRANGEBYSCORE | `zrem_range_by_score` | Medium |
  | ZREMRANGEBYLEX | `zrem_range_by_lex` | Low |

- [ ] **Task 3.5.3**: Implement blocking Sorted Set operations
  | Command | Method | Priority | Notes |
  |---------|--------|----------|-------|
  | BZPOPMIN | `bzpop_min` | Low | Max 30s timeout, returns 204 on timeout |
  | BZPOPMAX | `bzpop_max` | Low | Max 30s timeout, returns 204 on timeout |
  | ZMPOP | `zmpop` | Low | |
  | BZMPOP | `bzmpop` | Low | Max 30s timeout |

- [ ] **Task 3.5.4**: Create Sorted Set API routes
- [ ] **Task 3.5.5**: Create Sorted Set request/response schemas

### 3.6 Stream Operations (with Blocking and SSE Support)
- [ ] **Task 3.6.1**: Implement Stream repository trait
- [ ] **Task 3.6.2**: Implement Stream operations (non-blocking)
  | Command | Method | Priority |
  |---------|--------|----------|
  | XADD | `xadd` with options (MAXLEN, MINID, NOMKSTREAM) | High |
  | XRANGE | `xrange` | High |
  | XREVRANGE | `xrev_range` | High |
  | XLEN | `xlen` | High |
  | XTRIM | `xtrim` | Medium |
  | XDEL | `xdel` | Medium |
  | XGROUP CREATE | `xgroup_create` | High |
  | XGROUP DESTROY | `xgroup_destroy` | Medium |
  | XGROUP SETID | `xgroup_setid` | Low |
  | XGROUP DELCONSUMER | `xgroup_del_consumer` | Medium |
  | XGROUP CREATECONSUMER | `xgroup_create_consumer` | Medium |
  | XACK | `xack` | High |
  | XCLAIM | `xclaim` | Medium |
  | XAUTOCLAIM | `xauto_claim` | Medium |
  | XPENDING | `xpending` | High |
  | XINFO STREAM | `xinfo_stream` | High |
  | XINFO GROUPS | `xinfo_groups` | High |
  | XINFO CONSUMERS | `xinfo_consumers` | Medium |
  | XSETID | `xsetid` | Low |

- [ ] **Task 3.6.3**: Implement blocking Stream operations with SSE
  ```rust
  // XREAD with blocking - use Server-Sent Events for streaming
  pub async fn xread_stream(
      State(state): State<AppState>,
      Path(key): Path<String>,
      Query(params): Query<XReadStreamParams>,
  ) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
      let stream = async_stream::stream! {
          let mut conn = match state.command_pool.get().await {
              Ok(c) => c,
              Err(e) => {
                  yield Ok(Event::default().data(format!("error: {}", e)));
                  return;
              }
          };

          let mut last_id = params.start_id.unwrap_or_else(|| "$".to_string());
          let block_ms = params.block_ms.unwrap_or(5000).min(30000); // Max 30s per iteration

          loop {
              let result: Option<Vec<(String, Vec<(String, HashMap<String, String>)>)>> =
                  redis::cmd("XREAD")
                      .arg("COUNT").arg(params.count.unwrap_or(10))
                      .arg("BLOCK").arg(block_ms)
                      .arg("STREAMS").arg(&key).arg(&last_id)
                      .query_async(&mut *conn)
                      .await
                      .ok();

              if let Some(streams) = result {
                  for (_, entries) in streams {
                      for (id, fields) in entries {
                          last_id = id.clone();
                          let event = StreamEntry { id, fields };
                          yield Ok(Event::default()
                              .event("message")
                              .data(serde_json::to_string(&event).unwrap()));
                      }
                  }
              }

              // Check if client disconnected
              tokio::task::yield_now().await;
          }
      };

      Sse::new(stream).keep_alive(
          axum::response::sse::KeepAlive::new()
              .interval(Duration::from_secs(15))
              .text("ping")
      )
  }
  ```
  | Command | Method | Priority | Notes |
  |---------|--------|----------|-------|
  | XREAD | `xread` | High | Non-blocking version |
  | XREAD BLOCK | `xread_stream` | High | SSE endpoint for streaming |
  | XREADGROUP | `xread_group` | High | Non-blocking version |
  | XREADGROUP BLOCK | `xread_group_stream` | Medium | SSE endpoint for streaming |

- [ ] **Task 3.6.4**: Create Stream API routes
  ```
  # Non-blocking
  POST   /api/v1/streams/:key/add
  GET    /api/v1/streams/:key/range
  GET    /api/v1/streams/:key/length
  POST   /api/v1/streams/:key/read
  DELETE /api/v1/streams/:key/entries

  # Consumer groups
  POST   /api/v1/streams/:key/groups
  DELETE /api/v1/streams/:key/groups/:group
  POST   /api/v1/streams/:key/groups/:group/read
  POST   /api/v1/streams/:key/groups/:group/ack
  GET    /api/v1/streams/:key/groups/:group/pending

  # Streaming (SSE)
  GET    /api/v1/streams/:key/subscribe          # SSE stream
  GET    /api/v1/streams/:key/groups/:group/subscribe  # SSE stream
  ```

- [ ] **Task 3.6.5**: Create Stream request/response schemas

### 3.7 Key Operations
- [ ] **Task 3.7.1**: Implement Key repository trait
- [ ] **Task 3.7.2**: Implement Key operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | DEL | `del` | High |
  | EXISTS | `exists` | High |
  | EXPIRE | `expire` | High |
  | EXPIREAT | `expire_at` | High |
  | EXPIRETIME | `expire_time` | Medium |
  | TTL | `ttl` | High |
  | PTTL | `pttl` | High |
  | PERSIST | `persist` | High |
  | KEYS | `keys` (with warning for production) | Medium |
  | SCAN | `scan` | High |
  | RANDOMKEY | `random_key` | Low |
  | RENAME | `rename` | High |
  | RENAMENX | `rename_nx` | Medium |
  | TYPE | `type_of` | High |
  | OBJECT ENCODING | `object_encoding` | Medium |
  | OBJECT FREQ | `object_freq` | Low |
  | OBJECT IDLETIME | `object_idletime` | Low |
  | OBJECT REFCOUNT | `object_refcount` | Low |
  | TOUCH | `touch` | Low |
  | UNLINK | `unlink` | Medium |
  | WAIT | `wait` | Low |
  | DUMP | `dump` | Medium |
  | RESTORE | `restore` | Medium |
  | MIGRATE | `migrate` | Low |
  | SORT | `sort` | Medium |
  | SORT_RO | `sort_ro` | Low |
  | COPY | `copy` | Medium |

- [ ] **Task 3.7.3**: Create Key API routes
- [ ] **Task 3.7.4**: Create Key request/response schemas

---

## Phase 4: Redis Modules (Port from Go/Node)

> **Note**: All module routes are conditionally registered based on capability detection.
> Routes for unavailable modules return `501 Not Implemented` with error code `MODULE_NOT_AVAILABLE`.

### 4.1 RedisJSON Operations
- [ ] **Task 4.1.1**: Implement JSON repository trait (gated by `capabilities.modules.json`)
- [ ] **Task 4.1.2**: Implement JSON operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | JSON.SET | `json_set` | High |
  | JSON.GET | `json_get` | High |
  | JSON.MGET | `json_mget` | High |
  | JSON.DEL | `json_del` | High |
  | JSON.TYPE | `json_type` | Medium |
  | JSON.STRLEN | `json_str_len` | Medium |
  | JSON.ARRLEN | `json_arr_len` | Medium |
  | JSON.ARRAPPEND | `json_arr_append` | Medium |
  | JSON.ARRINDEX | `json_arr_index` | Medium |
  | JSON.ARRINSERT | `json_arr_insert` | Medium |
  | JSON.ARRPOP | `json_arr_pop` | Medium |
  | JSON.ARRTRIM | `json_arr_trim` | Medium |
  | JSON.OBJKEYS | `json_obj_keys` | Medium |
  | JSON.OBJLEN | `json_obj_len` | Medium |
  | JSON.NUMINCRBY | `json_num_incr_by` | Medium |
  | JSON.NUMMULTBY | `json_num_mult_by` | Low |
  | JSON.TOGGLE | `json_toggle` | Medium |
  | JSON.CLEAR | `json_clear` | Medium |
  | JSON.RESP | `json_resp` | Low |
  | JSON.DEBUG MEMORY | `json_debug_memory` | Low |

- [ ] **Task 4.1.3**: Create JSON API routes
- [ ] **Task 4.1.4**: Create JSON request/response schemas

### 4.2 RediSearch Operations
- [ ] **Task 4.2.1**: Implement Search repository trait (gated by `capabilities.modules.search`)
- [ ] **Task 4.2.2**: Implement Search Index operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | FT.CREATE | `ft_create` | High |
  | FT.DROPINDEX | `ft_drop_index` | High |
  | FT.INFO | `ft_info` | High |
  | FT.ALTER | `ft_alter` | Medium |
  | FT._LIST | `ft_list` | Medium |

- [ ] **Task 4.2.3**: Implement Search Query operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | FT.SEARCH | `ft_search` with all options | High |
  | FT.AGGREGATE | `ft_aggregate` | High |
  | FT.EXPLAIN | `ft_explain` | Medium |
  | FT.PROFILE | `ft_profile` | Low |

- [ ] **Task 4.2.4**: Implement Search Alias operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | FT.ALIASADD | `ft_alias_add` | Medium |
  | FT.ALIASDEL | `ft_alias_del` | Medium |
  | FT.ALIASUPDATE | `ft_alias_update` | Medium |

- [ ] **Task 4.2.5**: Implement Autocomplete operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | FT.SUGADD | `ft_sug_add` | Medium |
  | FT.SUGGET | `ft_sug_get` | Medium |
  | FT.SUGDEL | `ft_sug_del` | Medium |
  | FT.SUGLEN | `ft_sug_len` | Medium |

- [ ] **Task 4.2.6**: Implement Synonym/Spellcheck operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | FT.SYNDUMP | `ft_syn_dump` | Low |
  | FT.SYNUPDATE | `ft_syn_update` | Low |
  | FT.SPELLCHECK | `ft_spell_check` | Low |
  | FT.DICTADD | `ft_dict_add` | Low |
  | FT.DICTDEL | `ft_dict_del` | Low |
  | FT.DICTDUMP | `ft_dict_dump` | Low |

- [ ] **Task 4.2.7**: Create Search API routes
- [ ] **Task 4.2.8**: Create Search request/response schemas

### 4.3 RedisBloom Operations
- [ ] **Task 4.3.1**: Implement Bloom Filter operations (gated by `capabilities.modules.bloom`)
  | Command | Method | Priority |
  |---------|--------|----------|
  | BF.RESERVE | `bf_reserve` | High |
  | BF.ADD | `bf_add` | High |
  | BF.MADD | `bf_madd` | High |
  | BF.EXISTS | `bf_exists` | High |
  | BF.MEXISTS | `bf_mexists` | High |
  | BF.INSERT | `bf_insert` | Medium |
  | BF.INFO | `bf_info` | High |
  | BF.SCANDUMP | `bf_scandump` | Low |
  | BF.LOADCHUNK | `bf_loadchunk` | Low |
  | BF.CARD | `bf_card` | Medium |

- [ ] **Task 4.3.2**: Implement Cuckoo Filter operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | CF.RESERVE | `cf_reserve` | High |
  | CF.ADD | `cf_add` | High |
  | CF.ADDNX | `cf_addnx` | Medium |
  | CF.INSERT | `cf_insert` | Medium |
  | CF.INSERTNX | `cf_insertnx` | Medium |
  | CF.EXISTS | `cf_exists` | High |
  | CF.MEXISTS | `cf_mexists` | Medium |
  | CF.DEL | `cf_del` | High |
  | CF.COUNT | `cf_count` | Medium |
  | CF.SCANDUMP | `cf_scandump` | Low |
  | CF.LOADCHUNK | `cf_loadchunk` | Low |
  | CF.INFO | `cf_info` | High |

- [ ] **Task 4.3.3**: Create Bloom/Cuckoo API routes
- [ ] **Task 4.3.4**: Create Bloom/Cuckoo request/response schemas

### 4.4 Probabilistic Data Structures
- [ ] **Task 4.4.1**: Implement Count-Min Sketch operations (gated by `capabilities.modules.bloom`)
  | Command | Method | Priority |
  |---------|--------|----------|
  | CMS.INITBYDIM | `cms_init_by_dim` | High |
  | CMS.INITBYPROB | `cms_init_by_prob` | High |
  | CMS.INCRBY | `cms_incr_by` | High |
  | CMS.QUERY | `cms_query` | High |
  | CMS.MERGE | `cms_merge` | Medium |
  | CMS.INFO | `cms_info` | High |

- [ ] **Task 4.4.2**: Implement Top-K operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | TOPK.RESERVE | `topk_reserve` | High |
  | TOPK.ADD | `topk_add` | High |
  | TOPK.INCRBY | `topk_incr_by` | Medium |
  | TOPK.QUERY | `topk_query` | High |
  | TOPK.COUNT | `topk_count` | Medium |
  | TOPK.LIST | `topk_list` | High |
  | TOPK.INFO | `topk_info` | High |

- [ ] **Task 4.4.3**: Implement HyperLogLog operations (always available - core Redis)
  | Command | Method | Priority |
  |---------|--------|----------|
  | PFADD | `pf_add` | High |
  | PFCOUNT | `pf_count` | High |
  | PFMERGE | `pf_merge` | Medium |

- [ ] **Task 4.4.4**: Create Probabilistic API routes
- [ ] **Task 4.4.5**: Create Probabilistic request/response schemas

---

## Phase 5: NEW Features (Not in Go/Node)

### 5.1 Bitmap Operations (NEW)
- [ ] **Task 5.1.1**: Implement Bitmap repository trait
- [ ] **Task 5.1.2**: Implement Bitmap operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | SETBIT | `setbit` | High |
  | GETBIT | `getbit` | High |
  | BITCOUNT | `bitcount` | High |
  | BITPOS | `bitpos` | High |
  | BITOP | `bitop` (AND, OR, XOR, NOT) | High |
  | BITFIELD | `bitfield` | Medium |
  | BITFIELD_RO | `bitfield_ro` | Medium |

- [ ] **Task 5.1.3**: Create Bitmap API routes
  - `GET /api/v1/bitmaps/:key/bit/:offset`
  - `PUT /api/v1/bitmaps/:key/bit/:offset`
  - `GET /api/v1/bitmaps/:key/count`
  - `GET /api/v1/bitmaps/:key/pos`
  - `POST /api/v1/bitmaps/operations`
  - `POST /api/v1/bitmaps/:key/bitfield`

- [ ] **Task 5.1.4**: Create Bitmap request/response schemas

### 5.2 Geospatial Operations (NEW)
- [ ] **Task 5.2.1**: Implement Geo repository trait
- [ ] **Task 5.2.2**: Implement Geo operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | GEOADD | `geo_add` | High |
  | GEODIST | `geo_dist` | High |
  | GEOHASH | `geo_hash` | High |
  | GEOPOS | `geo_pos` | High |
  | GEORADIUS | `geo_radius` (deprecated but supported) | Medium |
  | GEORADIUSBYMEMBER | `geo_radius_by_member` | Medium |
  | GEOSEARCH | `geo_search` | High |
  | GEOSEARCHSTORE | `geo_search_store` | Medium |

- [ ] **Task 5.2.3**: Create Geo API routes
  - `POST /api/v1/geo/:key/add`
  - `GET /api/v1/geo/:key/distance`
  - `GET /api/v1/geo/:key/hash`
  - `GET /api/v1/geo/:key/pos`
  - `POST /api/v1/geo/:key/search`
  - `POST /api/v1/geo/:key/search/store`

- [ ] **Task 5.2.4**: Create Geo request/response schemas with proper types
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
  pub struct GeoPosition {
      pub longitude: f64,
      pub latitude: f64,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
  pub struct GeoMember {
      pub member: String,
      pub position: GeoPosition,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
  #[serde(rename_all = "lowercase")]
  pub enum GeoUnit {
      Meters,
      Kilometers,
      Miles,
      Feet,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
  pub struct GeoSearchRequest {
      #[serde(flatten)]
      pub center: GeoSearchCenter,
      pub radius: f64,
      pub unit: GeoUnit,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub count: Option<u32>,
      #[serde(default)]
      pub asc: bool,
      #[serde(default)]
      pub with_coord: bool,
      #[serde(default)]
      pub with_dist: bool,
      #[serde(default)]
      pub with_hash: bool,
  }
  ```

### 5.3 Pub/Sub Operations (NEW) - Dedicated Connection Architecture
- [ ] **Task 5.3.1**: Implement Pub/Sub service (using PubSubManager)
  ```rust
  pub struct PubSubService {
      command_pool: Arc<InstrumentedPool>,  // For PUBLISH and info commands
      pubsub_manager: Arc<PubSubManager>,   // For subscriptions
  }

  impl PubSubService {
      /// Publish uses command pool (short-lived connection)
      pub async fn publish(&self, channel: &str, message: &str) -> Result<i64, CacheError> {
          let mut conn = self.command_pool.get().await?;
          let receivers: i64 = redis::cmd("PUBLISH")
              .arg(channel)
              .arg(message)
              .query_async(&mut *conn)
              .await?;
          Ok(receivers)
      }

      /// Subscribe creates dedicated connection via PubSubManager
      pub async fn subscribe(&self, channels: Vec<String>) -> Result<PubSubConnection, CacheError> {
          self.pubsub_manager.create_subscription(channels).await
      }

      /// Info commands use command pool
      pub async fn channels(&self, pattern: Option<&str>) -> Result<Vec<String>, CacheError> {
          let mut conn = self.command_pool.get().await?;
          let mut cmd = redis::cmd("PUBSUB");
          cmd.arg("CHANNELS");
          if let Some(p) = pattern {
              cmd.arg(p);
          }
          Ok(cmd.query_async(&mut *conn).await?)
      }
  }
  ```

- [ ] **Task 5.3.2**: Implement Pub/Sub operations
  | Command | Method | Priority | Connection |
  |---------|--------|----------|------------|
  | PUBLISH | `publish` | High | Command Pool |
  | SUBSCRIBE | `subscribe` | High | Dedicated (WebSocket) |
  | PSUBSCRIBE | `psubscribe` | High | Dedicated (WebSocket) |
  | UNSUBSCRIBE | (handled by WS close) | High | - |
  | PUNSUBSCRIBE | (handled by WS close) | High | - |
  | PUBSUB CHANNELS | `channels` | Medium | Command Pool |
  | PUBSUB NUMSUB | `numsub` | Medium | Command Pool |
  | PUBSUB NUMPAT | `numpat` | Medium | Command Pool |
  | SSUBSCRIBE | `ssubscribe` (sharded) | Low | Dedicated |
  | SUNSUBSCRIBE | (handled by WS close) | Low | - |
  | SPUBLISH | `spublish` | Low | Command Pool |

- [ ] **Task 5.3.3**: Implement WebSocket subscription handler
  ```rust
  pub async fn ws_subscribe(
      ws: WebSocketUpgrade,
      State(state): State<AppState>,
      Query(params): Query<SubscribeParams>,
  ) -> Result<Response, CacheError> {
      // Validate channels
      if params.channels.is_empty() {
          return Err(CacheError::InvalidInput("No channels specified".into()));
      }

      // Check subscription limit before upgrade
      let current = state.pubsub_manager.active_subscriptions();
      if current >= state.config.pubsub.max_subscriptions {
          return Err(CacheError::SubscriptionLimitReached);
      }

      Ok(ws.on_upgrade(move |socket| handle_subscription(socket, state, params.channels)))
  }

  async fn handle_subscription(
      mut socket: WebSocket,
      state: AppState,
      channels: Vec<String>,
  ) {
      // Create dedicated subscription (not from pool)
      let mut pubsub = match state.pubsub_manager.create_subscription(channels.clone()).await {
          Ok(ps) => ps,
          Err(e) => {
              let _ = socket.send(Message::Text(
                  serde_json::to_string(&ErrorResponse::from(e)).unwrap()
              )).await;
              return;
          }
      };

      // Subscribe to channels
      for channel in &channels {
          if let Err(e) = pubsub.subscribe(channel).await {
              let _ = socket.send(Message::Text(format!(r#"{{"error":"{}"}}"#, e))).await;
              return;
          }
      }

      // Stream messages to WebSocket
      loop {
          tokio::select! {
              // Message from Redis
              msg = pubsub.on_message() => {
                  match msg {
                      Some(m) => {
                          let payload = PubSubMessage {
                              channel: m.get_channel_name().to_string(),
                              message: m.get_payload().unwrap_or_default(),
                              timestamp: Utc::now(),
                          };
                          if socket.send(Message::Text(serde_json::to_string(&payload).unwrap())).await.is_err() {
                              break; // Client disconnected
                          }
                      }
                      None => break, // Redis connection closed
                  }
              }
              // Message from client (for unsubscribe or close)
              client_msg = socket.recv() => {
                  match client_msg {
                      Some(Ok(Message::Close(_))) | None => break,
                      _ => {} // Ignore other messages
                  }
              }
          }
      }
      // PubSubConnection dropped here - automatically decrements counter
  }
  ```

- [ ] **Task 5.3.4**: Create Pub/Sub API routes
  ```
  # HTTP endpoints (use command pool)
  POST   /api/v1/pubsub/publish
  GET    /api/v1/pubsub/channels
  GET    /api/v1/pubsub/numsub
  GET    /api/v1/pubsub/numpat
  GET    /api/v1/pubsub/stats              # Subscription stats

  # WebSocket endpoints (use dedicated connections)
  WS     /api/v1/pubsub/subscribe?channels=ch1,ch2
  WS     /api/v1/pubsub/psubscribe?patterns=user:*,order:*
  ```

- [ ] **Task 5.3.5**: Create Pub/Sub request/response schemas
  ```rust
  #[derive(Debug, Serialize, Deserialize, ToSchema)]
  pub struct PublishRequest {
      pub channel: String,
      pub message: String,
  }

  #[derive(Debug, Serialize, ToSchema)]
  pub struct PublishResponse {
      pub channel: String,
      pub receivers: i64,
  }

  #[derive(Debug, Serialize, ToSchema)]
  pub struct PubSubMessage {
      pub channel: String,
      pub message: String,
      pub timestamp: DateTime<Utc>,
  }

  #[derive(Debug, Serialize, ToSchema)]
  pub struct PubSubStats {
      pub active_subscriptions: usize,
      pub max_subscriptions: usize,
      pub total_created: u64,
      pub total_messages: u64,
      pub errors: u64,
  }
  ```

### 5.4 Transaction Operations (NEW) - Single-Request Model
- [ ] **Task 5.4.1**: Implement Transaction service (single-request bundled model)
  ```rust
  pub struct TransactionService {
      pool: Arc<InstrumentedPool>,
  }

  impl TransactionService {
      /// Execute multiple commands atomically in a single request
      pub async fn execute(&self, request: TransactionRequest) -> Result<TransactionResponse, CacheError> {
          let mut conn = self.pool.get().await?;

          // Build pipeline with MULTI/EXEC
          let mut pipe = redis::pipe();
          pipe.atomic(); // Wraps in MULTI/EXEC

          for cmd in &request.commands {
              self.add_command_to_pipe(&mut pipe, cmd)?;
          }

          // Execute atomically
          let results: Vec<redis::Value> = pipe.query_async(&mut *conn).await
              .map_err(|e| CacheError::TransactionFailed(e.to_string()))?;

          // Parse results
          let parsed_results = self.parse_results(&request.commands, results)?;

          Ok(TransactionResponse {
              success: true,
              results: parsed_results,
          })
      }

      /// Optimistic update using Lua script (replaces WATCH pattern)
      pub async fn compare_and_set(&self, request: CompareAndSetRequest) -> Result<bool, CacheError> {
          let mut conn = self.pool.get().await?;

          let script = redis::Script::new(r#"
              local current = redis.call('GET', KEYS[1])
              if current == ARGV[1] then
                  redis.call('SET', KEYS[1], ARGV[2])
                  return 1
              else
                  return 0
              end
          "#);

          let result: i32 = script
              .key(&request.key)
              .arg(&request.expected_value)
              .arg(&request.new_value)
              .invoke_async(&mut *conn)
              .await?;

          Ok(result == 1)
      }

      /// Hash compare-and-set
      pub async fn hcompare_and_set(&self, request: HCompareAndSetRequest) -> Result<bool, CacheError> {
          let mut conn = self.pool.get().await?;

          let script = redis::Script::new(r#"
              local current = redis.call('HGET', KEYS[1], ARGV[1])
              if current == ARGV[2] then
                  redis.call('HSET', KEYS[1], ARGV[1], ARGV[3])
                  return 1
              else
                  return 0
              end
          "#);

          let result: i32 = script
              .key(&request.key)
              .arg(&request.field)
              .arg(&request.expected_value)
              .arg(&request.new_value)
              .invoke_async(&mut *conn)
              .await?;

          Ok(result == 1)
      }
  }
  ```

- [ ] **Task 5.4.2**: Define transaction command types
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
  #[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
  pub enum RedisCommand {
      // Strings
      Get { key: String },
      Set { key: String, value: String, ttl_seconds: Option<u64> },
      Incr { key: String },
      IncrBy { key: String, delta: i64 },

      // Hashes
      HGet { key: String, field: String },
      HSet { key: String, field: String, value: String },
      HIncrBy { key: String, field: String, delta: i64 },
      HDel { key: String, fields: Vec<String> },

      // Lists
      LPush { key: String, values: Vec<String> },
      RPush { key: String, values: Vec<String> },
      LPop { key: String, count: Option<u32> },
      RPop { key: String, count: Option<u32> },

      // Sets
      SAdd { key: String, members: Vec<String> },
      SRem { key: String, members: Vec<String> },

      // Sorted Sets
      ZAdd { key: String, members: Vec<(f64, String)> },
      ZRem { key: String, members: Vec<String> },
      ZIncrBy { key: String, delta: f64, member: String },

      // Keys
      Del { keys: Vec<String> },
      Expire { key: String, seconds: u64 },
  }

  #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
  pub struct TransactionRequest {
      /// Commands to execute atomically (wrapped in MULTI/EXEC)
      pub commands: Vec<RedisCommand>,
  }

  #[derive(Debug, Clone, Serialize, ToSchema)]
  pub struct TransactionResponse {
      pub success: bool,
      pub results: Vec<CommandResult>,
  }

  #[derive(Debug, Clone, Serialize, ToSchema)]
  pub struct CommandResult {
      pub index: usize,
      pub success: bool,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub value: Option<serde_json::Value>,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub error: Option<String>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
  pub struct CompareAndSetRequest {
      pub key: String,
      pub expected_value: String,
      pub new_value: String,
  }
  ```

- [ ] **Task 5.4.3**: Create Transaction API routes
  ```
  POST   /api/v1/transactions/execute     # Execute bundled commands
  POST   /api/v1/transactions/cas         # Compare-and-set (string)
  POST   /api/v1/transactions/hcas        # Compare-and-set (hash field)
  ```

- [ ] **Task 5.4.4**: Create Transaction request/response schemas

### 5.5 Lua Scripting Operations (NEW)
- [ ] **Task 5.5.1**: Implement Scripting repository trait
- [ ] **Task 5.5.2**: Implement Scripting operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | EVAL | `eval` | High |
  | EVALSHA | `evalsha` | High |
  | EVALSHA_RO | `evalsha_ro` | Medium |
  | EVAL_RO | `eval_ro` | Medium |
  | SCRIPT LOAD | `script_load` | High |
  | SCRIPT EXISTS | `script_exists` | High |
  | SCRIPT FLUSH | `script_flush` | Medium |
  | SCRIPT KILL | `script_kill` | Medium |
  | SCRIPT DEBUG | `script_debug` | Low |

- [ ] **Task 5.5.3**: Create Scripting API routes
  - `POST /api/v1/scripts/eval`
  - `POST /api/v1/scripts/evalsha`
  - `POST /api/v1/scripts/load`
  - `POST /api/v1/scripts/exists`
  - `POST /api/v1/scripts/flush`
  - `POST /api/v1/scripts/kill`

- [ ] **Task 5.5.4**: Create Scripting request/response schemas
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
  pub struct EvalRequest {
      pub script: String,
      pub keys: Vec<String>,
      #[serde(default)]
      pub args: Vec<serde_json::Value>,
      #[serde(default)]
      pub readonly: bool,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
  pub struct EvalShaRequest {
      pub sha: String,
      pub keys: Vec<String>,
      #[serde(default)]
      pub args: Vec<serde_json::Value>,
      #[serde(default)]
      pub readonly: bool,
  }

  #[derive(Debug, Clone, Serialize, ToSchema)]
  pub struct ScriptLoadResponse {
      pub sha: String,
  }

  #[derive(Debug, Clone, Serialize, ToSchema)]
  pub struct ScriptExistsResponse {
      pub results: Vec<ScriptExistsResult>,
  }

  #[derive(Debug, Clone, Serialize, ToSchema)]
  pub struct ScriptExistsResult {
      pub sha: String,
      pub exists: bool,
  }
  ```

### 5.6 Redis Functions Operations (NEW)
- [ ] **Task 5.6.1**: Implement Functions repository trait (gated by `capabilities.features.functions`)
- [ ] **Task 5.6.2**: Implement Functions operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | FUNCTION LOAD | `function_load` | High |
  | FUNCTION DELETE | `function_delete` | High |
  | FUNCTION FLUSH | `function_flush` | Medium |
  | FUNCTION DUMP | `function_dump` | Medium |
  | FUNCTION RESTORE | `function_restore` | Medium |
  | FUNCTION LIST | `function_list` | High |
  | FUNCTION STATS | `function_stats` | Medium |
  | FUNCTION KILL | `function_kill` | Medium |
  | FCALL | `fcall` | High |
  | FCALL_RO | `fcall_ro` | High |

- [ ] **Task 5.6.3**: Create Functions API routes
  - `POST /api/v1/functions/load`
  - `DELETE /api/v1/functions/:name`
  - `POST /api/v1/functions/flush`
  - `GET /api/v1/functions`
  - `POST /api/v1/functions/call`

- [ ] **Task 5.6.4**: Create Functions request/response schemas

### 5.7 RedisTimeSeries Operations (NEW)
- [ ] **Task 5.7.1**: Implement TimeSeries repository trait (gated by `capabilities.modules.timeseries`)
- [ ] **Task 5.7.2**: Implement TimeSeries operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | TS.CREATE | `ts_create` | High |
  | TS.ALTER | `ts_alter` | Medium |
  | TS.ADD | `ts_add` | High |
  | TS.MADD | `ts_madd` | High |
  | TS.INCRBY | `ts_incr_by` | Medium |
  | TS.DECRBY | `ts_decr_by` | Medium |
  | TS.DEL | `ts_del` | Medium |
  | TS.GET | `ts_get` | High |
  | TS.MGET | `ts_mget` | High |
  | TS.RANGE | `ts_range` | High |
  | TS.REVRANGE | `ts_rev_range` | High |
  | TS.MRANGE | `ts_mrange` | High |
  | TS.MREVRANGE | `ts_mrev_range` | High |
  | TS.QUERYINDEX | `ts_query_index` | Medium |
  | TS.INFO | `ts_info` | High |
  | TS.CREATERULE | `ts_create_rule` | Medium |
  | TS.DELETERULE | `ts_delete_rule` | Medium |

- [ ] **Task 5.7.3**: Create TimeSeries API routes
  - `POST /api/v1/timeseries`
  - `POST /api/v1/timeseries/:key/samples`
  - `GET /api/v1/timeseries/:key`
  - `GET /api/v1/timeseries/:key/range`
  - `POST /api/v1/timeseries/mget`
  - `POST /api/v1/timeseries/mrange`

- [ ] **Task 5.7.4**: Create TimeSeries request/response schemas
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
  pub struct Sample {
      pub timestamp: i64,  // Unix timestamp in milliseconds
      pub value: f64,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
  pub struct TimeSeriesCreateRequest {
      pub key: String,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub retention_ms: Option<u64>,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub chunk_size: Option<u64>,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub duplicate_policy: Option<DuplicatePolicy>,
      #[serde(default)]
      pub labels: HashMap<String, String>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
  #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
  pub enum DuplicatePolicy {
      Block,
      First,
      Last,
      Min,
      Max,
      Sum,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
  #[serde(rename_all = "lowercase")]
  pub enum Aggregation {
      Avg,
      Sum,
      Min,
      Max,
      Range,
      Count,
      First,
      Last,
      StdP,
      StdS,
      VarP,
      VarS,
      Twa,
  }
  ```

---

## Phase 6: Admin & Server Operations

### 6.1 Database Operations
- [ ] **Task 6.1.1**: Implement Database operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | FLUSHDB | `flush_db` (admin protected) | High |
  | FLUSHALL | `flush_all` (admin protected) | High |
  | DBSIZE | `db_size` | High |
  | SWAPDB | `swap_db` | Low |
  | SELECT | `select` | Medium |
  | MOVE | `move` | Low |
  | COPY | `copy` | Medium |

- [ ] **Task 6.1.2**: Create Database admin routes (protected)
- [ ] **Task 6.1.3**: Implement admin API key authentication
  ```rust
  pub async fn admin_auth(
      State(state): State<AppState>,
      TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
      request: Request,
      next: Next,
  ) -> Result<Response, CacheError> {
      if auth.token() != state.config.admin.api_key {
          return Err(CacheError::Unauthorized);
      }
      Ok(next.run(request).await)
  }
  ```

### 6.2 Server Operations
- [ ] **Task 6.2.1**: Implement Server info operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | INFO | `info` (all sections) | High |
  | TIME | `time` | Medium |
  | LASTSAVE | `lastsave` | Medium |
  | DEBUG | `debug` (limited) | Low |
  | MEMORY STATS | `memory_stats` | High |
  | MEMORY USAGE | `memory_usage` | High |
  | MEMORY DOCTOR | `memory_doctor` | Low |
  | MEMORY PURGE | `memory_purge` | Low |

- [ ] **Task 6.2.2**: Create Server info API routes

### 6.3 Configuration Operations
- [ ] **Task 6.3.1**: Implement Config operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | CONFIG GET | `config_get` | High |
  | CONFIG SET | `config_set` (admin protected) | High |
  | CONFIG REWRITE | `config_rewrite` (admin protected) | Medium |
  | CONFIG RESETSTAT | `config_reset_stat` | Medium |

- [ ] **Task 6.3.2**: Create Config API routes

### 6.4 Persistence Operations
- [ ] **Task 6.4.1**: Implement Persistence operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | SAVE | `save` (admin protected) | High |
  | BGSAVE | `bgsave` (admin protected) | High |
  | BGREWRITEAOF | `bgrewrite_aof` | Medium |
  | SHUTDOWN | `shutdown` (admin protected) | Low |

- [ ] **Task 6.4.2**: Create Persistence API routes

### 6.5 Client Operations
- [ ] **Task 6.5.1**: Implement Client operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | CLIENT LIST | `client_list` | High |
  | CLIENT KILL | `client_kill` (admin protected) | Medium |
  | CLIENT SETNAME | `client_setname` | Medium |
  | CLIENT GETNAME | `client_getname` | Medium |
  | CLIENT PAUSE | `client_pause` (admin protected) | Low |
  | CLIENT UNPAUSE | `client_unpause` | Low |
  | CLIENT ID | `client_id` | Medium |
  | CLIENT INFO | `client_info` | Medium |

- [ ] **Task 6.5.2**: Create Client API routes

### 6.6 Monitoring Operations
- [ ] **Task 6.6.1**: Implement Monitoring operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | SLOWLOG GET | `slowlog_get` | High |
  | SLOWLOG LEN | `slowlog_len` | High |
  | SLOWLOG RESET | `slowlog_reset` | Medium |
  | LATENCY DOCTOR | `latency_doctor` | Low |
  | LATENCY GRAPH | `latency_graph` | Low |
  | LATENCY HISTORY | `latency_history` | Low |
  | LATENCY LATEST | `latency_latest` | Medium |
  | LATENCY RESET | `latency_reset` | Low |

- [ ] **Task 6.6.2**: Create Monitoring API routes

### 6.7 ACL Operations (Optional)
- [ ] **Task 6.7.1**: Implement ACL operations (gated by `capabilities.features.acl`)
  | Command | Method | Priority |
  |---------|--------|----------|
  | ACL CAT | `acl_cat` | Low |
  | ACL DELUSER | `acl_deluser` | Low |
  | ACL GENPASS | `acl_genpass` | Low |
  | ACL GETUSER | `acl_getuser` | Low |
  | ACL LIST | `acl_list` | Low |
  | ACL LOAD | `acl_load` | Low |
  | ACL LOG | `acl_log` | Low |
  | ACL SAVE | `acl_save` | Low |
  | ACL SETUSER | `acl_setuser` | Low |
  | ACL USERS | `acl_users` | Low |
  | ACL WHOAMI | `acl_whoami` | Low |

- [ ] **Task 6.7.2**: Create ACL API routes

---

## Phase 7: Cluster & Sentinel Support (Optional)

### 7.1 Cluster Operations
- [ ] **Task 7.1.1**: Implement Cluster connection support (gated by `capabilities.features.cluster`)
- [ ] **Task 7.1.2**: Implement Cluster info operations
  | Command | Method | Priority |
  |---------|--------|----------|
  | CLUSTER INFO | `cluster_info` | Medium |
  | CLUSTER NODES | `cluster_nodes` | Medium |
  | CLUSTER SLOTS | `cluster_slots` | Medium |
  | CLUSTER KEYSLOT | `cluster_keyslot` | Medium |
  | CLUSTER SHARDS | `cluster_shards` | Low |

- [ ] **Task 7.1.3**: Create Cluster API routes

### 7.2 Sentinel Support
- [ ] **Task 7.2.1**: Implement Sentinel connection support
- [ ] **Task 7.2.2**: Implement Sentinel failover handling
- [ ] **Task 7.2.3**: Create Sentinel configuration options

---

## Phase 8: Testing

### 8.1 Unit Tests
- [ ] **Task 8.1.1**: Set up unit test infrastructure
  - Configure test utilities
  - Create mock Redis client
  - Set up test fixtures

- [ ] **Task 8.1.2**: Write unit tests for all services
  - String service tests
  - Hash service tests
  - List service tests (including blocking operations)
  - Set service tests
  - Sorted Set service tests (including blocking operations)
  - Stream service tests (including SSE)
  - JSON service tests
  - Search service tests
  - Bloom service tests
  - Probabilistic service tests
  - Geo service tests
  - Bitmap service tests
  - Pub/Sub service tests (including WebSocket)
  - Transaction service tests
  - Scripting service tests
  - Functions service tests
  - TimeSeries service tests
  - Admin service tests

- [ ] **Task 8.1.3**: Write unit tests for validators
- [ ] **Task 8.1.4**: Write unit tests for error handling
- [ ] **Task 8.1.5**: Write unit tests for capability detection

### 8.2 Integration Tests
- [ ] **Task 8.2.1**: Set up testcontainers with Redis Stack
- [ ] **Task 8.2.2**: Write integration tests for all repositories
- [ ] **Task 8.2.3**: Write integration tests for connection pool (including metrics)
- [ ] **Task 8.2.4**: Write integration tests for Pub/Sub manager
- [ ] **Task 8.2.5**: Write integration tests for blocking operations
- [ ] **Task 8.2.6**: Write integration tests for transactions
- [ ] **Task 8.2.7**: Write integration tests for error scenarios

### 8.3 E2E Tests
- [ ] **Task 8.3.1**: Set up E2E test infrastructure
- [ ] **Task 8.3.2**: Write E2E tests for all API endpoints
- [ ] **Task 8.3.3**: Write E2E tests for authentication
- [ ] **Task 8.3.4**: Write E2E tests for WebSocket subscriptions
- [ ] **Task 8.3.5**: Write E2E tests for SSE streaming
- [ ] **Task 8.3.6**: Write E2E tests for error responses

### 8.4 Benchmark Tests
- [ ] **Task 8.4.1**: Set up criterion for benchmarking
- [ ] **Task 8.4.2**: Write benchmarks for string operations
- [ ] **Task 8.4.3**: Write benchmarks for hash operations
- [ ] **Task 8.4.4**: Write benchmarks for concurrent operations
- [ ] **Task 8.4.5**: Write benchmarks for connection pool

---

## Phase 9: Documentation & Deployment

### 9.1 Documentation
- [ ] **Task 9.1.1**: Write README.md with usage instructions
- [ ] **Task 9.1.2**: Write API documentation
- [ ] **Task 9.1.3**: Write configuration guide
- [ ] **Task 9.1.4**: Write deployment guide
- [ ] **Task 9.1.5**: Create example client code
- [ ] **Task 9.1.6**: Document architectural decisions (blocking, pub/sub, transactions)

### 9.2 Docker & Deployment
- [ ] **Task 9.2.1**: Optimize Dockerfile for production
- [ ] **Task 9.2.2**: Create Kubernetes manifests
  - Deployment
  - Service
  - ConfigMap
  - Secret
  - HPA (Horizontal Pod Autoscaler)

- [ ] **Task 9.2.3**: Create Helm chart (optional)
- [ ] **Task 9.2.4**: Set up CI/CD pipeline
  - GitHub Actions workflow
  - Build and test on PR
  - Docker image build and push
  - Release automation

### 9.3 Production Readiness
- [ ] **Task 9.3.1**: Add metrics endpoint (Prometheus format)
  - Connection pool metrics
  - Pub/Sub subscription metrics
  - Request latency histograms
  - Error counters
- [ ] **Task 9.3.2**: Implement rate limiting
- [ ] **Task 9.3.3**: Add request size limits
- [ ] **Task 9.3.4**: Security audit
- [ ] **Task 9.3.5**: Load testing

---

## Summary Statistics

| Category | Total Tasks | High Priority | Medium Priority | Low Priority |
|----------|-------------|---------------|-----------------|--------------|
| Phase 1: Foundation | 10 | 8 | 2 | 0 |
| Phase 2: Infrastructure | 16 | 12 | 4 | 0 |
| Phase 3: Core Data Types | 35 | 22 | 10 | 3 |
| Phase 4: Redis Modules | 22 | 14 | 6 | 2 |
| Phase 5: NEW Features | 38 | 24 | 11 | 3 |
| Phase 6: Admin Operations | 18 | 10 | 6 | 2 |
| Phase 7: Cluster/Sentinel | 8 | 2 | 4 | 2 |
| Phase 8: Testing | 20 | 12 | 6 | 2 |
| Phase 9: Documentation | 16 | 8 | 6 | 2 |
| **Total** | **183** | **112** | **55** | **16** |

---

## Appendix A: Configuration Reference

```env
# Server Configuration
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
SERVER_REQUEST_TIMEOUT_MS=30000

# Redis Connection
REDIS_URL=redis://localhost:6379
REDIS_PASSWORD=
REDIS_DATABASE=0
REDIS_TLS_ENABLED=false
REDIS_TLS_CERT_PATH=
REDIS_TLS_KEY_PATH=
REDIS_TLS_CA_PATH=
REDIS_TLS_SKIP_VERIFY=false

# Command Connection Pool
REDIS_POOL_MIN_SIZE=2
REDIS_POOL_MAX_SIZE=10
REDIS_CONNECT_TIMEOUT_MS=5000
REDIS_COMMAND_TIMEOUT_MS=5000
REDIS_IDLE_TIMEOUT_MS=600000

# Pub/Sub Configuration (separate from pool)
PUBSUB_MAX_SUBSCRIPTIONS=100
PUBSUB_CONNECTION_TIMEOUT_MS=30000
PUBSUB_IDLE_TIMEOUT_MS=300000

# Blocking Commands
BLOCKING_MAX_TIMEOUT_SECONDS=30
BLOCKING_DEFAULT_TIMEOUT_SECONDS=5

# Admin
ADMIN_API_KEY=changeme-admin-key

# Logging
RUST_LOG=info
LOG_FORMAT=json
```

---

## Appendix B: Response Codes Reference

| Status Code | Condition |
|-------------|-----------|
| 200 OK | Successful operation with data |
| 201 Created | Resource created successfully |
| 204 No Content | Success with no data (e.g., blocking timeout) |
| 400 Bad Request | Invalid input / validation error |
| 401 Unauthorized | Missing or invalid admin API key |
| 404 Not Found | Key not found |
| 501 Not Implemented | Redis module not available |
| 503 Service Unavailable | Subscription limit reached |
| 504 Gateway Timeout | Operation timeout |
| 500 Internal Server Error | Unexpected error |

---

## Appendix C: Redis Commands Coverage Summary

### From Go Implementation (Port)
- Strings: 17 commands
- Hashes: 16 commands
- Lists: 20 commands (including blocking)
- Sets: 17 commands
- Sorted Sets: 32 commands (including blocking)
- Streams: 21 commands (including SSE streaming)
- Keys: 28 commands
- RedisJSON: 20 commands
- RediSearch: 26 commands
- RedisBloom: 10 commands
- Cuckoo Filter: 11 commands
- Count-Min Sketch: 6 commands
- Top-K: 7 commands
- HyperLogLog: 3 commands
- Admin/Server: 35 commands

### NEW Features (Not in Go/Node)
- Bitmaps: 7 commands
- Geospatial: 8 commands
- Pub/Sub: 11 commands (WebSocket-based subscriptions)
- Transactions: Bundled model + CAS helpers
- Lua Scripting: 9 commands
- Redis Functions: 10 commands
- RedisTimeSeries: 17 commands
- Cluster: 5 commands

### Grand Total: ~300+ Redis commands covered
