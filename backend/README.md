# backend

Rust + Axum service behind [www.amputatorbot.com](https://www.amputatorbot.com/). Hosts the `/api/v2/convert` endpoint (plus the legacy `/api/v1/convert` for backwards compatibility), the 11-method canonical-finding engine, and serves the Astro static site from the same binary via `tower-http::ServeDir`.

## Getting started

The toolchain is pinned via [mise](https://mise.jdx.dev). From the repo root, `just setup` installs Rust stable, Node 22, pnpm, just, and lefthook. After that, install the Rust-side tools used by the test + build flows:

```bash
cargo install \
    cargo-nextest \
    cargo-deny \
    cargo-watch \
    sqlx-cli --version "^0.8" --no-default-features --features rustls,postgres
```

You'll also need Docker (or OrbStack / colima) for the local Postgres. The first `just db-up` pulls `postgres:17-alpine` (~110 MB).

### Run the server

From the repo root:

```bash
just db-up           # boot local Postgres 17
just db-seed         # load the committed 10k sample into the `links` cache
just backend-dev     # cargo-watch, rebuilds on save
```

The server listens on `localhost:8080`. Two convert endpoints (default to v2 for anything new):

| Endpoint | Surface |
|---|---|
| `POST /api/v2/convert` | Modern JSON in / JSON out, camelCase both ways, `entryType` as a body field. Strict validation (typo'd field → 422). |
| `GET\|POST /api/v1/convert` | Legacy query-string contract, snake_case JSON response. Kept stable for existing external consumers — don't build new integrations against it. |

Quick smoke test (assumes `just backend-dev` is running):

```bash
URL='https://abcnews.com/amp/Politics/hhs-warns-states-removing-kids-homes-parents-approval/story?id=130696092'

# v2 — `jq -nc` safely injects the URL as JSON.
curl -s -X POST -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg q "$URL" '{query: $q, entryType: "COMMENT"}')" \
    http://localhost:8080/api/v2/convert | jq

# v1 (legacy) — `--data-urlencode` lets curl handle percent-encoding.
curl -s --get --data-urlencode "q=$URL" http://localhost:8080/api/v1/convert | jq
```

Or open the Scalar UI at [http://localhost:8080/api/docs](http://localhost:8080/api/docs) and use the try-it-now panel — same endpoints, no curl needed.

### Common commands

All run from `backend/`:

| Command | What it does |
|---|---|
| `just check` | Format-check, clippy `-D warnings`, nextest, cargo-deny. Same as CI. |
| `just test` | Test suite only (~150 tests, ~0.1s; parity excluded). |
| `just fmt` | Apply rustfmt. |
| `just lint` | Clippy with warnings denied. |
| `just dev` | Run with cargo-watch (auto-rebuild on save). |
| `just run` | Run the release binary once. |
| `just build` | Release build. |

## Database

Local Postgres 17 runs in Docker, defined in the root `docker-compose.yml`. The backend connects via `DATABASE_URL`, which defaults to `postgres://amputatorbot:amputatorbot@localhost:5432/amputatorbot`. All DB commands run from the repo root:

| Command | What it does |
|---|---|
| `just db-up` | Boot Postgres in the background. Idempotent. |
| `just db-down` | Stop the container (keeps the data volume). |
| `just db-nuke` | Stop + delete the data volume — full reset. |
| `just db-migrate` | Apply pending sqlx migrations explicitly. |
| `just db-seed` | Load the committed 10k URLConversions sample into `links`. |
| `just db-seed path=<csv>` | Load any CSV with the same column order (e.g. a full legacy export). |

Migrations are auto-applied on backend startup via `sqlx::migrate!()`, so `just db-migrate` is mainly for "fresh schema without booting the server".

### sqlx offline mode

The DATABASE-method query in `src/canonical/pg_database.rs` uses the compile-time-checked `sqlx::query!` macro. To avoid requiring a live DB during `cargo build` (which would break CI + Docker builds), each macro's metadata is cached in `backend/.sqlx/` and committed to git. `cargo build` reads from there when `SQLX_OFFLINE=true` or when no `DATABASE_URL` is set.

**When you change any `sqlx::query!` SQL or the schema:** re-run `cargo sqlx prepare` from `backend/` (with `DATABASE_URL` set and PG running) and commit the updated `.sqlx/` JSON. CI fails if `.sqlx/` drifts from the actual queries.

### CSV format for `just db-seed`

Comma-delimited, RFC-4180 quoting where needed, empty fields for `NULL`:

```
entry_id,entry_type,handled_utc,original_url,canonical_url,canonical_type,note
```

JetBrains' "Export Data → CSV" on the legacy `URLConversions` table produced the committed 10k samples in `tests/fixtures/urlconversions/`. The full ~1.7M-row export uses the same shape and drops into `just db-seed path=<file>` directly.

Both `original_url` and `canonical_url` are capped at 2048 chars (enforced by `001_initial.sql`, mirrored in Rust as `canonical::MAX_URL_LEN`). The seed recipe filters out longer rows via a constraint-free staging table and reports `skipped_too_long` at the end — that's not an error.

## Tests

Three layers.

### 1. Unit tests (`just test`)

Per-module tests for AMP detection, URL extraction, the 11 canonical methods, and the orchestration loop. `MockPageSource` injects hand-crafted HTML. Fast (< 0.1s), no network.

### 2. Snapshot tests (`cargo nextest run --test snapshots`)

Pin the JSON shape of `resolve()` for ~10 representative scenarios. Catches accidental response-shape drift (renamed fields, key reordering, SCREAMING_SNAKE_CASE regressions).

Regenerate after an *intentional* shape change:

```bash
INSTA_UPDATE=always cargo nextest run --test snapshots
cargo install cargo-insta  # one-time
cargo insta review         # interactive accept/reject
```

### 3. Parity tests — against real legacy data

The most informative test: takes a CSV export of the legacy Python bot's `URLConversions` table, fetches each URL's HTML once, then runs the new Rust resolver against that HTML and compares results to what the legacy bot recorded.

**Step 1 — Record fixtures** (one-time, ~1 hour for 10k URLs):

```bash
just record-fixtures
```

That uses the default CSV (`10000_conversions_unfiltered.csv` — a random 10k slice that includes failures and false positives). For the successes-only set:

```bash
just record-fixtures tests/fixtures/urlconversions/10000_conversions_with_canonical.csv
```

Properties:
- **Idempotent.** Already-recorded fixtures are skipped, so Ctrl+C and re-run is safe.
- **Respectful.** Default 4 concurrent fetches, 15s timeout, rotating Firefox user agent.
- **Slow.** Roughly an hour for the full 10k; much faster on repeat runs.

The HTML directory (`tests/fixtures/html/`) is gitignored — regenerable on demand.

**Step 2 — Run parity:**

```bash
just parity
```

Streams progress live and writes a Markdown summary to `tests/parity-report.md` with per-bucket counts and the first 25 mismatches.

| Bucket | Meaning |
|---|---|
| **matched** | Same canonical (or both found `None`). |
| **mismatch_url** | Both found a canonical but they disagree on the URL. |
| **legacy_only** | Legacy found one, we didn't. |
| **new_only** | We found one, legacy didn't — could be a legit false-positive fix (e.g. `amputeestore.com`) or a new bug. |
| **skipped** | Recorder failed for that URL — no HTML. |

The test only **fails** if zero matches across all fixtures (the resolver is broken). Tighter parity-rate floors land in follow-up commits as drift gets investigated.

**Step 3 — Read the report:**

```bash
cat tests/parity-report.md
```

To trace a specific mismatch, open `tests/fixtures/html/<csv_id>.json` — raw HTML and the legacy expected canonical are both in there.

## Project layout

```
backend/
├── Cargo.toml
├── Dockerfile                 # multi-stage build, cargo-chef caching
├── deny.toml                  # cargo-deny config (licenses, advisories)
├── justfile                   # local task runner — `just --list`
├── rust-toolchain.toml        # pins to stable
├── src/
│   ├── main.rs                # Axum binary
│   ├── lib.rs                 # library — used by bins + tests
│   ├── bin/
│   │   └── record_fixtures.rs # the CSV → HTML recorder
│   ├── canonical/             # the engine
│   │   ├── amp_detect.rs      # is_amp_url, is_cached_amp
│   │   ├── database.rs        # Database trait (PG abstraction for tests)
│   │   ├── domain.rs          # extract_domain via psl crate
│   │   ├── http_fetcher.rs    # production PageSource impl (reqwest)
│   │   ├── page.rs            # Page struct
│   │   ├── page_source.rs     # PageSource trait
│   │   ├── pg_database.rs     # PgDatabase — Database impl backed by sqlx
│   │   ├── resolver.rs        # resolve() — the depth loop
│   │   ├── resolve_opts.rs    # ResolveOpts struct
│   │   ├── url_extract.rs     # extract_urls, remove_markdown
│   │   └── methods/           # the 11 canonical-finding methods
│   │       ├── rel.rs
│   │       ├── canurl.rs
│   │       ├── og_url.rs
│   │       ├── google_manual.rs
│   │       ├── google_js.rs
│   │       ├── bing_original.rs
│   │       ├── schema_mainentity.rs
│   │       ├── tco_pagetitle.rs
│   │       ├── meta_redirect.rs
│   │       ├── guess_and_check.rs
│   │       └── database.rs    # DATABASE cache lookup
│   ├── models/                # API JSON shapes
│   │   ├── url_meta.rs        # UrlMeta (base shape)
│   │   ├── canonical.rs       # Canonical struct
│   │   ├── canonical_type.rs  # CanonicalType enum (11 variants)
│   │   └── link.rs            # Link (the top-level response item)
│   └── readability/
│       └── mod.rs             # dom_smoothie wrapper + similarity scoring
├── migrations/                # sqlx-managed schema migrations
│   └── 001_initial.sql        # links table + enums + indexes + URL-length checks
├── .sqlx/                     # cached query metadata (committed; see "sqlx offline mode")
└── tests/
    ├── fixtures/
    │   ├── urlconversions/    # committed CSVs from the legacy DB
    │   └── html/              # generated by record_fixtures (gitignored)
    ├── snapshots/             # insta golden JSON files
    ├── models_serde.rs        # API shape regression tests
    ├── parity.rs              # legacy-vs-new parity (this README §3)
    └── snapshots.rs           # insta snapshot tests
```

## Debugging notes

**A specific URL fails in `record_fixtures`.** Look at the error chain in the warning log (`?e` prints the full anyhow cause chain). DNS / TLS / HTTP-403 / timeout each look different.

**A parity mismatch you want to understand.** Open `tests/parity-report.md`, find the `csv_id`, then look at `tests/fixtures/html/<csv_id>.json` — raw HTML under `html`, legacy expected canonical under `expected_canonical`.

**Snapshot test failed.** The diff is in the test output. Either fix the code (response shape regressed) or run with `INSTA_UPDATE=always` to accept the new shape if the change was intentional.
