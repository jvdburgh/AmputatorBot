"""
License: GPL-3 (https://choosealicense.com/licenses/gpl-3.0/)

Killed_Mufasa (original author)
- GitHub:  https://github.com/KilledMufasa
- Reddit:  https://www.reddit.com/user/Killed_Mufasa
- Twitter: https://twitter.com/Killed_Mufasa

AmputatorBot
- Sponsor:  https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2
- GitHub:  https://github.com/KilledMufasa/AmputatorBot
- Reddit:  https://www.reddit.com/user/AmputatorBot
- Website: https://www.amputatorbot.com

Method description: This method starts an inbox stream that checks u/AmputatorBot's inbox for
opt-out requests, opt-back-in requests and most importantly, mentions. If AmputatorBot detects
an AMP link in the parent of a mention, a reply a reply is made by u/AmputatorBot with the
canonical link(s) and a DM is send to the summoner a link to the comment or an error description.
If a user opts out or back in, the user receives a confirmation through DM.
"""

import sys
import traceback
from datetime import datetime
from time import sleep

import praw
from praw.exceptions import RedditAPIException
from prawcore import Forbidden

from datahandlers.local_datahandler import update_local_data, remove_local_data
from datahandlers.remote_datahandler import add_data, get_engine_session
from helpers import logger
from helpers.comment_generator import generate_reply
from helpers.criteria_checker import check_criteria
from helpers.dm_generator import dm_generator
from helpers.utils import get_urls, get_urls_info, get_submission_body, check_if_banned
from models import stream
from models.item import Item
from models.resultcode import ResultCode
from models.type import Type

log = logger.get_log(sys)


# Run the bot
def run_bot(type=Type.MENTION, guess_and_check=True, reply_to_post=True, write_to_database=True):
    # Get the stream instance (contains session, type and data)
    s = stream.get_stream(type)
    log.info("Set up new stream")

    # Start the stream
    for message in s.praw_session.inbox.stream():
        # Mark the item as read
        message.mark_read()

        # Log the message type and id
        log.info(f"New message: {message.type}: {message.fullname}")

        # If the message is a comment_reply, ignore it
        if message.type == "comment_reply":
            continue
        # If the message is an username_mention, start summon process
        if message.type == "username_mention":
            parent = message.parent()
            i = Item(
                type=Type.MENTION,
                id=parent.name,
                subreddit=parent.subreddit,
                author=parent.author,
                context=message.context,
                summoner=message.author,
                parent_link=parent.permalink)

            # Check if the parent is a comment or submission
            if isinstance(parent, praw.models.Comment):
                i.body = parent.body
                i.parent_type = Type.COMMENT
            elif isinstance(parent, praw.models.Submission):
                i.body = get_submission_body(parent)
                i.parent_type = Type.SUBMISSION
            else:
                log.warning("Unknown parent instance")

            # Check if the item meets the criteria
            meets_criteria, result_code = check_criteria(
                item=i,
                data=s,
                history_failed=s.mentions_failed,
                history_success=s.mentions_success,
                mustBeAMP=True,
                mustBeNew=True,
                mustNotBeDisallowedSubreddit=True,
                mustNotHaveFailed=True,
                mustNotBeMine=True,
                mustNotBeOptedOut=True,
                mustNotHaveDisallowedMods=True
            )

            # If it meets the criteria, try to find the canonicals and make a reply
            if result_code != ResultCode.ERROR_NO_AMP:
                log.info(f"{i.id} in r/{i.subreddit} is AMP, result_code={result_code.value}")
                # Get the urls from the body and try to find the canonicals
                urls = get_urls(i.body)
                i.links = get_urls_info(urls, guess_and_check)

                # If a canonical was found, generate a reply, otherwise log a warning
                if any(link.canonical for link in i.links) or any(link.amp_canonical for link in i.links):
                    # Generate a reply
                    reply_text, reply_canonical_text = generate_reply(
                        stream_type=s.type,
                        np_subreddits=s.np_subreddits,
                        item_type=i.parent_type,
                        subreddit=i.subreddit,
                        links=i.links,
                        summoned_link=i.context)

                    # Send a DM if AmputatorBot can't reply because it's disallowed by a subreddit, mod or user
                    if result_code == ResultCode.ERROR_DISALLOWED_SUBREDDIT \
                            or result_code == ResultCode.ERROR_DISALLOWED_MOD \
                            or result_code == ResultCode.ERROR_USER_OPTED_OUT:

                        # Generate and send an error DM dynamically based on the error
                        subject, message = dm_generator(
                            result_code=result_code,
                            parent_link=i.parent_link,
                            parent_subreddit=i.subreddit,
                            parent_type=i.parent_type.value,
                            first_amp_url=i.links[0].url_clean,
                            canonical_text=reply_canonical_text)
                        s.praw_session.redditor(str(i.summoner)).message(subject, message)
                        log.info(f"Send summoner DM of type {result_code}")

                    # Try to post the reply, send a DM to the summoner
                    elif reply_to_post:
                        try:
                            reply = parent.reply(reply_text)
                            log.info(f"Replied to {i.id} with {reply.name}")
                            update_local_data("mentions_success", i.id)
                            s.mentions_success.append(i.id)

                            # Generate and send a SUCCESS DM to the summoner
                            result_code = ResultCode.SUCCESS
                            subject, message = dm_generator(
                                result_code=result_code,
                                reply_link=reply.permalink,
                                parent_subreddit=i.subreddit,
                                parent_type=i.parent_type.value,
                                parent_link=i.parent_link,
                                first_amp_url=i.links[0].url_clean,
                                canonical_text=reply_canonical_text)
                            s.praw_session.redditor(str(i.summoner)).message(subject, message)
                            log.info(f"Send summoner DM of type {result_code}")

                        except (Forbidden, Exception):
                            log.warning("Couldn't post reply!")
                            log.error(traceback.format_exc())
                            update_local_data("mentions_failed", i.id)
                            s.mentions_failed.append(i.id)

                            # Check if AmputatorBot is banned in the subreddit
                            is_banned = check_if_banned(i.subreddit)
                            if is_banned:
                                update_local_data("disallowed_subreddits", i.subreddit)
                                s.disallowed_subreddits.append(i.subreddit)

                            # Generate and send an ERROR_REPLY_FAILED DM to the summoner
                            result_code = ResultCode.ERROR_REPLY_FAILED
                            subject, message = dm_generator(
                                result_code=result_code,
                                parent_type=i.parent_type.value,
                                parent_link=i.parent_link,
                                first_amp_url=i.links[0].url_clean,
                                canonical_text=reply_canonical_text)
                            s.praw_session.redditor(str(i.summoner)).message(subject, message)
                            log.info(f"Send summoner DM of type {result_code}")

                # If no canonicals were found, log the failed attempt
                else:
                    log.warning("No canonicals found")
                    update_local_data("mentions_failed", i.id)
                    s.mentions_failed.append(i.id)

                    # Check if the domain is problematic (meaning it's raising frequent errors)
                    if any(link.domain in s.problematic_domains for link in i.links):
                        result_code = ResultCode.ERROR_PROBLEMATIC_DOMAIN
                    else:
                        result_code = ResultCode.ERROR_NO_CANONICALS

                    # Generate and send an
                    subject, message = dm_generator(
                        result_code=result_code,
                        parent_type=i.parent_type.value,
                        parent_link=i.parent_link,
                        first_amp_url=i.links[0].url_clean)

                    s.praw_session.redditor(str(i.summoner)).message(subject, message)

                # If write_to_database is enabled, make a new entry for every URL
                if write_to_database:
                    for link in i.links:
                        if link.is_amp:
                            add_data(session=get_engine_session(),
                                     entry_type=type.value,
                                     handled_utc=datetime.now().strftime('%Y-%m-%d %H:%M:%S'),
                                     original_url=link.url_clean,
                                     canonical_url=link.canonical)

        # If the message is a DM / message, check for opt-out and opt-back-in requests
        elif message.type == "unknown":
            subject = message.subject.lower()
            if subject == "opt me out of amputatorbot":
                try:
                    author = message.author.name
                    log.info(f"New opt-out request by {author}")

                    # If the user is already opted out, notify the user
                    if author.casefold() in list(user.casefold() for user in s.disallowed_users):
                        log.warning("User has already opted out!")
                        s.praw_session.redditor(author).message(
                            subject="You have already opted out of AmputatorBot",
                            message="You have already opted out, so AmputatorBot won't reply to your comments "
                                    "and submissions anymore. You will still be able to see AmputatorBot's replies to "
                                    "other people's content. Block u/AmputatorBot if you don't want that either. "
                                    "Cheers!")

                    # If the user hasn't been opted out yet, add user to the list and notify the user
                    else:
                        log.info("User has not opted out yet")
                        update_local_data("disallowed_users", author)
                        s.disallowed_users.append(author)
                        s.praw_session.redditor(author).message(
                            subject="You have successfully opted out of AmputatorBot",
                            message="You have successfully opted out of AmputatorBot. AmputatorBot won't reply to your "
                                    "comments and submissions anymore (although it can take up to 24 hours to fully "
                                    "process your opt-out request). You will still be able to see AmputatorBot's "
                                    "replies to other people's content. Block u/AmputatorBot if you don't want that "
                                    "either. Cheers!")

                except (RedditAPIException, Forbidden, Exception):
                    log.error(traceback.format_exc())
                    log.warning(f"Something went wrong while processing opt-out request {message.fullname}")

            elif subject == "opt me back in again of amputatorbot":
                try:
                    author = message.author.name
                    log.info(f"New opt-back-in request by {author}")

                    # If the user is not opted out, notify the user
                    if author.casefold() not in list(user.casefold() for user in s.disallowed_users):
                        log.warning("User is not opted out!")
                        s.praw_session.redditor(author).message(
                            subject="You don't have to opt in of AmputatorBot",
                            message="This opt-back-in feature is meant only for users who choose to opt-out earlier "
                                    "but now regret it. At no point did you opt out of AmputatorBot so there's no "
                                    "need to opt back in. Cheers!")

                    # If the user has opted out, remove user from the list and notify the user
                    else:
                        log.info("User is currently opted out")
                        remove_local_data("disallowed_users", author)
                        s.disallowed_users.remove(author)
                        s.praw_session.redditor(author).message(
                            subject="You have successfully opted back in of AmputatorBot",
                            message="You have successfully opted back in of AmputatorBot, meaning AmputatorBot can "
                                    "reply to your comments and submissions again (although it can take up to 24 hours "
                                    "to fully process your opt-back-in request). Thank you! Cheers!")

                except (RedditAPIException, Forbidden, Exception):
                    log.error(traceback.format_exc())
                    log.warning(f"Something went wrong while processing opt-back-in request {message.fullname}")

            elif "you've been permanently banned from participating in" in subject:
                subreddit = message.subreddit
                if subreddit:
                    log.info(f"New ban issued by r/{subreddit}")
                    is_banned = check_if_banned(subreddit)
                    if is_banned:
                        update_local_data("disallowed_subreddits", subreddit)
                        s.disallowed_subreddits.append(subreddit)
                        log.info(f"Added {subreddit} to disallowed_subreddits")
                else:
                    log.warning(f"Message wasn't send by a subreddit, but by {message.author.name}")

        else:
            log.warning(f"Unknown message type: {message.type}")
            continue
        log.info("\n")


while True:
    try:
        run_bot()
        log.info("\nCompleted running the bot")
        sleep(120)
    except (RuntimeError, Exception):
        log.error(traceback.format_exc())
        log.warning('\nSomething went wrong while running the bot')
        sleep(120)
