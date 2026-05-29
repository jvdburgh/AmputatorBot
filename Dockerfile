# syntax=docker/dockerfile:1.7
# Single combined image: Astro static site + Rust backend.
#
# Stages:
#   1. website-deps   pnpm install (cached on lockfile)
#   2. website-build  astro build -> /app/website/dist
#   3. rust-chef      cargo-chef base image
#   4. rust-planner   recipe.json (cached on source manifest changes)
#   5. rust-builder   cargo build --release (cached on recipe.json)
#   6. runtime        debian-slim + binary + dist/ -> /app/static
#
# Build from the repo root: `docker build -t amputatorbot .`

# -----------------------------------------------------------------------------
# Stage 1: install website dependencies (cached on lockfile/manifest changes)
# -----------------------------------------------------------------------------
FROM node:22-slim AS website-deps
RUN corepack enable && corepack prepare pnpm@11.3.0 --activate
WORKDIR /app

# Workspace manifests only — keeps this layer cacheable as long as no
# package.json / lockfile changes. devvit-app's package.json is needed too,
# because pnpm-workspace.yaml resolves every member, even when we filter to
# just the website below.
COPY pnpm-lock.yaml pnpm-workspace.yaml package.json ./
COPY website/package.json ./website/
COPY devvit-app/package.json ./devvit-app/

# `--filter amputatorbot-website` skips devvit-app's deps. pnpm v10+ skips
# postinstall scripts by default unless allow-listed in pnpm-workspace.yaml.
RUN pnpm install --frozen-lockfile --filter amputatorbot-website

# -----------------------------------------------------------------------------
# Stage 2: build Astro static bundle
# -----------------------------------------------------------------------------
FROM website-deps AS website-build
COPY website/ ./website/
RUN pnpm --filter amputatorbot-website build
# Produces /app/website/dist/

# -----------------------------------------------------------------------------
# Stage 3: cargo-chef base
# -----------------------------------------------------------------------------
FROM rust:1.95-slim AS rust-chef
RUN cargo install cargo-chef --locked
WORKDIR /app

# -----------------------------------------------------------------------------
# Stage 4: build the cargo recipe (deps fingerprint)
# -----------------------------------------------------------------------------
FROM rust-chef AS rust-planner
COPY backend/ ./
RUN cargo chef prepare --recipe-path recipe.json

# -----------------------------------------------------------------------------
# Stage 5: build the Rust binary
# -----------------------------------------------------------------------------
FROM rust-chef AS rust-builder
COPY --from=rust-planner /app/recipe.json recipe.json
# Cook compiles only the dependency tree. Cached until recipe.json changes.
RUN cargo chef cook --release --recipe-path recipe.json
COPY backend/ ./
RUN cargo build --release --bin amputatorbot-backend

# -----------------------------------------------------------------------------
# Stage 6: runtime image
# -----------------------------------------------------------------------------
FROM debian:trixie-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=rust-builder /app/target/release/amputatorbot-backend /usr/local/bin/amputatorbot-backend
COPY --from=website-build /app/website/dist /app/static

ENV PORT=8080 \
    RUST_LOG=info \
    STATIC_DIR=/app/static

EXPOSE 8080
ENTRYPOINT ["amputatorbot-backend"]
