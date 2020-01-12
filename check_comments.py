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
# AMP link, a reply is made with the direct link.

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
    filename="logs/v1.7/check_comments.log",
    level=logging.INFO,
    format="%(asctime)s:%(levelname)s:%(message)s"
)

# Main function. Gets the stream for comments in certain subreddits,
# scans the context for AMP links and replies with the direct link
def run_bot(r, allowed_subreddits, forbidden_users, comments_replied_to, comments_unable_to_reply):
    logging.info("Praw: obtaining stream of subreddits")
    # Get the latest 4000 comments in select subreddits using Praw.
    for comment in r.subreddit(("+").join(allowed_subreddits)).comments(limit=4000):
        # Resets for every comment
        item_urls = []
        canonical_url = ""
        canonical_urls_amount = 0
        canonical_urls = []
        reply = ""
        fits_criteria = False
        success = False
        item = comment

        # Check if the item fits all criteria
        fits_criteria = check_criteria(item)

        # If the item fits the criteria and the item contains an AMP link, fetch the canonical link
        if fits_criteria:
            try:
                logging.debug("#{}'s body: {}\nScanning for urls..".format(item.id, item.body))

                # Scan the item body for the links
                item_urls = re.findall("(?P<url>https?://[^\s]+)", item.body)
                
                # Loop through all found links
                for x in range(len(item_urls)):
                    # Remove the markdown from the URL
                    item_urls[x] = remove_markdown(item_urls[x])

                    # Check: Is the isolated URL really an amp link?
                    string_contains_amp_url = contains_amp_url(item_urls[x])

                    if string_contains_amp_url:
                        logging.debug("\nAn amp link was found: {}".format(item_urls[x]))

                        # Get the canonical link
                        canonical_url = get_canonical(item_urls[x], 1)
                        if canonical_url is not None:
                            logging.debug("Canonical_url returned is not None")
                            # Add markdown in case there are multiple URLs
                            canonical_urls_amount = len(canonical_urls)
                            canonical_url_markdown = "["+str(canonical_urls_amount+1)+"] **"+canonical_url+"**"
                            canonical_urls.append(canonical_url_markdown)
                            logging.debug("The array of canonical urls is now: {}".format(
                                canonical_urls))
                        else:
                            logging.debug("No canonical URLs were found, skipping this one")

                    # If the program fails to get the correct amp link, ignore it.
                    else:
                        logging.warning("This link is no AMP link: {}\n".format(item_urls[x]))

            # If the program fails to find any link at all, throw an exception
            except Exception as e:
                logging.error(traceback.format_exc())
                logging.warning("No links were found.\n")

            # If no canonical urls were found, don't reply
            if len(canonical_urls) == 0:
                fatal_error_message = "there were no canonical URLs found"
                logging.warning("[STOPPED] " + fatal_error_message + "\n\n")

            # If there were direct links found, reply!
            else:
                # Try to reply to OP
                try:
                    canonical_urls_amount = len(canonical_urls)

                    # If there was only one url found, generate a simple comment
                    if canonical_urls_amount == 1:
                        reply = "It looks like you shared a Google AMP link. These pages often load faster, but AMP is a [major threat to the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot).\n\nYou might want to visit **the normal page** instead: **["+canonical_url+"]("+canonical_url+")**.\n\n*****\n\n​^(I'm a bot | )[^(Why & About)](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot)^( | )[^(Mention me to summon me!)](https://www.reddit.com/r/AmputatorBot/comments/cchly3/you_can_now_summon_amputatorbot/)"

                    # If there were multiple urls found, generate a multi-url comment
                    if canonical_urls_amount > 1:
                        # Generate string of all found links
                        reply_generated = '\n\n'.join(canonical_urls)
                        # Generate entire comment
                        reply = "It looks like you shared a couple of Google AMP links. These pages often load faster, but AMP is a [major threat to the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot).\n\nYou might want to visit **the normal pages** instead: \n\n"+reply_generated+"\n\n*****\n\n​^(I'm a bot | )[^(Why & About)](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot)^( | )[^(Mention me to summon me!)](https://www.reddit.com/r/AmputatorBot/comments/cchly3/you_can_now_summon_amputatorbot/)"

                    # Reply to mention
                    item.reply(reply)
                    logging.debug("Replied to #{}\n".format(item.id))

                    # If the reply was successfully send, note this
                    success = True
                    with open("comments_replied_to.txt", "a") as f:
                        f.write(item.id + ",")
                    comments_replied_to.append(item.id)
                    logging.info("Added the item id to file: comments_replied_to.txt\n\n\n")

                # If the reply didn't got through, throw an exception (can occur when item gets deleted or when rate limits are exceeded)
                except Exception as e:
                    logging.error(traceback.format_exc())
                    fatal_error_message = "could not reply to item, it either got deleted or the rate-limits have been exceeded"
                    logging.warning("[STOPPED] " + fatal_error_message + "\n\n")
                    
            # If the reply could not be made or send, note this
            if not success:
                with open("comments_unable_to_reply.txt", "a") as f:
                    f.write(item.id + ",")
                comments_unable_to_reply.append(item.id)
                logging.info("Added the item id to file: comments_unable_to_reply.txt.")

    # Sleep for 90 seconds (to prevent exceeding of rate limits)
    logging.info("Sleeping for 90 seconds..\n")
    time.sleep(90)

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
        return True

    # If no AMP link was found in the string, return False
    return False

def remove_markdown(url):    
    # Isolate the actual URL (remove markdown) (part 1)
    try:
        url = url.split('](')[-1]
        logging.debug(
            "{} was stripped of this string: ']('".format(url))

    except Exception as e:
        logging.error(traceback.format_exc())
        logging.debug(
            "{} couldn't or didn't have to be stripped of this string: ']('.".format(url))

    # Isolate the actual URL (remove markdown) (part 2)
    if url.endswith(')?'):
        url = url[:-2]
        logging.debug("{} was stripped of this string: ')?'".format(url))

    if url.endswith('),'):
        url = url[:-2]
        logging.debug("{} was stripped of this string: ')'".format(url))
    
    if url.endswith(').'):
        url = url[:-2]
        logging.debug("{} was stripped of this string: ')'".format(url))
    
    if url.endswith(')'):
        url = url[:-1]
        logging.debug("{} was stripped of this string: ')'".format(url))
    
    return url

def get_canonical(url, depth):
# Fetch the amp page, search for the canonical link
    try:
        # Fetch amp page
        logging.info("Started fetching " + url)
        req = requests.get(url, headers=random_headers())

        # Make the received data searchable
        logging.info("Making a soup..")
        soup = BeautifulSoup(req.text, features="lxml")
        logging.info("Making a searchable soup..")
        soup.prettify()

        # Scan the received data for the direct link
        logging.info("Scanning for all links..")
        try:
            falsePositive = False # Reset check for false positives
            methodFailed = False # Assign false before reference
            
            # Get all canonical links in a list
            canonicalRels = soup.find_all(rel='canonical')
            if canonicalRels:
                for link in canonicalRels:
                    # Get the direct link
                    found_canonical_url = link.get('href')
                    # If the canonical url is the same as the submitted url, don't use it
                    if found_canonical_url == url:
                        falsePositive = True
                    # If the canonical url has a %%amp%% string, look deeper
                    elif contains_amp_url(found_canonical_url):
                        logging.info("Still amp!")
                        # Do another search for the canonical url of the canonical url
                        if depth > 0:
                            newUrl = get_canonical(found_canonical_url, 0)
                            # If it doesn't contain any amp urls, return it
                            if not contains_amp_url(newUrl):
                                return newUrl
                        else:
                            methodFailed = True
                    # If the canonical link is clean, return it
                    elif not contains_amp_url(found_canonical_url):
                        logging.info("Found the direct link with canonical: {}".format(found_canonical_url))
                        return found_canonical_url

            if not canonicalRels or methodFailed:
                # Get all canonical links in a list
                canonicalRels = soup.find_all('a','amp-canurl')
                if canonicalRels:
                    # Check for every a on the amp page if it is of the type class='amp-canurl'
                    for a in canonicalRels:
                        # Get the direct link
                        found_canonical_url = a.get('href')
                        # If the canonical url is the same as the submitted url, don't use it
                        if found_canonical_url == url:
                            falsePositive = True
                        # If the canonical url has a %%amp%% string, look deeper
                        elif contains_amp_url(found_canonical_url):
                            logging.info("Still amp")
                            # Do another search for the canonical url of the canonical url
                            if depth > 0:
                                newUrl = get_canonical(found_canonical_url, 0)
                                # If it doesn't contain any amp urls, return it
                                if not contains_amp_url(newUrl):
                                    return newUrl
                        # If the canonical link is clean, return it
                        elif not contains_amp_url(found_canonical_url):
                            logging.info("Found the direct link with amp-canurl: {}".format(found_canonical_url))
                            return found_canonical_url
                            
            if falsePositive:
                fatal_error_message = "The canonical URL was found but was the same AMP URL (the AMP specs were badly implemented on this website)"
                logging.warning(fatal_error_message + "\n\n")
                return None
                
            logging.error(traceback.format_exc())
            fatal_error_message = "the canonical URL could not be found (the AMP specs were badly implemented on this website)"
            logging.warning(fatal_error_message + "\n\n")
            return None

        # If no canonical links were found, throw an exception
        except Exception as e:
            logging.error(traceback.format_exc())
            fatal_error_message = "the canonical URL could not be found (the AMP specs were badly implemented on this website)"
            logging.warning(fatal_error_message + "\n\n")
            return None

    # If the submitted page couldn't be fetched, throw an exception
    except Exception as e:
        logging.error(traceback.format_exc())
        fatal_error_message = "the page could not be fetched (the website is probably blocking bots or geo-blocking)"
        logging.warning(fatal_error_message + "\n\n")
        return None

def check_criteria(item):
    # Check: Does the item contain any AMP links?
    string_contains_amp_url = contains_amp_url(item.body)
    if string_contains_amp_url:
        logging.debug(
            "#{} contains one or more '%%amp%%' strings".format(item.id))

        # Check: Has AmputatorBot tried (and failed) to respond to this item already?
        if item.id not in comments_unable_to_reply:
            logging.debug(
                "#{} hasn't been tried before".format(item.id))

            # Check: Has AmputatorBot replied to this item already?
            if item.id not in comments_replied_to:
                logging.debug(
                    "#{} hasn't been posted by AmputatorBot".format(item.id))

                # Check: Is the item written by u/AmputatorBot?
                if not item.author == r.user.me():
                    logging.debug(
                        "#{} hasn't been posted by AmputatorBot".format(item.id))

                    # Check: Is the submission posted by a user who opted out?
                    if not str(item.author) in forbidden_users:
                        logging.debug("#{} hasn't been posted by a user who opted out".format(item.id))
                        return True

                else:
                    logging.debug(
                        "#{} was posted by AmputatorBot".format(item.id))

            else:
                logging.debug(
                    "#{} has already been replied to".format(item.id))

        else:
            logging.debug(
                "#{} has already been tried before".format(item.id))
    
    return False

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

# Get list of users who opted out
def get_forbidden_users():
    if not os.path.isfile("forbidden_users.txt"):
        forbidden_users = []
        logging.warning("forbidden_users.txt could not be found.\n")

    else:
        with open("forbidden_users.txt", "r") as f:
            forbidden_users = f.read()
            forbidden_users = forbidden_users.split(",")
            logging.info("forbidden_users.txt was found.")

    return forbidden_users  

# Get the data of which comments have been replied to
def get_comments_replied():
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
def get_comments_errors():
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
forbidden_users = get_forbidden_users()
comments_replied_to = get_comments_replied()
comments_unable_to_reply = get_comments_errors()

# Run the program
while True:
    run_bot(r, allowed_subreddits, forbidden_users, comments_replied_to, comments_unable_to_reply)