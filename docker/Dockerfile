# syntax=docker/dockerfile:1

# ── Build ─────────────────────────────────────────────────────────────────────
FROM rust:1-bookworm AS builder

WORKDIR /app

# Install build deps (openssl for rustls/sqlx if needed)
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY lib.rs ./
COPY cmd ./cmd
COPY internal ./internal
COPY routes ./routes
COPY migrations ./migrations

# Cache-friendly release build (migrations embedded via sqlx::migrate!)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked \
    && cp /app/target/release/rsc-gateway /tmp/rsc-gateway

# ── Runtime ───────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates wget \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home --home-dir /nonexistent gateway

WORKDIR /app

COPY --from=builder /tmp/rsc-gateway /usr/local/bin/rsc-gateway

ENV RUST_LOG=rsc_gateway=info,tower_http=info \
    HOST=0.0.0.0 \
    PORT=8080

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD wget -q -O /dev/null http://127.0.0.1:8080/health/live || exit 1

USER gateway

CMD ["rsc-gateway"]
