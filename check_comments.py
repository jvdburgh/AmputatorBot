# This Python file uses the following encoding: utf-8
# License: GPL-3 (https://choosealicense.com/licenses/gpl-3.0/)
# Original author: Killed_Mufasa
# Twitter:https://twitter.com/Killed_Mufasa
# Reddit: https://www.reddit.com/user/Killed_Mufasa
# GitHub: https://github.com/KilledMufasa
# Donate: https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2

# This wonderfull little program is used by u/AmputatorBot
# (https://www.reddit.com/user/AmputatorBot) to scan comments
# in certain subreddits for AMP links. If AmputatorBot detects an
# AMP link, a reply is made with the direct link

# Import a couple of libraries
from bs4 import BeautifulSoup
from datetime import datetime
from random import choice
import requests
import praw
import config
import time
import os
import re
import traceback
import logging

# Configure logging
logging.basicConfig(
    filename="v1.5_check_comments.log",
    level=logging.DEBUG,
    format="%(asctime)s:%(levelname)s:%(message)s"
)

# Login to Reddit API using Praw.
# Reads configuration details out of config.py (not public)
def bot_login():
    logging.debug("Logging in..")
    r = praw.Reddit(username = config.username,
                    password = config.password,
                    client_id = config.client_id,
                    client_secret = config.client_secret,
                    user_agent = "eu.pythoneverywhere.com:AmputatorBot:v1.5 (by /u/Killed_Mufasa)")
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


# Main function. Gets the stream for comments in certain subreddits,
# scans the context for AMP links and replies with the direct link
def run_bot(r, allowed_subreddits, comments_replied_to, comments_unable_to_reply):
    logging.info("Praw: obtaining stream of submissions")

    # Get the latest 2000 comments in select subreddits using Praw.
    for comment in r.subreddit(("+").join(allowed_subreddits)).comments(limit=2000):
        
        # Resets for every comment
        meets_all_criteria = False
        comment_could_not_reply = False
        comment_could_reply = False
        comments_urls = []
        comments_non_amp_urls = []
        comments_non_amps_urls_amount = 0
        comments_canonical_url = ""

        # Check: Does the comment contain any AMP links?
        string_contains_amp_url = contains_amp_url(comment.body)
        if string_contains_amp_url:
            logging.debug(
                "#{} contains one or more '%%amp%%' strings".format(comment.id))

            # Check: Has AmputatorBot tried (and failed) to respond to this comment already?
            if comment.id not in comments_unable_to_reply: 
                logging.debug(
                    "#{} hasn't been tried before".format(comment.id))

                # Check: Has AmputatorBot replied to this comment already?
                if comment.id not in comments_replied_to:
                    logging.debug(
                        "#{} hasn't been posted by AmputatorBot".format(comment.id))

                    # Check: Is the comment written by u/AmputatorBot?
                    if not comment.author == r.user.me():
                        logging.debug(
                            "#{} hasn't been posted by AmputatorBot".format(comment.id))
                        
                        # Check: Is the submission posted by a user who opted out?
                        with open("forbidden_users.txt", "r") as f:
                            forbidden_users = f.read()
                            forbidden_users = forbidden_users.split(",")
                            logging.info("forbidden_users.txt was found.")

                            if not str(comment.author) in forbidden_users:
                                logging.debug("#{} hasn't been posted by a user who opted out".format(comment.id))
                                meets_all_criteria = True
                        
                    else:
                        logging.debug(
                            "#{} was posted by AmputatorBot".format(comment.id))

                else:
                    logging.debug(
                        "#{} has already been replied to".format(comment.id))

            else:
                logging.debug(
                    "#{} has already been tried before".format(comment.id))

        # If all criteria are met, start the main part
        if meets_all_criteria:
            try:
                logging.debug(
                    "#{}'s body: {}\nScanning for urls..".format(comment.id, comment.body))

                # Scan the comment body for the links
                comments_urls = re.findall("(?P<url>https?://[^\s]+)", 
                comment.body)
                comments_urls_amount = len(comments_urls)

                # Loop through all submitted links	
                for x in range(comments_urls_amount):

                    # Isolate the actual URL (remove markdown) (part 1)
                    try:
                        comments_urls[x] = comments_urls[x].split('](')[-1]
                        logging.debug(
                            "{} was stripped of this string: ']('".format(comments_urls[x]))

                    except Exception as e:
                        logging.error(traceback.format_exc())
                        logging.debug(
                            "{} couldn't or didn't have to be stripped of this string: ']('.".format(comments_urls[x]))

                    # Isolate the actual URL (remove markdown) (part 2)
                    if comments_urls[x].endswith(')?'):
                        comments_urls[x] = comments_urls[x][:-2]
                        logging.debug("{} was stripped of this string: ')?'".format(
                            comments_urls[x]))

                    if comments_urls[x].endswith(')'):
                        comments_urls[x] = comments_urls[x][:-1]
                        logging.debug("{} was stripped of this string: ')'".format(
                            comments_urls[x]))

                    # Check: Is the isolated URL really an amp link?
                    string_contains_amp_url = contains_amp_url(comments_urls[x])
                    if string_contains_amp_url:
                        logging.debug("\nAn amp link was found: {}".format(
                            comments_urls[x]))

                        # Fetch the submitted amp page, if canonical (direct link) was found, save these for later
                        try:
                            # Fetch submitted amp page
                            logging.info("Started fetching..")
                            req = requests.get(
                                comments_urls[x], headers=random_headers())
                            
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
                                    comments_canonical_url = link.get('href')
                                    logging.debug("Found the direct link: {}".format(
                                        comments_canonical_url))

                                    # If the canonical url is the same as the submitted url, don't use it
                                    if comments_canonical_url == comments_urls[x]:
                                        logging.warning(
                                            "False positive encounterd! (comments_canonical_url == comments_urls[x])")
                                        comment_could_not_reply = True

                                    # If the canonical url is unique, add the direct link to the array
                                    else:
                                        comments_non_amps_urls_amount = len(
                                            comments_non_amp_urls)
                                        comments_canonical_url_markdowned = "["+str(
                                            comments_non_amps_urls_amount+1)+"] **"+comments_canonical_url+"**"
                                        comments_non_amp_urls.append(
                                            comments_canonical_url_markdowned)
                                        logging.debug("The array of canonical urls is now: {}".format(
                                            comments_non_amp_urls))

                            # If no direct links were found, throw an exception	
                            except Exception as e:
                                logging.error(traceback.format_exc())
                                logging.warning("The direct link could not be found.\n")
                                comment_could_not_reply = True

                        # If the submitted page could't be fetched (or something else went wrong), throw an exception
                        except Exception as e:
                            logging.error(traceback.format_exc())
                            logging.warning("The page could not be fetched.\n")
                            comment_could_not_reply = True
                            
                    # If the program fails to get the correct amp link, ignore it.
                    else:
                        logging.warning(
                            "This link is no AMP link: {}\n".format(comments_urls[x]))

            # If the program fails to find any link at all, throw an exception
            except Exception as e:
                logging.error(traceback.format_exc())
                logging.warning("No links were found.\n")
                comment_could_not_reply = True

            # If no direct links were found, don't reply
            comments_non_amps_urls_amount = len(comments_non_amp_urls)
            
            if comments_non_amps_urls_amount == 0:
                logging.warning("[STOPPED] There were no canonical urls found.\n\n\n")
                comment_could_not_reply = True

            # If there were direct links found, reply
            else:

                # Try to reply to OP
                try:

                    # If there was only one url found, generate a simple comment
                    if comments_non_amps_urls_amount == 1:
                        comment_reply = "Beep boop, I'm a bot. It looks like you shared a Google AMP link. Google AMP pages often load faster, but AMP is a [major threat to the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot).\n\nYou might want to visit **the normal page** instead: **"+comments_canonical_url+"**.\n\n*****\n\n​[^(Why & About)](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot)^( | )[^(Mention me to summon me!)](https://www.reddit.com/r/AmputatorBot/comments/cchly3/you_can_now_summon_amputatorbot/)"

                    # If there were multiple urls found, generate a multi-url comment
                    if comments_non_amps_urls_amount > 1:
                        # Generate string of all found links
                        comment_reply_generated = '\n\n'.join(comments_non_amp_urls)
                        # Generate entire comment
                        comment_reply = "Beep boop, I'm a bot. It looks like you shared a couple of Google AMP links. Google AMP pages often load faster, but AMP is a [major threat to the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot).\n\nYou might want to visit **the normal pages** instead: \n\n"+comment_reply_generated+"\n\n*****\n\n​[^(Why & About)](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot)^( | )[^(Mention me to summon me!)](https://www.reddit.com/r/AmputatorBot/comments/cchly3/you_can_now_summon_amputatorbot/)"

                    # Reply to comment
                    comment.reply(comment_reply)
                    logging.debug("Replied to comment #{}\n".format(comment.id))
                    comment_could_reply = True

                # If the reply didn't got through, throw an exception (can occur when comment gets deleted or when rate limits are exceeded)
                except Exception as e:
                    logging.error(traceback.format_exc())
                    logging.warning("Could not reply to post.\n\n\n")
                    comment_could_not_reply = True

            # If the reply was successfully send, note this
            if comment_could_reply:
                with open ("comments_replied_to.txt", "a") as f:
                    f.write(comment.id + ",")
                    comments_replied_to.append(comment.id)
                    logging.info("Added the comment id to file: comments_replied_to.txt\n\n\n")
            
            # If the reply could not be made or send, note this
            if comment_could_not_reply:
                with open ("comments_unable_to_reply.txt", "a") as f:
                    f.write(comment.id + ",")
                    comments_unable_to_reply.append(comment.id)
                    logging.info(
                        "Added the comment id to file: comments_unable_to_reply.txt.\n")

    # Sleep for 90 seconds (to prevent exceeding of rate limits)
    logging.info("\nSleeping for 90 seconds..\n")
    time.sleep(90)


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


# Get the data of which comments have been replied to
def get_saved_comments():
    if not os.path.isfile("comments_replied_to.txt"):
        comments_replied_to = []
        logging.warning("comments_replied_to.txt could not be found.\n")

    else:
        with open("comments_replied_to.txt", "r") as f:
            comments_replied_to = f.read()
            comments_replied_to = comments_replied_to.split(",")
            logging.info("comments_replied_to.txt was found.")

    return comments_replied_to


# Get the data of which comments could not be replied to (for any reason)
def get_saved_unabletos():
    if not os.path.isfile("comments_unable_to_reply.txt"):
        comments_unable_to_reply = []
        logging.warning("comments_unable_to_reply.txt could not be found.\n")

    else:
        with open("comments_unable_to_reply.txt", "r") as f:
            comments_unable_to_reply = f.read()
            comments_unable_to_reply = comments_unable_to_reply.split(",")
            logging.info("comments_unable_to_reply.txt was found.")

    return comments_unable_to_reply


# Uses these functions to run the bot
r = bot_login()
allowed_subreddits = get_allowed_subreddits()
comments_replied_to = get_saved_comments()
comments_unable_to_reply = get_saved_unabletos()


# Run the program
while True:
    run_bot(r, allowed_subreddits, comments_replied_to, comments_unable_to_reply)