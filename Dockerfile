# syntax=docker/dockerfile:1.7

ARG RUST_VERSION=1.93.1
ARG BUN_VERSION=1.3.10

FROM rust:${RUST_VERSION}-slim-bookworm AS rust-builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends binutils build-essential ca-certificates pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo build --locked --release -p api -p ingest -p worker \
    && strip target/release/api target/release/ingest target/release/worker \
    && install -D target/release/api /out/api \
    && install -D target/release/ingest /out/ingest \
    && install -D target/release/worker /out/worker

FROM rust:${RUST_VERSION}-slim-bookworm AS migrate-builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install --locked sqlx-cli --no-default-features --features postgres,rustls
RUN install -D /usr/local/cargo/bin/sqlx /out/sqlx

FROM oven/bun:${BUN_VERSION}-alpine AS frontend-builder
WORKDIR /app
COPY . .
RUN bun install --frozen-lockfile
RUN bun run --filter @archivist/frontend build

FROM gcr.io/distroless/cc-debian12 AS api
WORKDIR /app
COPY --from=rust-builder /out/api /usr/local/bin/api
EXPOSE 4000
ENTRYPOINT ["/usr/local/bin/api"]

FROM gcr.io/distroless/cc-debian12 AS ingest
WORKDIR /app
COPY --from=rust-builder /out/ingest /usr/local/bin/ingest
EXPOSE 4001
ENTRYPOINT ["/usr/local/bin/ingest"]

FROM gcr.io/distroless/cc-debian12 AS worker
WORKDIR /app
COPY --from=rust-builder /out/worker /usr/local/bin/worker
EXPOSE 4002
ENTRYPOINT ["/usr/local/bin/worker"]

FROM gcr.io/distroless/cc-debian12 AS migrate
WORKDIR /app
COPY --from=migrate-builder /out/sqlx /usr/local/bin/sqlx
COPY crates/db/migrations /migrations
ENTRYPOINT ["/usr/local/bin/sqlx"]
CMD ["migrate", "run", "--source", "/migrations"]

FROM nginxinc/nginx-unprivileged:1.27-alpine AS frontend
COPY docker/nginx/frontend.conf /etc/nginx/conf.d/default.conf
COPY --from=frontend-builder /app/apps/frontend/dist /usr/share/nginx/html
EXPOSE 8080
