# This Python file uses the following encoding: utf-8
# License: GPL-3 (https://choosealicense.com/licenses/gpl-3.0/)
# Original author: Killed_Mufasa
# Twitter:https://twitter.com/Killed_Mufasa
# Reddit: https://www.reddit.com/user/Killed_Mufasa
# GitHub: https://github.com/KilledMufasa
# Donate: https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2

# This wonderful little program is used by u/AmputatorBot
# (https://www.reddit.com/user/AmputatorBot) to scan submissions
# in certain subreddits for AMP links. If AmputatorBot detects an
# AMP link, a reply is made with the direct link

import logging
# Import a couple of libraries
import traceback
from time import sleep

import util

logging.basicConfig(
    filename="logs/v1.8/check_submissions.log",
    level=logging.INFO,
    format="%(asctime)s:%(levelname)s:%(message)s"
)

# Main function. Gets the stream for submissions in certain subreddits,
# scans the context for AMP links and replies with the direct link
def run_bot(r, allowed_subreddits, forbidden_users, np_subreddits, submissions_replied_to, submissions_unable_to_reply):
    logging.info("Praw: obtaining stream of subreddits")

    # Get the submission stream of select subreddits using Praw.
    for submission in r.subreddit("+".join(allowed_subreddits)).stream.submissions():
        # Resets for every submission
        item = submission
        success = False
        note = "\n\n"
        domain = "www"
        try:
            # Check if the item fits all criteria
            fits_criteria = check_criteria(item)

            # If all criteria are met, try to comment with the direct link
            if fits_criteria:
                try:
                    logging.debug("All criteria were met.\nItem ID:{}\nItem Title:{}\nItem Body:{}\nItem URL:{}".format(item.id,item.title,item.selftext,item.url))

                    if util.check_if_google(item.url):
                        note = " This page is even entirely hosted on Google's servers (!).\n\n"

                    # Fetch the submitted amp page, if canonical (direct link) was found, generate and post comment
                    try:
                        canonical_url = util.get_canonical(item.url, 2)
                        if canonical_url is not None:
                            logging.debug("Canonical_url returned is not None")

                            # If the subreddit encourages the use of NP, make it NP
                            if item.subreddit in np_subreddits:
                                domain = "np"

                            reply = "It looks like OP posted an AMP link. These will often load faster, but Google's AMP [threatens the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://" + domain + ".reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot)." + note + "You might want to visit **the normal page** instead: **["+canonical_url+"]("+canonical_url+")**.\n\n*****\n\n​^(I'm a bot | )[^(Why & About)](https://" + domain + ".reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot)^( | )[^(Mention me to summon me!)](https://" + domain + ".reddit.com/r/AmputatorBot/comments/cchly3/you_can_now_summon_amputatorbot/)"

                            # Try to comment on OP's submission with a top-level comment
                            try:
                                item.reply(reply)
                                logging.debug("Replied to #{}\n".format(item.id))

                                # If the reply was successfully send, note this
                                success = True
                                with open("submissions_replied_to.txt", "a") as f:
                                    f.write(item.id + ",")
                                submissions_replied_to.append(item.id)
                                logging.info("Added the item id to file: submissions_replied_to.txt\n\n")

                            # If the reply didn't got through, throw an exception
                            # can occur when item gets deleted or when rate limits are exceeded
                            except:
                                logging.error(traceback.format_exc())
                                fatal_error_message = "could not reply to item, it either got deleted or the rate-limits have been exceeded"
                                logging.warning("[STOPPED] " + fatal_error_message + "\n\n")

                        else:
                            fatal_error_message = "There were no canonical URLs found"
                            logging.warning("[STOPPED] " + fatal_error_message + "\n\n")

                    # If no direct links were found, throw an exception
                    except:
                        logging.error(traceback.format_exc())
                        fatal_error_message = "There were no canonical URLs found"
                        logging.warning("[STOPPED] " + fatal_error_message + "\n\n")

                # If something else went wrong, throw an exception
                except:
                    logging.error(traceback.format_exc())
                    fatal_error_message = "Something went wrong while logging meta info"
                    logging.warning("[STOPPED] " + fatal_error_message + "\n\n")

                # If the reply could not be made or send, note this
                if not success:
                    with open("submissions_unable_to_reply.txt", "a") as f:
                        f.write(item.id + ",")
                    submissions_unable_to_reply.append(item.id)
                    logging.info("Added the item id to file: submissions_unable_to_reply.txt.")

        # If something went wrong while checking the criteria, throw an exception
        except:
            logging.error(traceback.format_exc())
            fatal_error_message = "Couldn't check if the item fits all criteria"
            logging.warning("[STOPPED] " + fatal_error_message + "\n\n")


def check_criteria(item):
    # Must contain an AMP link
    if not util.check_if_amp(item.url):
        return False
    # Must not be an item that previously failed
    if item.id in submissions_unable_to_reply:
        return False
    # Must not be already replied to
    if item.id in submissions_replied_to:
        return False
    # Must not be posted by me
    if item.author == r.user.me():
        return False
    # Must not be posted by a user who opted out
    if str(item.author) in forbidden_users:
        return False

    # If all criteria were met, return True
    return True


# Uses these functions to run the bot
r = util.bot_login()
forbidden_users = util.get_forbidden_users()
allowed_subreddits = util.get_allowed_subreddits()
np_subreddits = util.get_np_subreddits()
submissions_replied_to = util.get_submissions_replied()
submissions_unable_to_reply = util.get_submissions_errors()


# Run the program
while True:
    try:
        run_bot(r, allowed_subreddits, forbidden_users, np_subreddits, submissions_replied_to,
                submissions_unable_to_reply)
    except:
        logging.warning("Couldn't log in or find the necessary files! Waiting 120 seconds")
        sleep(120)