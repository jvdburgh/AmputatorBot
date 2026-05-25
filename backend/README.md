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
    cargo-watch
lefthook install          # registers pre-commit hooks
```

If you only ever do `just` commands you'll never need to type `cargo`
directly.

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
`tests/fixtures/html/<csv_id>.json`:

```bash
just record-fixtures
```

That uses the default CSV (the 500-row sample). For the larger 7000-row
set:

```bash
just record-fixtures tests/fixtures/urlconversions/URLConversions_7000_successful.csv
```

Properties:

- **Idempotent.** Already-recorded fixtures are skipped, so Ctrl+C and
  re-run is safe.
- **Respectful.** Default 4 concurrent fetches, 15s timeout, rotating
  Firefox user agent.
- **Slow.** Roughly 5 minutes for 500 rows, 30+ minutes for 7000.

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
│   │   ├── domain.rs          # extract_domain via psl crate
│   │   ├── http_fetcher.rs    # production PageSource impl (reqwest)
│   │   ├── page.rs            # Page struct
│   │   ├── page_source.rs     # PageSource trait
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
│   │       └── (database — wired in M3)
│   ├── models/                # API JSON shapes
│   │   ├── url_meta.rs        # UrlMeta (base shape)
│   │   ├── canonical.rs       # Canonical struct
│   │   ├── canonical_type.rs  # CanonicalType enum (11 variants)
│   │   └── link.rs            # Link (the top-level response item)
│   └── readability/
│       └── mod.rs             # dom_smoothie wrapper + similarity scoring
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
