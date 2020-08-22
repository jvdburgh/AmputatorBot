import sys

from helpers import logger
from helpers.twitter_utils import check_if_retweet
from helpers.utils import check_if_amp, check_if_cached
from models.resultcode import ResultCode

log = logger.get_log(sys)


# Check if the tweet meets the specified criteria
def check_tweet_criteria(item, cached_urls=None, tweet=None, data=None, history_failed=None, history_success=None,
                         return_if_false=True, mustBeAMP=False, mustNotBeRetweet=False, mustBeCached=False,
                         mustBeNew=False, mustNotHaveFailed=False, mustNotBeMine=False, mustNotBeOptedOut=False):
    # Must contain AMP
    if mustBeAMP:
        if not check_if_amp(", ".join(cached_urls)):
            result_code = ResultCode.ERROR_NO_AMP
            if return_if_false:
                return False, result_code

    # Must not be retweeted
    if mustNotBeRetweet:
        if check_if_retweet(tweet, item.body):
            result_code = ResultCode.TWITTER_ERROR_IS_RETWEET
            if return_if_false:
                return False, result_code

    # Must not be cached
    if mustBeCached:
        if check_if_cached(item.body):
            result_code = ResultCode.TWITTER_ERROR_IS_NOT_CACHED
            if return_if_false:
                return False, result_code

    # Must be new
    if mustBeNew:
        if item.id in history_success:
            result_code = ResultCode.ERROR_OTHER
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
        if item.author == "AmputatorBot":
            result_code = ResultCode.ERROR_OTHER
            if return_if_false:
                return False, result_code

    # Must not be posted by a user who opted out
    if mustNotBeOptedOut:
        if item.author.casefold() in list(user.casefold() for user in data.disallowed_twitterers):
            result_code = ResultCode.ERROR_USER_OPTED_OUT
            return False, result_code

    return True, ResultCode.MEETS_CRITERIA
