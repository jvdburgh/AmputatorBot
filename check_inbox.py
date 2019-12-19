# This Python file uses the following encoding: utf-8
# License: GPL-3 (https://choosealicense.com/licenses/gpl-3.0/)
# Original author: Killed_Mufasa
# Twitter:https://twitter.com/Killed_Mufasa
# Reddit: https://www.reddit.com/user/Killed_Mufasa
# GitHub: https://github.com/KilledMufasa
# Donate: https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2

# This wonderfull little program is used by u/AmputatorBot
# (https://www.reddit.com/user/AmputatorBot) to scan u/AmputatorBot's
# inbox for opt-out requests, opt-back-in requests and mentions.
# If AmputatorBot detects an AMP link in the parent of a mention
# a reply is made with the direct link. If a user opts out, the username
# will be added to a dynamic file which will be confirmed with a DM.

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
    filename="v1.5_check_inbox.log",
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


# Main function. Gets the inbox stream, filters for mentions,
# scans the context for AMP links and replies with the direct link
def run_bot(r, forbidden_subreddits, mentions_replied_to, mentions_unable_to_reply):
    # Get the inbox stream using Praw
    for message in r.inbox.unread(limit=None):
        # Resets for every inbox item
        user_opting_in = ""
        user_opting_out = ""
        reply_to_parent = False
        item_could_not_reply = False
        item_could_reply = False
        parent_is_comment = False
        parent_is_submission = False
        mentions_urls = []
        mentions_non_amp_urls = []
        mentions_non_amps_urls_amount = 0
        mentions_canonical_url = ""

        # Get the subject of the message (can be username mention, 
        # a comment reply notification, an opt-out request, an opt-in request
        # or another type of inbox message. Mark the item as read afterwards.
        try:
            subject = message.subject.lower()
            logging.info("\nNEW UNREAD INBOX ITEM\n\nSubject: {}\nAuthor: {}\nMessage body: {}\n\n".format(
                subject, message.author, message.body))

            # Mark the item as read
            message.mark_read()

            # Check if the inbox item is a mention
            if subject == "username mention" and isinstance(message, praw.models.Comment):
                logging.debug("The bot was mentioned in this message: #"+message.id)
                mention = message

                try:
                # Log that and where u/AmputatorBot has been mentioned
                    parent = mention.parent()
                    logging.debug("Mention detected (comment ID: {} & parent ID: {})".format(
                        mention.id, parent.id))

                    # Firguring out if parent is a submission or comment
                    logging.info("Checking instance of parent..")

                    try:
                        # Check if the parent is a comment, if yes get comment body
                        if(isinstance(parent, praw.models.Comment)):
                            logging.debug("The parent #{} is a comment".format(parent.id))
                            parent_body = parent.body
                            parent_is_comment = True

                        # Check if the parent is a submission, if yes get submission body
                        if(isinstance(parent, praw.models.Submission)):
                            logging.debug("The parent #{} is a submission".format(parent.id))
                            parent_body = parent.url
                            parent_is_submission = True

                    except:
                        logging.error(traceback.format_exc())
                        logging.warning("Unexpected instance\n\n")
                        item_could_not_reply = True

                    # Check if the parent comment contains an AMP link
                    string_contains_amp_url = contains_amp_url(parent_body)
                    if string_contains_amp_url:
                        logging.debug(
                            "#{} contains one or more '%%amp%%' strings".format(parent.id))

                        # Check: Is AmputatorBot allowed in called subreddit?
                        if mention.subreddit.display_name not in forbidden_subreddits:
                            logging.debug(
                                "#{} may post in this subreddit".format(parent.id))

                            # Check: Has AmputatorBot tried (and failed) to respond to this mention already?
                            if parent.id not in mentions_unable_to_reply:
                                logging.debug(
                                    "#{} hasn't been tried before".format(parent.id))

                                # Check: Has AmputatorBot replied to this mention already?
                                if parent.id not in mentions_replied_to:
                                    logging.debug(
                                        "#{} hasn't been replied to yet".format(parent.id))

                                    # Check: Is the mention posted by u/AmputatorBot?
                                    if not parent.author == r.user.me():
                                        logging.debug(
                                            "#{} hasn't been posted by AmputatorBot".format(parent.id))

                                        # Check: Is the mention posted by a user who opted out?
                                        with open("forbidden_users.txt", "r") as f:
                                            forbidden_users = f.read()
                                            forbidden_users = forbidden_users.split(",")

                                        if not str(parent.author) in forbidden_users:
                                            logging.debug("#{} hasn't been posted by a user who opted out".format(parent.id))
                                            reply_to_parent = True

                                    else:
                                        logging.debug(
                                            "#{} was posted by AmputatorBot".format(parent.id))

                                else:
                                    logging.debug(
                                        "#{} has already been replied to".format(parent.id))

                            else:
                                logging.debug(
                                    "#{} has already been tried before".format(parent.id))

                        else:
                            logging.debug(
                                "#{} the bot may not post in this subreddit".format(parent.id))

                    else:
                        logging.debug(
                            "#{} doesn't contain any '%%amp%%' strings".format(parent.id))

                    # If the parent is a comment, try to reply with the direct link
                    if reply_to_parent:
                        try:
                            logging.debug(
                                "#{}'s body: {}\nScanning for urls..".format(parent.id, parent_body))

                            # Scan the comment body for the links
                            mentions_urls = re.findall("(?P<url>https?://[^\s]+)", parent_body)
                            mentions_urls_amount = len(mentions_urls)

                            # Loop through all submitted links
                            for x in range(mentions_urls_amount):

                                # Isolate the actual URL (remove markdown) (part 1)
                                try:
                                    mentions_urls[x] = mentions_urls[x].split('](')[-1]
                                    logging.debug(
                                        "{} was stripped of this string: ']('".format(mentions_urls[x]))

                                except Exception as e:
                                    logging.error(traceback.format_exc())
                                    logging.debug(
                                        "{} couldn't or didn't have to be stripped of this string: ']('.".format(mentions_urls[x]))

                                # Isolate the actual URL (remove markdown) (part 2)
                                if mentions_urls[x].endswith(')?'):
                                    mentions_urls[x] = mentions_urls[x][:-2]
                                    logging.debug("{} was stripped of this string: ')?'".format(
                                        mentions_urls[x]))
                                        
                                if mentions_urls[x].endswith(')'):
                                    mentions_urls[x] = mentions_urls[x][:-1]
                                    logging.debug("{} was stripped of this string: ')'".format(
                                        mentions_urls[x]))
                                        
                                # Check: Is the isolated URL really an amp link?
                                string_contains_amp_url = contains_amp_url(
                                    mentions_urls[x])
                                if string_contains_amp_url:
                                    logging.debug("\nAn amp link was found: {}".format(
                                        mentions_urls[x]))

                                    # Fetch the submitted amp page, search for the canonical link
                                    try:
                                        # Fetch submitted amp page
                                        logging.info("Started fetching..")
                                        req = requests.get(
                                            mentions_urls[x], headers=random_headers())

                                        # Make the received data searchable
                                        logging.info("Making a soup..")
                                        soup = BeautifulSoup(req.text, features="lxml")
                                        logging.info("Making a searchable soup..")
                                        soup.prettify()

                                        # Scan the received data for the direct link
                                        logging.info("Scanning for all links..")
                                        try:
                                            # Check for every link on the amp page if it is of the type rel='canonical'
                                            for link in soup.find_all(rel='canonical'):
                                                # Get the direct link
                                                mentions_canonical_url = link.get('href')
                                                logging.debug("Found the direct link: {}".format(
                                                    mentions_canonical_url))

                                                # If the canonical url is the same as the submitted url, don't use it
                                                if mentions_canonical_url == mentions_urls[x]:
                                                    logging.warning(
                                                        "False positive encounterd! (mentions_canonical_url == mentions_urls[x])")
                                                    item_could_not_reply = True

                                                # If the canonical url is unique, add the direct link to the array
                                                else:
                                                    mentions_non_amps_urls_amount = len(
                                                        mentions_non_amp_urls)
                                                    mentions_canonical_url_markdown = "["+str(
                                                        mentions_non_amps_urls_amount+1)+"] **"+mentions_canonical_url+"**"
                                                    mentions_non_amp_urls.append(
                                                        mentions_canonical_url_markdown)
                                                    logging.debug("The array of canonical urls is now: {}".format(
                                                        mentions_non_amp_urls))

                                        # If no direct links were found, throw an exception
                                        except Exception as e:
                                            logging.error(traceback.format_exc())
                                            logging.warning("The direct link could not be found.\n")
                                            item_could_not_reply = True

                                    # If the submitted page could't be fetched, throw an exception
                                    except Exception as e:
                                        logging.error(traceback.format_exc())
                                        logging.warning("The page could not be fetched.\n")
                                        item_could_not_reply = True

                                # If the program fails to get the correct amp link, ignore it.
                                else:
                                    logging.warning("This link is no AMP link: {}\n".format(mentions_urls[x]))

                        # If the program fails to find any link at all, throw an exception
                        except Exception as e:
                            logging.error(traceback.format_exc())
                            logging.warning("No links were found.\n")
                            item_could_not_reply = True

                        # Count the canonical urls
                        mentions_non_amps_urls_amount = len(mentions_non_amp_urls)

                        # If no canonical urls were found, don't reply
                        if mentions_non_amps_urls_amount == 0:
                            logging.warning("[STOPPED] There were no canonical urls found.\n\n\n")
                            item_could_not_reply = True

                        # If there were direct links found, reply!
                        else:

                            # Try to reply to OP
                            try:

                                # If there was only one url found, generate a simple comment
                                if mentions_non_amps_urls_amount == 1:
                                    mention_reply = "Beep boop, I'm a bot. It looks like OP shared a Google AMP link. Google AMP pages often load faster, but AMP is a [major threat to the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot).\n\nYou might want to visit **the normal page** instead: **"+mentions_canonical_url+"**.\n\n*****\n\n​[^(Why & About)](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot)^( | )[^(Mention me to summon me!)](https://www.reddit.com/r/AmputatorBot/comments/cchly3/you_can_now_summon_amputatorbot/)^( | **Summoned by a** )[^(**good human here!**)](https://www.reddit.com"+mention.context+")"

                                # If there were multiple urls found, generate a multi-url comment
                                if mentions_non_amps_urls_amount > 1:
                                    # Generate string of all found links
                                    mention_reply_generated = '\n\n'.join(mentions_non_amp_urls)
                                    # Generate entire comment
                                    mention_reply = "Beep boop, I'm a bot. It looks like OP shared a couple of Google AMP links. Google AMP pages often load faster, but AMP is a [major threat to the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot).\n\nYou might want to visit **the normal pages** instead: \n\n"+mention_reply_generated+"\n\n*****\n\n[^(Why & About)](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot)^( | )[^(Mention me to summon me!)](https://www.reddit.com/r/AmputatorBot/comments/cchly3/you_can_now_summon_amputatorbot/)^( | **Summoned by a** )[^(**good human here!**)](https://www.reddit.com"+mention.context+")"

                                # Reply to mention
                                parent.reply(mention_reply)
                                logging.debug("Replied to #{}\n".format(parent.id))
                                item_could_reply = True

                                # Send a DM to the summoner with confirmation and link to parent comment
                                r.redditor(str(mention.author)).message("Thx for summoning me!", "The bot has successfully replied to this comment: https://www.reddit.com"+parent.permalink+".\n\nAn easy way to find the comment is by checking my comment history. Thanks for summoning me, I couldn't do this without you (no but literally). You're a very good human <3\n\nFeel free to leave feedback by contacting u/killed_mufasa, by posting on [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/) or by [opening an issue on GitHub](https://github.com/KilledMufasa/AmputatorBot/issues/new).")

                                logging.info("Confirmed the reply to the summoner.\n")

                            # If the reply didn't got through, throw an exception (can occur when comment gets deleted or when rate limits are exceeded)
                            except Exception as e:
                                logging.error(traceback.format_exc())
                                logging.warning("Could not reply to post.\n\n\n")
                                item_could_not_reply = True

                        # If the reply was successfully send, note this
                        if item_could_reply:
                            with open("mentions_replied_to.txt", "a") as f:
                                f.write(parent.id + ",")
                                mentions_replied_to.append(parent.id)
                                logging.info("Added the parent id to file: mentions_replied_to.txt\n\n\n")

                        # If the reply could not be made or send, note this
                        if item_could_not_reply:
                            with open("mentions_unable_to_reply.txt", "a") as f:
                                f.write(parent.id + ",")
                                mentions_unable_to_reply.append(parent.id)
                                logging.info("Added the parent id to file: mentions_unable_to_reply.txt.")

                                # Send a DM about the error to the summoner
                                r.redditor(str(mention.author)).message("AmputatorBot ran into an error..", "AmputatorBot couldn't reply to the comment or submission you summoned it for: https://www.reddit.com"+parent.permalink+".\n\nThis error has been logged and is being investigated. Common causes for this error are: bot- and geoblocking websites, badly implemented AMP specs and the disallowance of the bot in [certain subreddits](https://www.reddit.com/r/AmputatorBot/comments/c88zm3/why_did_i_build_amputatorbot/).\n\nThat said, you can leave feedback by contacting u/killed_mufasa, by posting on [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/) or by [opening an issue on GitHub](https://github.com/KilledMufasa/AmputatorBot/issues/new).\n\nYou're a very good human for trying <3")

                                logging.info("Notified the summoner of the error.\n")

                    else:
                        logging.debug(
                            "[STOPPED] #{} didn't meet the requirements.\n\n\n".format(parent.id))

                except:
                    logging.error(traceback.format_exc())
                    logging.warning("This mention is weird\n\n")

            # Check if the inbox item is an opt-out request
            if subject == "opt me out of amputatorbot":
                try:
                    logging.debug("[OPT-OUT] was requested by {} in {}".format(message.author, message.id))

                    # Convert Redditor to username string
                    user_opting_out = str(message.author)
                    
                    # If the username is already opted out, notify the user
                    with open("forbidden_users.txt", "r") as f:
                        forbidden_users = f.read()
                        forbidden_users = forbidden_users.split(",")

                    if user_opting_out in forbidden_users:
                        logging.warning("This user was already opted out.")

                        # Send a DM to the summoner with confirmation
                        r.redditor(user_opting_out).message("You have already opted out","The bot won't reply to your comments and submissions, but you will still see my replies to other peoples content. Block u/AmputatorBot if you don't want to see those either.\n\nFeel free to leave feedback by contacting u/killed_mufasa, by posting on [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/) or by [opening an issue on GitHub](https://github.com/KilledMufasa/AmputatorBot/issues/new).")

                    # If the username is not already in the opt-out list, add it
                    if user_opting_out not in forbidden_users:
                        with open("forbidden_users.txt", "a") as f:
                            f.write(user_opting_out + ",")
                            forbidden_users.append(user_opting_out)
                            logging.debug("Added the username to file: forbidden_users.txt\n\n\n")
                        
                        # Send a DM to the summoner with confirmation
                        r.redditor(user_opting_out).message("You have successfully opted out of AmputatorBot","The bot won't reply to your comments and submissions anymore, but you will still see my replies to other peoples content. Block u/AmputatorBot if you don't want to see those either.\n\nFeel free to leave feedback by contacting u/killed_mufasa, by posting on [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/) or by [opening an issue on GitHub](https://github.com/KilledMufasa/AmputatorBot/issues/new).")

                except:
                    logging.error(traceback.format_exc())
                    logging.warning("Something went wrong while processing an opt-out request.\n\n")
                    item_could_not_reply = True

            # Check if the inbox item is an opt-in request
            if subject == "opt me back in again of amputatorbot":
                try:
                    logging.debug("[OPT-IN] was requested by {} in {}".format(
                        message.author, message.id))

                    # Convert Redditor to username string
                    user_opting_in = str(message.author)

                    # If the username is not in the opt-out list, notify the user
                    with open("forbidden_users.txt", "r") as f:
                        forbidden_users = f.read()
                        forbidden_users = forbidden_users.split(",")

                    if user_opting_in not in forbidden_users:
                        logging.warning('This user never opted-out.')
                        
                        # Send a DM to the summoner with confirmation
                        r.redditor(user_opting_in).message("You don't have to opt-in","This opt-in feature is only meant for users who choose to opt out earlier and now regret that decision. According to our systems, you didn't opt out of AmputatorBot so there's no need for you to opt in.\n\nRemember that the bot only works in a couple of subreddits. You can summon the bot almost everywhere else by mentioning u/AmputatorBot in a direct reply to a submission or comment containing an AMP link.\n\nFeel free to leave feedback by contacting u/killed_mufasa, by posting on [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/) or by [opening an issue on GitHub](https://github.com/KilledMufasa/AmputatorBot/issues/new).")

                        logging.info('DM successfully send.')

                    # If the username is indeed in the opt-out list, remove the user from the list
                    if user_opting_in in forbidden_users:
                        logging.info("Removing user from opt-out list.")
                        with open("forbidden_users.txt", "a+") as f:
                            updatedList=f.read().replace(user_opting_in+",", "")
                        with open("forbidden_users.txt", "w") as f:
                            f.write(updatedList)

                        logging.info("Removed the username to file: forbidden_users.txt and removed user from opted-out list\n\n\n")
                        forbidden_users.remove(user_opting_in)

                        # Send a DM to the summoner with confirmation
                        r.redditor(user_opting_in).message("You have succesfully opted back in","Remember that the bot only works in a couple of subreddits. You can summon the bot almost everywhere else by mentioning u/AmputatorBot in a direct reply to a submission or comment containing an AMP link.\n\nFeel free to leave feedback by contacting u/killed_mufasa, by posting on [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/) or by [opening an issue on GitHub](https://github.com/KilledMufasa/AmputatorBot/issues/new).")

                        logging.info('DM successfully send.')

                except:
                    logging.error(traceback.format_exc())
                    logging.warning("Something went wrong while processing an opt-in request.\n\n")
                    item_could_not_reply = True
        
        except:
            logging.error(traceback.format_exc())
            logging.warning("Unexpected instance\n\n")
            item_could_not_reply = True


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


# Get the data of which mentions have been replied to
def get_saved_mentions_repliedtos():
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
def get_saved_mentions_unabletos():
    if not os.path.isfile("mentions_unable_to_reply.txt"):
        mentions_unable_to_reply = []
        logging.warning("mentions_unable_to_reply.txt could not be found.\n")

    else:
        with open("mentions_unable_to_reply.txt", "r") as f:
            mentions_unable_to_reply = f.read()
            mentions_unable_to_reply = mentions_unable_to_reply.split(",")
            logging.info("mentions_unable_to_reply.txt was found.")

    return mentions_unable_to_reply


# Uses these functions to run the bot
r = bot_login()
forbidden_subreddits = get_forbidden_subreddits()
mentions_replied_to = get_saved_mentions_repliedtos()
mentions_unable_to_reply = get_saved_mentions_unabletos()


# Run the program
while True:
    run_bot(r, forbidden_subreddits, mentions_replied_to,
            mentions_unable_to_reply)
