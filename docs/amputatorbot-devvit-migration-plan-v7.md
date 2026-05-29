 Let# AmputatorBot → Devvit Migration Plan v7

**Status:** living draft
**Drafted:** 2026-05-25
**Budget cap:** €15/month total

---

## Working style — READ THIS FIRST

Joris likes to be consulted on architectural decisions before code is written. For any choice that's:

- About which library, framework, or pattern to use
- About data model design or schema decisions
- About how to structure a new module or layer
- About cost vs. complexity tradeoffs
- About deviating from this plan in any non-trivial way

…stop and ask before proceeding. Surface the options, explain the tradeoffs honestly (including pushback if you disagree), and wait for a decision. Mechanical implementation work doesn't need consultation; architectural shape does.

**External actions Joris does manually, not Claude:**

- Running queries against the production MySQL / Postgres databases (Claude can write the SQL; Joris executes it)
- Signing up for services (Scaleway, Reddit developer account, App Directory submission, bounty program registration)
- Anything that hits an external account with Joris's identity (OAuth logins, `devvit login`, `scw init`, billing changes)
- DNS changes in Cloudflare
- Stopping/starting the production PythonAnywhere bot
- `git push` to production-affecting remotes
- Publishing the Devvit app (`devvit upload --publish`)

Claude prepares the exact commands and tells Joris when to run them. Reading code, writing code, running local tests, local Docker builds, local `cargo`/`npm` — Claude does those.

---

## Project context

**What AmputatorBot is:** A Reddit bot that detects AMP URLs in comments/posts/PMs and replies with the canonical (non-AMP) link. Maintained by Joris (`u/Killed_Mufasa`). Live at https://github.com/jvdburgh/AmputatorBot. ~181 GitHub stars. Runs at scale. Reddit's Developer Platform migration program offers $1,000 to bots that migrate from the Data API to Devvit by Dec 31, 2026.

**What's being migrated:**
- Reddit bot (Python + PRAW) → Devvit app (TypeScript, `@devvit/web`)
- HTTP API (Flask) → Rust (Axum + Mozilla-Readability-port + sqlx)
- Website (Flask + Bootstrap) → Astro 5 + Tailwind 4 + shadcn/ui
- Database (MySQL on PythonAnywhere) → Postgres 17 on Scaleway Managed Database
- Host (PythonAnywhere) → Scaleway (Paris, France — EU-originated managed cloud)

**Architectural model change:** Devvit is per-subreddit opt-in, not global. Mods install the app per sub; it can't see content where it isn't installed. Replies post as per-install app identity, not `u/AmputatorBot`. Accepted. Estimated reply-volume drop: 30–60%.

**Goals (user-stated):**
1. Modern stack, learn things along the way (Rust, Devvit, Astro/Tailwind4/shadcn)
2. API contract stays backwards-compatible
3. Old bot keeps running indefinitely as a fallback — we can break things on the new side without consequence
4. Sledgehammer through the rewrite; preserve old code in `praw-python-archive/`

---

## Locked decisions

| | |
|---|---|
| **Path** | Per-subreddit opt-in, parallel-run cutover (old bot kept alive) |
| **Devvit SDK** | `@devvit/web` (modern server model) |
| **Devvit config** | `devvit.json` (official format with `$schema`) |
| **Devvit scaffold** | `reddit/devvit-template-bare`, stripped of post/menu UI |
| **Backend language** | Rust |
| **Backend framework** | Axum 0.8 (Tokio + Hyper) |
| **Article extraction** | `dom_smoothie` crate (≈0.17, Rust Readability port that closely tracks Mozilla's `readability.js`, actively maintained as of 2026-03) |
| **HTML parsing** | `scraper` crate (built on `html5ever`) |
| **HTTP client** | `reqwest` |
| **Postgres driver** | `sqlx` (compile-time-checked queries) |
| **Database** | Postgres 17 on Scaleway Managed Database (DB DEV-S equivalent or next tier up, sized for ~2GB + growth headroom). 1.7M rows confirmed. |
| **Website** | Astro 5 + Tailwind 4 + shadcn/ui |
| **Hosting** | Scaleway Serverless Containers (Paris/AMS, EU-originated French cloud) |
| **Single service** | One Rust binary serves `/api/*` and Astro static via `tower-http::ServeDir` |
| **Observability** | Scaleway Cockpit (built-in logs + metrics) only initially |
| **Code hosting** | GitHub (existing monorepo) |
| **DNS** | Cloudflare (existing) — also handles rate limiting |
| **API auth** | None on any route — fully open. Cloudflare handles abuse. |
| **Rate limiting** | Cloudflare only, no Rust middleware |
| **Domain** | Single — everything on `www.amputatorbot.com` |
| **Test subreddit** | r/test |
| **AI tooling** | Claude Code + Devvit MCP + JetBrains MCP in IntelliJ |
| **Per-user opt-out** | Dropped. Users block the bot account; the per-install app identity makes block-based opt-out work at the install level. |
| **Per-subreddit opt-out** | Dropped (mods uninstall, or toggle `autoReply` off in install settings) |
| **Modmail / DMs** | None. No inbound modmail handler, no welcome modmail, no DMs — Reddit's "App" badge handles bot disclosure, and the Devvit App Directory listing covers onboarding. |
| **Karma threshold** | None |
| **App Directory submitter** | `u/Killed_Mufasa` |

---

## Why these choices

The rationale behind the decisions above. Each was a deliberate pick over a credible alternative.

- **Rust + Axum over Node/TS or Python.** Single static binary, no runtime, no virtualenv, no `node_modules`. The hard parts of Rust (lifetimes, unsafe, advanced generics) don't really show up in this kind of service (HTTP fetch + regex + HTML query + DB write), so the learning curve is mostly `Result`/`?`, async/Tokio, and sqlx macros — all worth knowing. Axum 0.8 is the consensus framework in 2026 (Tokio + Hyper + Tower middleware). sqlx gives compile-time-checked queries.

C- **`dom_smoothie` over `rs-trafilatura` and other Readability ports.** The existing bot uses `newspaper3k` + `difflib.SequenceMatcher` for guess-and-check confidence scoring (`helpers/article_comparer.py`). For the Rust port we want a Mozilla-Readability-aligned extractor. The candidates as of May 2026:
  - `dom_smoothie` 0.17 (last release 2026-03, ~40k recent downloads, repo `niklak/dom_smoothie`) — closely follows `readability.js`, two modes (Mozilla-mimic + custom), JSON-LD + OG metadata parsing, actively maintained.
  - `readability` 0.3 by kumabook (last release 2023-12, stagnant) — port of arc90's original Readability (predates Mozilla's fork). Higher historical download count but unmaintained.
  - `readable-readability` 0.4 (last release 2022-12, stagnant).
  - `llm_readability` 0.0.17 (active but 0.0.x — too early-stage for production).

  `dom_smoothie` wins on active maintenance + explicit Mozilla `readability.js` lineage. We use its standard (Mozilla-mimic) mode for similarity scoring. No spike needed; pin to `dom_smoothie = "0.17"` in M2.

- **No auth, anywhere.** Production currently has `authorization_required=False` on `/convert` (`AmputatorBotCom/main.py:162`). No one depends on bearer auth, and with opt-out gone the backend has no privileged routes at all. Cloudflare handles abuse.

- **Cloudflare-side rate limiting.** Cloudflare already fronts the domain. Per-IP rules there are zero Rust code and configurable without redeploy.

- **No opt-out mechanic, no modmail, no DMs.** Users and mods who don't want the bot can block the per-install app identity or uninstall the app respectively. That's enough — building a separate opt-out registry on top is mechanism we don't need. Cuts the modmail trigger, the welcome modmail, the bearer-auth privileged routes, the `optouts` table, the Redis opt-out cache, and the legacy "summoned via DM" pathway in one stroke.

- **Modernize the DB schema during migration.** The migration tool runs once. Cleaning up `VARCHAR(4000)` → `TEXT`, adding the indices that `models/table.py` lacks, and dropping any dead columns is cheap to do once and never again.

- **Logs-only observability initially.** Scaleway Cockpit (the built-in log + metric stream) is enough to grep when issues show up. Add Sentry/Bugsink only if log-grep becomes painful.

- **Port the existing AMP detection + canonical methods, but fix false positives in place.** The 14 substring patterns (`static/static.py`) are noisy (`&amp;` HTML entities, `lamp_post`, `streaming_amplifier`). The 10 canonical methods and their order in `helpers/canonical_methods.py` are well-tuned and stay. The improvements happen at the AMP-detection layer (word boundaries, scoping to URL components after extraction) and in fast-path allowlists for high-signal markers.

- **Refresh bot reply markdown.** One-time design pass on a new template, applied at migration. Doesn't need to look identical to the old bot.

- **`git mv` everything old into `praw-python-archive/` in one commit.** Sledgehammer. Old PRAW bot keeps running on PythonAnywhere as a fallback — there's no race to retire it.

- **Six milestones, no calendar.** Local-first: M1–M5 are all local development (scaffolding, canonical engine, API parity, Astro, Devvit playtest). M6 is the cloud + public launch (Scaleway provisioning, prod data migration, DNS cutover, App Directory). No external deadline (bounty is Dec 2026). Old bot covers production. Learning Rust + Devvit + Astro is part of the goal, so pacing should match comprehension, not a schedule.

---

## Architecture

```
┌─────────────────────────────────────────────┐
│ Subreddit X (mod installed amputatorbot)    │
│   Comments / posts                          │
└──────────────┬──────────────────────────────┘
               │ Devvit triggers (HTTP POST to internal routes)
               ▼
┌─────────────────────────────────────────────┐
│ Devvit app (Node, @devvit/web)              │
│   Runs on Reddit infrastructure             │
│                                             │
│   Internal HTTP routes:                     │
│     /internal/triggers/comment-submit       │
│     /internal/triggers/post-submit          │
│                                             │
│   Devvit Redis (built-in):                  │
│     handled:{commentId}  TTL 1h             │
│                                             │
│   Per-install settings:                     │
│     autoReply, customFooter, killSwitch     │
│                                             │
│   Allowlist (devvit.json):                  │
│     http.domains: ["www.amputatorbot.com"]  │
└──────────────┬──────────────────────────────┘
               │ HTTPS (no auth):
               │   GET /api/v1/convert?q=...&gac=true&md=3
               ▼
┌─────────────────────────────────────────────┐
│ Cloudflare (DNS + rate limiting + WAF)      │
└──────────────┬──────────────────────────────┘
               ▼
┌─────────────────────────────────────────────┐
│ Scaleway (Paris/AMS, EU)                    │
│ www.amputatorbot.com                        │
│                                             │
│   Rust backend (Axum 0.8) — single binary   │
│     /api/v1/convert    (existing contract)  │
│     /api/v2/convert    (camelCase JSON)     │
│     /api/v1/stats      (cached count)       │
│     /api/v1/health                          │
│     fallback → Astro static (ServeDir)      │
│                                             │
│   Canonical-finding (10 methods, same       │
│   order as Python impl):                    │
│     REL, CANURL, OG_URL,                    │
│     GOOGLE_MANUAL_REDIRECT, GOOGLE_JS_…,    │
│     BING_ORIGINAL_URL, SCHEMA_MAINENTITY,   │
│     TCO_PAGETITLE, META_REDIRECT,           │
│     GUESS_AND_CHECK (article-similarity     │
│       confidence via Mozilla Readability),  │
│     DATABASE (cache lookup)                 │
│                                             │
│   Postgres 17 (Scaleway Managed Database):  │
│     links                                   │
└─────────────────────────────────────────────┘
```

**Why single Rust service serving API + static site:** Devvit's allowlist is domain-level. One Scaleway Serverless Container keeps the deploy story simple. `tower-http::ServeDir` as fallback is ~5 lines.

---

## API contract

Two endpoints ship side by side:

- **`GET|POST /api/v1/convert`** — legacy contract preserved (snake_case query string, snake_case JSON response). Deprecated for new callers but supported indefinitely so external services don't break.
- **`POST /api/v2/convert`** — modern contract: JSON body in camelCase, JSON response in camelCase. `entryType` is a body field instead of a header. Strict validation (unknown fields → 422). Recommended for Devvit (M5) + website (M4) + new external callers.

Every write to the `links` cache records `api_version` (1 or 2 — NULL for the pre-v7 legacy bot's CSV-imported rows) so adoption is queryable: `SELECT api_version, COUNT(*) FROM links GROUP BY 1`.

### v1 — `GET|POST /api/v1/convert` (from `AmputatorBotCom/main.py:161+`)

**Query params:**
- `q` (required) — URL or text containing URLs
- `gac` — guess-and-check, default `true`
- `md` — max depth, default `static.MAX_DEPTH` (port the value)
- `gc` — generate reply markdown in response, default `false`
- `r` — redirect mode (303 to canonical instead of returning JSON), default `false`

### URL encoding behavior — both must work

The API supports two callers: well-behaved clients that percent-encode their URLs, and humans who paste raw URLs into a browser address bar. The v7 contract:

1. **Encoded URLs always work**, regardless of `q`'s position in the param list. `?q=https%3A%2F%2F...&gac=true` and `?gac=true&q=https%3A%2F%2F...` are both valid.
2. **Unencoded URLs work when `q` is the last param.** `?gac=true&q=https://example.eu/article?id=1` is fine — the URL's tail params (`?id=1`) are preserved. `q` between other known params is best-effort.

The legacy code used a `%20`-presence heuristic to distinguish the two cases, which silently broke for any encoded URL that didn't happen to contain a literal space (the common case once you actually look at production traffic). The v7 port replaces that with the cleaner `://`-in-strip-output check: try the raw-strip path (handles unencoded URLs with tail params); if its output has a literal `://`, use it; otherwise fall back to args-decoded `q` (handles encoded URLs).

The Rust impl is in `backend/src/routes/query_parser.rs`. Test fixtures must cover:
- Encoded URL, `q` first
- Encoded URL, `q` last
- Encoded URL containing no spaces or special chars (the case the legacy heuristic silently broke)
- Unencoded URL, `q` last, with URL-internal `?` and `&`
- Unencoded URL with what looks like `gac=` or `md=` literally in the path (documented limitation — the strip pass would mangle these; rare in real traffic)

**Response (200):** array of `Link` objects matching `models/link.py`. Each contains:
```jsonc
{
  "origin": { "domain": "google", "url": "...", "is_amp": true, "is_cached": true, "is_valid": true },
  "canonicals": [ /* array of {domain, url, type, is_amp, is_cached, is_valid, is_alt, url_similarity} */ ],
  "canonical":   { /* best pick, same shape as canonicals[i] */ },
  "amp_canonical": null  // or a Canonical when origin is the AMP page itself
}
```

Confirmed against live API probe — same shape, same `type` enum values (`DATABASE`, `GOOGLE_MANUAL_REDIRECT`, `REL`, `CANURL`, `OG_URL`, etc. from `CanonicalType` in `models/link.py:23-34`).

**Status codes:**
- `200` — at least one canonical found
- `303` — redirect mode (`r=true`) successful
- `400` — missing `q`
- `406` — no AMP detected (`result_code=error_no_amp`)
- `500` — unknown error

The legacy 560 ("no canonicals") and 561 ("problematic domain") codes are collapsed to 200 + null canonical (per v7 web-standards decision). The 561 problematic-domain list is dropped entirely.

**Bearer auth:** present in code, currently `authorization_required=False`. **Drop in v7 entirely.**

### v2 — `POST /api/v2/convert`

Modern JSON-in / JSON-out contract for new callers.

**Request body** (`Content-Type: application/json`):
```jsonc
{
  "query": "https://www.google.com/amp/s/example.eu/article",  // required
  "guessAndCheck": true,        // optional, default true
  "maxDepth": 3,                // optional, default 3
  "redirect": false,            // optional, default false
  "entryType": "COMMENT"        // optional, default "API". One of:
                                //   "API", "COMMENT", "SUBMISSION",
                                //   "MENTION", "ONLINE".
}
```

Strict validation (`#[serde(deny_unknown_fields)]` on the deserializer):
- Unknown field names → 422 Unprocessable Entity with a "expected one of …" message
- Invalid `entryType` values → 422
- Wrong casing on enum values (`"comment"` vs `"COMMENT"`) → 422

**Response (200)**: array of `Link` objects, identical structure to v1 but every key recursively camelCased.

```jsonc
[
  {
    "origin": { "domain": "google", "url": "...", "isAmp": true, "isCached": true, "isValid": true },
    "canonicals": [ /* {domain, url, type, isAmp, isCached, isValid, isAlt, urlSimilarity} */ ],
    "canonical": { /* best pick, same shape as canonicals[i] */ },
    "ampCanonical": null
  }
]
```

**Error response shape**: same `errorMessage` + `resultCode` body, camelCased.

**Status codes**: same as v1 — 200, 303, 400, 406, 500 — plus 422 for body-deserialize failures (Axum's default `JsonRejection`).

### Cache schema annotation

The `links` table grows a nullable `api_version SMALLINT` column (migration `002_api_version.sql`):

- `NULL` — pre-v7 legacy bot's rows (CSV-imported from PythonAnywhere)
- `1` — `/api/v1/convert` writes
- `2` — `/api/v2/convert` writes

Adding the column on the production 1.7M-row table is metadata-only in Postgres (nullable, no default → no rewrite), millisecond-scale even on Scaleway's smallest tier.

---

## AMP detection — port + improve

**Existing logic:** 14 substring patterns in `static/static.py:8-9` (`/amp`, `amp/`, `.amp`, `amp.`, `?amp`, `amp?`, `=amp`, `amp=`, `&amp`, `amp&`, `%amp`, `amp%`, `_amp`, `amp_`), scanned against lowercase body in `helpers/checker_utils.py:check_if_amp`.

**Port direction:** keep the spirit (broad keyword match) but reduce false positives:

- `&amp` → require not followed by `;` (HTML entity)
- `_amp` / `amp_` → require word boundary on at least one side, or scope to URL path components
- All patterns: apply to extracted URL strings only (after URL extraction), not raw body text
- Add small allowlist of high-signal AMP markers as fast-path: `*.cdn.ampproject.org`, `www.google.com/amp/s/`, `www.bing.com/amp/`, `?amp=1`, `?output=amp`, `amp.*` subdomain — short-circuit to "is AMP" without further pattern matching

Tests per pattern with positive + negative fixtures (incl. `&amp;` HTML entity, `lamp_post`, `streaming_amplifier`, etc.).

---

## Canonical-finding — port verbatim, same order

10 methods in `CanonicalType` enum (`models/link.py:23-34`). Execution flow in `helpers/canonical_methods.py`. Port to Rust with the same priority order:

1. **REL** — `<link rel="canonical">`
2. **CANURL** — regex on `canurl` query/path
3. **OG_URL** — `<meta property="og:url">`
4. **GOOGLE_MANUAL_REDIRECT** — Google AMP cache URL pattern parsing
5. **GOOGLE_JS_REDIRECT** — Google AMP cache JS-redirect handling
6. **BING_ORIGINAL_URL** — Bing AMP cache original-URL extraction
7. **SCHEMA_MAINENTITY** — `<script type="application/ld+json">` schema.org `mainEntity`
8. **TCO_PAGETITLE** — t.co page title heuristic
9. **META_REDIRECT** — `<meta http-equiv="refresh">`
10. **GUESS_AND_CHECK** — pattern-based canonical guess + article-similarity confidence scoring via Mozilla Readability port. Thresholds: `>0.6` high (`is_valid=true`), `>0.35` medium, else reject. Mirrors `canonical_methods.py:152-157`.
11. **DATABASE** — cached canonical lookup (separate from the 10; used as a fast-path before/after methods)

`is_valid` confidence flag on each canonical reflects how strong the signal is. Don't drop this — public consumers may filter on it.

---

## Repo layout (post-archive)

```
AmputatorBot/
├── praw-python-archive/                          # M1: everything below moved here in one git mv
│   ├── check_comments.py
│   ├── check_submissions.py
│   ├── check_inbox.py
│   ├── check_tweets.py
│   ├── helpers/
│   ├── datahandlers/
│   ├── models/
│   ├── data/
│   ├── static/
│   ├── AmputatorBotCom/              # full Flask app, incl. main.py with the API contract
│   ├── img/
│   ├── test*.py
│   ├── requirements.txt
│   └── README-archive.md             # pointer back to v7 plan
├── backend/                          # Rust
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── Dockerfile
│   ├── rust-toolchain.toml           # pins stable + rustfmt + clippy
│   ├── deny.toml                     # cargo-deny config
│   ├── justfile                      # project-local tasks
│   ├── migrations/                   # sqlx
│   ├── src/
│   │   ├── main.rs
│   │   ├── routes/
│   │   ├── canonical/                # 10 methods + amp_detect + url_extract
│   │   ├── db/
│   │   └── readability/              # dom_smoothie wrapper + similarity
│   ├── tests/
│   │   ├── fixtures/
│   │   │   └── urlconversions.json   # user-supplied test cases
│   │   ├── snapshots/                # insta golden JSON
│   │   ├── parity.rs                 # new-vs-live API parity
│   │   ├── canonical_methods.rs
│   │   └── amp_detect.rs
│   └── tools/
│       └── record_fixtures/         # CSV → recorded HTML for parity tests
│                                     # (MySQL → Postgres migration uses
│                                     # `psql \copy` of a CSV export, not
│                                     # a Rust binary — see M3 §migration)
├── devvit-app/                       # TypeScript
│   ├── devvit.json
│   ├── package.json
│   ├── tsconfig.json
│   ├── biome.json                    # Biome lint + format config
│   ├── justfile
│   └── src/server/
│       ├── index.ts
│       ├── routes/triggers.ts        # comment-submit + post-submit only
│       ├── core/
│       │   ├── ampDetect.ts          # mirrors backend logic
│       │   ├── urlExtract.ts
│       │   └── reply.ts              # new reply markdown
│       ├── backend/client.ts         # fetch wrapper (no auth)
│       ├── storage/dedup.ts          # Redis handled:{commentId} TTL 1h
│       └── settings.ts
├── website/                          # Astro
│   ├── astro.config.mjs
│   ├── package.json
│   ├── tsconfig.json
│   ├── biome.json
│   ├── justfile
│   ├── src/
│   │   ├── pages/                    # index.astro, faq.mdx, about.mdx
│   │   ├── components/react/         # ConverterForm island
│   │   └── components/ui/            # shadcn
│   └── public/                       # icons/logos copied from praw-python-archive/AmputatorBotCom/static/
├── .github/
│   ├── workflows/                    # path-filtered CI per project (backend, devvit-app, website, security)
│   └── dependabot.yml                # Cargo + npm + Actions + Docker dep updates
├── docs/
│   └── amputatorbot-devvit-migration-plan-v7.md   # this file
├── .editorconfig
├── .gitignore
├── mise.toml                         # pins Rust + Node versions for the repo
├── lefthook.yml                      # pre-commit hooks dispatcher
├── justfile                          # root tasks: just test, just lint, just fmt
├── pnpm-workspace.yaml               # ties devvit-app + website together
└── README.md                         # rewritten for new architecture
```

---

## Development environment

One-time setup before M1. Doing this up front means the IDE and AI tooling are ready when implementation starts.

### IntelliJ + Claude Code + MCPs

IntelliJ 2025.2+ ships with a built-in MCP server. Enable it so Claude Code can read open files, run code, and use IDE-side tooling.

1. **Enable IntelliJ MCP server.** Settings → Tools → MCP Server → check **Enable MCP Server**.
2. **Auto-configure for Claude Code.** Same panel → "Clients Auto-Configuration" → **Auto-Configure for Claude Code**. This writes the JetBrains entry to `~/.claude.json`.
3. **Add the Devvit MCP.** From a terminal:
   ```bash
   claude mcp add devvit -- npx -y @devvit/mcp
   ```
   This registers the official Devvit MCP server (`reddit/devvit-mcp`). Restart Claude Code afterward.
4. **Verify both are connected.** In Claude Code, run `/mcp` — should list `jetbrains` and `devvit`.

### Reddit/Devvit reference material for Claude

Two docs to add to the context Claude Code sees during this migration:

- **https://developers.reddit.com/docs/llms-full.txt** — the full Devvit docs flattened for LLM consumption. Either WebFetch on demand during a task, or save a snapshot to `docs/reference/devvit-llms-full.txt` and reference it from prompts so it's available offline.
- **https://developers.reddit.com/docs/guides/ai** — Reddit's official guide for using AI assistants (Claude included) to build Devvit apps. Worth reading once and skimming when you hit Devvit-specific decisions.

When asking Claude Code to write Devvit code, the prompt should ideally cite which section of the llms-full doc applies. The Devvit MCP can answer many of these without you having to paste.

### Other toolchain checks

```bash
node --version      # need v22.x
rustc --version     # need 1.80+; if missing: curl https://sh.rustup.rs -sSf | sh
cargo --version

npm install -g devvit
devvit login        # browser OAuth as u/Killed_Mufasa
devvit whoami       # confirms username

brew install scw   # or download from github.com/scaleway/scaleway-cli releases
scw init           # browser OAuth to a Scaleway account, sets up org + project
```

If Rust is brand new on this machine: `rustup default stable`.

### Reddit Developer Platform migration program

Apply at https://support.reddithelp.com/hc/en-us/articles/47822311698452 before M6:
- Bot: `u/AmputatorBot`, operating since ~2019
- Migration target: Devvit, ETA in line with M6
- Submitter: `u/Killed_Mufasa`

Anchoring the registration date probably helps bounty selection.

### Pre-M1 data sizing

Known (measured 2026-05-25 by Joris): `URLConversions` table is **~1.7M rows at 42.56 MB total**. That's ~26 bytes/row on disk (MySQL InnoDB compression on the URL strings is doing real work). Plenty of headroom in any managed PG plan — the smallest Scaleway tier (DB DEV-S, ~€13–15/mo) handles this with **100x growth headroom** before we'd need to size up. Joris provisions it during M1; no further sizing analysis needed.

---

## Development tooling & CI/CD

Modern stack. Each tool picked for "what devs in 2026 actually like and use" rather than legacy familiarity. All tools below are either Rust-written (fast), industry-default, or both.

### Tool version management
- **`mise`** (https://mise.jdx.dev) — replaces `nvm` + `rustup` + `asdf` + similar with one tool. `mise.toml` at repo root pins Rust + Node versions; `mise install` reproduces the toolchain on any machine.

### Rust (`backend/`)

| Concern | Tool | Notes |
|---|---|---|
| Format | `rustfmt` | Default config + `imports_granularity = "Crate"`. `cargo fmt --check` in CI. |
| Lint | `clippy` | `-D warnings` in CI. Curated `clippy.toml` if we hit noisy lints. |
| Test | **`cargo nextest`** | ~60% faster than `cargo test`, better grouped output, per-test isolation. |
| Snapshot tests | **`insta`** | JSON golden-file tests for API responses. `cargo insta review` for diffs. |
| Coverage | `cargo-llvm-cov` | Optional. Native, fast. |
| Security + license audit | **`cargo deny`** | One tool for: known vuln advisories, license violations, duplicate transitive deps. CI-enforced. |
| Pre-commit | Triggered by `lefthook` | Runs `cargo fmt --check` + `cargo clippy --no-deps` on staged Rust files only. |

`rust-toolchain.toml` in `backend/` pins the toolchain channel (stable) and required components (`rustfmt`, `clippy`).

### TypeScript (`devvit-app/`) + Astro (`website/`)

| Concern | Tool | Notes |
|---|---|---|
| Package manager | **`pnpm`** | Fast, disk-efficient via content-addressed store, monorepo-friendly. Devvit + Astro both support it. |
| Format + Lint | **Biome** (https://biomejs.dev) | Single Rust-written tool replacing ESLint + Prettier. 10–100× faster. Handles JS, TS, JSX, JSON. One config file (`biome.json`) per project. |
| Type check | **`tsgo --noEmit`** (TypeScript Native Preview, Go port — ~10× faster than `tsc`) for devvit-app + `astro check` (website, uses bundled TS) | CI-enforced. Installed via `@typescript/native-preview` (currently in preview/alpha — fine for a personal project, downgrade to `tsc` if any incompatibility surfaces). |
| Test | **Vitest** | Devvit's default test runner. Astro's recommended choice. ESM-native, Vite-aligned. |
| Pre-commit | Triggered by `lefthook` | Runs `biome check --write` on staged JS/TS files only. |

### Cross-cutting

- **`just`** (https://github.com/casey/just) — Rust-written task runner, replaces Make. One `justfile` at repo root and one per subproject. `just test`, `just lint`, `just fmt`, `just dev` work consistently across all three projects.
- **`lefthook`** (https://lefthook.dev) — Git hook manager. Fast, parallel, language-agnostic. One `lefthook.yml` at root dispatches to per-project formatters/linters on staged files.
- **`.editorconfig`** at root — tabs/spaces, line endings, trailing whitespace.
- **Dependabot** — dependency updates. Configured via `.github/dependabot.yml`. Manages Cargo, npm, GitHub Actions, Docker base images. Native GitHub integration, zero external app to install.
- **Conventional Commits** — lightly recommended in commit messages (not enforced by hook). Helpful for future changelog generation if we ever want it.

### CI — GitHub Actions

Path-filtered workflows so a TS change doesn't run Rust tests and vice versa.

| Workflow file | Triggers on | Steps |
|---|---|---|
| `.github/workflows/backend.yml` | `backend/**`, `Cargo.lock` | Cache cargo + sccache → `cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo nextest run` → `cargo deny check` |
| `.github/workflows/devvit-app.yml` | `devvit-app/**` | Cache pnpm store → `pnpm install --frozen-lockfile` → `pnpm biome ci` → `pnpm tsgo --noEmit` → `pnpm vitest run` |
| `.github/workflows/website.yml` | `website/**` | Cache pnpm → `pnpm install --frozen-lockfile` → `pnpm biome ci` → `pnpm astro check` → `pnpm vitest run` → `pnpm astro build` |
| `.github/workflows/security.yml` | weekly cron + push to `master` | CodeQL (built-in) + `cargo deny check advisories` + secret scanning verify |
| `.github/workflows/release.yml` | tag push (optional, M6+) | Build Docker image → push to Scaleway Container Registry → deploy/update Serverless Container |

GitHub's free tier covers all of this for a public repo.

### Why these (rather than alternatives)

- **Biome over ESLint + Prettier:** one tool, one config, Rust-written, 10–100× faster. Increasingly the 2026 default. The only reasons to keep ESLint+Prettier are existing config bulk or a specific plugin Biome doesn't support yet — neither applies here.
- **`cargo nextest` over `cargo test`:** faster, better output, per-test isolation. Drop-in replacement.
- **`cargo deny` over `cargo audit` + `cargo license`:** one tool covers vulns, licenses, and dep duplication. CI step is unified.
- **`just` over Make:** modern, fast, predictable cross-shell behavior, no `.PHONY` ceremony.
- **`lefthook` over `husky`:** language-agnostic (we have Rust + TS + Astro in one repo), fast, parallel by default.
- **`mise` over `asdf` + per-language version managers:** single tool covers Node, Rust, anything else we add. Rust-written, fast.
- **`pnpm` over `npm` or `yarn`:** faster installs, disk-efficient. Devvit + Astro both support it.
- **Not Bun:** great runtime, but Devvit's CLI + MCP are tested against Node + pnpm. Sticking with the ecosystem default reduces friction. Revisit later if Bun gains official Devvit support.

---

## Milestones

No calendar. Each milestone has a clear "done" criterion.

### M1 — Sledgehammer + scaffold + tooling (local only) ✓ DONE

**Done when:** old code in `praw-python-archive/`, three empty project skeletons in place, root tooling configured, every project's local check pipeline (`just check`) is green, lefthook pre-commit hooks fire.

Tasks (code/structure):
- `git mv` all old code into `praw-python-archive/`, single commit, write `praw-python-archive/README-archive.md`
- Init `backend/` with `cargo init`, minimal `main.rs` exposing `GET /api/v1/health`, Dockerfile (multi-stage, `cargo chef` for cached deps)
- Init `devvit-app/` with minimal Hono server using `@devvit/web/server`'s `createServer` + `@hono/node-server`'s `getRequestListener`; configure `devvit.json` and `vite.config.ts` per Devvit docs (Vite plugin via `@devvit/start/vite`)
- Init `website/` (Astro 6 + Tailwind 4 + React 19), single landing page, `pnpm astro build` outputs to `dist/`

Tasks (tooling, in order):
- Add `.editorconfig` and `.gitignore` at root
- Add `mise.toml` pinning Rust stable + Node 22 + pnpm 11 + just + lefthook; verify `mise install` reproduces
- Add `pnpm-workspace.yaml` declaring `devvit-app` and `website` as workspaces (incl. `allowBuilds` for esbuild/protobufjs/sharp under pnpm v10+)
- Add `lefthook.yml` at root with per-project pre-commit dispatchers (rustfmt + clippy / biome)
- Add `.github/dependabot.yml` with ecosystems: `cargo`, `npm`, `github-actions`, `docker`
- Add root `justfile` with `test`, `lint`, `fmt`, `dev` recipes that fan out to per-project justfiles
- Per-project: `backend/rust-toolchain.toml`, `backend/deny.toml`, `backend/justfile`, `devvit-app/biome.json`, `devvit-app/justfile`, `website/biome.json`, `website/justfile`
- Add the four GitHub Actions workflows (`backend.yml`, `devvit-app.yml`, `website.yml`, `security.yml`) with path filters
- Open a draft PR; confirm all four CI workflows trigger correctly and pass on the empty projects

**Ask points:** Devvit MCP availability. Any remaining framework picks (Hono vs. Express was resolved in favor of Hono for modernity).

**Scaleway is intentionally NOT in M1.** Local-first: prove the toolchain works end-to-end before engaging with any cloud provider. Cloud provisioning + deployment is M6.

### M2 — Canonical engine in Rust + parity tests ✓ DONE

**Done when:** all 10 canonical methods ported; AMP detection ported with false-positive fixes; fixture-based parity test suite green on user-supplied URLConversations.

Tasks:
- Wire `dom_smoothie` (≈0.17) as the article extractor. Use its Mozilla-mimic mode. Wrap in `backend/src/readability/` with a small adapter (`fn extract_article_text(html: &str) -> String`) so the rest of the code doesn't depend on the crate's surface directly.
- Port 10 canonical methods one by one, with per-method unit tests
- Port AMP detection with the improvements outlined above
- Port URL extraction (Rust `url` crate + regex pass for unstructured text)
- Write `backend/tests/parity.rs` — reads `fixtures/urlconversions.json`, runs each through both the local Rust impl and the live API at `www.amputatorbot.com/api/v1/convert`, diffs the canonical pick. Reports drift with readable output.
- Snapshot tests: ~20 representative cases as golden JSON files in `backend/tests/snapshots/`
- All `cargo test` green

**Ask points:** Edge cases in canonical methods that don't translate cleanly (duck typing, ad-hoc normalization) — surface, don't paper over. If `dom_smoothie`'s standard mode produces similarity scores that misalign with the existing `>0.6` / `>0.35` thresholds on the fixture set, retune the thresholds rather than swapping the extractor.

### M3 — `/api/v1/convert` endpoint + `links` cache (local) ✓ DONE

**Done when:** the Axum handler returns shape-compatible responses to the legacy `/api/v1/convert` against a local Docker Postgres seeded with the historical `URLConversions` data; `?r=true` redirect mode works end-to-end via live HTTP; the `X-AmputatorBot-Entry-Type` header is honored on writes. Two locked deviations from the legacy contract: `?gc=true` is silently ignored (deferred to M5 alongside the Devvit-side reply template refresh — legacy `run_api` never read it either), and the custom 5xx codes 560 + 561 are collapsed to 200 + null canonical (web-standard shape; clients differentiate via the response body).

**Result:** 167 tests (up from 112 at M2). End-to-end smoke verified against the local DB seeded with 9998 historical rows. Both endpoints ship:

- **v1** (`GET|POST /api/v1/convert`) — legacy contract preserved; encoded URLs via `curl --data-urlencode` work, `?r=true` returns 303. `entry_type` always logs `API`.
- **v2** (`POST /api/v2/convert`) — JSON-in/JSON-out, camelCase both ways, `entryType` body field, `api_version=2` written. Strict body deserialization (typos → 422).
- **Schema**: `links.api_version SMALLINT` added (NULL = legacy CSV-imported rows, 1 = v1 writes, 2 = v2 writes).

#### Schema (locked)

Two enums + one table — table name `links` (snake_case, matches `legacy URLConversions` content but with a cleaner name). Migration lives at `backend/migrations/001_initial.sql`, managed by sqlx-migrate.

```sql
CREATE TYPE canonical_type AS ENUM (
    'REL', 'CANURL', 'OG_URL',
    'GOOGLE_MANUAL_REDIRECT', 'GOOGLE_JS_REDIRECT',
    'BING_ORIGINAL_URL',
    'SCHEMA_MAINENTITY', 'TCO_PAGETITLE',
    'META_REDIRECT', 'GUESS_AND_CHECK', 'DATABASE'
);
CREATE TYPE entry_type AS ENUM (
    'ONLINE', 'COMMENT', 'SUBMISSION', 'MENTION',
    'TEST', 'TWEET',    -- historical only; new code never inserts these
    'API'
);
CREATE TABLE links (
    entry_id       BIGSERIAL     PRIMARY KEY,
    entry_type     entry_type,                            -- nullable (some legacy rows)
    handled_utc    TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    original_url   TEXT          NOT NULL,
    canonical_url  TEXT,                                  -- null = failure / false positive
    canonical_type canonical_type,                        -- null when canonical_url is null
    note           TEXT
);
-- Composite supports `WHERE original_url = $1 ORDER BY handled_utc DESC
-- LIMIT 1` (the DATABASE-method lookup) as an index-only scan. Btree
-- prefix matching means it also covers plain `WHERE original_url = $1`,
-- so a separate single-column index on `original_url` is redundant.
CREATE INDEX idx_links_original_url_handled ON links (original_url, handled_utc DESC);
CREATE INDEX idx_links_canonical_url        ON links (canonical_url);
CREATE INDEX idx_links_handled_utc          ON links (handled_utc);
```

Schema decisions (these supersede earlier "drop dead columns" / "modernize" wording in v7):

- **Mirror the legacy columns** — `entry_id`, `entry_type`, `note` all stay. Cleanup in *behavior* (new Rust code only ever inserts `COMMENT` / `SUBMISSION` / `ONLINE` / `MENTION` / `API`), not in *historical preservation*.
- **`canonical_type` is a Postgres ENUM** with SCREAMING_SNAKE_CASE values matching the Rust `CanonicalType` enum's JSON serialization byte-for-byte. sqlx maps both sides cleanly.
- **`entry_type` includes `TEST` and `TWEET`** for the legacy rows that have them; new code never inserts these values but they import cleanly.
- **`entry_id` is `BIGSERIAL`** — autoincrement, explicit inserts during import keep their original IDs, new rows get fresh ones.

#### Migration approach (locked)

**CSV ingest via `psql \copy`** — no Rust binary. M2 already exports the table to CSV (`backend/tests/fixtures/urlconversions/*.csv`); for M3 Joris exports the full ~1.7M rows the same way. Production cutover (M6) reuses the same command against the Scaleway endpoint.

```bash
psql "$DATABASE_URL" -c "\
    \copy links(entry_id, entry_type, handled_utc, original_url, canonical_url, canonical_type, note) \
    FROM 'tests/fixtures/urlconversions/URLConversions_full.csv' \
    WITH (FORMAT csv, HEADER true, NULL '')"
```

~1.7M rows in under a minute. No SSH tunnel, no sqlx wiring just for the import, no Rust binary to maintain.

#### Tasks

1. `docker-compose.yml` at repo root for local Postgres 17 + `DATABASE_URL` plumbing in the backend.
2. `backend/migrations/001_initial.sql` with the schema above; sqlx-migrate setup.
3. **DATABASE canonical method** — port the 11th method (stubbed in M2.5). Modernized from the legacy `LIMIT 1`: prefer the most-recently-resolved canonical, since the bot has run for years and the "right" canonical for a given URL can change over time (sites move, canonical-finding has improved). `entry_id DESC` is a deterministic tiebreaker. The composite index makes this an index-only scan.
   ```sql
   SELECT canonical_url FROM links
   WHERE original_url = $1 AND canonical_url IS NOT NULL
   ORDER BY handled_utc DESC, entry_id DESC
   LIMIT 1
   ```
4. `PgPool` plumbed into `MethodContext` (parallels how `PageSource` got added in M2.8).
5. Axum handler for `GET /api/v1/convert` wrapping `canonical::resolve()`.
6. The `%20` query-parsing heuristic from `praw-python-archive/AmputatorBotCom/main.py:116-127` so unencoded URLs with `q` last still work (the load-bearing public-API contract per §"API contract").
7. Status code mapping: 200, 303 (`r=true`), 400 (no `q`), 406 (no AMP), 500 (unknown), 560 (no canonicals), 561 (problematic domain), with the `result_code` field on error responses.
8. `r=true` — 303 redirect to `link.canonical.url`.
9. `gc=true` reply markdown — port the legacy template, refresh per the v7 decision. Surface the draft for visual review before locking.
10. Integration tests against the handler with real query strings.

**Ask points:** the refreshed `gc=true` reply markdown — needs visual review before lock-in. Whether to also write the legacy `URLConversions_full.csv` export workflow into `backend/README.md` so it's reproducible later.

**Production data migration to Scaleway PG happens in M6**, not here. M3 just verifies the schema + `\copy` flow against a local Docker Postgres.

### M4 — Astro website (local) ✓ DONE

**Done when:** the full Astro site builds locally, all content is in place, the ConverterForm works against the local Rust backend, and the two-stage Dockerfile produces a single image containing both the Rust binary and the Astro static bundle.

**Why M4 (before Devvit):** the website turns the abstract Rust backend into something visible — buttons to click, a form to paste URLs into, immediate "it works" feedback. Easier to keep momentum and easier to validate the API contract by hand.

#### Result

Combined Astro + Rust image (`Dockerfile` at repo root, multi-stage `node:22 → rust:1.95 → debian-slim`, **~165 MB stripped**). `tower-http::ServeDir` mounted as Axum's fallback when `STATIC_DIR` points at an existing directory; the binary stays API-only when unset. Container runs Astro static on `/` and the API on `/api/*`. All 134 backend lib tests + Vitest smoke test green; `astro check` + `biome check` clean.

Homepage is composed of six section components (`HeroSection`, `WhyAmpSection`, `TechSection`, `UkraineSection`, `CanonicalMethodsSection`, plus the layout shell). Order: Hero → WhyAmp → Tech → Ukraine → Methods. No footer.

Locked deviations from the original M4 task list:

- **`/about` page dropped.** Long-form AMP write-up lives on the Reddit "why I built it" thread (`r/AmputatorBot/comments/ehrq3z/...`), linked from the header nav and from the WhyAmpSection card.
- **`/faq` page dropped.** Content folded back to the same Reddit thread; `@astrojs/mdx` and `MdxLayout.astro` removed since nothing else used MDX.
- **`SiteFooter` dropped.** All navigation lives in the header. Mobile nav uses a native `<details>`/`<summary>` collapsing menu — keyboard- and screen-reader-accessible by default, zero JS, near-full-width on mobile.
- **`/amputatorbot` legacy route NOT preserved** (Joris confirmed it was never used in practice). The `?q=<url>` pre-fill on `/` is preserved (legacy bot DM templates deep-link with that pattern).
- **`GET /api/v1/stats` added** (not in original plan). Returns `{"convertedTotal": N}` from `SELECT COUNT(*) FROM links`, cached 1h via `Arc<RwLock<...>>` in `src/stats.rs`. Powers the homepage's live count via the `ConvertedCount` React island.
- **Tiny zero-dep token-based highlighter** at `website/src/lib/highlight.ts` covers html/json/js/url. Both Astro `CodeSnippet.astro` and the React `Snippet` in `ConverterForm.tsx` render the same tokens (no `dangerouslySetInnerHTML`). Token CSS in `global.css`.
- **`gc=true` and the bot reply markdown stay deferred to M5** (already noted in M3 lock). The "Generate comment" option was deliberately *not* added to the converter form to avoid building UI against a template that's about to change.

Tasks (as-built):
- ConverterForm: ?q= prefill, "forward to canonical" via JS navigation, prominent copy-canonical button, "how we found it" card per result with syntax-highlighted snippet, optional-settings panel with breathing room when expanded.
- `canonical-methods.ts` maps every `CanonicalType` to `{label, summary, explanation, snippet, snippetLanguage}`. Consumed by both the result card and the homepage methods explainer.
- WhyAmpSection content sourced from the Reddit "why" thread — antitrust receipts (40% revenue drop, "how to publicly justify making something slower"), AMP committee resignations, empowerment pull-quote.
- UkraineSection: U24 as primary CTA, PayPal sponsorship as alternative. Copy makes the not-affiliated stance explicit.
- TechSection: "rewritten in Rust for perf + because it's fun, now also a Devvit app", EU-hosted (Scaleway Paris/AMS), live count.
- `postcss.config.mjs` wires `@tailwindcss/postcss` — was missing initially, so the first M4 build emitted theme + preflight but no utility classes.
- Image build context now ~10 MB after `.dockerignore` excludes `target/`, `backend/tests/` (~3 GB of recorded HTML fixtures), `node_modules/`, `dist/`, `praw-python-archive/`.

**No cloud, no DNS, no Devvit publish here** — that's M6.

### M5 — Devvit bot E2E

**Done when:** AMP comment in r/test → bot replies with correct canonical; dedup prevents double-reply; the `autoReply` setting silences the bot when toggled off.

**Scope-down vs. earlier drafts.** This milestone was originally going to ship a modmail handler, a welcome modmail on install, a Redis-cached opt-out lookup, an `/api/v1/optout` privileged route on the backend, and a Postgres `optouts` table. All dropped — see the "Locked decisions" table and the "No opt-out mechanic" rationale. Users and mods who don't want the bot block the per-install app identity or uninstall the app respectively.

Tasks:
- Port AMP detection patterns to TS (`devvit-app/src/server/core/ampDetect.ts`) — mirror Rust logic in `backend/src/canonical/amp_detect.rs`
- Port URL extraction to TS (`devvit-app/src/server/core/urlExtract.ts`) — mirror `backend/src/canonical/url_extract.rs`
- Write `backend/client.ts` — fetch wrapper for `POST /api/v2/convert` (camelCase JSON, recorded with `entry_type=COMMENT|SUBMISSION` and `api_version=2`, giving us per-source visibility into the `links` cache). No auth. Configurable base URL via setting (prod = `https://www.amputatorbot.com`, dev playtest = local Rust backend over a tunnel).
- Write `core/reply.ts` — generate the comment markdown from a backend `Link[]` response. See "Reply markdown (locked)" below.
- Write `storage/dedup.ts` — wraps Devvit Redis: `setHandled(commentId)` with 1h TTL, `isHandled(commentId)` returns bool.
- Settings (`devvit.json` + `settings.ts`): `autoReply` (bool, default true — the single on/off toggle), `customFooter` (optional string — appended after the standard footer when set).
- Implement `/internal/triggers/comment-submit`:
  - Early return if `autoReply` is off
  - Extract URLs from comment body, filter to AMP
  - Skip if dedup says we've handled this `commentId` in the last hour
  - `GET /api/v1/convert?q=<url>&gac=true&md=3`
  - Build reply via `reply.ts`, `context.reddit.submitComment`
  - Mark dedup with 1h TTL
- Implement `/internal/triggers/post-submit` — same flow, URLs sourced from `title + url + selftext`, dedup key is `postId`.
- Vitest unit tests for `ampDetect`, `urlExtract`, `reply` (snapshot), and the trigger handlers (mocked backend client, mocked Reddit context).
- Playtest end-to-end via `just dev` (rebuild loop in one terminal) + `just playtest` (defaults to r/AmputatorBotTest) in another. App slug landed on `amputatorbot-app` — Reddit account-namespace collision with the legacy `u/AmputatorBot` blocked the simpler slug.

#### Playtest setup gotchas

- **First-time only:** `just init` registers the app on Reddit's side. The slug in `devvit.json` (`amputatorbot-app`) must not collide with an existing Reddit account.
- **`vite: command not found` in `pnpm exec vite build`:** the workspace's per-project `node_modules/.bin/` got out of sync (commonly after `npx devvit init` or hand-editing `package.json`). Fix by running `pnpm install` from the **repo root**, not from `devvit-app/`. If that still leaves `.bin/` empty, `rm -rf devvit-app/node_modules && pnpm install` from the repo root.
- **CLI version mismatch:** the `devvit` binary must match `@devvit/web` major.minor. Both are pinned at `0.13.0` via `devvit-app/package.json`; if you ever see `@devvit/cli/0.12.x` from `pnpm exec devvit --version`, re-run `pnpm install` from the root.

#### Reply markdown (locked)

Preserves the legacy template's structure and quirks (singular/plural agreement, who/what, cached-AMP note, alt-domain canonical, `*****` rule, superscript footer). Changes from legacy: drop the `^(I'm a bot | )` opener (Reddit's "App" badge handles disclosure), drop the `Summon: u/AmputatorBot` link (doesn't work in Devvit's per-install model), and replace the AMP-disclaimer sentence.

Singular (one AMP URL, one canonical):

```
It looks like {who} {what} an AMP link. AMP is supposed to be faster, but it's controversial because of [concerns over privacy and the Open Web]({FAQ_LINK}).{cached_note}

Maybe check out **the canonical page** instead: **[{canonical_url}]({canonical_url})**{alt_canonical}

*****

^([Why & About]({FAQ_LINK})^( | )[r/AmputatorBot](https://reddit.com/r/AmputatorBot)^( | )[Source](https://github.com/KilledMufasa/AmputatorBot))
```

Plural (multiple AMP URLs):

```
It looks like {who} {what} some AMP links. AMP is supposed to be faster, but it's controversial because of [concerns over privacy and the Open Web]({FAQ_LINK}).{cached_note}

Maybe check out **the canonical pages** instead:

- **[{url1}]({url1})**{alt1}
- **[{url2}]({url2})**{alt2}

*****

^([Why & About]({FAQ_LINK})^( | )[r/AmputatorBot](https://reddit.com/r/AmputatorBot)^( | )[Source](https://github.com/KilledMufasa/AmputatorBot))
```

Variables:
- `{who}` / `{what}` — `OP` / `posted` for post-submit, `you` / `shared` for comment-submit
- `{cached_note}` — when at least one origin URL was cached (`origin.is_cached=true`): ` Fully cached AMP pages (like the one {who} {what}), are [especially problematic]({FAQ_LINK}).` (use "the ones" plural when >1, "some of the ones" when only some are cached — mirror `reddit_comment_generator.py:84-90`)
- `{alt_canonical}` — when a canonical at a different domain is also available: ` | {AltDomain.capitalize()} canonical: **[{alt_url}]({alt_url})**` (mirror `reddit_comment_generator.py:48`)
- `{FAQ_LINK}` — `https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot/`
- `{customFooter}` — when set on the install: appended as ` | {customFooter}` inside the superscript group

**Ask points:** none — content and tasks both locked.

### M6 — Cloud deployment + public launch

**Done when:** the Astro+Rust container is live at `www.amputatorbot.com` on Scaleway, prod Postgres holds the migrated dataset, the Devvit app is in the App Directory, r/AmputatorBot has the announcement sticky, and the old PythonAnywhere bot is still running in parallel as a safety net.

Pre-deploy audit (do BEFORE any Scaleway provisioning — finding stale deps or leaked secrets after deploy is much more expensive):

- **Dependency freshness.** Bump everything to the latest compatible version we can — Postgres stays on 17 (the schema is sized for it; we don't gain anything by chasing a `pg_*` bump now), everything else fair game:
  - Rust: `cargo update` then `cargo outdated -R` to spot anything held back by semver caret pins. Patch `Cargo.toml` for the ones worth pulling forward (Axum, sqlx, reqwest, scraper, dom_smoothie). Re-run `cargo nextest run` + `cargo deny check` after each.
  - TS: `pnpm -r outdated` across `devvit-app/` and `website/`. Update `@devvit/web`, Astro 5/6 LTS, Tailwind, Vitest, Biome, tsgo's preview tag.
  - GitHub Actions: dependabot already handles weekly bumps, but verify the `actions/checkout@v4` / `Swatinem/rust-cache@v2` / `jdx/mise-action@v2` pins in `.github/workflows/` are still current.
  - Docker base image: bump the Rust + Debian base tags in `backend/Dockerfile` to current LTS.
- **Secret scan.** `gitleaks detect --no-banner --redact` against the full history (not just current tree — the `praw-python-archive/static/static.py` credentials are still in older commits). Confirm every leaked credential has either (a) been rotated upstream (Reddit OAuth, MySQL, Twitter, SSH) or (b) is a now-defunct service we no longer use. Also `git grep -nE "Bearer |password|secret|client_secret|api_key" -- ':!archive'` for stray modern leaks.
- **Vulnerability sweep.** `cargo deny check advisories` (force-refresh the RustSec DB), `pnpm -r audit --prod`, GitHub Dependabot alerts dashboard. Triage each finding; fix or accept-with-rationale.
- **Performance sanity.** Build release: `cargo build --release` — confirm the binary stays under ~20 MB stripped (M2 baseline was around there). Run the full parity test (`just parity-full`) and check the report's success rate hasn't regressed since M3 lock. Boot the container locally and `curl` a cold request — record the latency so we have a baseline for Scaleway's cold-start measurement.
- **Code-quality sweep.** `cargo clippy --all-targets -- -D warnings` (already CI-gated, force-run anyway), `pnpm biome ci`, `astro check`. `git grep -nE "TODO|FIXME|XXX|HACK" -- backend/src devvit-app/src website/src` — every hit should either get resolved or be promoted to an issue with a milestone.
- **Docs sanity.** Re-read `README.md`, `backend/README.md`, `CLAUDE.md`, and this plan. Anything that drifted during M3–M5 gets fixed in-place. The Postman API docs link in §References should still describe the live shape (or have a TODO to update post-cutover).

Done criteria for the audit: zero open advisories, zero unrotated secrets in current `HEAD`, all CI checks green on a fresh push, parity report at or above the last locked rate.

Cloud setup (Joris does the signups; Claude prepares exact commands):
- Provision Scaleway Managed Database for PostgreSQL (DB DEV-S tier; the live MySQL is 42.56 MB so smallest tier is plenty)
- Provision Scaleway Container Registry namespace
- Note connection strings + registry URL; Joris sets the env vars on the Serverless Container

Production data migration:
- Export the full `URLConversions` table to CSV from PythonAnywhere MySQL (same export workflow as the M2 fixture CSVs).
- `\copy` it into Scaleway PG using the M3-validated command:
  ```bash
  psql "$SCALEWAY_DATABASE_URL" -c "\copy links(entry_id, entry_type, handled_utc, original_url, canonical_url, canonical_type, note) FROM 'URLConversions_full.csv' WITH (FORMAT csv, HEADER true, NULL '')"
  ```
- Run a small-batch smoke test first (e.g. head -1000 of the CSV) before the full load; verify row counts match the source.

Deploy:
- Build the two-stage Astro+Rust Docker image, push to Scaleway Container Registry
- Deploy the Serverless Container; configure env vars (`DATABASE_URL`)
- Smoke-test the container endpoint via `curl` (health, `/api/v1/convert`, static page, `?r=true` redirect)
- Switch DNS (`www.amputatorbot.com` → Scaleway Serverless Container endpoint) in Cloudflare; verify TLS

Public launch:
- Submit Devvit app to App Directory (`devvit upload --publish`), write `termsprivacy.md`
- Install on r/AmputatorBot (canary), then r/test
- Draft + post r/AmputatorBot sticky announcement explaining the migration + per-install identity reality
- Send modmail wave to top N subreddits (by historical link volume)

Post-launch:
- Old PythonAnywhere bot stays running — no kill, no DNS pressure
- Monitor Scaleway Cockpit logs for 72h

**Ask points:** DNS switch timing. Outreach list size and tone. Whether to keep the old PythonAnywhere bot running indefinitely or set a sunset date.

---

## Testing strategy

The load-bearing question: did we preserve canonical correctness while changing everything else?

### Fixture-based parity (M2 + M3)

User provides `backend/tests/fixtures/urlconversions.json`:
```json
[
  {
    "original_url": "https://www.google.com/amp/s/electrek.co/2018/06/19/.../amp/",
    "expected_canonical": "https://electrek.co/2018/06/19/.../",
    "expected_type": "GOOGLE_MANUAL_REDIRECT",
    "notes": "optional"
  }
]
```

`cargo test --test parity` runs each through:
1. New Rust impl directly
2. Live `https://www.amputatorbot.com/api/v1/convert` (old API)

Asserts:
- New canonical matches expected
- OR new canonical "better" by rule (non-AMP when old returned AMP)
- Reports any drift in a readable diff

### Snapshot tests (M2)

~20 representative responses checked into `backend/tests/snapshots/*.json`. Catches response-shape drift. `INSTA_UPDATE=1 cargo test` to regenerate when intentionally changing shape.

### Per-method unit tests (M2)

Each canonical method tested in isolation with HTML fixtures in `backend/tests/fixtures/html/`. No network. Fast feedback.

### AMP detection tests (M2)

Per-pattern positive + negative fixtures, including the false-positive cases (`&amp;`, `lamp_post`, etc.).

### Devvit-side unit tests (M5)

Vitest. Mock backend client, mock Reddit context. Assert dedup, `autoReply` short-circuit, reply markdown (snapshot test against fixture `Link[]` responses).

### Manual smoke (M3, M4, M5, M6)

- `curl` 10 known URLs against new endpoint, compare to live
- `devvit playtest r/test`: post 5+ fixtures as AMP comments, eyeball replies
- Browser test the Astro ConverterForm with the same fixtures

### What we don't test

- Production traffic — the old bot keeps running, so the new one isn't load-bearing for users until M6+
- DDoS / abuse — Cloudflare's problem
- Devvit infra behavior — Reddit's problem

---

## Gotchas (relevant ones only)

### Rust-specific
- First `cargo build` pulls all transitive deps, ~5 min. Subsequent incremental ~10 sec. Use `cargo check` for fast iteration.
- `sqlx::query!` is compile-time-checked against live DB. Set `DATABASE_URL` at compile time, or use `cargo sqlx prepare` for offline.
- `reqwest` defaults to no redirects-on-POST. For redirect-chain canonical methods, configure: `ClientBuilder::redirect(Policy::limited(10))`.
- Borrow checker errors are educational — read them; the compiler usually tells you exactly what to fix.

### API contract
- The `%20` heuristic is the most fragile thing in the port. Test extensively. Public consumers depend on unencoded-URL behavior.
- `is_alt`, `is_cached`, `url_similarity` are all in the response shape — port them, don't skip.
- 303 redirect mode (`?r=true`) — implement, even if you'd rather not.

### Devvit
- Use `@devvit/web`, not `@devvit/public-api` (legacy Blocks).
- Bot replies post as **per-install app identity**, not `u/AmputatorBot`. Communicate in launch announcement.
- `permissions.http.domains` is domain-level only.
- Triggers are HTTP routes — Reddit POSTs to internal paths.
- `devvit playtest` for one-sub dev install; only `--publish` at M6.

### Scaleway
- Deploy flow: build Docker image locally → push to Scaleway Container Registry → deploy/update Serverless Container at that image tag. Automate via the `scw` CLI or the Terraform provider once it's working manually.
- Serverless Containers bind to the port set via `PORT` env var (default 8080). The Rust binary should read `PORT` and bind to `0.0.0.0:$PORT`.
- Cold-start latency on first request after idle (auto-pause). For a Rust binary this is typically <500ms but worth measuring. If unacceptable, set min-scale to 1 (costs a bit more, no pauses).
- Container memory + CPU are configured per-deployment. Start small (256MB / 100m vCPU) and scale up if profiling shows pressure.
- Managed Database: use the `DATABASE_URL` Scaleway exposes; sslmode=require by default.
- Cockpit (logs/metrics) is enabled per-project — turn it on at provisioning time.
- Cost watch: Serverless Containers free tier is generous, but check the dashboard occasionally — runaway loops can chew through GB-seconds quickly.

### Astro + shadcn
- Path alias `@/*` required in `tsconfig.json` before `shadcn init`.
- React-context-heavy shadcn components (Popover, Dialog, Tabs) need single-`.tsx`-file composition. Not an issue for a single ConverterForm.
- `client:load` for the converter island.

### Don't-shoot-yourself-in-the-foot
- Never commit the Postgres URL. Set it via Scaleway container env vars.
- Old PRAW bot keeps running — don't accidentally stop it.
- Rotate credentials in `praw-python-archive/static/static.py` and `praw-python-archive/AmputatorBotCom/main.py` — they're checked into git history. Anything reachable (Reddit OAuth secrets, Twitter keys, SSH passwords, MySQL passwords) needs rotation regardless of the archive move.

---

## Cost summary

| Component | Provider | Cost |
|---|---|---|
| Devvit app runtime | Reddit | €0 |
| Backend (Rust) | Scaleway Serverless Containers | ~€0–3/mo (free tier covers bot-scale traffic) |
| Postgres (42.56 MB, 1.7M rows) | Scaleway Managed Database, smallest tier (DB DEV-S) | ~€13–15/mo |
| Container Registry (Docker images) | Scaleway Container Registry | ~€0–1/mo (small image footprint) |
| Logs + metrics | Scaleway Cockpit | included |
| Code hosting | GitHub | €0 |
| DNS + rate limiting | Cloudflare | €0 (free tier) |
| Old site (kept running) | PythonAnywhere | $14/mo, indefinitely |

**Honest framing:** all-in for the new stack is **~€15–18/mo**, roughly the same as the current PythonAnywhere bill. This migration is not a cost-reduction play; it's modernization + learning + bounty eligibility. With PythonAnywhere kept in parallel, the total monthly outlay during co-existence is ~€28–32. Steady state (if PA is ever cancelled) returns to ~€15–18.

---

## References

### Reddit
- Migration program: https://support.reddithelp.com/hc/en-us/articles/47822311698452
- Devvit docs: https://developers.reddit.com/docs
- Devvit MCP: https://github.com/reddit/devvit-mcp
- Devvit template: https://github.com/reddit/devvit-template-bare

### Stack
- Axum: https://docs.rs/axum/latest/axum/
- sqlx: https://docs.rs/sqlx/latest/sqlx/
- scraper: https://docs.rs/scraper/latest/scraper/
- reqwest: https://docs.rs/reqwest/latest/reqwest/
- Astro: https://docs.astro.build
- shadcn Astro install: https://ui.shadcn.com/docs/installation/astro

### Scaleway
- Serverless Containers: https://www.scaleway.com/en/docs/serverless-containers/
- Managed PostgreSQL: https://www.scaleway.com/en/docs/managed-databases-for-postgresql-and-mysql/
- Pricing: https://www.scaleway.com/en/pricing/
- `scw` CLI: https://github.com/scaleway/scaleway-cli

### Existing AmputatorBot
- Repo: https://github.com/jvdburgh/AmputatorBot
- Postman docs: https://documenter.getpostman.com/view/12422626/UVC3n93T
