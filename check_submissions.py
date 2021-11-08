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

Method description: This method starts a submission stream that checks for AMP links in Reddit submissions.
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
from helpers.reddit.reddit_utils import get_submission_body, check_if_banned
from helpers.utils import get_urls_info, get_urls
from models import stream
from models.item import Item
from models.type import Type

log = logger.get_log(sys)


def run_bot(type=Type.SUBMISSION, use_gac=False, reply_to_item=True, save_to_database=True):
    # Get the stream instance (contains session, type and data)
    s = stream.get_stream(type)
    log.info("Set up new stream")
    streamed_subreddits = "+".join(s.allowed_subreddits + s.contributor_subreddits)

    # Start the stream
    for submission in s.praw_session.subreddit(streamed_subreddits).stream.submissions():
        # Generate an item with all the relevant data
        i = Item(
            type=type,
            id=submission.name,
            subreddit=submission.subreddit,
            author=submission.author,
            body=get_submission_body(submission))

        # Check if the item meets the criteria
        meets_criteria, result_code = check_criteria(
            item=i,
            data=s,
            history_failed=s.submissions_failed,
            history_success=s.submissions_success,
            mustBeAMP=True,
            mustBeNew=True,
            mustNotBeDisallowedSubreddit=False,
            mustNotHaveFailed=True,
            mustNotBeMine=True,
            mustNotBeOptedOut=True,
            mustNotHaveDisallowedMods=False
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
                        reply = submission.reply(reply_text)
                        log.info(f"Replied to {i.id} with {reply.name}")
                        update_local_data("submissions_success", i.id)
                        s.submissions_success.append(i.id)

                    except (Forbidden, Exception):
                        log.warning("Couldn't post reply!")
                        log.error(traceback.format_exc())
                        update_local_data("submissions_failed", i.id)
                        s.submissions_failed.append(i.id)

                        # Check if AmputatorBot is banned in the subreddit
                        is_banned = check_if_banned(i.subreddit)
                        if is_banned:
                            update_local_data("disallowed_subreddits", i.subreddit, unique=True)
                            s.disallowed_subreddits.append(i.subreddit)

            # If no canonicals were found, log the failed attempt
            else:
                log.warning("No canonicals found")
                update_local_data("submissions_failed", i.id)
                s.submissions_failed.append(i.id)

            save_entry(save_to_database=save_to_database, entry_type=type.value, links=i.links)


while True:
    try:
        run_bot()
        log.info("\nCompleted running the bot")
        sleep(120)
    except (RuntimeError, Exception):
        log.error(traceback.format_exc())
        log.warning("\nSomething went wrong while running the bot")
        sleep(120)
