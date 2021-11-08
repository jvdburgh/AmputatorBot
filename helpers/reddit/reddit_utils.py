import sys
import traceback

from praw.exceptions import RedditAPIException
from prawcore import Forbidden, NotFound

from helpers import logger

log = logger.get_log(sys)


# Get the body of the submission
def get_submission_body(submission) -> str:
    if submission.selftext != "":
        return submission.selftext
    else:
        return submission.url


# Check if AmputatorBot is banned in subreddit X
def check_if_banned(subreddit) -> bool:
    try:
        if subreddit.user_is_banned:
            log.info(f"Ban by r/{subreddit} has been validated")
            return True

        else:
            log.warning("Couldn't validate ban, user isn't banned!")
            return False

    except (Forbidden, NotFound, RedditAPIException, Exception):
        log.error(traceback.format_exc())
        log.warning("Couldn't validate ban, an error was raised!")
        return False
