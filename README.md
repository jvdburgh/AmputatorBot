![#AmputatorBot](/img/amputatorbot_logo_banner.png)

TL;DR: AmputatorBot is a Reddit bot that replies to comments and submissions containing AMP URLs with the canonical URL.

**[FAQ, About & Why](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot/)**

## Features

![#AmputatorBot demo](/img/amputatorbot_demo.png)

Features include:
- Automatically create required log and datafiles
- Scan for comments, submissions, mentions or tweets
- Check each item against specified criteria
- Strip URLs of artifacts
- Check URLs for AMP links
- Find canonicals using 9 different methods
- Calculate which canonical is 'best'
- Return 2 canonicals if the canonicals are from more than 1 domain
- Return an AMP-canonical if the real canonical can't be found
- Generate and send automatic replies to AMP items with the canonical(s) and some info
- Automatically keep track of bans and contributor statuses
- Keep track of items interacted with
- Let users opt-out and opt-back-in
- Send DMs when summoned by users
- Log both locally and to a MySQL database

Please note:
- AmputatorBot works automatically in a select number of subreddits
- AmputatorBot won't work in subreddits where it is banned or forbidden
- The online version of AmputatorBot can be found at [AmputatorBot.com](https://www.amputatorbot.com/)
- You can find the changelog [here](https://www.reddit.com/r/AmputatorBot/comments/ch9fxp/changelog_of_amputatorbot/)!

## Set up

1. Clone the repository
2. Run `pip install -r requirements.txt` to install dependencies
3. Fill in and change the required values in static.txt (see /static)
4. Change the filename of static.txt to .py
5. Choose which script(s) you want to run (check_comments.py, check_inbox.py or check_submissions.py)
6. Set the 'settings' in the run_bot function of the script. Set everything (guess_and_check, reply_to_post, write_to_database) to False when starting out.
7. Run the script - All logs and required datafiles will be automatically and dynamically created. In /data: allowed_subreddits.txt, comments_failed.txt, comments_success.txt, disallowed_mods.txt, disallowed_subreddits.txt, disallowed_users.txt, mentions_failed.txt, mentions.success.txt, np_subreddits.txt, problematic_domains.txt, submissions_failed.txt, submissions_success.txt and in /logs: check_comments_X.X.log, check_inbox_X.X.log and check_submissions_X.X.log.
8. Stop the script to see and edit the newly generated files. Odds are you want to add subreddits to allowed_subreddits.txt (for example: ,subreddit1,subreddit2)
9. Re-run the script

## Support the project

**.. By summoning the bot**: If you've spotted an AMP URL on Reddit and [u/AmputatorBot](https://www.reddit.com/u/AmputatorBot/) seems absent, you can summon the bot by mentioning [u/AmputatorBot](https://www.reddit.com/u/AmputatorBot/) in a reply to the comment or submission containing the AMP URL. You'll receive a confirmation through PM. For more details, check out [this post](https://www.reddit.com/r/AmputatorBot/comments/cchly3/you_can_now_summon_amputatorbot/)!

**.. By giving feedback**: Most of the new features were made after suggestions from you guys, so hit me up if you have any feedback! You can contact me on Reddit, [fill an issue](https://github.com/KilledMufasa/AmputatorBot/issues) or [make a pull request](https://github.com/KilledMufasa/AmputatorBot/issues).

**.. By sponsoring**: The bot and website cost approximately €8.26 a month to host and while that might not seem like much, it adds up. All donations will be used ONLY to pay for hosting. You can specify any amount you want, but please keep in mind that I only want to try to cover some of the costs. Thank you so much! - [https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2](https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2)

**.. By spreading the word**: In the end, the only goal of AmputatorBot is to allow people to have an informed choice. You can help by spreading the word in whatever way you deem the most appropriate.

**From the bottom of my heart, thank you so much for the tremendous support you've given me and AmputatorBot <3**