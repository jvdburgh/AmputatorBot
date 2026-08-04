![AmputatorBot](https://www.amputatorbot.com/amputatorbot_logo_banner.png)

AmputatorBot detects AMP links in comments and posts, and replies with the
canonical, non-AMP link.

## Why?

Not all AMP links are equal. The worst offenders are **cached AMP links**
(the `google.com/amp/...` and `bing.com/amp/...` kind): the article is served
from Google's or Bing's servers, the publisher's domain is hidden behind
someone else's URL, and — per Google's own documentation — both Google *and*
the publisher may collect data about your visit.

**Publisher-hosted AMP pages** at least live on the publisher's own domain,
but they're stripped-down pages built to Google's component rules, with
speed benefits that real-world benchmarks found mixed at best — while
internal Google documents put publisher revenue on AMP at roughly 40% less.

Either way, the canonical link gives you the same article, full-fat,
straight from the source. That's what the bot serves.
[Read the full why here.](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot/)

## What it does

- Watches new comments and posts in subreddits that install it.
- Detects AMP URLs (14 detection patterns, scoped to avoid false positives).
- Resolves the canonical URL with 11 specialised methods at +98% accuracy,
  backed by a cache of ~1.7M previously-resolved links.
- Replies once, with the canonical link(s). No spam, no double replies.
- Can be summoned: reply to a comment or post containing an AMP link and
  mention the bot — it answers you with the canonical link.

## For mods

Install the app on your subreddit and it just works. Two settings:

- **Reply to AMP links** — the on/off switch, on by default. Turn it off to
  silence the bot without uninstalling.
- **Custom footer** — optional markdown snippet appended to the bot's reply,
  e.g. a link to your modmail.

## Beyond Reddit

The bot runs on a free, open REST API you can use too: paste a URL on
[amputatorbot.com](https://www.amputatorbot.com/) or call the API directly —
docs at [amputatorbot.com/api/docs](https://www.amputatorbot.com/api/docs).

## Links

[FAQ & Why](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot/) ·
[Changelog](https://www.reddit.com/r/AmputatorBot/comments/ch9fxp/changelog_of_amputatorbot/) ·
[Community](https://www.reddit.com/r/AmputatorBot/) ·
[Source code](https://github.com/jvdburgh/AmputatorBot)

Fully open source (GPL-3.0). Technical details, architecture, and
contribution docs live in the
[GitHub README](https://github.com/jvdburgh/AmputatorBot#readme).
