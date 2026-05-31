![#AmputatorBot](website/public/amputatorbot_logo_banner.png)

TL;DR: Remove AMP from your URLs to reduce Google's control over independent publishers, improve privacy, and support the Open Web.

[AmputatorBot](https://github.com/jvdburgh/AmputatorBot) is a [Reddit](https://www.reddit.com/user/AmputatorBot)
Devvit app that automatically detects AMP links in submissions and comments
and replies with the canonical URL(s).

The core of AmputatorBot is a free public REST API, available at
[amputatorbot.com/api/docs](https://www.amputatorbot.com/api/docs).
It handles the canonical URL conversion, and powers both the
Reddit app and [amputatorbot.com](https://www.amputatorbot.com/).  It's way overengineered with [Rust](https://rust-lang.org/) 
(because it's fun, fast, and prevents memory leaks) and 130+ tests.

The original Python + Flask implementation, whose development began in 2019,
has been preserved in [`praw-python-archive/`](praw-python-archive/) for historical reference.

[**FAQ & Why**](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot/) · [**Changelog**](https://www.reddit.com/r/AmputatorBot/comments/ch9fxp/changelog_of_amputatorbot/) · [**Community**](https://www.reddit.com/r/AmputatorBot/)

## Features

![#AmputatorBot demo](praw-python-archive/img/amputatorbot_demo.png)

- **11 specialised canonical-finding methods, +98% accuracy.** Tried in priority order:
  - `REL` — `<link rel="canonical">`, the HTML5 standard signal used by ~every SEO-aware CMS.
  - `CANURL` — custom `a="amp-canurl"` attribute some publishers set alongside (or instead of) `rel=canonical`.
  - `OG_URL` — Open Graph `<meta property="og:url">`.
  - `GOOGLE_MANUAL_REDIRECT` — pulls the destination out of `<a>` links inside Google's `www.google.com/url?q=...` interstitial pages.
  - `GOOGLE_JS_REDIRECT` — same idea, but for Google interstitials that only set the destination in inline JS (`var redirectUrl = "..."`).
  - `BING_ORIGINAL_URL` — Bing's AMP cache embeds the publisher URL as `"originalUrl": "..."` in an inline JS blob.
  - `SCHEMA_MAINENTITY` — Schema.org's `mainEntityOfPage` inside `<script type="application/ld+json">`.
  - `TCO_PAGETITLE` — Twitter's `t.co?amp=1` shortlinks render an interstitial whose `<title>` is the destination URL.
  - `META_REDIRECT` — `<meta http-equiv="refresh">` URL extraction.
  - `GUESS_AND_CHECK` — strip AMP keywords from the URL, fetch the result, and accept it if the article text is similar enough to the AMP page (similarity via Mozilla Readability port `dom_smoothie`).
  - `DATABASE` — cache lookup against the ~1.7M previously-resolved canonicals in Postgres.
- **14 AMP-detection patterns**, applied with word boundaries and URL-component scoping to keep false positives down.
- Reads Reddit comments and posts via Devvit triggers, per opt-in subreddit.
- ~1.7M historical conversions cached in Postgres for instant lookups.
- Free, open, no-auth REST API at `/api/v2/convert` (camelCase JSON in/out). Both encoded and unencoded URLs work. Docs at [`/api/docs`](https://www.amputatorbot.com/api/docs) (Scalar).

## Repo structure

All parts of AmputatorBot are open-sourced in this monorepo:

- **[`backend/`](backend/)** — Rust + Axum service. Hosts the `/api/v2/convert` endpoint (plus the legacy `/api/v1/convert`), the canonical-finding engine, the Scalar API docs at `/api/docs`, and serves the website's static files from the same binary.
- **[`devvit-app/`](devvit-app/)** — TypeScript Devvit app. Listens to comment and submission triggers and replies per opt-in subreddit.
- **[`website/`](website/)** — Astro 6 + Tailwind 4 + shadcn/ui frontend.
- **[`praw-python-archive/`](praw-python-archive/)** — the original Python bot (PRAW + Flask). Read-only reference.
- **[`.claude/skills/amputatorbot-migration/`](.claude/skills/amputatorbot-migration/)** — I used Claude to modernize our bot a bit. Kept as it contains architecture and historical context.

## Tools

Before we dive in, here's a quick overview of the tools we use.

- **[mise](https://mise.jdx.dev)** — pins Rust stable, Node 22, pnpm 11, just, and lefthook for this repo. `mise install` reproduces the whole toolchain from `mise.toml`.
- **[just](https://github.com/casey/just)** — task runner. Every subproject has its own `justfile`. `just <recipe>` from the repo root fans out to all three projects where it makes sense. Try `just --list`.
- **[pnpm](https://pnpm.io)** — workspace package manager for `devvit-app/` and `website/`. `pnpm install` from the repo root sets both up.
- **[cargo](https://doc.rust-lang.org/cargo/)** — Rust's build + test runner. We add **[cargo-nextest](https://nexte.st)** (faster tests + per-test isolation) and **[cargo-deny](https://embarkstudios.github.io/cargo-deny/)** (license + vuln audit).
- **[biome](https://biomejs.dev)** — TypeScript/JavaScript formatter + linter, in one fast Rust binary. Alternative to ESLint + Prettier.
- **[lefthook](https://lefthook.dev)** — git hook manager. `just setup` registers it so `biome` and `rustfmt`+`clippy` run on staged files at commit time.
- **[Docker](https://www.docker.com/)** — only needed for the local Postgres 17 that `just db-up` boots.

## Getting started

```bash
git clone https://github.com/jvdburgh/AmputatorBot
cd AmputatorBot

# Install pinned toolchains + register git hooks
mise install
just setup

# Boot Postgres 17 in Docker and seed the 10k-row historical sample
just db-up
just db-seed

# Run backend + website (cargo-watch rebuilds on save)
just backend-dev
```

Open:

- Website: `http://localhost:8080`
- API docs (Scalar): `http://localhost:8080/api/docs`

Then click around the Scalar UI to call `POST /api/v2/convert`, or curl directly as shown under [The REST API](#the-rest-api) below.

## The Reddit app

The TypeScript Devvit app lives in [`devvit-app/`](devvit-app/). 
It's a per-subreddit Reddit app: mods install it on subs they moderate, 
and the bot replies to AMP URLs in comments and submissions on those subs only.

### How it works

On `onCommentSubmit` or `onPostSubmit` in a subreddit that has installed the app, the trigger handler:

1. Extracts URLs from the comment body (or post title + url + selftext).
2. Filters locally to AMP-looking URLs.
3. If any survive, calls `POST https://www.amputatorbot.com/api/v2/convert` to resolve the canonical.
4. Builds the reply markdown and posts it via `reddit.submitComment`.
5. Marks the comment/post handled in Devvit Redis for 1 hour to prevent double-replies on trigger retries.

### Per-install settings

Mods see these on the install settings page in Reddit's developer-platform UI:

- **Reply to AMP links** (`autoReply`, default on) — single on/off toggle.
- **Custom footer** (`customFooter`, optional) — appended to the bot's reply footer, e.g. a link to your subreddit's modmail.

### Local playtest

```bash
cd devvit-app

# First-time only
just login      # browser OAuth as u/Killed_Mufasa
just init       # registers the app slug on Reddit's side

# Per session
just dev        # vite build --watch (run in one terminal)
just playtest   # installs the dev build on r/AmputatorBotTest
```

Override the playtest subreddit with `SUB=foo just playtest`.

### Tests

`cd devvit-app && just test` — Vitest covers AMP-detection mirror, URL extraction mirror, reply markdown (snapshot-pinned), and the trigger handler with a mocked Reddit context.

## The website

Astro 6 + Tailwind 4 + shadcn/ui at [www.amputatorbot.com](https://www.amputatorbot.com/). Includes the URL converter form (paste a URL, get the canonical, or a copy-paste-ready Reddit comment), a live "X converted so far" badge backed by `/api/v2/stats`, and explainer sections sourced from the FAQ Reddit thread. Lives in [`website/`](website/).

The Astro static bundle is built into the Rust backend's container image (see [`backend/Dockerfile`](backend/Dockerfile)) and served from the same binary via `tower-http::ServeDir`. Maybe not nicely separated, but it saves me some time.

### Local dev

```bash
cd website && just dev
```

Runs the Astro dev server on its own. No backend, no DB needed — but the converter form won't work without the API on the same origin. For the full stack, see [Getting started](#getting-started) above.

### Tests

`cd website && just test` — Vitest, light unit coverage on the React islands.

## The REST API

Two convert endpoints live side by side:

| Endpoint | Surface |
|---|---|
| `POST /api/v2/convert` | Modern JSON in / JSON out, camelCase both ways. Response is an envelope `{ links, comment }`; set `generateMarkdownComment: true` in the body to get a ready-to-post Reddit reply alongside the canonical resolution. Strict validation (typo'd field → 422). |
| `GET /api/v1/convert` | Legacy query-string contract, snake_case JSON response. GET-only since the legacy bot was always query-string-based. Kept stable for existing third-party callers — don't build new integrations against it. |

Plus three utility endpoints: `GET /api/v2/health` (liveness probe with version), `GET /api/v2/stats` (`{ "convertedTotal": N }`, cached 1h), and `GET /api/openapi.json` (the spec).

The `query` field on v2 (and `q` on v1) accepts either a bare AMP URL or free-form text containing one or more URLs — the same URL-extractor used for Reddit comment bodies handles both, so pasting a chat message or a Reddit post body works the same as pasting a single URL.

### Docs

Scalar UI at [`/api/docs`](https://www.amputatorbot.com/api/docs) — try-it-now panels for both endpoints, derived from the live OpenAPI spec the backend emits at `/api/openapi.json`. No login needed.

### Smoke test

If you'd rather curl than click:

```bash
URL='https://www.google.com/amp/s/www.psychologytoday.com/ca/blog/the-state-of-our-unions/202402/the-anatomy-of-an-apology/amp'

# v2 — `jq -nc` safely injects the URL as JSON.
curl -s -X POST -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg q "$URL" '{query: $q}')" \
    https://www.amputatorbot.com/api/v2/convert | jq

# v1 (legacy) — `--data-urlencode` lets curl handle percent-encoding.
curl -s --get --data-urlencode "q=$URL" https://www.amputatorbot.com/api/v1/convert | jq
```

### The backend

Lives in [`backend/`](backend/). Rust + Axum 0.8, single binary, hosted on Scaleway in the EU (Paris/AMS). Cargo-chef-cached Docker build, ~165 MB stripped runtime image. The 1.7M-row Postgres cache lives on Scaleway Managed Database.

### Local database

The backend depends on Postgres 17 for the cache. Run it locally in Docker via the recipes in the root `justfile`:

| Command | What it does |
|---|---|
| `just db-up` | Boot Postgres 17 in the background. Idempotent. |
| `just db-down` | Stop the container (keeps the data volume). |
| `just db-nuke` | Stop + delete the data volume — full reset. |
| `just db-migrate` | Apply pending sqlx migrations explicitly. |
| `just db-seed` | Load the committed 10k URLConversions sample into `links`. |
| `just db-seed path=<csv>` | Load any CSV with the same column order (e.g. a full legacy export). |

Migrations auto-apply on backend startup via `sqlx::migrate!()`, so `just db-migrate` is mainly for "fresh schema without booting the server."

### sqlx offline mode

The DATABASE-method query in `backend/src/canonical/pg_database.rs` uses the compile-time-checked `sqlx::query!` macro. To avoid requiring a live DB during `cargo build` (which would break CI + Docker builds), each macro's metadata is cached in `backend/.sqlx/` and committed to git. `cargo build` reads from there when `SQLX_OFFLINE=true` or when no `DATABASE_URL` is set.

When you change any `sqlx::query!` SQL or the schema: re-run `cargo sqlx prepare` from `backend/` (with `DATABASE_URL` set and PG running) and commit the updated `.sqlx/` JSON. CI fails if `.sqlx/` drifts from the actual queries.

### Tests

Three layers in the backend:

- **Unit tests** — per-module tests for AMP detection, URL extraction, the 11 canonical methods, and the orchestration loop. Mock-driven, no network, sub-second. `cd backend && just test`.
- **Snapshot tests** (`insta`) — pin the JSON shape of `resolve()` for ~10 representative scenarios. Catches accidental response-shape drift. Regenerate after an intentional shape change with `INSTA_UPDATE=always cargo nextest run --test snapshots` then `cargo insta review`.
- **Parity tests** — replay a 10k-row legacy sample against the new resolver and compare each result to what the legacy Python bot recorded. Records HTML fixtures once (`just record-fixtures`, ~1 hour), then `just parity-full` runs ~minute per run, and writes `tests/parity-report.md`.

## Support the project

- **Give feedback** — most new features come straight from user feedback. [Contact me on Reddit](https://www.reddit.com/message/compose/?to=Killed_Mufasa) or [file an issue](https://github.com/jvdburgh/AmputatorBot/issues).
- **Star** — by starring on GitHub, more folks find it. Also gives me something to brag about :p
- **Contribute** — [PRs welcome](https://github.com/jvdburgh/AmputatorBot/issues), big or small.
- **Spread the word** — the only goal here is to give people the canonical link to read instead of the AMP one. Sharing the bot, the API, or the website helps.

### Sponsor

Running the app, website, and API costs roughly €15 ($17) per month,
covering a Scaleway container and managed Postgres.

If you find AmputatorBot useful and want to support it, donations help cover
infrastructure costs directly.

> Donate to Ukraine relief: [u24.gov.ua](https://u24.gov.ua/)  
> Donate via PayPal (server costs only): [paypal.com/.../EU6ZFKTVT9VH2](https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2)

**Thanks for the support <3**
