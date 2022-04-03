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

Method description: This method starts a comment stream that checks for AMP links in Reddit comments.
If one is detected, a reply is made by u/AmputatorBot with the canonical link(s)
"""

import sys
import traceback
from time import sleep

from prawcore import Forbidden

from datahandlers.local_datahandler import update_local_data
from datahandlers.remote_datahandler import save_entry
from helpers import logger
from helpers.criteria_checker import check_criteria
from helpers.reddit.reddit_comment_generator import generate_reply
from helpers.utils import get_urls_info, get_urls
from models import stream
from models.item import Item
from models.type import Type

log = logger.get_log(sys)


def run_bot(type=Type.COMMENT, use_gac=False, reply_to_item=True, save_to_database=True):
    # Get the stream instance (contains session, type and data)
    s = stream.get_stream(type)
    log.info("Set up new stream")

    # Start the stream
    for comment in s.praw_session.subreddit("all").stream.comments():
        # Generate an item with all the relevant data
        i = Item(type=type, id=comment.name, subreddit=comment.subreddit, author=comment.author, body=comment.body)

        # Check if the item meets the criteria
        meets_criteria, result_code = check_criteria(
            item=i,
            data=s,
            history_err=s.comments_err,
            history_ok=s.comments_ok,
            mustBeAMP=True,
            mustBeNew=True,
            mustNotBeBannedInSubreddit=True,
            mustNotHaveFailed=True,
            mustNotBeMine=True,
            mustNotBeOptedOut=True
        )

        # If it meets the criteria, try to find the canonicals and make a reply
        if meets_criteria:
            log.info(f"{i.id} in r/{i.subreddit} meets criteria")
            # Get the urls from the body and try to find the canonicals
            urls = get_urls(i.body)
            i.links = get_urls_info(urls, use_gac)

            # If a canonical was found, generate a reply, otherwise log a warning
            if any(link.canonical for link in i.links) or any(link.amp_canonical for link in i.links):
                # Generate a reply
                reply_text, reply_canonical_text = generate_reply(
                    links=i.links,
                    stream_type=s.type,
                    np_subreddits=s.np_subreddits,
                    item_type=i.type,
                    subreddit=i.subreddit)

                # Try to post the reply
                if reply_to_item:
                    try:
                        reply = comment.reply(reply_text)
                        log.info(f"Replied to {i.id} with {reply.name}")
                        update_local_data("comments_ok", i.id)
                        s.comments_ok.append(i.id)

                    except (Forbidden, Exception):
                        log.warning("Couldn't post reply!")
                        log.error(traceback.format_exc())
                        update_local_data("comments_err", i.id)
                        s.comments_err.append(i.id)

            # If no canonicals were found, log the failed attempt
            else:
                log.warning("No canonicals found")
                update_local_data("comments_err", i.id)
                s.comments_err.append(i.id)

            save_entry(save_to_database=save_to_database, entry_type=type.value, links=i.links)


while True:
    try:
        run_bot()
        log.info("\nCompleted running the bot")
        sleep(120)
    except Forbidden:
        log.warning("Error: 403 HTTP response")
        log.warning("\nSomething went wrong while running the bot")
        sleep(120)
    except (RuntimeError, Exception):
        log.error(traceback.format_exc())
        log.warning("\nSomething went wrong while running the bot")
        sleep(120)
