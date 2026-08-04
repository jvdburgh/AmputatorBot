![AmputatorBot](https://www.amputatorbot.com/amputatorbot_logo_banner.png)

AmputatorBot detects AMP links in comments and posts, and replies with the
canonical, non-AMP link.

## Why?

AMP was Google's attempt at "speeding up the mobile web". In practice the
speed gains were mixed at best, publishers surrendered control over their
own pages (and earned about 40% less on them, per Google's own internal
documents), and cached AMP pages keep you inside Google's ecosystem, with
the publisher's domain hidden behind a `google.com/amp` prefix. AMP's flaws
threaten the Open Web — and user privacy along with it. AmputatorBot exists
to empower individuals to push back.

[Read the full why, with sources →](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot/)

## What it does

- Watches new comments and posts in subreddits that install it.
- Detects AMP URLs (14 detection patterns, scoped to avoid false positives).
- Resolves the canonical URL with 11 specialised methods at +98% accuracy,
  backed by a cache of ~1.7M previously-resolved links.
- Replies once, with the canonical link(s). No spam, no double replies.
- Can be summoned: reply to a comment or post containing an AMP link and
  mention the bot — it posts the canonical link under that comment/post and
  pings you. If the reply can't be posted, you get the links by DM instead.

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
