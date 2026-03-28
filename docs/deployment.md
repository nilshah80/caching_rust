# Deployment Guide

This guide covers deploying the Redis Caching Service in various environments.

## Docker

### Building the Image

The project includes an optimized multi-stage Dockerfile:

```bash
docker build -t redis-caching-service .
```

**Build features:**
- **Cargo Chef dependency caching** — dependency layers are separated from application code for faster incremental rebuilds
- **Non-root user** — the service runs as `appuser` for security
- **Health check** — built-in Docker HEALTHCHECK hitting `/health`
- **Minimal runtime** — `debian:bookworm-slim` base with only `ca-certificates` and `curl`

### Running with Docker

```bash
docker run -d \
  --name caching-service \
  -p 8080:8080 \
  -e REDIS__URL=redis://:password@redis-host:6379 \
  -e ADMIN__API_KEY=your-secret-key \
  -e LOG__LEVEL=info \
  -e LOG__FORMAT=json \
  redis-caching-service
```

## Docker Compose

### Development Stack

The included `docker-compose.yml` starts a complete development environment:

```bash
docker-compose up -d
```

This starts:
- **Redis 8** on port 6379 (includes all modules: JSON, Search, Bloom, TimeSeries)
- **RedisInsight** on port 8001 (Redis management UI)
- **Caching Service** on port 8080

```bash
# Verify the stack is healthy
docker-compose ps

# Check service health
curl http://localhost:8080/health

# View logs
docker-compose logs -f caching-service

# View Swagger UI
open http://localhost:8080/swagger-ui

# Redis management UI
open http://localhost:8001
```

### Benchmark Stack

For performance comparison with a Go implementation:

```bash
docker-compose -f docker-compose.benchmark.yml up -d --build
```

This starts the Rust service on port 8080 and a Go service on port 8081 against a shared Redis instance.

## Production Deployment

### Pre-flight Checklist

1. **Change the default admin API key**: `ADMIN__API_KEY=<strong-random-value>`
2. **Configure appropriate pool sizes**: Set `POOL__MAX_SIZE` based on expected concurrency
3. **Enable TLS** if Redis is accessible over a network
4. **Set log format to JSON** for structured logging: `LOG__FORMAT=json`
5. **Configure resource limits** for `MAX_BODY_SIZE_BYTES`, `MAX_BATCH_SIZE`, `MAX_VALUE_SIZE_BYTES`

### Resource Requirements

| Load Level | CPU | Memory | Pool Size | Pub/Sub Limit |
|------------|-----|--------|-----------|---------------|
| Light (< 100 req/s) | 0.5 core | 64 MiB | 5-10 | 50 |
| Moderate (100-1000 req/s) | 1-2 cores | 128 MiB | 25-50 | 100 |
| Heavy (1000+ req/s) | 2-4 cores | 256 MiB | 50-100 | 200+ |

The service itself is lightweight. Memory usage depends primarily on connection pool size and concurrent SSE/WebSocket connections.

### Health Checks

The service exposes three health endpoints:

| Endpoint | Purpose | Checks |
|----------|---------|--------|
| `GET /health` | General health | Service is running |
| `GET /health/live` | Liveness probe | Service process is alive |
| `GET /health/ready` | Readiness probe | Service can serve traffic (Redis connected) |

**Docker health check** (built into Dockerfile):
```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8080/health || exit 1
```

### TLS Configuration

For production Redis connections over a network:

```env
REDIS__URL=rediss://redis-host:6380
REDIS__TLS_ENABLED=true
REDIS__TLS_CA_PATH=/etc/ssl/certs/redis-ca.pem
```

For mutual TLS (client certificate authentication):

```env
REDIS__TLS_ENABLED=true
REDIS__TLS_CERT_PATH=/etc/ssl/certs/client.pem
REDIS__TLS_KEY_PATH=/etc/ssl/private/client-key.pem
REDIS__TLS_CA_PATH=/etc/ssl/certs/redis-ca.pem
```

### Reverse Proxy

When running behind a reverse proxy (Nginx, Traefik, etc.):

**Nginx example:**

```nginx
upstream caching_service {
    server 127.0.0.1:8080;
    keepalive 32;
}

server {
    listen 443 ssl;
    server_name cache-api.example.com;

    ssl_certificate /etc/ssl/certs/server.pem;
    ssl_certificate_key /etc/ssl/private/server-key.pem;

    location / {
        proxy_pass http://caching_service;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # WebSocket support (for Pub/Sub)
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";

        # SSE support (disable buffering)
        proxy_buffering off;
        proxy_cache off;

        # Timeout for long-lived connections
        proxy_read_timeout 300s;
        proxy_send_timeout 300s;
    }
}
```

Key proxy considerations:
- **WebSocket support** is required for Pub/Sub endpoints (`/api/v1/pubsub/subscribe`, `/api/v1/pubsub/psubscribe`)
- **SSE support** requires disabling response buffering for stream subscription endpoints
- **Timeout settings** should accommodate blocking operations (default max 30s) and SSE connections

### Monitoring

#### Connection Pool Stats

```bash
curl http://localhost:8080/api/v1/admin/pool/stats
```

Returns pool utilization, available connections, and wait times.

#### Capabilities

```bash
curl http://localhost:8080/api/v1/admin/capabilities
```

Returns detected Redis version and available modules.

#### Pub/Sub Stats

```bash
curl http://localhost:8080/api/v1/pubsub/stats
```

Returns active subscriptions, total created, message counts, and error counts.

#### Redis Server Monitoring

```bash
curl -H "X-Admin-Api-Key: $API_KEY" http://localhost:8080/api/v1/admin/server/info
curl -H "X-Admin-Api-Key: $API_KEY" http://localhost:8080/api/v1/admin/server/memory/stats
curl -H "X-Admin-Api-Key: $API_KEY" http://localhost:8080/api/v1/admin/latency/latest
curl -H "X-Admin-Api-Key: $API_KEY" http://localhost:8080/api/v1/admin/slowlog/get -X POST -H "Content-Type: application/json" -d '{"count": 10}'
```

### Scaling

The service is stateless and horizontally scalable. Run multiple instances behind a load balancer:

```
                    ┌──────────────────┐
                    │   Load Balancer   │
                    └────────┬─────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
        ┌─────┴─────┐ ┌─────┴─────┐ ┌─────┴─────┐
        │ Instance 1 │ │ Instance 2 │ │ Instance 3 │
        └─────┬─────┘ └─────┬─────┘ └─────┬─────┘
              │              │              │
              └──────────────┼──────────────┘
                             │
                    ┌────────┴─────────┐
                    │  Redis Primary    │
                    └──────────────────┘
```

**Considerations:**
- All instances connect to the same Redis server/cluster
- Pub/Sub subscriptions are per-instance; each instance maintains its own WebSocket connections
- SSE connections are per-instance; clients reconnect to any instance on disconnect
- Pool sizes should be set per-instance (total Redis connections = instances × `POOL__MAX_SIZE`)

## Kubernetes

The repository includes baseline manifests in `k8s/` for a production-style deployment:

- `configmap.yaml` — non-secret runtime configuration
- `secret.yaml` — Redis connection URL and admin API key placeholders
- `deployment.yaml` — hardened `Deployment` with liveness, readiness, and startup probes
- `service.yaml` — internal `ClusterIP` service
- `hpa.yaml` — CPU and memory based horizontal autoscaling
- `kustomization.yaml` — applies the full set together

### Deploying

Update `k8s/secret.yaml` before applying:

```yaml
stringData:
  REDIS__URL: "redis://:change-me@redis:6379"
  ADMIN__API_KEY: "change-me-admin-key"
```

Then apply the manifests:

```bash
kubectl apply -k k8s/
kubectl rollout status deployment/redis-caching-service
kubectl get hpa redis-caching-service
```

### Kubernetes Notes

- The default `Deployment` runs **2 replicas** so the `HPA` has room to scale horizontally
- `readinessProbe` uses `/health/ready`, which verifies Redis connectivity before serving traffic
- The container runs as a non-root user and drops all Linux capabilities
- Replace the `image` field in `k8s/deployment.yaml` with your published registry image before deployment

### Graceful Shutdown

The service handles `SIGTERM` gracefully:
1. Stops accepting new connections
2. Completes in-flight requests
3. Closes Redis connections
4. Exits with code 0

Docker Compose and Kubernetes send `SIGTERM` by default when stopping containers.
