_![#AmputatorBot](praw-python-archive/img/amputatorbot_logo_banner.png)

TL;DR: Remove AMP from your URLs. [AmputatorBot](https://github.com/KilledMufasa/AmputatorBot) is a [Reddit](https://www.reddit.com/user/AmputatorBot) bot that automatically replies to comments and submissions containing AMP URLs with the canonical link(s). It's also available as a [website](https://www.amputatorbot.com/) and [free REST API](https://www.amputatorbot.com/api/docs).

[**FAQ, About & Why**](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot/)

## What it is

AmputatorBot has been running on Reddit since 2019 (\~181 GitHub stars, ~1.7M URLs converted so far). Three surfaces, same engine:

- **The bot** — replies to AMP comments/submissions on subreddits where mods have installed it.
- **The website** — paste a URL at [www.amputatorbot.com](https://www.amputatorbot.com/), get the canonical back.
- **The REST API** — `/api/v1/convert` and `/api/v2/convert`, doing all the hard work.

As of v5, the bot is now a [Devvit](https://developers.reddit.com/) app (Reddit's official app platform) and the backend is rewritten in Rust. The old Python + Flask version is preserved in [`praw-python-archive/`](praw-python-archive/) for reference.

## Repo structure

This is a **monorepo**. Each part can be developed independently:

- **[`backend/`](backend/)** — Rust + Axum service. Hosts the `/api/v1/convert` and `/api/v2/convert` endpoints, the canonical-finding engine (11 methods, +99% accuracy), the Scalar API docs at `/api/docs`, and serves the website's static files from the same binary.
- **[`devvit-app/`](devvit-app/)** — TypeScript Devvit app. Listens to comment and post triggers and replies per opt-in subreddit.
- **[`website/`](website/)** — Astro 5 + Tailwind 4 + shadcn/ui frontend at [www.amputatorbot.com](https://www.amputatorbot.com/), including the URL converter form.
- **[`praw-python-archive/`](praw-python-archive/)** — the original Python bot (PRAW + Flask). Read-only reference. See [`praw-python-archive/README-legacy.md`](praw-python-archive/README-legacy.md) for the original project README.

Each subproject has its own README with deeper detail.

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
- Free, open, no-auth REST API at `/api/v1/convert` and `/api/v2/convert` — both encoded and unencoded URLs work. Docs at [`/api/docs`](https://www.amputatorbot.com/api/docs) (Scalar).

### See also

- Website: [AmputatorBot.com](https://www.amputatorbot.com/)
- REST API docs: [www.amputatorbot.com/api/docs](https://www.amputatorbot.com/api/docs) (Scalar UI on the live OpenAPI spec)
- Changelog: [r/AmputatorBot post](https://www.reddit.com/r/AmputatorBot/comments/ch9fxp/changelog_of_amputatorbot/)
- Community: [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/)

## Getting started

The repo uses a small but specific toolchain. None of it is exotic — but `mise` and `just` together replace what'd otherwise be ten one-off install steps:

- **[mise](https://mise.jdx.dev)** — one tool that pins Rust stable, Node 22, pnpm 11, just, and lefthook for this repo. `mise install` reproduces the whole toolchain from `mise.toml`.
- **[just](https://github.com/casey/just)** — task runner (think modern Make). Every subproject has its own `justfile`. `just <recipe>` from the repo root fans out to all three projects where it makes sense. Try `just --list`.
- **[pnpm](https://pnpm.io)** — workspace package manager for `devvit-app/` and `website/`. Faster than npm, disk-efficient via content-addressed store. `pnpm install` from the repo root sets both up.
- **[cargo](https://doc.rust-lang.org/cargo/)** — Rust's build + test runner. We add `cargo-nextest` (faster tests + per-test isolation) and `cargo-deny` (license + vuln audit).
- **[lefthook](https://lefthook.dev)** — git hook manager. `just setup` registers it so `biome` and `rustfmt`+`clippy` run on staged files at commit time.
- **Docker** (or [OrbStack](https://orbstack.dev/) / [colima](https://github.com/abiosoft/colima)) — only needed for the local Postgres 17 that `just db-up` boots.

### First time

```bash
git clone https://github.com/KilledMufasa/AmputatorBot
cd AmputatorBot

# Pulls every pinned toolchain version and registers git hooks.
mise install
just setup

# Boots local Postgres 17 in Docker and seeds it with the 10k-row
# historical sample committed to the repo (real data, just a slice).
just db-up
just db-seed

# Starts the backend (cargo-watch rebuilds on save). API at /api/v1/convert,
# website at /, Scalar docs at /api/docs.
just backend-dev
```

That gives you the site at `http://localhost:8080` and the API responding at `/api/v1/convert?q=<amp-url>`. The Scalar UI lives at `http://localhost:8080/api/docs` — paste a URL into the try-it-now panel for a quick sanity check.

### Common workflows

- **Tests:** `just test` runs everything (Rust + TS). Per-project: `cd backend && just test` or `cd website && just test`.
- **Lint + format:** `just fmt` writes formatting fixes, `just lint` is read-only check (what CI runs).
- **Astro dev server only:** `cd website && just dev` (no backend, no DB needed — but the converter form won't work without the API).
- **Devvit playtest:** `cd devvit-app && just playtest` installs the dev build on `r/AmputatorBotTest` (override with `SUB=foo`). First-time setup: `just login` then `just init` — see `devvit-app/README.md` for the gotchas.
- **Parity test (full):** `cd backend && just parity-full`. Replays all 10k recorded fixtures against the resolver, writes a report to `backend/tests/parity-report.md`.

For deeper per-project workflows, see:

- [`backend/README.md`](backend/README.md)
- [`devvit-app/README.md`](devvit-app/README.md)
- `website/` — Astro standard, no special README needed beyond the recipes above

## Support the project

- **Give feedback** — most new features come straight from user feedback. [Contact me on Reddit](https://www.reddit.com/message/compose/?to=Killed_Mufasa) or [file an issue](https://github.com/KilledMufasa/AmputatorBot/issues).
- **Star** — by starring on GitHub, more folks find it. Also gives me something to brag about :p
- **Contribute** — [PRs welcome](https://github.com/KilledMufasa/AmputatorBot/issues), big or small.
- **Spread the word** — the only goal here is to give people the canonical link to read instead of the AMP one. Sharing the bot, the API, or the site helps.

### Sponsor

Hosting the bot, website, and API runs about €12–15 ($14–17) per month between the Scaleway container and the managed Postgres. If you support what AmputatorBot does and want to chip in, any donation is a huge help — every bit goes straight into server costs. Thanks a bunch!

> Donate to our friends in Ukraine: [u24.gov.ua](https://u24.gov.ua/)  
> Donate to AmputatorBot PayPal: [paypal.com/.../EU6ZFKTVT9VH2](https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2)

**From the bottom of my heart, huge thanks for the tremendous support! <3**_
