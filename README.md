![#AmputatorBot](archive/img/amputatorbot_logo_banner.png)

TL;DR: Remove AMP from your URLs. [AmputatorBot](https://github.com/KilledMufasa/AmputatorBot) is a [Reddit](https://www.reddit.com/user/AmputatorBot) bot that automatically replies to comments and submissions containing AMP URLs with the canonical link(s). It's also available as a [website](https://www.amputatorbot.com/) and [free REST API](https://documenter.getpostman.com/view/12422626/UVC3n93T).

[**FAQ, About & Why**](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot/)

## Repo structure

This is a **monorepo**. Each part of AmputatorBot lives in its own subdirectory and can be developed independently:

- **[`backend/`](backend/)** — Rust + Axum service. Hosts the `/api/v1/convert` endpoint, the canonical-finding engine (11 methods, +97% accuracy), and serves the website's static files from the same binary.
- **[`devvit-app/`](devvit-app/)** — TypeScript [Devvit](https://developers.reddit.com/) app. Listens to Reddit's comment and post triggers and posts the AMP-free reply per subreddit.
- **[`website/`](website/)** — Astro 5 + Tailwind 4 + shadcn/ui frontend at [www.amputatorbot.com](https://www.amputatorbot.com/), including the URL converter form.
- **[`archive/`](archive/)** — the original Python bot (PRAW + Flask) kept for reference as we are testing out the Devvit/Rust rewrite. See [`archive/README-legacy.md`](archive/README-legacy.md) for the original project README.
- **[`docs/`](docs/)** — design + migration docs.

Each subproject has its own README with deeper detail.

## Features

![#AmputatorBot demo](archive/img/amputatorbot_demo.png)

- **11 specialised canonical-finding methods, +97% accuracy.** Tried in priority order:
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
- Streams Reddit comments, submissions, and inbox messages — now via Devvit, per opted-in subreddit.
- ~1.7M historical conversions cached in Postgres for instant lookups.
- Lets users opt out per-account via modmail.
- Free, open, no-auth REST API at `/api/v1/convert` — both encoded and unencoded URLs work.

### See also

- Website: [AmputatorBot.com](https://www.amputatorbot.com/)
- REST API docs: [Postman documentation](https://documenter.getpostman.com/view/12422626/UVC3n93T)
- Changelog: [r/AmputatorBot post](https://www.reddit.com/r/AmputatorBot/comments/ch9fxp/changelog_of_amputatorbot/)
- Community: [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/)

## Getting started

You'll need [mise](https://mise.jdx.dev) for the pinned Rust + Node toolchain, and Docker (or OrbStack / colima) for the local Postgres.

```bash
git clone https://github.com/KilledMufasa/AmputatorBot
cd AmputatorBot

# Installs Rust stable, Node 22, pnpm, just, lefthook; registers git hooks.
just setup

# Boot local Postgres 17 in Docker and seed it with the 10k sample
# of historical conversions committed to the repo.
just db-up
just db-seed

# Run the API + website (cargo-watch rebuilds on save).
just backend-dev
```

That gives you the site at `http://localhost:8080` and the API responding at `/api/v1/convert?q=<amp-url>`.

For project-specific workflows — running the Devvit app against r/test, the Astro dev server in isolation, the full parity test suite against legacy fixtures — see each project's README:

- [`backend/README.md`](backend/README.md)
- [`devvit-app/README.md`](devvit-app/README.md)
- `website/` (README arrives with M6)

## Support the project

- **Summon AmputatorBot** on Reddit, like so: [u/AmputatorBot](https://www.reddit.com/u/AmputatorBot/). For more info, [see here](https://www.reddit.com/r/AmputatorBot/comments/cchly3/you_can_now_summon_amputatorbot/).
- **Give feedback**: Most new features and improvements are directly influenced by your feedback. So, hit me up if you have any feedback. [Contact me on Reddit](https://www.reddit.com/message/compose/?to=Killed_Mufasa) or [File an issue](https://github.com/KilledMufasa/AmputatorBot/issues).
- **Star**: By starring the project here on GitHub, we can reach more folks and unlock new options. It also gives me something to brag about :p
- **Contribute**: [Pull requests](https://github.com/KilledMufasa/AmputatorBot/issues) are a great way to contribute directly to the code and functionality.
- **Spread the word**: In the end, the only goal of AmputatorBot is to allow people to have an informed choice. You can help by simply spreading the word!

### Sponsor

Hosting the bot, website, and API runs about €12–15 ($14–17) per month between the Scaleway container and the managed Postgres. If you support AmputatorBot's mission and can chip in, any donation is a huge help — every bit goes straight into server costs. Thanks a bunch!

> Donate to our friends in Ukraine: [u24.gov.ua](https://u24.gov.ua/)
> Donate to AmputatorBot PayPal: [paypal.com/.../EU6ZFKTVT9VH2](https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2)  

**From the bottom of my heart, huge thanks for the tremendous support! <3**
