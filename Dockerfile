# syntax=docker/dockerfile:1.7

FROM rust:1.92-slim AS chef

WORKDIR /app

# cargo-chef keeps dependency builds cacheable across source-only changes.
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    curl \
    && cargo install cargo-chef --locked \
    && rm -rf /var/lib/apt/lists/*

FROM chef AS planner

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY benches ./benches
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo chef cook --release --locked --recipe-path recipe.json

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY benches ./benches
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --locked --bin redis-caching-service

FROM gcr.io/distroless/cc-debian12 AS runtime

WORKDIR /app

# The distroless image runs entirely without a shell. No additional installation needed.
COPY --from=builder /app/target/release/redis-caching-service /usr/local/bin/redis-caching-service

USER nonroot:nonroot

EXPOSE 8080

# Health checks: distroless has no shell/curl, so HEALTHCHECK is omitted.
# Use orchestrator probes instead:
#   Kubernetes liveness:  GET /health/live
#   Kubernetes readiness: GET /health/ready
#   Docker Compose:       test: ["CMD-SHELL", "curl -f http://localhost:8080/health/live || exit 1"]
#                         (requires a sidecar or non-distroless base)

ENTRYPOINT ["/usr/local/bin/redis-caching-service"]
