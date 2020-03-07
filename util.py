# This Python file uses the following encoding: utf-8
# License: GPL-3 (https://choosealicense.com/licenses/gpl-3.0/)
# Original author: Killed_Mufasa

# Twitter: https://twitter.com/Killed_Mufasa
# Reddit:  https://www.reddit.com/user/Killed_Mufasa
# GitHub:  https://github.com/KilledMufasa
# Website: https://www.amputatorbot.com
# Donate:  https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2

# This wonderful little program is used by u/AmputatorBot (https://www.reddit.com/user/AmputatorBot)
# to perform a couple of tasks: log in, generate a random header, check for amp links,
# check for google amp links, remove markdown, getting the canonical url,

# Import a couple of libraries
import logging
import os
import re
import traceback
from random import choice

import praw
import requests
from bs4 import BeautifulSoup

import config

warning_log = []


def send_warning(warning):
    logging.warning(warning + "\n" + traceback.format_exc() + "\n\n")
    warning_log.append(warning)


# Login to Reddit API using Praw.
# Reads configuration details out of config.py (not public)
def bot_login():
    try:
        logging.debug("Logging in..")
        r = praw.Reddit(username=config.username,
                        password=config.password,
                        client_id=config.client_id,
                        client_secret=config.client_secret,
                        user_agent="eu.pythonanywhere.com:AmputatorBot:v1.8 (by /u/Killed_Mufasa)")
        logging.debug("Successfully logged in!\n")

        return r

    except:
        logging.error(traceback.format_exc())
        fatal_error_message = "Couldn't log in!\n\n"
        logging.warning("[STOPPED] " + fatal_error_message + "\n\n")

        return None


# Get randomized user agent, set default accept and request English page
# This is done to prevent 403 errors.
def random_headers():
    return {
        'User-Agent': choice(config.headers),
        'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8',
        'Accept-Language': 'en-US'
    }


def check_if_amp(string):
    string = string.lower()  # Make string lowercase

    # If the string contains an AMP link, return True
    if "/amp" in string or "amp/" in string or ".amp" in string or "amp." in string or "?amp" in string \
            or "amp?" in string or "=amp" in string or "amp=" in string or "&amp" in string or "amp&" in string \
            and "https://" in string:
        return True

    # If no AMP link was found in the string, return False
    return False


def check_if_google(string):
    string = string.lower()  # Make string lowercase

    # If the string contains an Google AMP link, return True
    if "www.google." in string or "ampproject.net" in string or "ampproject.org" in string:
        return True

    # If no Google AMP link was found in the string, return False
    return False


def remove_markdown(url):
    # Isolate the actual URL (remove markdown) (part 1)
    try:
        url = url.split('](')[-1]
        logging.debug(
            "{} was stripped of this string: ']('".format(url))

    except:
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


def get_canonical(og_url, depth):
    logging.info("NEW: {} with depth {}".format(og_url, depth))
    # Check if the URL is an AMP URL

    next_url = og_url
    og_depth = depth

    i = 0
    while i < depth:
        soup = get_soup(next_url)

        found_canonical_link, is_solved = get_canonical_with_rel(soup, og_url)

        if is_solved:
            logging.info("SUCCESS: Found canonical with rel: " + found_canonical_link)

            return found_canonical_link

        if not found_canonical_link:
            found_canonical_link_alt, is_solved_alt = get_canonical_with_canurl(soup, og_url)
            if is_solved_alt:
                logging.info("SUCCESS: Found canonical with canurl: " + found_canonical_link)
                return found_canonical_link_alt

            if found_canonical_link_alt:
                next_url = found_canonical_link_alt

            if not found_canonical_link and not found_canonical_link_alt:
                return None

        if found_canonical_link:
            next_url = found_canonical_link

        logging.info("next_url = " + next_url)

        depth = depth - 1

    if og_depth < 2:
        logging.info("Consider increasing the maximum amount of referrals below")
    send_warning("Couldn't find any canonical links with this depth")
    return None


def get_canonical_with_rel(soup, url):
    # Get all canonical links in a list using rel
    try:
        canonical_links = soup.find_all(rel='canonical')
        if canonical_links:
            for link in canonical_links:
                # Get the direct link
                found_canonical_url = link.get('href')
                # If the canonical url is the submitted url, don't use it
                if found_canonical_url == url:
                    send_warning("Encountered a false positive")
                else:
                    if not check_if_amp(found_canonical_url):
                        return found_canonical_url, True
                    else:
                        return found_canonical_url, False

        send_warning("Couldn't find any canonical links")
        return None, False
    except:
        send_warning("Couldn't scrape the website")
        return None, False


def get_canonical_with_canurl(soup, url):
    # Get all canonical links in a list using rel
    try:
        canonical_links = soup.find_all(a='amp-canurl')
        if canonical_links:
            for a in canonical_links:
                # Get the direct link
                found_canonical_url = a.get('href')
                # If the canonical url is the submitted url, don't use it
                if found_canonical_url == url:
                    send_warning("Encountered a false positive")
                else:
                    if not check_if_amp(found_canonical_url):
                        return found_canonical_url, True
                    else:
                        return found_canonical_url, False

        send_warning("Couldn't find any canonical links")
        return None, False
    except:
        send_warning("Couldn't scrape the website")


def get_soup(url):
    try:
        # Fetch amp page
        logging.info("Started fetching " + url)
        req = requests.get(url, headers=random_headers())

        # Make the received data searchable
        logging.info("Making a soup..")
        soup = BeautifulSoup(req.text, features="lxml")
        return soup

    # If the submitted page couldn't be fetched, throw an exception
    except:
        logging.error(traceback.format_exc())
        send_warning("the page could not be fetched (the website is probably blocking bots or geo-blocking)")
        return None


def get_body(item):
    # Check if the parent is a comment, if yes get comment body
    if isinstance(item, praw.models.Comment):
        return item.body

    # Check if the parent is a submission, if yes get submission url or selftext
    if isinstance(item, praw.models.Submission):
        if check_if_amp(item.url):
            return item.url
        if check_if_amp(item.selftext):
            return item.selftext

    return "None"


def get_amp_urls(item_body):
    # Scan the item body for the links
    item_urls = re.findall("(?P<url>https?://[^\s]+)", item_body)
    amp_urls = []
    # Loop through all found links
    for x in range(len(item_urls)):
        # Remove the markdown from the URL
        item_urls[x] = remove_markdown(item_urls[x])

        # Check: Is the isolated URL really an amp link?
        if check_if_amp(item_urls[x]):
            logging.debug("\nAn amp link was found: {}".format(item_urls[x]))
            amp_urls.append(item_urls[x])

        # If the program fails to get the correct amp link, ignore it.
        else:
            logging.warning("This link is no AMP link: {}\n".format(item_urls[x]))

    return amp_urls


def get_canonicals(amp_urls, use_markdown):
    canonical_urls = []
    canonical_urls_clean = []
    for x in range(len(amp_urls)):
        canonical_url = get_canonical(amp_urls[x], 3)
        if canonical_url is not None:
            logging.debug("Canonical_url returned is not None")
            canonical_urls_clean.append(canonical_url)
            # Add markdown in case there are multiple URLs
            if use_markdown:
                # Calculate which number to prefix
                canonical_urls_amount = len(canonical_urls) + 1
                # Make a string out of the prefix and the canonical url
                canonical_url_markdown = "[" + str(
                    canonical_urls_amount) + "] **[" + canonical_url + "](" + canonical_url + ")**"
                # And append this to the list
                canonical_urls.append(canonical_url_markdown)
                logging.debug("The array of canonical urls is now: {}".format(
                    canonical_urls))
            else:
                canonical_urls.append(canonical_url)
        else:
            logging.debug("No canonical URLs were found, skipping this one")

    if len(canonical_urls_clean) == 1:
        return canonical_urls_clean, warning_log

    return canonical_urls, warning_log


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


# Get the data of which subreddits the bot is forbidden to post in
def get_forbidden_subreddits():
    if not os.path.isfile("forbidden_subreddits.txt"):
        forbidden_subreddits = []
        logging.warning("forbidden_subreddits.txt could not be found.\n")

    else:
        with open("forbidden_subreddits.txt", "r") as f:
            forbidden_subreddits = f.read()
            forbidden_subreddits = forbidden_subreddits.split(",")
            logging.info("forbidden_subreddits.txt was found.")

    return forbidden_subreddits


# Get the data of which subreddits the bot should use NP URLs
def get_np_subreddits():
    if not os.path.isfile("np_subreddits.txt"):
        np_subreddits = []
        logging.warning("np_subreddits.txt could not be found.\n")

    else:
        with open("np_subreddits.txt", "r") as f:
            np_subreddits = f.read()
            np_subreddits = np_subreddits.split(",")
            logging.info("np_subreddits.txt was found.")

    return np_subreddits


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


# Get the data of which submissions have been replied to
def get_submissions_replied():
    if not os.path.isfile("submissions_replied_to.txt"):
        submissions_replied_to = []
        logging.warning("submissions_replied_to.txt could not be found.\n")

    else:
        with open("submissions_replied_to.txt", "r") as f:
            submissions_replied_to = f.read()
            submissions_replied_to = submissions_replied_to.split(",")
            logging.info("submissions_replied_to.txt was found.")

    return submissions_replied_to


# Get the data of which submissions could not be replied to
def get_submissions_errors():
    if not os.path.isfile("submissions_unable_to_reply.txt"):
        submissions_unable_to_reply = []
        logging.warning("submissions_unable_to_reply.txt could not be found.\n")

    else:
        with open("submissions_unable_to_reply.txt", "r") as f:
            submissions_unable_to_reply = f.read()
            submissions_unable_to_reply = submissions_unable_to_reply.split(",")
            logging.info("submissions_unable_to_reply.txt was found.")

    return submissions_unable_to_reply


# Get the data of which mentions have been replied to
def get_mentions_replied():
    if not os.path.isfile("mentions_replied_to.txt"):
        mentions_replied_to = []
        logging.warning("mentions_replied_to.txt could not be found.\n")

    else:
        with open("mentions_replied_to.txt", "r") as f:
            mentions_replied_to = f.read()
            mentions_replied_to = mentions_replied_to.split(",")
            logging.info("mentions_replied_to.txt was found.")

    return mentions_replied_to


# Get the data of which mentions could not be replied to
def get_mentions_errors():
    if not os.path.isfile("mentions_unable_to_reply.txt"):
        mentions_unable_to_reply = []
        logging.warning("mentions_unable_to_reply.txt could not be found.\n")

    else:
        with open("mentions_unable_to_reply.txt", "r") as f:
            mentions_unable_to_reply = f.read()
            mentions_unable_to_reply = mentions_unable_to_reply.split(",")
            logging.info("mentions_unable_to_reply.txt was found.")

    return mentions_unable_to_reply
