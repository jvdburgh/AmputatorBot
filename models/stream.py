import sys
import traceback

from datahandlers.local_datahandler import get_data_by_filename as get_data
from helpers import logger, login
from models.type import Type

log = logger.get_log(sys)


# Return an instance that correspondents with the requested type
def get_stream(type):
    log.info(f"Setting up stream of type {type.value}")
    if type == Type.COMMENT:
        return Comment(type)
    elif type == Type.SUBMISSION:
        return Submission(type)
    elif type == Type.MENTION:
        return Mention(type)
    elif type == Type.TWEET:
        return Tweet(type)
    else:
        log.error(traceback.format_exc())
        log.warning("Unknown stream type!")
        return None


class Stream:
    def __init__(self, type):
        # Basic items
        self.type = type
        self.allowed_subreddits = get_data("allowed_subreddits")
        self.contributor_subreddits = get_data("contributor_subreddits")
        self.disallowed_subreddits = get_data("disallowed_subreddits")
        self.disallowed_users = get_data("disallowed_users")
        self.np_subreddits = get_data("np_subreddits")
        self.praw_session = login.get_praw_session()


class Comment(Stream):
    def __init__(self, type):
        super().__init__(type)
        # Items specific to the comment stream
        self.comments_success = get_data("comments_success")
        self.comments_failed = get_data("comments_failed")


class Submission(Stream):
    def __init__(self, type):
        super().__init__(type)
        # Items specific to the submission stream
        self.submissions_success = get_data("submissions_success")
        self.submissions_failed = get_data("submissions_failed")


class Mention(Stream):
    def __init__(self, type):
        super().__init__(type)
        # Items specific to the mention stream
        self.disallowed_mods = get_data("disallowed_mods")
        self.mentions_success = get_data("mentions_success")
        self.mentions_failed = get_data("mentions_failed")
        self.problematic_domains = get_data("problematic_domains")


class Tweet:
    def __init__(self, type):
        # Items specific to the tweet stream
        self.type = type
        self.tweets_success = get_data("tweets_success")
        self.tweets_failed = get_data("tweets_failed")
        self.disallowed_twitterers = get_data("disallowed_twitterers")
