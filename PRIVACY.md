# AmputatorBot — Privacy Policy

_Last updated: 2026-05-28_

AmputatorBot is a free, open-source Reddit bot maintained by
[u/Killed_Mufasa](https://www.reddit.com/u/Killed_Mufasa). It detects AMP
URLs in comments and posts on subreddits that have installed it, and
replies with the canonical (non-AMP) link.

This policy explains what data the bot collects, what it does with that
data, and the choices you have. If anything below is unclear or you have
a concern, message [u/Killed_Mufasa](https://www.reddit.com/message/compose/?to=Killed_Mufasa)
or post in [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot).

## What data the bot processes

When the bot is installed on a subreddit and a user submits a comment or
post containing a URL, the bot processes:

- **The URL itself**, in order to detect whether it is AMP and (if so)
  resolve it to the canonical non-AMP URL.
- **The Reddit "thing ID"** of the comment or post (e.g. `t1_abc123`,
  `t3_def456`), used to deduplicate replies. Stored in the bot's
  Devvit-managed Redis cache for 1 hour after the bot processes the
  item, then automatically deleted.
- **The subreddit name**, only as part of routing the trigger event —
  not stored.

The bot **does not** collect or store:

- Reddit usernames of commenters / posters.
- Comment or post text other than the URLs extracted from it.
- IP addresses, browser data, or device identifiers.
- Voting history, saved content, subscriptions, or any other private
  Reddit account data (Devvit's platform does not expose these to apps).

## Where the data is processed

- URL resolution happens on the AmputatorBot backend, a Rust service
  hosted on [Scaleway](https://www.scaleway.com/) in Paris/Amsterdam
  (EU). The backend receives only the URL(s) the bot extracted from
  the comment or post.
- The backend's PostgreSQL database stores a long-lived **URL cache**:
  for each URL the bot has ever seen, the canonical it resolved to,
  the method used, and a timestamp. This cache speeds up subsequent
  lookups of the same URL. It does not include any user-identifying
  information.
- The bot's per-install Redis state (dedup keys, install settings) is
  managed by Reddit's Devvit platform.

## How the data is used

- To resolve AMP URLs to their canonical non-AMP versions and reply
  with that information.
- To avoid double-replying when Devvit re-fires the same trigger.
- The aggregated total count of resolved URLs is exposed publicly at
  [www.amputatorbot.com/api/v1/stats](https://www.amputatorbot.com/api/v1/stats)
  (a single integer; no per-URL or per-user breakdown).

The bot does not sell, share, or transfer data to third parties. The
project is non-commercial.

## Public API
The `/api/v1/convert` and `/api/v2/convert` endpoints on
[www.amputatorbot.com](https://www.amputatorbot.com/) are also available
for direct use, without authentication, by anyone. When you call them
directly the same data-handling rules above apply — the URL you send is
processed for AMP detection and stored in the cache.

## Public-data stance

The URLs the bot processes are already public — either posted on Reddit
in a comment/post that anyone with the link can read, or sent to the
unauthenticated `/api/v2/convert` endpoint by direct callers. The URL
cache contains no user-identifying information beyond the URL itself, so
the bot does not provide a separate cache-retraction mechanism.

To stop the bot from interacting with new content:

- **As a subreddit moderator:** uninstall the app from the subreddit's
  settings page, or set the per-install **Reply to AMP links** toggle to
  off to silence it without uninstalling.
- **As a Reddit user:** block the bot's Reddit account
  (`u/amputatorbot-app`) to suppress its replies on your future
  comments/posts.

## Reddit's own policies

Whatever data Reddit itself processes when you use Reddit (your account
data, the comments you post, etc.) is governed by Reddit's
[Privacy Policy](https://www.redditinc.com/policies/privacy-policy)
and [User Agreement](https://www.redditinc.com/policies/user-agreement),
not by this document. AmputatorBot is a third-party app running on
Reddit's developer platform.

## Changes to this policy

Material changes will be noted in this file's edit history on GitHub and
summarised in the "Changelog" section of the bot's
[README](https://github.com/KilledMufasa/AmputatorBot/blob/main/devvit-app/README.md).

## Contact

- Reddit: [u/Killed_Mufasa](https://www.reddit.com/user/Killed_Mufasa)
- Reddit DM: [send a message](https://www.reddit.com/message/compose/?to=Killed_Mufasa)
- Subreddit: [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot)
- Source code + issues: <https://github.com/KilledMufasa/AmputatorBot>
