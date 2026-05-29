import sys

from helpers import logger
from helpers.reddit.reddit_utils import check_if_banned
from helpers.utils import check_if_amp
from models.resultcode import ResultCode
from static import static

log = logger.get_log(sys)


# Check if the item meets the specified criteria
def check_criteria(item, data=None, history_err=None, history_ok=None, return_if_false=True,
                   mustBeAMP=False, mustBeNew=False, mustNotBeBannedInSubreddit=False, mustNotHaveFailed=False,
                   mustNotBeMine=False, mustNotBeOptedOut=False):
    # Must contain AMP
    if mustBeAMP:
        if not check_if_amp(item.body):
            result_code = ResultCode.ERROR_NO_AMP
            if return_if_false:
                return False, result_code

    # Must be new
    if mustBeNew:
        if item.id in history_ok:
            result_code = ResultCode.ERROR_OTHER
            if return_if_false:
                return False, result_code

    # Must not be banned in the subreddit
    if mustNotBeBannedInSubreddit:
        if check_if_banned(item.subreddit, keepLog=False):
            result_code = ResultCode.ERROR_BANNED
            if return_if_false:
                return False, result_code

    # Must not have failed before
    if mustNotHaveFailed:
        if item.id in history_err:
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

    return True, ResultCode.MEETS_CRITERIA
