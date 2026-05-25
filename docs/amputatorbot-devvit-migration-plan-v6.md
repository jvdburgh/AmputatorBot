# AmputatorBot → Devvit Migration Plan v6

**Status:** final, hackathon-ready
**Format:** standalone document — drop into the repo and reference from Claude Code. Does not assume any prior conversation.
**Budget cap:** €15/month total (today: ~€10/month on PythonAnywhere)
**Bounty deadline:** Dec 31, 2026 (Reddit Developer Platform App Migration Program)

---

## Working style — READ THIS FIRST

**Joris likes to be consulted on architectural decisions before code is written.**

For any choice that's:
- About which library, framework, or pattern to use
- About data model design or schema decisions
- About how to structure a new module or layer
- About cost vs. complexity tradeoffs
- About deviating from this plan in any non-trivial way

…stop and ask before proceeding. Surface the options, explain the tradeoffs honestly (including pushback on Joris's instinct if you disagree), and wait for a decision. Mechanical implementation work doesn't need consultation; architectural shape does.

This plan went through 6 revisions specifically because pushback produced better answers. That pattern continues during the build.

---

## Project context (for fresh Claude Code sessions)

**What AmputatorBot is:** a Reddit bot that detects AMP URLs in comments/posts and replies with the canonical (non-AMP) link. Maintained by Joris (`u/Killed_Mufasa`). Live at https://github.com/jvdburgh/AmputatorBot. Has ~181 GitHub stars, runs at scale, is well-known in the Reddit anti-AMP community. Reddit's Developer Platform migration program offers $1,000 to bots that migrate from the Data API to Devvit by Dec 31, 2026 — this plan is the migration.

**What's being migrated:**
- The Reddit bot (Python + PRAW) → Devvit app (TypeScript, `@devvit/web`)
- The HTTP API (Flask) → Rust (Axum + rs-trafilatura + sqlx)
- The website (Flask + Bootstrap) → Astro 5 + Tailwind 4 + shadcn/ui
- The database (MySQL on PythonAnywhere) → Postgres 16 on Clever Cloud
- The host (PythonAnywhere) → Clever Cloud (Paris, France — EU sovereign)

**Architectural model change:** Devvit is per-subreddit opt-in, not global like the PRAW model. Mods install the app on their subreddit; it can't see content in subreddits where it isn't installed. We're accepting this — long-tail subreddits and cross-sub summons go dark. Estimated reply-volume drop: 30–60%. Worth it for the modern architecture and bounty.

---

## Locked decisions

| | |
|---|---|
| **Path** | Per-subreddit opt-in, big-bang cutover |
| **Devvit SDK** | `@devvit/web` (modern server model) |
| **Devvit config** | `devvit.json` (official format with `$schema`) |
| **Devvit scaffold** | `reddit/devvit-template-bare`, stripped of post/menu UI |
| **Backend language** | **Rust** (replaces Python entirely) |
| **Backend framework** | **Axum 0.8** (replaces Flask) |
| **Article extraction** | **rs-trafilatura** (F1 0.966, best-in-class — beats newspaper3k) |
| **HTML parsing** | `scraper` crate (built on `html5ever`) |
| **HTTP client** | `reqwest` (built on hyper) |
| **Postgres driver** | `sqlx` (compile-time-checked queries) |
| **Database** | Postgres 16 (migrate from MySQL) |
| **Website framework** | Astro 5 + Tailwind 4 + shadcn/ui (modernize from Flask + Bootstrap) |
| **Hosting** | Clever Cloud (French PaaS, EU sovereign) |
| **Error tracking** | None initially — Clever Cloud built-in logs; add Bugsink later if needed |
| **Code hosting** | GitHub (existing monorepo at `jvdburgh/AmputatorBot`) |
| **DNS** | Cloudflare (existing) |
| **AI tooling** | Claude Code + Devvit MCP + JetBrains MCP in IntelliJ |
| **Test subreddit** | r/test |
| **Domain structure** | Single domain — everything on `www.amputatorbot.com` |
| **API surface** | Existing `GET /api/v1/convert` kept identical — no new endpoints |
| **Per-user opt-out** | Kept (PM-based via Reddit) |
| **Per-subreddit opt-out** | Dropped (mods uninstall instead) |
| **Karma threshold** | None |
| **App Directory submitter** | `u/Killed_Mufasa` |

---

## What we're losing — accepted

- Cross-subreddit summons (`u/AmputatorBot` in subs without the app installed → silence)
- Long-tail subreddits won't install
- Estimated 30–60% drop in reply volume
- The `u/AmputatorBot` Reddit account effectively retires from active commenting (Devvit posts as per-install app identity)

What we keep: website, public REST API at unchanged URL, r/AmputatorBot community, canonical-finding accuracy (possibly improved — rs-trafilatura F1 0.966 vs newspaper3k's ~0.89 in benchmarks).

---

## Final stack & cost

| Component | Provider | Jurisdiction | Cost |
|---|---|---|---|
| Devvit app runtime | Reddit | 🇺🇸 (unavoidable) | €0 |
| Backend (Rust on Clever Cloud) | Clever Cloud Docker app | 🇫🇷 EU | ~€5/mo |
| Website (Astro static) | Clever Cloud Static | 🇫🇷 EU | ~€1–3/mo |
| Postgres | Clever Cloud add-on | 🇫🇷 EU | €0 (DEV tier) → €7/mo (prod) if needed |
| Logs/errors | Clever Cloud built-in | 🇫🇷 EU | included |
| Code hosting | GitHub | 🇺🇸 (kept) | €0 |
| DNS | Cloudflare | 🇺🇸 (kept) | €0 |
| Existing site (during cutover only) | PythonAnywhere | (kept ~2 weeks) | €10/mo → €0 |

**Cutover-period total:** ~€15/month worst case (briefly)
**Post-migration steady state:** ~€6–11/month

---

## Architecture

```
┌─────────────────────────────────────────────┐
│ Subreddit X (mod installed amputatorbot)    │
│   Comments / posts / modmail                │
└──────────────┬──────────────────────────────┘
               │ Devvit triggers → POST to internal HTTP routes
               ▼
┌─────────────────────────────────────────────┐
│ Devvit app (Node, @devvit/web)              │
│  Runs on Reddit infrastructure              │
│                                             │
│  Internal HTTP routes (triggers):           │
│   /internal/triggers/comment-submit         │
│   /internal/triggers/post-submit            │
│   /internal/triggers/modmail                │
│   /internal/on-app-install                  │
│                                             │
│  Devvit Redis (built-in):                   │
│   handled:{commentId}  TTL 1h               │
│   optout:{username}    TTL 24h              │
│                                             │
│  Per-install settings:                      │
│   autoReply, customFooter, killSwitch       │
│                                             │
│  Allowlist (devvit.json):                   │
│   http.domains: ["www.amputatorbot.com"]    │
└──────────────┬──────────────────────────────┘
               │ HTTPS GET /api/v1/convert?gac=true&md=3&q=<URL>
               │ + HMAC headers (internal callers only)
               ▼
┌─────────────────────────────────────────────┐
│ Clever Cloud (Paris, EU)                    │
│ www.amputatorbot.com                        │
│                                             │
│  ┌─────────────────────────────────────┐    │
│  │ Rust backend (Axum 0.8)             │    │
│  │  GET /api/v1/convert  ← unchanged   │    │
│  │  (and any other existing endpoints) │    │
│  │                                     │    │
│  │  HMAC verification if header        │    │
│  │  present → privileged mode          │    │
│  │  (used by Devvit app for opt-out    │    │
│  │  checks and rate-limit bypass)      │    │
│  │                                     │    │
│  │  Canonical-finding (10 methods):    │    │
│  │   - HTML canonical link tag         │    │
│  │   - og:url meta                     │    │
│  │   - HTTP redirect chain             │    │
│  │   - rs-trafilatura article extract  │    │
│  │     + similarity scoring            │    │
│  │   - … 6 more from existing impl     │    │
│  │                                     │    │
│  │  Also serves Astro static build at  │    │
│  │  / (via tower-http ServeDir)        │    │
│  └─────────────────────────────────────┘    │
│                                             │
│  ┌─────────────────────────────────────┐    │
│  │ Postgres 16 add-on                  │    │
│  │  - links (200K+ rows, migrated)     │    │
│  │  - optouts                          │    │
│  │  - replies (stats)                  │    │
│  │  - kill_switch (single-row safety)  │    │
│  └─────────────────────────────────────┘    │
└─────────────────────────────────────────────┘
```

**Why single-service backend serving both API and static files:**

Devvit's allowlist is domain-level (not path-level). One entry, `www.amputatorbot.com`, covers everything. Clever Cloud doesn't natively do path-routing between two apps on one hostname. Simplest: one Rust service serves `/api/*` and falls through to serving the Astro build for everything else. `tower-http`'s `ServeDir` middleware in Axum does this in ~5 lines. No reverse proxy, no path-routing config, no shared-hostname gymnastics.

---

## Repository layout (monorepo)

Existing `jvdburgh/AmputatorBot` repo gets these additions/changes:

```
AmputatorBot/
├── backend/                        # NEW: Rust backend (replaces all Python)
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── Dockerfile                  # for Clever Cloud deploy
│   ├── migrations/                 # sqlx migrations
│   │   ├── 001_initial.sql
│   │   └── 002_indices.sql
│   ├── src/
│   │   ├── main.rs                 # Axum app, route registration
│   │   ├── routes/
│   │   │   ├── convert.rs          # GET /api/v1/convert
│   │   │   ├── health.rs
│   │   │   └── static_files.rs     # serves Astro build
│   │   ├── canonical/
│   │   │   ├── mod.rs
│   │   │   ├── methods.rs          # 10 canonical-finding methods
│   │   │   ├── amp_detect.rs       # 14 AMP regex patterns
│   │   │   └── url_extract.rs
│   │   ├── db/
│   │   │   ├── mod.rs
│   │   │   ├── links.rs
│   │   │   ├── optouts.rs
│   │   │   └── replies.rs
│   │   ├── auth/
│   │   │   └── hmac.rs             # optional HMAC for privileged callers
│   │   └── lib.rs
│   ├── tests/
│   │   └── canonical_test.rs
│   └── tools/
│       └── mysql_to_postgres.rs    # one-shot data migration binary
├── devvit-app/                     # NEW: Devvit TypeScript app
│   ├── devvit.json
│   ├── package.json
│   ├── tsconfig.json
│   ├── vitest.config.ts
│   ├── .nvmrc
│   ├── tools/build.ts              # esbuild config from template
│   └── src/
│       └── server/
│           ├── index.ts
│           ├── routes/triggers.ts
│           ├── core/
│           │   ├── ampDetect.ts    # 14 patterns mirrored from backend
│           │   ├── urlExtract.ts
│           │   └── reply.ts
│           ├── backend/
│           │   ├── client.ts       # HMAC-signed fetch to backend
│           │   └── hmac.ts
│           ├── storage/
│           │   ├── dedup.ts        # Devvit Redis
│           │   └── optoutCache.ts
│           └── settings.ts
├── website/                        # NEW: Astro site (replaces Flask)
│   ├── astro.config.mjs
│   ├── package.json
│   ├── tsconfig.json
│   ├── components.json             # shadcn config
│   ├── src/
│   │   ├── pages/                  # .astro + .mdx
│   │   ├── content/                # MDX: FAQ, About, Why
│   │   ├── components/
│   │   │   ├── react/              # converter form (interactive island)
│   │   │   └── ui/                 # shadcn components
│   │   ├── layouts/
│   │   └── styles/
│   └── public/
├── .github/
│   └── workflows/
│       ├── backend.yml             # Rust build + test, path-filtered
│       ├── devvit.yml              # TS test on devvit-app/
│       └── website.yml             # Astro build on website/
├── docs/
│   └── migration-plan.md           # this file
├── data/                           # existing — retire after cutover
├── datahandlers/                   # existing — superseded by backend/src/canonical/
├── helpers/                        # existing — superseded by backend/src/canonical/
├── models/                         # existing — keep until cutover
├── static/                         # existing — retire (website/public/ replaces)
├── img/                            # existing
├── logs/                           # existing — retire (logs go to Clever Cloud)
├── check_comments.py               # RETIRE Day 10
├── check_submissions.py            # RETIRE Day 10
├── check_inbox.py                  # RETIRE Day 10
├── check_tweets.py                 # DELETE (inactive)
├── requirements.txt                # RETIRE (Python gone after cutover)
└── README.md                       # update to reflect new architecture
```

**Path-filtered CI** so Rust tests don't run on TS-only changes and vice versa.

---

## Backend API — unchanged contract

**Keep the existing API exactly as documented at https://documenter.getpostman.com/view/12422626/UVC3n93T.**

This means:
- Same URL: `GET https://www.amputatorbot.com/api/v1/convert`
- Same query parameters (presumably `q=<url>`, `gac` for guess-and-check, etc. — **inspect existing Flask code Day 1 and replicate exactly**)
- Same response JSON shape
- Same status codes
- Same rate-limit behavior for unauthenticated callers

**One addition, invisible to public callers:** if request includes `X-AmpBot-Signature` and `X-AmpBot-Timestamp` headers with valid HMAC-SHA256 over `<timestamp>.<query-string>`, treat as privileged caller — bypass rate limits, include opt-out status for `user` query param if provided. This is what the Devvit app uses. Public consumers see no behavioral change.

**Why HMAC instead of API key:** key rotation is easier; signed payload prevents replay (60s window); secret never traverses the wire as a token.

---

## Devvit `devvit.json`

```json
{
  "$schema": "https://developers.reddit.com/schema/config-file.v1.json",
  "name": "amputatorbot",
  "server": {
    "dir": "dist/server",
    "entry": "index.js"
  },
  "triggers": {
    "onCommentSubmit": "/internal/triggers/comment-submit",
    "onPostSubmit": "/internal/triggers/post-submit",
    "onModMail": "/internal/triggers/modmail",
    "onAppInstall": "/internal/on-app-install"
  },
  "permissions": {
    "http": {
      "domains": ["www.amputatorbot.com"]
    }
  },
  "scripts": {
    "dev": "node --experimental-strip-types ./tools/build.ts --watch",
    "build": "node --experimental-strip-types ./tools/build.ts --minify"
  }
}
```

No `post` section (no UI), no `menu` items (pure trigger bot).

---

## Why Rust + Axum + rs-trafilatura

**rs-trafilatura is the highest-scoring article extraction library on the ScrapingHub benchmark** — F1 0.966, beating Python's original trafilatura, all Node alternatives, and all JVM options. Active development (last release ~March 2026). Mozilla Readability and similar Node options score ~F1 0.89. This is a real upgrade over newspaper3k, not a regression.

**Axum 0.8 is the consensus Rust web framework in 2026.** Built on Tokio + Hyper (same team), Tower middleware ecosystem, type-safe handlers, async-first. Production-ready, broadly adopted.

**sqlx gives compile-time-checked SQL queries.** Typos against the schema fail at `cargo build`, not in production. Postgres-native, async, no ORM ceremony.

**Single static binary deploy.** No runtime, no virtualenv, no `node_modules`, no JVM. Drops as Dockerfile + binary on Clever Cloud.

**Not a Google project.** Rust is an independent open-source project under the Rust Foundation, no single corporate sponsor. Ideologically consistent for an anti-AMP project (AMP being a Google initiative).

**Honest tradeoff: Rust has a learning curve** if Joris hasn't used it before. The borrow checker, lifetimes, async semantics take some time. Mitigation: the canonical-finding logic is mostly mechanical (regex + HTML queries + HTTP fetches + DB lookups) — the hard parts of Rust (lifetimes, unsafe, advanced generics) don't show up in this service. **If Rust experience is zero, expect Day 1 backend work to take 2-3 days instead of 1.** Adjust hackathon expectations accordingly.

---

# Day 0 — TONIGHT

Do these tonight so Day 1 isn't burned on signups.

## 0.1 — MCP setup in IntelliJ + Claude Code

IntelliJ 2025.2+ has built-in MCP server support.

1. IntelliJ → Settings → Tools → MCP Server → check "Enable MCP Server"
2. "Clients Auto-Configuration" → click **Auto-Configure for Claude Code**
3. This writes the JetBrains MCP entry to `~/.claude.json` automatically

Then manually merge the Devvit MCP into the same file:

```json
{
  "mcpServers": {
    "jetbrains": {
      "command": "npx",
      "args": ["-y", "@jetbrains/mcp-proxy"]
    },
    "devvit-mcp": {
      "command": "npx",
      "args": ["-y", "@devvit/mcp"]
    }
  }
}
```

Restart Claude Code. Run `/mcp` — should show both. **Fix any issues tonight, not tomorrow morning.**

## 0.2 — Clever Cloud account

https://www.clever-cloud.com → sign up. SEPA accepted, EU billing.

Just create the account tonight. App provisioning happens tomorrow.

## 0.3 — Reddit Migration Program registration

Apply at https://support.reddithelp.com/hc/en-us/articles/47822311698452:
- Bot: `u/AmputatorBot`
- Operating since: check repo's first commit date (~2019)
- Weekly active users: cite r/AmputatorBot subscriber count + reply-volume estimate from link DB
- Migration target: Devvit, ETA Q3 2026
- Submitter: `u/Killed_Mufasa`

Anchoring the registration date probably helps bounty selection.

## 0.4 — Reddit developer account

Visit https://developers.reddit.com — log in with `u/Killed_Mufasa`, complete any developer onboarding.

## 0.5 — Verify Devvit CLI

```bash
npm install -g devvit
devvit login        # browser OAuth
devvit whoami       # confirms username
```

## 0.6 — Toolchain check

```bash
node --version      # need v22.x
rustc --version     # need 1.80+; if missing: curl https://sh.rustup.rs -sSf | sh
cargo --version
```

If Rust is brand new: `rustup default stable` to install the toolchain. Skim https://doc.rust-lang.org/book/ tonight if you've never used it — even just chapters 1-4 (ownership, structs, error handling) help.

## 0.7 — Check existing data size

SSH'd into PythonAnywhere:

```sql
SELECT COUNT(*) FROM links;
SELECT table_name,
       ROUND((data_length + index_length) / 1024 / 1024, 2) AS size_mb
FROM information_schema.tables
WHERE table_schema = DATABASE();
```

If total > 200MB, provision Clever Cloud Postgres prod tier (€7/mo) Day 1 instead of dev (256MB cap).

## 0.8 — Inspect existing API endpoint shape

While SSH'd in or looking at the live site:

```bash
curl 'https://www.amputatorbot.com/api/v1/convert?gac=true&md=3&q=https%3A%2F%2Fwww-cnn-com.cdn.ampproject.org%2Fc%2Fs%2Fwww.cnn.com%2Fsample'
```

**Save the response.** This is the contract the Rust rewrite must match exactly. Look at the live Python/Flask code for the route handler — note all query params, response fields, status codes, edge cases. The Rust implementation will be measured against this; any drift breaks public API consumers.

## 0.9 — Save the plan into the repo

```bash
cd /path/to/AmputatorBot
git checkout -b devvit-migration
mkdir -p docs
# copy this file as docs/migration-plan.md
git add docs/migration-plan.md
git commit -m "docs: add Devvit migration plan v6"
git push -u origin devvit-migration
```

---

# Day 1 — HACKATHON (~8 hours, possibly 10-12 if Rust is new)

## Hour 0 — warmup (30 min)

1. Open Claude Code in IntelliJ
2. Run `/mcp` — confirm both `jetbrains` and `devvit-mcp` are connected
3. Open `docs/migration-plan.md` (this file) in IntelliJ
4. Tell Claude Code: "Read docs/migration-plan.md. We're at Day 1, Hour 0. Scan the existing repo via JetBrains MCP, identify anything that surprises you, then start Hour 1. Ask before architectural decisions."

## Hour 1–4: backend foundation

**Provision Clever Cloud infra:**

```bash
npm install -g clever-tools
clever login

cd /path/to/AmputatorBot/backend     # create dir first
clever create -t docker -r par amputatorbot-backend
clever addon create postgresql-addon --plan dev amputatorbot-db
clever service link-addon amputatorbot-db
```

**Initialize Rust project:**

```bash
cd backend
cargo init --name amputatorbot-backend
```

Add dependencies to `Cargo.toml`:
```toml
[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["fs", "trace", "cors"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "macros", "chrono"] }
reqwest = { version = "0.12", features = ["json", "gzip"] }
scraper = "0.21"
rs-trafilatura = "0.5"     # check crates.io for current version
serde = { version = "1", features = ["derive"] }
serde_json = "1"
regex = "1.11"
url = "2.5"
hmac = "0.12"
sha2 = "0.10"
hex = "0.4"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

**Write `migrations/001_initial.sql`** — Postgres schema for `links`, `optouts`, `replies`, `kill_switch`. Inspect existing MySQL schema first to mirror column types and constraints.

**Run migration:**
```bash
clever env  # get POSTGRESQL_ADDON_URI
psql "$POSTGRESQL_ADDON_URI" < migrations/001_initial.sql
```

**Write `tools/mysql_to_postgres.rs`** as a one-shot binary that:
- Connects to existing MySQL via SSH tunnel (use `ssh -L` from outside Rust, simpler)
- Pages through `links` table with `mysql` crate
- Inserts into Postgres via `sqlx` with `ON CONFLICT DO NOTHING`
- Same for `optouts`

Run with small batch first (`LIMIT 100`), then full.

**Write minimal `src/main.rs`** with `GET /api/v1/health` only — confirms Axum is wired correctly:

```rust
use axum::{routing::get, Router, Json};
use serde_json::json;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/v1/health", get(|| async {
            Json(json!({"ok": true, "version": env!("CARGO_PKG_VERSION")}))
        }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

**Write `Dockerfile`** — multi-stage build, produces small static binary:

```dockerfile
FROM rust:1.80 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/amputatorbot-backend /usr/local/bin/
EXPOSE 8080
CMD ["amputatorbot-backend"]
```

**Deploy:**
```bash
git push clever devvit-migration:master
```

Hit `https://amputatorbot-backend.cleverapps.io/api/v1/health` → JSON response.

**Ask point:** if Clever Cloud Docker build fails or the Rust toolchain doesn't behave on their infra, stop and discuss. Building Rust in Docker on a 1GB build env can be tight on memory.

## Hour 4–6: port canonical-finding to Rust

This is the biggest single piece of work. The existing Python code in `helpers/` and `datahandlers/` has 10 canonical-finding methods. Port each:

1. **HTML `<link rel="canonical">` tag** — use `scraper` crate, ~5 lines
2. **HTML `<meta property="og:url">`** — same, ~5 lines
3. **HTTP redirect chain** — `reqwest::Client::builder().redirect(Policy::limited(10))`, fetch and read final URL
4. **AMP URL pattern transformations** — direct regex translation of existing Python patterns
5. **Database cache lookup** — `sqlx::query!` against `links` table
6. **rs-trafilatura article extraction + similarity** — fetch candidate canonicals, extract article content, compare to AMP page's extracted content. This *replaces* the newspaper3k path with higher accuracy.
7-10. Other methods from existing implementation — port mechanically

Each method returns `Option<(String, &'static str, f32)>` — canonical URL, method name, confidence — or `None`.

**Also port:**
- 14 AMP detection regex patterns into `src/canonical/amp_detect.rs`
- URL extraction (URLExtract equivalent — Rust's `url` crate + a regex pass)

Unit tests as you go. Don't skip — `cargo test` is fast and these are the methods the bot's reputation rests on.

**Ask point:** if you find Python edge cases that don't translate cleanly (e.g. duck-typing tricks, ad-hoc data normalization), surface them. Don't paper over them — replicate intent, ask if unclear.

## Hour 6–7: `GET /api/v1/convert` endpoint

Match the existing API contract **exactly** (from §0.8 inspection):

```rust
async fn convert(
    Query(params): Query<ConvertParams>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ConvertResponse>, AppError> {
    // Check optional HMAC headers → privileged mode
    let privileged = verify_hmac_if_present(&headers, &params, &state.hmac_secret)?;

    // Rate limit if not privileged
    if !privileged {
        state.rate_limiter.check(&extract_client_id(&headers))?;
    }

    // Resolve canonicals
    let results = state.resolver.resolve(&params.q, params.gac, params.md).await?;

    // Optional opt-out check (only for privileged callers with user param)
    let user_opted_out = if privileged && params.user.is_some() {
        state.db.is_opted_out(&params.user.unwrap()).await?
    } else {
        false
    };

    Ok(Json(ConvertResponse { results, user_opted_out, /* ... */ }))
}
```

(Exact field names match what existing API returns. Don't invent.)

Deploy, test from `curl` with a known AMP URL.

## Hour 7–8: Devvit scaffold + first end-to-end

```bash
cd /path/to/AmputatorBot
mkdir devvit-app && cd devvit-app
git clone --depth 1 https://github.com/reddit/devvit-template-bare temp-template
cp -r temp-template/{src,tools,package.json,tsconfig.json,.nvmrc,devvit.json,.gitignore} .
rm -rf temp-template
npm install
```

1. Edit `devvit.json` per §"Devvit devvit.json" above. **Remove `post` and `menu` sections** from the template.
2. Port the 14 AMP detection patterns from Rust (or Python — same regexes) to `src/server/core/ampDetect.ts`. Use Devvit MCP to confirm current `@devvit/web` patterns for trigger handlers.
3. Write `src/server/backend/client.ts` — HMAC-signed fetch to `https://www.amputatorbot.com/api/v1/convert` (use Web Crypto API for HMAC).
4. Write `/internal/triggers/comment-submit` end-to-end:
   - Detect AMP URLs in comment body
   - Check Devvit Redis dedup
   - Call backend `/api/v1/convert` with HMAC headers + `user=<commenter>` query param
   - If `user_opted_out` or no canonicals → return early
   - Build reply markdown
   - `context.reddit.submitComment(...)`
   - Mark in Redis with 1h TTL
5. Set the shared HMAC secret:
   ```bash
   # Backend
   clever env set HMAC_SECRET "$(openssl rand -hex 32)"
   # Devvit (same value)
   devvit settings set HMAC_SECRET "<same value>"
   ```
6. `devvit upload && devvit playtest r/test`
7. Post AMP comment on r/test → verify bot replies with correct canonical

**End of Day 1:** end-to-end working on r/test. Migration MVP done.

---

# Day 2 (~4 hours): opt-out, settings, modmail

1. `/internal/triggers/post-submit` — same as comment but URL sources include `post.title + post.url + post.body`
2. `/internal/triggers/modmail` — parse `opt me out` / `opt me in`, call backend (extend the existing API or add a small additional route — discuss before coding), reply confirmation
3. Settings in `src/server/settings.ts`:
   - `autoReply` (bool, default true)
   - `customFooter` (string, optional)
   - `killSwitch` (bool, default false)
4. `/internal/on-app-install` — send welcome modmail to installing subreddit

**Ask point:** the existing API doesn't have an opt-out endpoint exposed publicly. We need a write path. Options: add a privileged-only `POST /api/v1/optout` endpoint, or call DB directly from Devvit (no — Devvit can't talk to Postgres). Discuss.

---

# Day 3 (~3 hours): polish, rate limits, kill switch

1. Per-subreddit rate limiter in Devvit Redis: max 10 replies / 10 min, configurable per install
2. Global kill switch: `SELECT enabled FROM kill_switch` at request entry — if true, backend returns 503 → Devvit handler exits cleanly
3. Reply markdown — match existing bot tone (look at recent r/AmputatorBot replies for template)
4. Structured logging via `tracing` crate on backend, `console.log` with JSON on Devvit side

---

# Day 4 (~2 hours): App Directory submission

1. Write `devvit-app/README.md` for App Directory listing
2. Write `devvit-app/termsprivacy.md`:
   - Data sent to `www.amputatorbot.com`: AMP URLs from public comments/posts, subreddit, comment ID, username
   - Storage: Clever Cloud (EU, France)
   - Retention: link cache (URLs only), opt-out list (usernames only)
   - GDPR data subject request process
3. `devvit upload --publish`
4. Reddit review: few business days

---

# Days 5–8: website rewrite + comms prep

Reddit is reviewing. Use the wait.

## Astro website

```bash
cd /path/to/AmputatorBot
npm create astro@latest website -- --template with-tailwindcss --install --add react --git=false
cd website
npx shadcn@latest init
npx shadcn@latest add button card input form
```

Set up path alias `@/*` → `./src/*` in `tsconfig.json` (shadcn requires).

Port existing Flask template content:
- Landing (`pages/index.astro`)
- FAQ (`pages/faq.mdx`)
- About / Why (`pages/about.mdx`)
- Changelog (link to Reddit, or `pages/changelog.mdx`)

Build the AMP converter form as React island in `src/components/react/ConverterForm.tsx` — wraps shadcn `<Input>`, `<Button>`, `<Card>`. POSTs to `/api/v1/convert` on same origin (no CORS needed).

**Gotcha:** shadcn components using React context (Dialog, Popover, Tabs) must compose within a single `.tsx` file, not be split across `.astro` files. For a single converter form, this isn't an issue.

**Build into the Rust backend's static dir:**

The Astro build outputs to `website/dist/`. The Rust backend serves these files via `tower-http`'s `ServeDir` when `/api/*` doesn't match:

```rust
let app = Router::new()
    .route("/api/v1/convert", get(convert))
    .route("/api/v1/health", get(health))
    .fallback_service(ServeDir::new("../website/dist"));
```

Deploy strategy: build Astro as part of the Dockerfile, copy `dist/` into the Rust container. Two-stage Dockerfile, one image, one Clever Cloud app.

**Ask point:** existing site might have content I haven't anticipated. Day 5 morning: audit live site, list every page, surface anything that changes scope.

## Comms prep

- Draft r/AmputatorBot sticky announcement
- Write FAQ entry: "Why did the bot stop replying in my subreddit?"
- Pull top 50 subreddits from link DB by AMP volume
- Draft personalized modmails to those mod teams

---

# Days 9–10: big-bang launch

Assuming app approved:

1. Install app on r/AmputatorBot (you're a mod)
2. Install on r/test
3. Send modmail wave to top 50 subreddits
4. Publish r/AmputatorBot announcement sticky
5. Switch DNS in Cloudflare: `www.amputatorbot.com` → Clever Cloud
6. **Stop the PRAW bot** on PythonAnywhere (kill cron, scheduled tasks)
7. Monitor Clever Cloud logs + Devvit logs for 72 hours straight

**Ask point:** before DNS switch, discuss rollback plan. Keep PythonAnywhere alive as hot standby until Day 14 minimum.

---

# Days 11–14: soak

- Watch error logs, fix what breaks
- Track install conversions from modmail outreach
- If volume way below pre-cutover by Day 14, second outreach wave (subs 51–200)
- Day 14+: cancel PythonAnywhere once stable

---

# Day 30: bounty submission

Submit at https://support.reddithelp.com/hc/en-us/articles/47822311698452:
- Old bot was on Data API ✓
- New Devvit app is live ✓
- App serves ≥1000 WAU community ✓
- Submitter: `u/Killed_Mufasa`

Expected: $1000, weeks later.

---

## Gotchas — read before hitting walls

### Devvit-specific

- **`@devvit/web` vs `@devvit/public-api`.** Use `@devvit/web` (modern server model). Public-api is legacy Blocks.
- **`permissions.http.domains` is domain-level only**, not path-level. One entry covers all paths on that domain. Reddit reviews the domain when you publish.
- **Reddit maintains the allowlist, not us.** Some domains are pre-approved (e.g. Discord webhooks). Test access; if blocked, request via Reddit support. `www.amputatorbot.com` is unlikely to face issues but worth confirming early.
- **Triggers in `@devvit/web` are HTTP routes.** `"onCommentSubmit": "/internal/triggers/comment-submit"` means Reddit POSTs there with event payload.
- **Bot replies post as per-install app identity**, not `u/AmputatorBot`. Communicate this in launch announcement.
- **`devvit playtest` = dev install on one sub.** Don't `--publish` until Day 4.
- **`devvit logs` is your friend.** Tail during hackathon.
- **No `r/all` stream equivalent.** Triggers only fire in installed subreddits.

### Rust-specific (if new to Rust)

- **Compile times suck the first time.** First `cargo build` pulls all transitive deps, ~5 min. Subsequent incremental builds are ~10 sec. Use `cargo check` (no codegen) for the fastest feedback loop while iterating.
- **`sqlx::query!` and `query_as!` macros are compile-time-checked against the live database.** You need `DATABASE_URL` env var pointing at Postgres at *compile time*, not just runtime. Either set it in shell, or use `cargo sqlx prepare` to generate an offline cache.
- **`tokio::main` is required** on `fn main()` for async.
- **Async error handling: use `anyhow::Result<T>` for application code**, `thiserror::Error` for library boundaries. Don't fight `?` — it's the right tool.
- **Borrow checker errors are educational, not punitive.** Read them carefully. The compiler usually tells you exactly what to do.
- **`reqwest` defaults to no redirects-on-POST.** For redirect-chain canonical method, configure explicitly: `ClientBuilder::redirect(Policy::limited(10))`.

### Clever Cloud-specific

- **Deploy via `git push` to Clever Cloud remote.** Configure GitHub auto-deploy from their dashboard if preferred.
- **Docker apps need port 8080** (their convention; bind to `0.0.0.0:8080`).
- **Dev Postgres tier: 256MB.** Check MySQL size in §0.7. If close to limit, provision prod tier (€7/mo).
- **Logs in dashboard + `clever logs` CLI.**
- **`www.amputatorbot.com` DNS:** CNAME to the Clever Cloud subdomain. Set in Cloudflare; flatten if you also want apex (`amputatorbot.com`) to redirect.
- **Build memory limit:** Docker builds run with ~1GB RAM by default. Rust release builds can use more. If OOM during build, set `CC_OVERRIDE_BUILDCACHE` or use cargo's `--profile dev` for first deploy, then `--release` later. Or build locally and push image.

### Astro + shadcn gotchas

- **Path alias `@/*` required** in `tsconfig.json` before `shadcn init`.
- **React-context-heavy shadcn components** (Popover, Dialog, Tabs) need single-`.tsx`-file composition.
- **`client:load` vs `client:visible` vs `client:idle`** — Astro hydration directives. Use `client:load` for converter form.
- **Tailwind 4 setup differs from Tailwind 3** — follow shadcn's current Astro guide.
- **Astro's `output: 'static'` is default and what we want.** Don't accidentally enable SSR.

### Migration data gotchas

- **MySQL → Postgres charset.** MySQL is probably `utf8mb4`; Postgres is `UTF8`. Should work but verify rows with non-ASCII chars.
- **`AUTO_INCREMENT` → `BIGSERIAL`** in schema SQL.
- **`TINYINT(1)` → `BOOLEAN`** if present.
- **200K-link dataset is private** per existing README. Don't commit. Run migration through encrypted SSH tunnel.

### API compatibility gotchas

- **Existing `/api/v1/convert` callers depend on exact response shape.** Inspect carefully Day 1. Any drift breaks unknown external consumers.
- **Rate-limit semantics must match.** If existing API returns 429 with specific headers (`Retry-After`, etc.), Rust impl must do the same.
- **Error response shapes** — match existing JSON error structure exactly.

### Reddit/operational gotchas

- **r/AmputatorBot installs first** as canary.
- **Users will PM the bot from non-installed subs asking why it stopped.** Templated response ready.
- **App approval takes days.** Plan launch Day 9–10, not the day after submit.
- **Bounty selection isn't guaranteed.** Apply, don't depend on it.

### Don't-shoot-yourself-in-the-foot

- **Never commit HMAC secret or DB connection string.** Clever Cloud env vars + `devvit settings set`.
- **Test opt-out before launch.** Broken opt-out re-engages every historical opt-out — PR disaster. Migrate opt-out list Day 1, verify Day 2.
- **Keep PythonAnywhere alive until Day 14.** Hot-standby in case Clever Cloud has surprises.
- **Don't trust Claude's Rust output blindly.** Claude writes correct Rust but sometimes idiomatic-but-suboptimal Rust. Compile errors are real feedback — read them.

---

## MCP usage during hackathon

**Devvit MCP** — for current SDK questions:
- "What's the type signature for the CommentSubmit event payload in @devvit/web?"
- "Show me an example of context.reddit.submitComment in the server model"
- "What's the right way to read Devvit settings from a trigger handler?"

**JetBrains MCP** — for codebase work:
- "Read the existing AMP detection code in helpers/ and port it to Rust"
- "Find all callers of canonical-finding methods in the existing Python"
- "Show me the exact response shape of the existing /api/v1/convert route"

Combination: Claude reads existing Python via JetBrains MCP, writes new Rust + TypeScript informed by current Devvit docs.

---

## Approach validation — does this actually work?

Walking the full flow:

1. **User posts AMP comment in r/installed-sub.** Reddit fires `CommentSubmit` → POSTs to Devvit app's `/internal/triggers/comment-submit`. ✓
2. **Devvit handler runs.** Extracts URLs, filters AMP, checks Redis dedup, checks Redis opt-out cache. ✓
3. **Devvit calls our backend.** HMAC-signed `fetch` to `https://www.amputatorbot.com/api/v1/convert?q=<url>&user=<username>`. Domain in `permissions.http.domains`. ✓
4. **Rust backend verifies HMAC**, queries Postgres, runs canonical-finding (including rs-trafilatura). Returns response within 8s. ✓
5. **Devvit handler builds reply, posts via `context.reddit.submitComment`.** ✓
6. **Redis dedup key written.** Future retriggers skip. ✓
7. **User PMs bot "opt me out".** `MessageCreate` → POST to backend optout endpoint → Postgres write. ✓
8. **Website user pastes AMP URL.** Astro React island → POST to `/api/v1/convert` on same origin → response rendered. ✓

No structural gaps.

**Known unknowns we'll discover Day 1:**
- Exact existing API contract (mitigation: §0.8 inspection)
- Clever Cloud Docker build behavior with our Rust deps (mitigation: build locally as fallback)
- Whether the Postgres dev tier holds 200K rows (mitigation: upgrade to prod tier, €7/mo)
- Devvit `CommentSubmit` payload shape (mitigation: Devvit MCP)
- Rust compile-on-Clever-Cloud memory limits (mitigation: local build + image push)

---

## References (verified)

### Reddit
- Migration program: https://support.reddithelp.com/hc/en-us/articles/47822311698452
- Devvit docs: https://developers.reddit.com/docs
- Devvit MCP (official): https://github.com/reddit/devvit-mcp
- Devvit template (official): https://github.com/reddit/devvit-template-bare
- Devvit examples: https://github.com/reddit/devvit-examples
- Devvit monorepo: https://github.com/reddit/devvit

### Clever Cloud
- Docker apps: https://www.clever-cloud.com/doc/applications/docker/
- Postgres add-on: https://www.clever-cloud.com/doc/addons/postgresql/
- Astro static deploy: https://docs.astro.build/en/guides/deploy/clever-cloud/
- Static apps: https://www.clever-cloud.com/product/static-applications/
- Pricing: https://developers.clever-cloud.com/doc/billing/pricing/

### Rust
- Axum: https://docs.rs/axum/latest/axum/
- sqlx: https://docs.rs/sqlx/latest/sqlx/
- rs-trafilatura: https://crates.io/crates/rs-trafilatura
- rs-trafilatura GitHub: https://github.com/Murrough-Foley/rs-trafilatura
- ScrapingHub article extraction benchmark: https://github.com/scrapinghub/article-extraction-benchmark
- Rust book (if learning): https://doc.rust-lang.org/book/
- scraper crate: https://docs.rs/scraper/latest/scraper/
- reqwest: https://docs.rs/reqwest/latest/reqwest/

### Web frontend
- shadcn Astro install: https://ui.shadcn.com/docs/installation/astro
- Astro docs: https://docs.astro.build
- Tailwind 4 docs: https://tailwindcss.com/docs

### Existing AmputatorBot
- Repo: https://github.com/jvdburgh/AmputatorBot
- Postman API docs: https://documenter.getpostman.com/view/12422626/UVC3n93T
- Subreddit: https://www.reddit.com/r/AmputatorBot/

### Tooling
- JetBrains MCP: https://www.jetbrains.com/help/ai-assistant/mcp.html
- Claude Code: https://docs.anthropic.com/en/docs/claude-code

---

**Final reminder before tomorrow:**
- MCPs verified tonight ✓
- Clever Cloud account created ✓
- Migration program applied ✓
- `devvit login` works ✓
- Node 22 + Rust 1.80+ confirmed ✓
- MySQL data size checked ✓
- Existing API response shape captured ✓
- Plan committed to repo at `docs/migration-plan.md` ✓

Get some sleep.
