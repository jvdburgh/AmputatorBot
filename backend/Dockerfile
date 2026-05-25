# syntax=docker/dockerfile:1.7
# Multi-stage build with cargo-chef for cached dep compilation.

FROM rust:1.85-slim AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin amputatorbot-backend

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/amputatorbot-backend /usr/local/bin/amputatorbot-backend

ENV PORT=8080 \
    RUST_LOG=info

EXPOSE 8080
ENTRYPOINT ["amputatorbot-backend"]
