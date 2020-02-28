# This Python file uses the following encoding: utf-8
# License: GPL-3 (https://choosealicense.com/licenses/gpl-3.0/)
# Original author: Killed_Mufasa
# Twitter:https://twitter.com/Killed_Mufasa
# Reddit: https://www.reddit.com/user/Killed_Mufasa
# GitHub: https://github.com/KilledMufasa
# Donate: https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2

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
            false_positive = False  # Reset check for false positives
            method_failed = False  # Assign false before reference

            # Get all canonical links in a list
            canonical_rels = soup.find_all(rel='canonical')
            if canonical_rels:
                for link in canonical_rels:
                    # Get the direct link
                    found_canonical_url = link.get('href')
                    # If the canonical url is the same as the submitted url, don't use it
                    if found_canonical_url == url:
                        false_positive = True
                    # If the canonical url has a %%amp%% string, look deeper
                    elif check_if_amp(found_canonical_url):
                        logging.info("Still amp!")
                        # Do another search for the canonical url of the canonical url
                        if depth > 0:
                            new_url = get_canonical(found_canonical_url, depth - 1)
                            # If it doesn't contain any amp urls, return it
                            if not check_if_amp(new_url):
                                return new_url
                        else:
                            method_failed = True
                    # If the canonical link is clean, return it
                    elif not check_if_amp(found_canonical_url):
                        logging.info("Found the direct link with canonical: {}".format(found_canonical_url))
                        return found_canonical_url

            if not canonical_rels or method_failed:
                # Get all canonical links in a list
                canonical_rels = soup.find_all('a', 'amp-canurl')
                if canonical_rels:
                    # Check for every a on the amp page if it is of the type class='amp-canurl'
                    for a in canonical_rels:
                        # Get the direct link
                        found_canonical_url = a.get('href')
                        # If the canonical url is the same as the submitted url, don't use it
                        if found_canonical_url == url:
                            false_positive = True
                        # If the canonical url has a %%amp%% string, look deeper
                        elif check_if_amp(found_canonical_url):
                            logging.info("Still amp")
                            # Do another search for the canonical url of the canonical url
                            if depth > 0:
                                new_url = get_canonical(found_canonical_url, depth - 1)
                                # If it doesn't contain any amp urls, return it
                                if not check_if_amp(new_url):
                                    return new_url
                        # If the canonical link is clean, return it
                        elif not check_if_amp(found_canonical_url):
                            logging.info("Found the direct link with amp-canurl: {}".format(found_canonical_url))
                            return found_canonical_url

            if false_positive:
                fatal_error_message = "The canonical URL was found but was the same AMP URL" \
                                      "(the AMP specs were badly implemented on this website)"
                logging.warning(fatal_error_message + "\n\n")
                return None

            logging.error(traceback.format_exc())
            fatal_error_message = "the canonical URL could not be found " \
                                  "(the AMP specs were badly implemented on this website)"
            logging.warning(fatal_error_message + "\n\n")
            return None

        # If no canonical links were found, throw an exception
        except:
            logging.error(traceback.format_exc())
            fatal_error_message = "the canonical URL could not be found " \
                                  "(the AMP specs were badly implemented on this website)"
            logging.warning(fatal_error_message + "\n\n")
            return None

    # If the submitted page couldn't be fetched, throw an exception
    except:
        logging.error(traceback.format_exc())
        fatal_error_message = "the page could not be fetched (the website is probably blocking bots or geo-blocking)"
        logging.warning(fatal_error_message + "\n\n")
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
        canonical_url = get_canonical(amp_urls[x], 2)
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
        return canonical_urls_clean

    return canonical_urls


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
