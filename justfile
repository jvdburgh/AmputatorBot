# AmputatorBot root tasks — fan out to per-project justfiles.

# Local Postgres URL — override on the CLI:
#   just DATABASE_URL=postgres://other... db-migrate
export DATABASE_URL := env_var_or_default("DATABASE_URL", "postgres://amputatorbot:amputatorbot@localhost:5432/amputatorbot")

# Default: list recipes
default:
    @just --list

# Run all checks across all projects (CI equivalent locally)
check: backend-check devvit-check website-check

# Format every project
fmt: backend-fmt devvit-fmt website-fmt

# Lint every project
lint: backend-lint devvit-lint website-lint

# Run all tests
test: backend-test devvit-test website-test

# --- backend ---
backend-check:
    cd backend && just check

backend-fmt:
    cd backend && just fmt

backend-lint:
    cd backend && just lint

backend-test:
    cd backend && just test

backend-dev:
    cd backend && just dev

# --- devvit-app ---
devvit-check:
    cd devvit-app && just check

devvit-fmt:
    cd devvit-app && just fmt

devvit-lint:
    cd devvit-app && just lint

devvit-test:
    cd devvit-app && just test

devvit-dev:
    cd devvit-app && just dev

# --- website ---
website-check:
    cd website && just check

website-fmt:
    cd website && just fmt

website-lint:
    cd website && just lint

website-test:
    cd website && just test

website-dev:
    cd website && just dev

# --- local database (Postgres 17 via docker compose) ---

# Boot Postgres in the background. Idempotent; safe to re-run.
db-up:
    docker compose up -d postgres
    @echo "Waiting for Postgres to accept connections..."
    @until docker exec amputatorbot-postgres pg_isready -U amputatorbot -d amputatorbot >/dev/null 2>&1; do sleep 1; done
    @echo "Postgres ready at $DATABASE_URL"

# Stop and remove the Postgres container (keeps the data volume).
db-down:
    docker compose stop postgres

# Wipe the Postgres data volume — full reset.
db-nuke:
    docker compose down -v

# Apply pending sqlx migrations. The backend also auto-migrates on startup,
# so this is mainly useful when you want a fresh schema without running the server.
db-migrate:
    cd backend && sqlx migrate run

# Seed the `links` table from a CSV export of the legacy URLConversions table.
#
# Default is the 10k-row sample committed to the repo. Pass any CSV with the
# same column order to override (e.g. your full ~1.7M-row export):
#
#   just db-seed                                              # 10k sample
#   just db-seed path=/Users/jvdb/Downloads/URLConversions_full.csv
#
# Rows whose original_url or canonical_url exceed the 2048-char cap are
# filtered out (would otherwise violate the table's CHECK constraints). The
# recipe streams the CSV into a constraint-free staging table inside a
# transaction, then INSERTs only the rows that fit. Reports imported vs.
# skipped at the end so you can see what the cap cost you.
db-seed path="backend/tests/fixtures/urlconversions/10000_conversions_unfiltered.csv":
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Seeding links from {{path}} (filtering URLs > 2048 chars)..."
    # `\copy ... FROM '<path>'` reads on the psql side. Since psql runs in
    # the container, the file must be in the container — `docker cp` it in,
    # then drop it after. Cleans up even if psql fails (trap on EXIT).
    trap 'docker exec amputatorbot-postgres rm -f /tmp/seed.csv 2>/dev/null || true' EXIT
    docker cp "{{path}}" amputatorbot-postgres:/tmp/seed.csv
    docker exec -i amputatorbot-postgres psql -U amputatorbot -d amputatorbot -v ON_ERROR_STOP=1 <<'SQL'
    BEGIN;
    -- `LIKE ... INCLUDING DEFAULTS` copies the column defaults but NOT the
    -- CHECK constraints, so the staging table is constraint-free and accepts
    -- any URL length. The length filter is enforced by the INSERT below.
    CREATE TEMP TABLE links_staging (LIKE links INCLUDING DEFAULTS);
    \copy links_staging(entry_id, entry_type, handled_utc, original_url, canonical_url, canonical_type, note) FROM '/tmp/seed.csv' WITH (FORMAT csv, HEADER true, NULL '')
    INSERT INTO links
    SELECT * FROM links_staging
    WHERE length(original_url) <= 2048
      AND (canonical_url IS NULL OR length(canonical_url) <= 2048);
    SELECT
        (SELECT COUNT(*) FROM links_staging) AS staged,
        (SELECT COUNT(*) FROM links)         AS imported,
        (SELECT COUNT(*) FROM links_staging) - (SELECT COUNT(*) FROM links) AS skipped_too_long;
    COMMIT;
    SQL

# --- combined Astro + Rust container image ---

# Build the production image: Astro static bundle + Rust binary in one image.
# `STATIC_DIR=/app/static` is baked in by the Dockerfile.
#
# Always builds linux/amd64 because Scaleway Serverless Containers don't support
# arm64. On Apple Silicon this uses buildx + QEMU emulation — slower than a
# native build but produces a Scaleway-pushable image.
image:
    docker buildx build --platform=linux/amd64 -t amputatorbot:dev --load .

# Run the combined image locally. Requires `just db-up` first (the container
# talks to the Postgres on the host docker-compose). On macOS,
# host.docker.internal resolves to the host; on Linux, --add-host wires it up.
image-run:
    docker run --rm -it \
        -p 8080:8080 \
        --add-host=host.docker.internal:host-gateway \
        -e DATABASE_URL="postgres://amputatorbot:amputatorbot@host.docker.internal:5432/amputatorbot" \
        -e RUST_LOG=info \
        amputatorbot:dev

# --- one-time setup after clone ---
setup:
    mise install
    pnpm install -r
    lefthook install
    @echo "Setup complete. Try: just check"
