![#AmputatorBot](/img/amputatorbot_logo_banner.png)

TL;DR: Remove AMP from your URLs. [AmputatorBot](https://github.com/KilledMufasa/AmputatorBot) is a highly specialised [Reddit](https://www.reddit.com/user/AmputatorBot)
and [Twitter](https://twitter.com/AmputatorBot) bot that automatically replies to comments, submissions
and tweets containing AMP URLs with the canonical link(s). It's also available as a
[website](https://www.amputatorbot.com/) and [REST API](https://documenter.getpostman.com/view/12422626/UVC3n93T), but those haven't been made open source here.

[**FAQ, About & Why**](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot/)

## Features

![#AmputatorBot demo](/img/amputatorbot_demo.png)

### Main features:
- **10 specialised canonical-finding methods, allowing for an accuracy rate of +97%**. For example, by:
  - Scanning the HTML contents
  - Detecting and following redirects
  - Guessing, and then checking article similarity with [newspaper](https://github.com/codelucas/newspaper/)
  - … and many more!
- Detect AMP links using 14 patterns, and reply to items containing them with the canonical link and some info
- Compare and test canonicals and pick the best
- Stream Reddit comments, submissions, inbox messages and Tweets
- Extensively tested using a (private) database of over 200K AMP links and their canonicals, also functioning as caching

### Nice bonuses:
- Detect unique URLs with [URLExtract](https://github.com/lipoja/URLExtract) and strip them of any artifacts
- Object-oriented, allowing for a handy, free and publicly available API
- Allow users to opt out and undo this
- Send DMs when summoned by a user
- Items interacted with are automatically being tracked
- Log and datafiles are automatically generated

### See also:
- Online version (recommended): [AmputatorBot.com](https://www.amputatorbot.com/)
- Free and publicly available REST API to convert AMP URLs to canonical links: [API Documentation](https://documenter.getpostman.com/view/12422626/UVC3n93T) & [Postman](https://www.postman.com/amputatorbot)
- User-oriented, simplified changelog: [Changelog](https://www.reddit.com/r/AmputatorBot/comments/ch9fxp/changelog_of_amputatorbot/)
- Community & Subreddit: [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/)
## Set up

1. Clone the repository
2. Run `pip install -r requirements.txt` to install dependencies
3. Change the filename of `static.txt` to `.py` (see `/static`)
4. Configure the application by tweaking `static.py` (required)
6. Choose a `check-[...].py` script to run
7. Configure the script's settings in `run_bot()`. Set everything (`guess_and_check`, `reply_to_post`, `save_to_database`) to `False` when starting out. Consider deleting or disabling the database canonical method.
8. Run the script - All logs and required datafiles should be automatically and dynamically created.
9. Stop the script.
10. Check out the new files in `/data` and edit them to your liking.
11. Re-run the script and enjoy!

## Support the project

- **Summon AmputatorBot** on Reddit, like so: [u/AmputatorBot](https://www.reddit.com/u/AmputatorBot/). For more info, [see here](https://www.reddit.com/r/AmputatorBot/comments/cchly3/you_can_now_summon_amputatorbot/).
- **Give feedback**: Most new features and improvements are directly influenced by your feedback. So, hit me up if you have any feedback. [Contact me on Reddit](https://www.reddit.com/message/compose/?to=Killed_Mufasa) or [Fill an issue](https://github.com/KilledMufasa/AmputatorBot/issues).
- **Star**: By starring the project here on GitHub, we can reach more folks and unlock new options. It also gives me something to brag about :p
- **Contribute**: [Pull requests](https://github.com/KilledMufasa/AmputatorBot/issues) are a great way to contribute directly to the code and functionality.
- **Spread the word**: In the end, the only goal of AmputatorBot is to allow people to have an informed choice. You can help by simply spreading the word!

### Sponsor
The bots, website, and API cost about 10 euros (12 dollars) per month to host. I will use all donations strictly to break even. You can donate any amount via PayPal or with crypto. Thank you so much!
- **PayPal**: [https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2](https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2)
- **Bitcoin (BTC)**: 1GsspnGwbaXfMP2P6t9Hr5oQwCYZdsPHr
- **Cardano (ADA)**: DdzFFzCqrht1gHfopZ7ddXfJFz9tXkhQERc6dzfP71Ve9NoJYk4jQ1wtW1LNCWokMPoDZ7xr7YvHqvt82tG3MsEukkkcQyvUxrwjLWqx
- **Dogecoin (DOGE)**: D8T2QaHiyUSNRvbu2D4L1W44Ge8NPtpPgy
- **BNB, ETH & Binance Smart Chain (BEP20)**: 0xa705c939c7537984f41e0ad07c5dc3e60ca53691

**From the bottom of my heart, huge thanks for the tremendous support! <3**