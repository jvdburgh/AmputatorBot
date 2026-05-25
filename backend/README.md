# backend

The Rust + Axum service behind `www.amputatorbot.com`. Hosts the
`/api/v1/convert` endpoint and (in M5+) serves the Astro static site from
the same binary via `tower-http::ServeDir`.

The full architectural plan lives in
[`docs/amputatorbot-devvit-migration-plan-v7.md`](../docs/amputatorbot-devvit-migration-plan-v7.md).
This README covers the **day-to-day workflow** — how to build, test, and
validate against real production data.

---

## One-time setup

Everything's pinned through [mise](https://mise.jdx.dev). From the repo
root:

```bash
mise install              # Rust stable + Node 22 + pnpm + just + lefthook
cargo install \
    cargo-nextest \
    cargo-deny \
    cargo-watch \
    sqlx-cli --version "^0.8" --no-default-features --features rustls,postgres
lefthook install          # registers pre-commit hooks
```

If you only ever do `just` commands you'll never need to type `cargo`
directly.

Docker Desktop (or OrbStack / colima) is required for the local Postgres
container that backs the `/api/v1/convert` cache. The first `just db-up`
will pull `postgres:17-alpine` (~110 MB).

## Database

Local Postgres 17 runs in Docker, defined in the root `docker-compose.yml`.
The backend connects via `DATABASE_URL`; defaults to
`postgres://amputatorbot:amputatorbot@localhost:5432/amputatorbot`.

Day-to-day (all from repo root):

| Command | What it does |
|---|---|
| `just db-up` | Boot Postgres in the background. Idempotent. |
| `just db-down` | Stop the container (keeps the data volume). |
| `just db-nuke` | Stop + delete the data volume — full reset. |
| `just db-migrate` | Apply pending sqlx migrations explicitly. |
| `just db-seed` | Load the committed 10k URLConversions sample into `links`. |
| `just db-seed path=<csv>` | Load any CSV with the same column order (e.g. your full ~1.7M-row legacy export). |

Migrations are also auto-applied on backend startup via `sqlx::migrate!()`,
so `just db-migrate` is mainly for "fresh schema without booting the
server".

### sqlx offline mode

The DATABASE-method query in `src/canonical/pg_database.rs` uses the
compile-time-checked `sqlx::query!` macro. To avoid requiring a live DB
during `cargo build` (which would also break CI and Docker builds),
metadata for each macro invocation is cached in `backend/.sqlx/` and
committed to git. `cargo build` reads from there when `SQLX_OFFLINE=true`
or when no `DATABASE_URL` is set.

**When you change any `sqlx::query!` SQL or the schema:** re-run
`cargo sqlx prepare` (from `backend/`, with `DATABASE_URL` set and PG
running) and commit the updated `.sqlx/` JSON. CI will fail if `.sqlx/`
drifts from the actual queries.

### URL-length cap

Both `original_url` and `canonical_url` are constrained to ≤ 2048 chars
in `001_initial.sql`. Mirrored in Rust as `canonical::MAX_URL_LEN` and
enforced by the resolver's validity gate. Legacy rows longer than this
are filtered during `just db-seed` via a staging-table pass; the recipe
reports the imported/skipped counts.

## Daily commands

All run from `backend/`:

| Command | What it does |
|---|---|
| `just check` | Full local check: format-check, clippy `-D warnings`, nextest, cargo-deny. Same as CI. |
| `just test` | Just the test suite (nextest). ~100 tests, ~0.1s. |
| `just fmt` | Apply rustfmt to everything. |
| `just lint` | Clippy with warnings denied. |
| `just dev` | Run the Axum server with cargo-watch (rebuilds on file change). |
| `just build` | Release build. |

## Testing the canonical-finding engine

The resolver is the heart of the backend — given an AMP URL, find its
canonical non-AMP version. Three layers of tests:

### 1. Unit tests (`just test`, always green)

Per-module tests for each piece: AMP detection, URL extraction, the 11
canonical methods, the orchestration loop. Use `MockPageSource` to inject
hand-crafted HTML. Fast (< 0.1s for the lot), no network.

### 2. Snapshot tests (`cargo nextest run --test snapshots`)

Pin the JSON output shape of `resolve()` for ~10 representative scenarios.
Catches accidental response-shape drift (renamed fields, key reordering,
SCREAMING_SNAKE_CASE regressions).

To regenerate snapshots after an **intentional** shape change:

```bash
INSTA_UPDATE=always cargo nextest run --test snapshots
cargo install cargo-insta  # one-time
cargo insta review         # interactive accept/reject
```

### 3. Parity tests — against real URLConversions fixtures

This is the most informative test: takes the legacy Python bot's
`URLConversions` table, fetches each URL's HTML once, then runs our new
Rust resolver against that HTML and compares the result to what the legacy
bot recorded.

#### Step 1 — Record fixtures

Fetch every URL in a CSV export and save the response to
`tests/fixtures/html/<entry_id>.json`:

```bash
just record-fixtures
```

That uses the default CSV (`10000_conversions_unfiltered.csv` — a random
10k-row slice including failures and false positives). For the
successes-only set:

```bash
just record-fixtures tests/fixtures/urlconversions/10000_conversions_with_canonical.csv
```

Properties:

- **Idempotent.** Already-recorded fixtures are skipped, so Ctrl+C and
  re-run is safe.
- **Respectful.** Default 4 concurrent fetches, 15s timeout, rotating
  Firefox user agent.
- **Slow.** Roughly an hour for the full 10k set; faster for repeat runs
  since existing fixtures are skipped.

The HTML directory (`tests/fixtures/html/`) is gitignored — generated
content, can be re-built on demand.

#### Step 2 — Run parity

```bash
just parity
```

Streams progress in real time and writes a Markdown summary to
`tests/parity-report.md` with per-bucket counts and the URLs of the
first 25 mismatches.

Categories:

| Bucket | Meaning |
|---|---|
| **matched** | Same canonical (or both found `None`). |
| **mismatch_url** | Both found a canonical but they disagree on the URL. |
| **legacy_only** | Legacy found a canonical, we didn't. |
| **new_only** | We found a canonical, legacy didn't. Could be a legit false-positive fix (e.g. the `amputeestore.com` shape) or a new bug. |
| **skipped** | Fixture has no HTML (recorder failed for that URL). |

The test only **fails** if zero matches across all fixtures (resolver is
broken). Tighter parity-rate floors land in follow-up commits as drift
gets investigated and fixed.

#### Step 3 — Look at the report

```bash
cat tests/parity-report.md
```

Or open it in your editor. The Markdown is human-readable.

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
│   │   └── record_fixtures.rs # the CSV→HTML recorder
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
│   │       └── database.rs    # DATABASE cache lookup (M3)
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

## Common debugging

**A specific URL fails in `record_fixtures`.** Look at the error chain in
the warning log (we use `?e` which prints the full anyhow cause chain).
DNS / TLS / HTTP-403 / timeout each look different.

**A parity mismatch you want to understand.** Open
`tests/parity-report.md`, find the `csv_id`, then look at the recorded
fixture: `tests/fixtures/html/<csv_id>.json`. The raw HTML is in there
under `html`; the legacy expected canonical is `expected_canonical`.

**Snapshot test failed.** The diff is in the test output. Either fix the
code (response shape regressed) or run with `INSTA_UPDATE=always` to
accept the new shape if it's intentional.

## See also

- [v7 migration plan](../docs/amputatorbot-devvit-migration-plan-v7.md)
- [archive/README-archive.md](../archive/README-archive.md) — the legacy
  Python bot that lives in `archive/` for reference
- [CLAUDE.md](../CLAUDE.md) — conventions for AI-assisted work in this repo
