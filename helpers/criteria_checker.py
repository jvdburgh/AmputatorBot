import sys
import traceback

from praw.exceptions import RedditAPIException
from prawcore import Forbidden

from helpers import logger
from helpers.utils import check_if_amp
from models.resultcode import ResultCode
from static import static

log = logger.get_log(sys)


# Check if the item meets the specified criteria
def check_criteria(item, data=None, history_failed=None, history_success=None, return_if_false=True,
                   mustBeAMP=False, mustBeNew=False, mustNotBeDisallowedSubreddit=False, mustNotHaveFailed=False,
                   mustNotBeMine=False, mustNotBeOptedOut=False, mustNotHaveDisallowedMods=False):
    # Must contain AMP
    if mustBeAMP:
        if not check_if_amp(item.body):
            result_code = ResultCode.ERROR_NO_AMP
            if return_if_false:
                return False, result_code

    # Must be new
    if mustBeNew:
        if item.id in history_success:
            result_code = ResultCode.ERROR_OTHER
            if return_if_false:
                return False, result_code

    # Must not be in a disallowed subreddit
    if mustNotBeDisallowedSubreddit:
        if item.subreddit.display_name.casefold() in list(sub.casefold() for sub in data.disallowed_subreddits):
            result_code = ResultCode.ERROR_DISALLOWED_SUBREDDIT
            if return_if_false:
                return False, result_code

    # Must not have failed before
    if mustNotHaveFailed:
        if item.id in history_failed:
            result_code = ResultCode.ERROR_OTHER
            if return_if_false:
                return False, result_code

    # Must not be posted by AmputatorBot
    if mustNotBeMine:
        if item.author == data.praw_session.redditor(static.USERNAME):
            result_code = ResultCode.ERROR_OTHER
            if return_if_false:
                return False, result_code

    # Must not be posted by a user who opted out
    if mustNotBeOptedOut:
        if item.author.name.casefold() in list(user.casefold() for user in data.disallowed_users):
            result_code = ResultCode.ERROR_USER_OPTED_OUT
            return False, result_code

    # Must not have mods that are disallowed
    if mustNotHaveDisallowedMods:
        try:
            # Make the list case-insensitive
            disallowed_mods = list(disallowed_mod.casefold() for disallowed_mod in data.disallowed_mods)
            subreddit_mods = list(subreddit_mod.name.casefold() for subreddit_mod in item.subreddit.moderator())

            # Loop through the mods of the subreddit, check if one is a disallowed mod
            if any(subreddit_mod in disallowed_mods for subreddit_mod in subreddit_mods):
                log.info(f"r/{item.subreddit} has a disallowed_mod")

                # Make the list case-insensitive and check if the subreddit is in the allowed_subreddits list
                allowed_subreddits = list(allowed_sub.casefold() for allowed_sub in data.allowed_subreddits)
                if item.subreddit.display_name.casefold() not in allowed_subreddits:
                    log.info(f"r/{item.subreddit} is not included in allowed_subreddits")
                    result_code = ResultCode.ERROR_DISALLOWED_MOD
                    return False, result_code
                else:
                    log.info(f"r/{item.subreddit} is included in allowed_subreddits, ignoring disallowed_mod")

        # Catch exceptions
        except (RedditAPIException, Forbidden, Exception):
            log.warning(traceback.format_exc())
            log.warning(f"Couldn't check moderators of r/{item.subreddit}")
            result_code = ResultCode.ERROR_DISALLOWED_SUBREDDIT
            return False, result_code

    return True, ResultCode.MEETS_CRITERIA
