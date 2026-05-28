# amputatorbot-app (Devvit)

The Devvit half of AmputatorBot — a per-subreddit Reddit app that watches
comment/post submissions for AMP URLs and replies with the canonical
non-AMP link.

Replaces the legacy PRAW bot at [u/AmputatorBot](https://www.reddit.com/u/AmputatorBot/),
which has been running on PythonAnywhere since 2019 and stays alive as a
parallel fallback during the migration. See the top-level
[README](../README.md) for the monorepo overview and the
[migration plan](../docs/amputatorbot-devvit-migration-plan-v7.md) for the
broader context.

## How it works

On `onCommentSubmit` or `onPostSubmit` in a subreddit that has installed
the app, the trigger handler:

1. Extracts URLs from the comment body (or post title + url + selftext).
2. Filters them locally to AMP-looking URLs.
3. If any survive, calls `POST https://www.amputatorbot.com/api/v2/convert`
   to resolve the canonical via the open public AmputatorBot API.
4. Builds the reply markdown (legacy AmputatorBot template with a refreshed
   disclaimer line) and posts it via `reddit.submitComment`.
5. Marks the comment/post handled in Devvit Redis for 1 hour to prevent
   double-replies on trigger retries.

Per-install settings (mods see these on the install settings page):

- **Reply to AMP links** (`autoReply`, default on) — single on/off toggle.
- **Custom footer** (`customFooter`, optional) — appended to the bot's
  reply footer, e.g. a link to your subreddit's modmail.

## Fetch Domains

This app needs to fetch one external domain. All fetches are HTTPS
(`https://www.amputatorbot.com/...`); Devvit enforces TLS at the platform
layer and rejects `http://` URIs regardless of allowlist configuration.
The `permissions.http.enable: true` flag in `devvit.json` is the
on/off switch for the *fetch* SDK capability, not a transport-protocol
downgrade.

### `www.amputatorbot.com`

**Purpose:** Canonical URL resolution. The bot sends each AMP URL it sees
to `POST https://www.amputatorbot.com/api/v2/convert` and receives the
canonical (non-AMP) URL plus metadata (method used, confidence, alt
domains).

**Why it's a public API, not a personal domain:**

- The API has been publicly documented and available without authentication
  since 2020: <https://documenter.getpostman.com/view/12422626/UVC3n93T>
- Existing third-party integrations rely on it (browser extensions, Discord
  bots, IFTTT scripts, etc.).
- The endpoint is a stateless transformation — given any URL, returns the
  canonical version. No user data crosses the wire beyond the URL itself.
- The backend is open-source Rust (`backend/` in this monorepo) hosted on
  Scaleway in the EU; not a hobby endpoint behind a developer's home IP.
- It's the same API that powers <https://www.amputatorbot.com>, which is a
  public-facing single-page URL converter that anyone can use.

**Why we need it (vs. doing canonical-finding inside Devvit):** the
resolver runs 11 specialised canonical-finding methods (rel=canonical,
og:url, schema.org, Google/Bing AMP cache parsing, meta-refresh,
article-similarity scoring via a Mozilla Readability port, etc.), backed
by a 1.7M-row Postgres cache. The resolver depends on Rust crates
(`dom_smoothie`, `scraper`, `psl`, sqlx) that don't have TypeScript
equivalents we'd want to maintain in parallel, and the cache reduces
re-fetching the same URLs repeatedly — both of which would be impractical
inside the Devvit runtime.

## Local development

Toolchain is pinned via [mise](https://mise.jdx.dev) and pnpm. From the
repo root, `just setup` installs everything. After that, all commands
below can be run via `just` from `devvit-app/`:

```bash
just check       # biome + tsgo + vitest
just test        # vitest only
just build       # compile dist/server/index.cjs (vite)
just dev         # vite build --watch (run in one terminal)
just playtest    # devvit playtest r/AmputatorBotTest (run in another)
```

The Devvit app is registered under `u/Killed_Mufasa` with the slug
`amputatorbot-app` — when published, the bot's Reddit username will be
`u/amputatorbot-app`. The original `u/AmputatorBot` Reddit account stays
attached to the legacy PRAW bot for now.

### First-time setup

```bash
just whoami      # confirm logged in as u/Killed_Mufasa
just init        # registers the app slug on Reddit's side (one-time)
```

### Troubleshooting

- **`Command "vite" not found`** — the workspace's per-project
  `node_modules/.bin/` got out of sync. Fix: `pnpm install` from the
  **repo root** (not from `devvit-app/`). If that's not enough,
  `rm -rf devvit-app/node_modules && pnpm install`.
- **`HTTP request to domain: www.amputatorbot.com is not allowed`** —
  the domain is pending Reddit admin review. Check status at
  <https://developers.reddit.com/apps/amputatorbot-app/developer-settings>.
- **`devvit` CLI shows version 0.12.x** despite `@devvit/web@0.13.x` —
  re-run `pnpm install` from the repo root; the local CLI in
  `devvit-app/node_modules/.bin/devvit` should match.

## Layout

```
devvit-app/
├── devvit.json                       # app config: permissions, settings, triggers
├── package.json
├── justfile                          # local task runner — `just --list`
├── vite.config.ts
├── biome.json
├── tsconfig.json
└── src/server/
    ├── index.ts                      # Hono routes for the trigger HTTP endpoints
    ├── settings.ts                   # loads autoReply + customFooter
    ├── backend/
    │   ├── client.ts                 # POST /api/v2/convert wrapper
    │   └── types.ts                  # TS mirror of `backend/src/models/`
    ├── core/
    │   ├── ampDetect.ts              # mirrors backend/src/canonical/amp_detect.rs
    │   ├── urlExtract.ts             # mirrors backend/src/canonical/url_extract.rs
    │   └── reply.ts                  # reply markdown (legacy template, refreshed)
    ├── storage/
    │   └── dedup.ts                  # Devvit Redis: handled:{scope}:{id} TTL 1h
    └── triggers/
        └── handler.ts                # shared orchestration with self-reply guard
```