# This Python file uses the following encoding: utf-8
# License: GPL-3 (https://choosealicense.com/licenses/gpl-3.0/)
# Original author: Killed_Mufasa
# Twitter:https://twitter.com/Killed_Mufasa
# Reddit: https://www.reddit.com/user/Killed_Mufasa
# GitHub: https://github.com/KilledMufasa
# Donate: https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2

# This wonderfull little program is used by u/AmputatorBot
# (https://www.reddit.com/user/AmputatorBot) to scan submissions
# in certain subreddits for AMP links. If AmputatorBot detects an
# AMP link, a reply is made with the direct link

# Import a couple of libraries
from bs4 import BeautifulSoup
from random import choice
import requests
import praw
import config
import os
import re
import traceback
import logging

# Configure logging
logging.basicConfig(
    filename="v1.5_check_submissions.log",
    level=logging.INFO,
    format="%(asctime)s:%(levelname)s:%(message)s"
)

# Login to Reddit API using Praw.
# Reads configuration details out of config.py (not public)
def bot_login():
    logging.debug("Logging in..")
    r = praw.Reddit(username=config.username,
                    password=config.password,
                    client_id=config.client_id,
                    client_secret=config.client_secret,
                    user_agent="eu.pythoneverywhere.com:AmputatorBot:v1.5 (by /u/Killed_Mufasa)")
    logging.debug("Successfully logged in!\n")
    return r


def random_headers():
    # Get randomized user agent, set default accept and request English page
    # This is done to prevent 403 errors.
    return {
        'User-Agent': choice(config.headers),
        'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8',
        'Accept-Language': 'en-US'
    }


def contains_amp_url(string_to_check):
    # If the string contains an AMP link, return True
    if "/amp" in string_to_check or "/AMP" in string_to_check or "amp/" in string_to_check or "AMP/" in string_to_check or ".amp" in string_to_check or ".AMP" in string_to_check or "amp." in string_to_check or "AMP." in string_to_check or "?amp" in string_to_check or "?AMP" in string_to_check or "amp?" in string_to_check or "AMP?" in string_to_check or "=amp" in string_to_check or "=AMP" in string_to_check or "amp=" in string_to_check or "AMP/" in string_to_check and "https://" in string_to_check:
        string_contains_amp_url = True
        return string_contains_amp_url

    # If no AMP link was found in the string, return False
    string_contains_amp_url = False
    return string_contains_amp_url


# Main function. Gets the stream for submissions in certain subreddits,
# scans the context for AMP links and replies with the direct link
def run_bot(r, allowed_subreddits, submissions_replied_to, submissions_unable_to_reply):
    logging.info("Praw: obtaining stream of submissions")

    # Get the submission stream of select subreddits using Praw.
    for submission in r.subreddit(("+").join(allowed_subreddits)).stream.submissions():

        # Resets for every submission
        submission_meets_all_criteria = False
        submission_could_not_reply = False
        submission_could_reply = False

        # Check: Does the submitted URL contain any amp links?
        string_contains_amp_url = contains_amp_url(submission.url)
        if string_contains_amp_url:
            logging.debug(
                "#{} contains one or more '%%amp%%' strings".format(submission.id))

            # Check: Has AmputatorBot tried (and failed) to respond to this submission already?
            if submission.id not in submissions_unable_to_reply:
                logging.debug(
                    "#{} hasn't been tried before".format(submission.id))

                # Check: Has AmputatorBot replied to this submission already?
                if submission.id not in submissions_replied_to:
                    logging.debug(
                        "#{} hasn't been replied to yet".format(submission.id))

                    # Check: Is the submission posted by u/AmputatorBot?
                    if not submission.author == r.user.me():
                        logging.debug(
                            "#{} hasn't been posted by AmputatorBot".format(submission.id))

                        # Check: Is the submission posted by a user who opted out?
                        with open("forbidden_users.txt", "r") as f:
                            forbidden_users = f.read()
                            forbidden_users = forbidden_users.split(",")
                            logging.info("forbidden_users.txt was found.")

                        if not str(submission.author) in forbidden_users:
                            logging.debug("#{} hasn't been posted by a user who opted out".format(submission.id))
                            submission_meets_all_criteria = True

                    else:
                        logging.debug(
                            "#{} was posted by AmputatorBot".format(submission.id))

                else:
                    logging.debug(
                        "#{} has already been replied to".format(submission.id))

            else:
                logging.debug(
                    "#{} has already been tried before".format(submission.id))

        # If no %amp% strings were found, don't do nor log anything

        # If all criteria are met, try to comment with the direct link
        if submission_meets_all_criteria:
            try:
                logging.debug("All criteria were met.\nSubmission ID:{}\nSubmission Title:{}\nSubmission Body:{}\nSubmission URL:{}".format(submission.id,submission.title,submission.selftext,submission.url))

                # Fetch the submitted amp page, if canonical (direct link) was found, generate and post comment
                try:
                    # Fetch submitted amp page
                    logging.debug("Started fetching {}..".format(submission.url))
                    req = requests.get(submission.url,headers=random_headers())

                    # Make the received data readable
                    logging.info("Making a soup..")
                    soup = BeautifulSoup(req.text, features= "lxml")
                    logging.info("Making a searchable soup..")
                    soup.prettify()

                    # Scan the received data for the direct link
                    logging.info("Scanning for all links..")
                    try:
                        # Check for every link on the amp page if it is of the type rel='canonical'
                        for link in soup.find_all(rel='canonical'):
                            # Get the direct link
                            submission_non_amp_url = link.get('href')
                            logging.debug("Found the direct link: {}".format(
                                        submission_non_amp_url))
                            # If the canonical url is the same as the submitted url, don't use it
                            if submission_non_amp_url == submission.url:
                                logging.warning(
                                    "False positive encounterd! (submission_non_amp_url == submission.url)")
                                submission_could_not_reply = True

                            # If the canonical url is unique, generate and post a comment
                            else:
                                # Generate a comment
                                submission_reply = "It looks like OP posted a Google AMP link. These pages often load faster, but AMP is a [major threat to the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot).\n\nYou might want to visit **the normal page** instead: **["+submission_non_amp_url+"]("+submission_non_amp_url+")**.\n\n*****\n\n​^(I'm a bot | )[^(Why & About)](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot)^( | )[^(Mention me to summon me!)](https://www.reddit.com/r/AmputatorBot/comments/cchly3/you_can_now_summon_amputatorbot/)"

                                # Try to comment on OP's submission with a top-level comment
                                try:
                                    submission.reply(submission_reply)
                                    logging.debug("Replied to #{}\n".format(submission.id))
                                    submission_could_reply = True

                                # If the reply didn't got through, throw an exception (can occur when the submisstion gets deleted or when rate limits are exceeded)
                                except Exception as e:
                                    logging.error(traceback.format_exc())
                                    logging.warning("Could not reply to post.\n\n\n")
                                    submission_could_not_reply = True

                    # If no direct links were found, throw an exception
                    except Exception as e:
                        logging.error(traceback.format_exc())
                        logging.warning("The direct link could not be found.\n")
                        submission_could_not_reply = True

                # If the submitted page couldn't be fetched, throw an exception
                except Exception as e:
                    logging.error(traceback.format_exc())
                    logging.warning("The page could not be fetched.\n")
                    submission_could_not_reply = True

            # If something else went wrong, throw an exception
            except Exception as e:
                logging.error(traceback.format_exc())
                logging.warning("Something went wrong while logging meta info.\n")
                submission_could_not_reply = True

            # If the comment was successfully send, note this
            if submission_could_reply:
                submissions_replied_to.append(submission.id)
                with open ("submissions_replied_to.txt", "a") as f:
                    f.write(submission.id + ",")
                    logging.info("Added the parent id to file: submissions_replied_to.txt\n\n\n")

            # If the reply could not be made or send, note this
            if submission_could_not_reply:
                submissions_unable_to_reply.append(submission.id)
                with open ("submissions_unable_to_reply.txt", "a") as f:
                    f.write(submission.id + ",")
                    logging.info("Added the parent id to file: submissions_unable_to_reply.txt.")


# Get the data of which submissions have been replied to
def get_saved_submissions_repliedtos():
    if not os.path.isfile("submissions_replied_to.txt"):
        submissions_replied_to = []
        logging.warning("submissions_replied_to.txt could not be found.\n")

    else:
        with open("submissions_replied_to.txt", "r") as f:
            submissions_replied_to = f.read()
            submissions_replied_to = submissions_replied_to.split(",")
            logging.info("submissions_replied_to.txt was found.")

    return submissions_replied_to


# Get the data of which submissions could not be replied to (for any reason)
def get_saved_submissions_unabletos():
    if not os.path.isfile("submissions_unable_to_reply.txt"):
        submissions_unable_to_reply = []
        logging.warning("submissions_unable_to_reply.txt could not be found.\n")

    else:
        with open("submissions_unable_to_reply.txt", "r") as f:
            submissions_unable_to_reply = f.read()
            submissions_unable_to_reply = submissions_unable_to_reply.split(",")
            logging.info("submissions_unable_to_reply.txt was found.")

    return submissions_unable_to_reply


# Get list of subreddits where the bot is allowed
def get_allowed_subreddits():
    if not os.path.isfile("allowed_subreddits.txt"):
        allowed_subreddits = []
        logging.warning("allowed_subreddits.txt could not be found.\n")

    else:
        with open("allowed_subreddits.txt", "r") as f:
            allowed_subreddits = f.read()
            allowed_subreddits = allowed_subreddits.split(",")
            logging.info("allowed_subreddits.txt was found.")

    return allowed_subreddits


# Uses these functions to run the bot
r = bot_login()
submissions_replied_to = get_saved_submissions_repliedtos()
submissions_unable_to_reply = get_saved_submissions_unabletos()
allowed_subreddits = get_allowed_subreddits()


# Run the program
while True:
    run_bot(r, allowed_subreddits, submissions_replied_to,
            submissions_unable_to_reply)