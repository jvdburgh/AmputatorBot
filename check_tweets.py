"""
License: GPL-3 (https://choosealicense.com/licenses/gpl-3.0/)

Killed_Mufasa (original author)
- GitHub:  https://github.com/KilledMufasa
- Reddit:  https://www.reddit.com/user/Killed_Mufasa
- Twitter: https://twitter.com/Killed_Mufasa

AmputatorBot
- Sponsor: https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2
- GitHub:  https://github.com/KilledMufasa/AmputatorBot
- Reddit:  https://www.reddit.com/user/AmputatorBot
- Website: https://www.amputatorbot.com

Method description: This method starts a Twitter stream that checks for cached AMP links in tweets,
if one is detected, a reply is made by u/AmputatorBot with the canonical link(s)
"""

import sys
import traceback
from datetime import datetime
from time import sleep

import tweepy
from tweepy import Stream, TweepError

from datahandlers.local_datahandler import update_local_data
from datahandlers.remote_datahandler import add_data, get_engine_session
from helpers import logger
from helpers.tweet_criteria_checker import check_tweet_criteria
from helpers.twitter_utils import generate_tweet, get_cached_tweet_urls, get_twitterer_name
from helpers.utils import get_urls_info
from models import stream
from models.item import Item
from models.type import Type
from static import static

log = logger.get_log(sys)


# Run the bot
def run_bot():
    # Set up OAuth Twitter API connection
    auth = tweepy.OAuthHandler(static.TW_API_KEY, static.TW_API_SECRET_KEY, callback="https://twitter.com/AmputatorBot")
    auth.set_access_token(static.TW_ACCESS_TOKEN, static.TW_ACCESS_TOKEN_SECRET)
    api = tweepy.API(auth)
    api.wait_on_rate_limit = True

    # Get the stream instance (contains session, type and data), add it to the listener
    log.info("Setting up new stream")
    listener = StdOutListener()

    s = stream.get_stream(Type.TWEET)
    listener.__setattr__("s", s)

    # Save settings as attributes
    listener.__setattr__("api", api)
    listener.__setattr__("guess_and_check", False)
    listener.__setattr__("reply_to_item", True)
    listener.__setattr__("write_to_database", True)

    # Generate the stream object with auth and the listener
    twitter_stream = Stream(auth, listener)
    twitter_stream.filter(track=static.AMP_KEYWORDS)

    sleep(120)


# override tweepy.StreamListener to add logic to on_status
class StdOutListener(tweepy.StreamListener):
    def on_status(self, data):
        try:
            i = Item(
                type=Type.TWEET,
                id=data.id,
                body=data.text.encode(encoding='utf-8', errors='ignore').decode(),
                author=get_twitterer_name(data))

            cached_urls = get_cached_tweet_urls(data)

            if len(cached_urls) >= 1:
                # Check if the item meets the criteria
                meets_criteria, result_code = check_tweet_criteria(
                    item=i,
                    cached_urls=cached_urls,
                    tweet=data,
                    data=self.s,
                    history_failed=self.s.tweets_failed,
                    history_success=self.s.tweets_success,
                    mustBeAMP=True,
                    mustNotBeRetweet=True,
                    mustBeCached=True,
                    mustBeNew=True,
                    mustNotHaveFailed=True,
                    mustNotBeMine=True,
                    mustNotBeOptedOut=True
                )

                # If it meets the criteria, try to find the canonicals and make a reply
                if meets_criteria:
                    log.info(f"{i.id} meets criteria")
                    # Get the urls from the body and try to find the canonicals
                    i.links = get_urls_info(cached_urls, self.guess_and_check)

                    # If a canonical was found, generate a reply, otherwise log a warning
                    if any(link.canonical for link in i.links):
                        # Generate a reply
                        generated_tweet = generate_tweet(i.links)
                        print(generated_tweet)

                        # Try to post the reply
                        if self.reply_to_item:
                            try:
                                reply = self.api.update_status(status=generated_tweet,
                                                               in_reply_to_status_id=i.id,
                                                               auto_populate_reply_metadata=True)
                                log.info(f"Replied to {i.id} with {reply.id}")
                                update_local_data("tweets_success", i.id)
                                self.s.tweets_success.append(i.id)

                            except (TweepError, Exception):
                                log.warning("Couldn't post reply!")
                                log.error(traceback.format_exc())
                                update_local_data("tweets_failed", i.id)
                                self.s.tweets_failed.append(i.id)

                    # If no canonicals were found, log the failed attempt
                    else:
                        log.warning("No canonicals found")
                        update_local_data("tweets_failed", i.id)
                        self.s.tweets_failed.append(i.id)

                    # If write_to_database is enabled, make a new entry for every URL
                    if self.write_to_database:
                        for link in i.links:
                            add_data(session=get_engine_session(),
                                     entry_type=i.type.value,
                                     handled_utc=datetime.now().strftime('%Y-%m-%d %H:%M:%S'),
                                     original_url=link.url_clean,
                                     canonical_url=link.canonical)

        except (TweepError, Exception):
            log.error(traceback.format_exc())
            log.warning("\nSomething went wrong while handling a tweet")
            sleep(120)
        return True

    # Traceback limit error on limit error
    def on_limit(self, status_code):
        log.error(traceback.format_exc())
        log.warning(f"Rate limit error: {status_code}")
        return True

    # Traceback error on error
    def on_error(self, status_code):
        log.error(traceback.format_exc())
        log.warning(f"Error: {status_code}")
        return True


while True:
    try:
        run_bot()
        log.info("\nCompleted running the bot")
        sleep(120)
    except (RuntimeError, Exception):
        log.error(traceback.format_exc())
        log.warning("\nSomething went wrong while running the bot")
        sleep(120)
