![#AmputatorBot](praw-python-archive/img/amputatorbot_logo_banner.png)

TL;DR: Remove AMP from your URLs. [AmputatorBot](https://github.com/jvdburgh/AmputatorBot) is a [Reddit](https://www.reddit.com/user/AmputatorBot) bot that automatically replies to comments and submissions containing AMP URLs with the canonical link(s). It's also available as a [website](https://www.amputatorbot.com/) and a [free REST API](https://www.amputatorbot.com/api/docs).

[**FAQ & Why**](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot/) · [**Changelog**](https://www.reddit.com/r/AmputatorBot/comments/ch9fxp/changelog_of_amputatorbot/) · [**Community**](https://www.reddit.com/r/AmputatorBot/)

## What it is

AmputatorBot has been running on Reddit since 2019 (~181 GitHub stars, ~1.7M URLs converted so far). Three surfaces, same engine:

- **The Reddit app** — replies to AMP comments/submissions on subreddits where mods have installed it.
- **The website** — paste a URL at [www.amputatorbot.com](https://www.amputatorbot.com/), get the canonical back.
- **The REST API** — `/api/v2/convert` is the core engine, used by the bot and website; `/api/v1/convert` is kept around for backwards compatibility. Docs at [www.amputatorbot.com/api/docs](https://www.amputatorbot.com/api/docs).

As of v5, the bot is a [Devvit](https://developers.reddit.com/) app (Reddit's official app platform) and the backend is rewritten in Rust. The old Python + Flask version is preserved in [`praw-python-archive/`](praw-python-archive/) for reference.

## Heads-up for Devvit users

**Custom domain.** Devvit apps can only fetch from domains they've gotten allow-listed by Reddit. See Reddit's [HTTP-Fetch docs](https://developers.reddit.com/docs/capabilities/server/http-fetch#requesting-a-domain-to-be-allow-listed) — short version: you declare the exact hostname in `devvit.json`, and an admin approves or denies on review. The bot fetches from `www.amputatorbot.com` (our own backend on Scaleway).

**Per-install identity.** In Devvit's model, each install posts under a per-install app identity rather than `u/AmputatorBot`. That's how Devvit works — it's not negotiable. Functionally the bot replies the same way; the username next to the reply is just per-subreddit now.

## Repo structure

This is a monorepo. Each part can be developed independently:

- **[`backend/`](backend/)** — Rust + Axum service. Hosts the `/api/v2/convert` endpoint (plus the legacy `/api/v1/convert`), the canonical-finding engine (11 methods, +99% accuracy), the Scalar API docs at `/api/docs`, and serves the website's static files from the same binary.
- **[`devvit-app/`](devvit-app/)** — TypeScript Devvit app. Listens to comment and post triggers and replies per opt-in subreddit.
- **[`website/`](website/)** — Astro 5 + Tailwind 4 + shadcn/ui frontend at [www.amputatorbot.com](https://www.amputatorbot.com/), including the URL converter form.
- **[`praw-python-archive/`](praw-python-archive/)** — the original Python bot (PRAW + Flask). Read-only reference. See [`praw-python-archive/README-legacy.md`](praw-python-archive/README-legacy.md) for the original project README.

## How the bot replies

On `onCommentSubmit` or `onPostSubmit` in a subreddit that has installed the app, the trigger handler:

1. Extracts URLs from the comment body (or post title + url + selftext).
2. Filters locally to AMP-looking URLs.
3. If any survive, calls `POST https://www.amputatorbot.com/api/v2/convert` to resolve the canonical.
4. Builds the reply markdown and posts it via `reddit.submitComment`.
5. Marks the comment/post handled in Devvit Redis for 1 hour to prevent double-replies on trigger retries.

Per-install settings (mods see these on the install settings page):

- **Reply to AMP links** (`autoReply`, default on) — single on/off toggle.
- **Custom footer** (`customFooter`, optional) — appended to the bot's reply footer, e.g. a link to your subreddit's modmail.

### Why the bot calls a public API instead of doing it inside Devvit

The resolver runs 11 specialised canonical-finding methods (rel=canonical, og:url, schema.org, Google/Bing AMP cache parsing, meta-refresh, article-similarity scoring via a Mozilla Readability port, etc.), backed by a 1.7M-row Postgres cache. It depends on Rust crates (`dom_smoothie`, `scraper`, `psl`, `sqlx`) without practical TypeScript equivalents, and the cache reduces re-fetching the same URLs repeatedly — both of which would be impractical inside the Devvit runtime. The API has been publicly documented and available without authentication since 2020; existing third-party integrations (browser extensions, Discord bots, IFTTT scripts) rely on it.

## Features

![#AmputatorBot demo](praw-python-archive/img/amputatorbot_demo.png)

- **11 specialised canonical-finding methods, +99% accuracy.** Tried in priority order:
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

## Tools we use

A small, specific toolchain. None of it is exotic — `mise` and `just` together replace what'd otherwise be ten one-off install steps:

- **[mise](https://mise.jdx.dev)** — pins Rust stable, Node 22, pnpm 11, just, and lefthook for this repo. `mise install` reproduces the whole toolchain from `mise.toml`.
- **[just](https://github.com/casey/just)** — task runner. Every subproject has its own `justfile`. `just <recipe>` from the repo root fans out to all three projects where it makes sense. Try `just --list`.
- **[pnpm](https://pnpm.io)** — workspace package manager for `devvit-app/` and `website/`. `pnpm install` from the repo root sets both up.
- **[cargo](https://doc.rust-lang.org/cargo/)** — Rust's build + test runner. We add `cargo-nextest` (faster tests + per-test isolation) and `cargo-deny` (license + vuln audit).
- **[lefthook](https://lefthook.dev)** — git hook manager. `just setup` registers it so `biome` and `rustfmt`+`clippy` run on staged files at commit time.
- **Docker** — only needed for the local Postgres 17 that `just db-up` boots.

## Getting started

```bash
git clone https://github.com/jvdburgh/AmputatorBot
cd AmputatorBot

# Install pinned toolchains + git hooks
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

Sanity check: open the Scalar UI, expand `POST /api/v2/convert`, paste any AMP URL into the try-it-now panel, and execute. Or via curl:

```bash
URL='https://www.google.com/amp/s/electrek.co/2018/06/19/tesla-model-3-assembly-line-inside-tent-elon-musk/amp/'
curl -s -X POST -H 'Content-Type: application/json' \
    -d "$(jq -nc --arg q "$URL" '{query: $q, entryType: "API"}')" \
    http://localhost:8080/api/v2/convert | jq
```

### Common workflows

- **Tests:** `just test` runs everything (Rust + TS). Project-specific: `cd backend && just test`, `cd website && just test`, `cd devvit-app && just test`.
- **Format + lint:** `just fmt` writes fixes, `just lint` is the CI-equivalent check.
- **Astro dev server only:** `cd website && just dev` (no backend, no DB — but the converter form needs the API).
- **Devvit playtest:** `cd devvit-app && just playtest` installs the dev build on `r/AmputatorBotTest` (override with `SUB=foo`). First-time setup is `just login` then `just init`.
- **Parity test (full):** `cd backend && just parity-full`. Replays all 10k recorded fixtures against the resolver, writes a report to `backend/tests/parity-report.md`.

## Support the project

- **Give feedback** — most new features come straight from user feedback. [Contact me on Reddit](https://www.reddit.com/message/compose/?to=Killed_Mufasa) or [file an issue](https://github.com/jvdburgh/AmputatorBot/issues).
- **Star** — by starring on GitHub, more folks find it. Also gives me something to brag about :p
- **Contribute** — [PRs welcome](https://github.com/jvdburgh/AmputatorBot/issues), big or small.
- **Spread the word** — the only goal here is to give people the canonical link to read instead of the AMP one. Sharing the bot, the API, or the site helps.

### Sponsor

Hosting the bot, website, and API runs about €12–15 ($14–17) per month between the Scaleway container and the managed Postgres. If you support what AmputatorBot does and want to chip in, any donation is a huge help! Thanks a bunch :)

> Donate to our friends in Ukraine: [u24.gov.ua](https://u24.gov.ua/)  
> Donate to AmputatorBot PayPal (used for server cost only): [paypal.com/.../EU6ZFKTVT9VH2](https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2)

**From the bottom of my heart, huge thanks for the tremendous support! <3**
