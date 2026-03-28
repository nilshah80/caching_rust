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

FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --system --gid 10001 appuser && \
    useradd --system --uid 10001 --gid 10001 --home-dir /app --shell /usr/sbin/nologin appuser

COPY --from=builder /app/target/release/redis-caching-service /usr/local/bin/redis-caching-service

USER 10001:10001

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["redis-caching-service"]
