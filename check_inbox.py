# This Python file uses the following encoding: utf-8
# License: GPL-3 (https://choosealicense.com/licenses/gpl-3.0/)
# Original author: Killed_Mufasa
# Twitter:https://twitter.com/Killed_Mufasa
# Reddit: https://www.reddit.com/user/Killed_Mufasa
# GitHub: https://github.com/KilledMufasa
# Donate: https://www.paypal.com/cgi-bin/webscr?cmd=_s-xclick&hosted_button_id=EU6ZFKTVT9VH2

# This wonderful little program is used by u/AmputatorBot
# (https://www.reddit.com/user/AmputatorBot) to scan u/AmputatorBot's
# inbox for opt-out requests, opt-back-in requests and mentions.
# If AmputatorBot detects an AMP link in the parent of a mention
# a reply is made with the direct link, the summoner receives a DM.
# If a user opts out, the username will be added to a dynamic file
# which will be confirmed with a DM.

# Import a couple of libraries
import logging
import traceback
from time import sleep

import praw

import util

logging.basicConfig(
    filename="logs/v1.9/check_inbox.log",
    level=logging.INFO,
    format="%(asctime)s:%(levelname)s:%(message)s"
)


# Main function. Gets the inbox stream, filters for mentions,
# scans the context for AMP links and replies with the direct link
def run_bot(r, forbidden_subreddits, forbidden_users, np_subreddits, mentions_replied_to, mentions_unable_to_reply):
    # Get the inbox stream using Praw
    for message in r.inbox.unread(limit=None):
        # Resets for every inbox item
        canonical_urls = []
        fatal_error_message = "a not so common one"
        parent_body = ""
        reply = ""
        reply_generated = ""
        success = False
        domain = "www"
        note = "\n\n"
        note_alt = "\n\n"
        # Get the subject of the message (can be username mention,
        # a comment reply notification, an opt-out request, an opt-in request
        # or another type of inbox message. Mark the item as read afterwards.
        try:
            subject = message.subject.lower()
            logging.info("\nNEW UNREAD INBOX ITEM\nSubject: {}\nAuthor: {}\nMessage body: {}\n\n".format(
                subject, message.author, message.body))

            # Mark the item as read
            message.mark_read()

            # Check if the inbox item is a mention
            if subject == "username mention" and isinstance(message, praw.models.Comment):
                logging.debug("The bot was mentioned in this message: #" + message.id)
                item = message

                try:
                    # Log that and where u/AmputatorBot has been mentioned
                    parent = item.parent()
                    logging.debug("Mention detected (comment ID: {} & parent ID: {})".format(
                        item.id, parent.id))

                    try:
                        # Set body in accordance to the instance
                        logging.info("Fetching body of parent..")
                        parent_body = util.get_body(parent)

                    except:
                        logging.error(traceback.format_exc())
                        logging.warning("Couldn't find a body containing an amp link\n\n")

                    # If the item contains an AMP link and meets the other criteria, fetch the canonical link(s)
                    if util.check_if_amp(parent_body) and check_criteria(item, parent):
                        try:
                            logging.debug("#{}'s body: {}\nScanning for urls..".format(parent.id, parent_body))
                            try:
                                amp_urls = util.get_amp_urls(parent_body)
                                if not amp_urls:
                                    logging.warning("Couldn't find any amp urls in: {}".format(parent_body))
                                else:
                                    for x in range(len(amp_urls)):
                                        if util.check_if_google(amp_urls[x]):
                                            note = " This page is even entirely hosted on Google's servers (!).\n\n"
                                            note_alt = " Some of these pages are even entirely hosted on Google's servers (!).\n\n"
                                            break
                                    canonical_urls = util.get_canonicals(amp_urls, True)
                                    if canonical_urls:
                                        reply_generated = '\n\n'.join(canonical_urls)

                                    else:
                                        logging.info("No canonical urls were found\n")
                            except:
                                logging.warning("Couldn't check amp_urls")

                        # If the program fails to find any link at all, throw an exception
                        except:
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

                                # If the subreddit encourages the use of NP, make it NP
                                if item.subreddit in np_subreddits:
                                    domain = "np"

                                # If there was only one url found, generate a simple comment
                                if canonical_urls_amount == 1:
                                    reply = "It looks like OP shared an AMP link. These will often load faster, but Google's AMP [threatens the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://" + domain + ".reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot)." + note + "You might want to visit **the normal page** instead: **[" + canonical_urls[0] + "](" + canonical_urls[0] + ")**.\n\n*****\n\n​^(I'm a bot | )[^(Why & About)](https://" + domain + ".reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot)^( | )[^(Mention me to summon me!)](https://" + domain + ".reddit.com/r/AmputatorBot/comments/cchly3/you_can_now_summon_amputatorbot/)^( | **Summoned by a** )[^(**good human here!**)](https://" + domain + ".reddit.com" + item.context + ")"

                                # If there were multiple urls found, generate a multi-url comment
                                if canonical_urls_amount > 1:
                                    # Generate entire comment
                                    reply = "It looks like OP shared a couple of AMP links. These will often load faster, but Google's AMP [threatens the Open Web](https://www.socpub.com/articles/chris-graham-why-google-amp-threat-open-web-15847) and [your privacy](https://" + domain + ".reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot)." + note_alt + "You might want to visit **the normal pages** instead: \n\n" + reply_generated + "\n\n*****\n\n^(I'm a bot | )[^(Why & About)](https://" + domain + ".reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot)^( | )[^(Mention me to summon me!)](https://" + domain + ".reddit.com/r/AmputatorBot/comments/cchly3/you_can_now_summon_amputatorbot/)^( | **Summoned by a** )[^(**good human here!**)](https://" + domain + ".reddit.com" + item.context + ")"

                                # Reply to mention
                                reply = parent.reply(reply)
                                logging.debug("Replied to #{}: {} : {}\n".format(parent.id, reply.id, reply.permalink))

                                # Send a DM to the summoner with confirmation and link to parent comment
                                r.redditor(str(item.author)).message("Thx for summoning me!", "AmputatorBot has [successfully replied](https://www.reddit.com" + reply.permalink + ") to [the item you summoned it for](https://www.reddit.com" + parent.permalink + ").\n\nThanks for summoning me, I couldn't do this without you (no but literally). You're a very good human <3\n\nFeel free to leave feedback by contacting u/killed_mufasa, by posting on [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/) or by [opening an issue on GitHub](https://github.com/KilledMufasa/AmputatorBot/issues/new).\n\nNEW: With AmputatorBot.com you can remove AMP from your URLs in just one click! Check out an example with your amp link here: https://AmputatorBot.com/?" + amp_urls[0])

                                logging.info("Confirmed the reply to the summoner.\n")

                                # If the reply was successfully send, note this
                                success = True
                                with open("mentions_replied_to.txt", "a") as f:
                                    f.write(parent.id + ",")
                                mentions_replied_to.append(parent.id)
                                logging.info("Added the parent id to file: mentions_replied_to.txt\n\n\n")

                            # If the reply didn't got through, throw an exception
                            # can occur when comment gets deleted or when rate limits are exceeded
                            except:
                                logging.error(traceback.format_exc())
                                fatal_error_message = "could not reply to item, it either got deleted or the rate-limits have been exceeded"
                                logging.warning("[STOPPED] " + fatal_error_message + "\n\n")

                        # If the reply could not be made or send, note this
                        if not success:
                            with open("mentions_unable_to_reply.txt", "a") as f:
                                f.write(parent.id + ",")
                            mentions_unable_to_reply.append(parent.id)
                            logging.info("Added the parent id to file: mentions_unable_to_reply.txt.")

                            # Send a DM about the error to the summoner
                            r.redditor(str(item.author)).message("AmputatorBot ran into an error..",
                                                                 "AmputatorBot couldn't reply to [the comment or submission you summoned it for](https://www.reddit.com" + parent.permalink + ").\n\nAmputatorBot ran into the following error: " + fatal_error_message + ".\n\nThis error has been logged and is being investigated. Common causes for this error are: bot- and geoblocking websites and badly implemented AMP specs.\n\nFeel free to leave feedback by contacting u/killed_mufasa, by posting on [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/) or by [opening an issue on GitHub](https://github.com/KilledMufasa/AmputatorBot/issues/new).\n\nYou're a very good human for trying <3\n\nNEW: With AmputatorBot.com you can remove AMP from your URLs in just one click! You could try it again there but it will probably raise an error again: https://AmputatorBot.com/?" + amp_urls[0])

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
                    if user_opting_out in forbidden_users:
                        logging.warning("This user was already opted out.")

                        # Send a DM to the summoner with confirmation
                        r.redditor(user_opting_out).message("You have already opted out",
                                                            "The bot won't reply to your comments and submissions, but you will still see my replies to other peoples content. Block u/AmputatorBot if you don't want to see those either.\n\nFeel free to leave feedback by contacting u/killed_mufasa, by posting on [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/) or by [opening an issue on GitHub](https://github.com/KilledMufasa/AmputatorBot/issues/new).")

                    # If the username is not already in the opt-out list, add it
                    if user_opting_out not in forbidden_users:
                        with open("forbidden_users.txt", "a") as f:
                            f.write(user_opting_out + ",")
                            forbidden_users.append(user_opting_out)
                            logging.debug("Added the username to file: forbidden_users.txt\n\n\n")

                        # Send a DM to the summoner with confirmation
                        r.redditor(user_opting_out).message("You have successfully opted out of AmputatorBot",
                                                            "The bot won't reply to your comments and submissions anymore, but you will still see my replies to other peoples content. Block u/AmputatorBot if you don't want to see those either.\n\nFeel free to leave feedback by contacting u/killed_mufasa, by posting on [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/) or by [opening an issue on GitHub](https://github.com/KilledMufasa/AmputatorBot/issues/new).")

                except:
                    logging.error(traceback.format_exc())
                    logging.warning("Something went wrong while processing an opt-out request.\n\n")

            # Check if the inbox item is an opt-in request
            if subject == "opt me back in again of amputatorbot":
                try:
                    logging.debug("[OPT-IN] was requested by {} in {}".format(
                        message.author, message.id))

                    # Convert Redditor to username string
                    user_opting_in = str(message.author)

                    # If the username is not in the opt-out list, notify the user
                    if user_opting_in not in forbidden_users:
                        logging.warning('This user never opted-out.')

                        # Send a DM to the summoner with confirmation
                        r.redditor(user_opting_in).message("You don't have to opt-in",
                                                           "This opt-in feature is only meant for users who choose to opt out earlier and now regret that decision. According to our systems, you didn't opt out of AmputatorBot so there's no need for you to opt in.\n\nRemember that the bot only works in a couple of subreddits. You can summon the bot almost everywhere else by mentioning u/AmputatorBot in a direct reply to a submission or comment containing an AMP link.\n\nFeel free to leave feedback by contacting u/killed_mufasa, by posting on [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/) or by [opening an issue on GitHub](https://github.com/KilledMufasa/AmputatorBot/issues/new).")

                        logging.info('DM successfully send.')

                    # If the username is indeed in the opt-out list, remove the user from the list
                    if user_opting_in in forbidden_users:
                        logging.info("Removing user from opt-out list.")
                        with open("forbidden_users.txt", "a+") as f:
                            updated_list = f.read().replace(user_opting_in + ",", "")
                        with open("forbidden_users.txt", "w") as f:
                            f.write(updated_list)

                        logging.info("Successfully opted user out.\n\n\n")
                        forbidden_users.remove(user_opting_in)

                        # Send a DM to the summoner with confirmation
                        r.redditor(user_opting_in).message("You have successfully opted back in",
                                                           "Remember that the bot only works in a couple of subreddits. You can summon the bot almost everywhere else by mentioning u/AmputatorBot in a direct reply to a submission or comment containing an AMP link.\n\nFeel free to leave feedback by contacting u/killed_mufasa, by posting on [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/) or by [opening an issue on GitHub](https://github.com/KilledMufasa/AmputatorBot/issues/new).")

                        logging.info('DM successfully send.')

                except:
                    logging.error(traceback.format_exc())
                    logging.warning("Something went wrong while processing an opt-in request.\n\n")

        except:
            logging.error(traceback.format_exc())
            logging.warning("Unexpected instance\n\n")


def check_criteria(item, parent):
    # Must not be in forbidden_subreddits
    if item.subreddit.display_name in forbidden_subreddits:
        try:
            # Set body in accordance to the instance
            parent_body = util.get_body(parent)

            if util.check_if_amp(parent_body):
                logging.info("The parent body contains an amp url")
                # Try to find the first url in the comment
                try:
                    amp_urls = util.get_amp_urls(parent_body)
                    if not amp_urls:
                        logging.info("Couldn't find any amp_urls")
                    else:
                        canonical_urls = util.get_canonicals(amp_urls, True)
                        if canonical_urls:
                            # Generate string of all found links
                            reply_generated = '\n\n'.join(canonical_urls)
                            # Send a DM about the error to the summoner
                            r.redditor(str(item.author)).message(
                                "AmputatorBot ran into an error: Disallowed subreddit",
                                "AmputatorBot couldn't reply to [the comment or submission you summoned it for](https://www.reddit.com" + parent.permalink + ") because AmputatorBot is disallowed and/or banned in r/" + item.subreddit.display_name + ", just like it is in [some others](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot). Unfortunately, this means that it can't post there. But that doesn't stop us! Here are the canonical URLs you requested:\n\n" + reply_generated + "\n\nMaybe _you_ could post it instead?\n\nYou can leave feedback by contacting u/killed_mufasa, by posting on [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/) or by [opening an issue on GitHub](https://github.com/KilledMufasa/AmputatorBot/issues/new).\n\nNEW: With AmputatorBot.com you can remove AMP from your URLs in just one click! Check out an example with your amp link here: https://AmputatorBot.com/?" +
                                amp_urls[0])
                            logging.info("Notified the summoner of the error.\n")
                        else:
                            logging.info("No canonical urls were found\n")

                # If the program fails to find any link at all, throw an exception
                except:
                    logging.error(traceback.format_exc())
                    logging.warning("No links were found or couldn't reply\n")

        except:
            logging.error(traceback.format_exc())
            logging.warning("Unexpected instance\n\n")

        return False
    # Must not be a mention that previously failed
    if parent.id in mentions_unable_to_reply:
        return False
    # Must not be a mention already replied to
    if parent.id in mentions_replied_to:
        return False
    # Must not be posted by me
    if parent.author == r.user.me():
        return False
    # Must not be posted by a user who opted out
    if str(parent.author) in forbidden_users:
        return False

    # If all criteria were met, return True
    return True


# Uses these functions to run the bot
r = util.bot_login()
forbidden_subreddits = util.get_forbidden_subreddits()
forbidden_users = util.get_forbidden_users()
np_subreddits = util.get_np_subreddits()
mentions_replied_to = util.get_mentions_replied()
mentions_unable_to_reply = util.get_mentions_errors()

# Run the program
while True:
    try:
        run_bot(r, forbidden_subreddits, forbidden_users, np_subreddits, mentions_replied_to, mentions_unable_to_reply)
    except:
        logging.warning("Couldn't log in or find the necessary files! Waiting 120 seconds")
        sleep(120)
