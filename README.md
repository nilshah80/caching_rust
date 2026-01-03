# Redis Caching Service

A high-performance Redis caching service built in Rust, providing a clean REST API for Redis operations.

## Features

- **String Operations**: GET, SET, MGET, MSET, INCR, DECR, APPEND, STRLEN, GETRANGE, SETRANGE, GETEX, GETDEL
- **Admin Operations**: Server info, memory stats, client management, slowlog, latency monitoring, ACL
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

### Admin Operations (API Key Required)

Set the `X-Admin-Api-Key` header for protected endpoints.

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

## License

MIT
