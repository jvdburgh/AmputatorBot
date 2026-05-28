# AmputatorBot — Terms & Conditions

_Last updated: 2026-05-28_

AmputatorBot ("the bot") is a free, open-source Reddit app maintained by
[u/Killed_Mufasa](https://www.reddit.com/u/Killed_Mufasa). By installing
the bot on a subreddit you moderate, or by using the public API at
[www.amputatorbot.com](https://www.amputatorbot.com), you agree to these
terms.

## What the bot does

The bot detects AMP URLs in comments and posts on subreddits that have
installed it and replies with the canonical (non-AMP) URL. The same
URL-resolution capability is exposed as a free public REST API.

See the [Privacy Policy](./PRIVACY.md) for how data is handled.

## What the bot does not do

- The bot does not vote, send DMs, follow users, or take any moderator
  actions.
- The bot does not collect Reddit usernames or any private user data
  beyond the URLs it extracts from the comments and posts it sees.
- The bot is not affiliated with, endorsed by, or sponsored by Reddit,
  Inc.

## Acceptable use

- You may install the bot on subreddits you moderate.
- You may use the public REST API for any lawful purpose.
- Don't abuse the API (e.g. high-volume scraping). Cloudflare-side rate
  limiting protects the service; persistent abuse may be blocked at
  the network layer.

## No warranty

The bot is provided "as is," without warranty of any kind, express or
implied. The maintainer makes no guarantees about the accuracy of
canonical URL resolution, uptime, or availability. Use at your own risk.

The bot ports years of canonical-finding heuristics from the legacy
PRAW-based AmputatorBot. Accuracy is high (~97% on the test fixture
set) but never 100%. If the bot posts an incorrect canonical, please
report it via the channels in the Contact section so it can be fixed.

## Limitation of liability

To the maximum extent permitted by law, the maintainer is not liable for
any direct, indirect, incidental, special, consequential, or exemplary
damages arising from your use of the bot or the public API, including
but not limited to loss of profits, goodwill, or data.

## Open source

The bot and backend are open source under the
[GNU General Public License v3.0](https://github.com/jvdburgh/AmputatorBot/blob/main/LICENSE).
You may fork, modify, or self-host the code subject to the terms of
that license.

## Changes to these terms

Material changes will be noted in this file's edit history on GitHub and
summarised in the "Changelog" section of the bot's
[README](https://github.com/KilledMufasa/AmputatorBot/blob/main/devvit-app/README.md).
Continued use of the bot after changes constitutes acceptance of the
revised terms.

## Reddit's own policies

Use of Reddit itself is governed by Reddit's
[User Agreement](https://www.redditinc.com/policies/user-agreement) and
[Content Policy](https://www.redditinc.com/policies/content-policy);
nothing in these terms overrides Reddit's policies.

## Contact

- Reddit: [u/Killed_Mufasa](https://www.reddit.com/user/Killed_Mufasa)
- Reddit DM: [send a message](https://www.reddit.com/message/compose/?to=Killed_Mufasa)
- Subreddit: [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot)
- Source code + issues: <https://github.com/KilledMufasa/AmputatorBot>