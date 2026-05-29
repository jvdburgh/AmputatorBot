import sys
import traceback
from typing import Optional

import praw
from praw import Reddit
from praw.exceptions import RedditAPIException

from helpers import logger
from static import static

log = logger.get_log(sys)


def get_praw_session() -> Optional[Reddit]:
    # Try to get a Reddit session using praw
    try:
        log.info("Logging in..")
        session = praw.Reddit(username=static.USERNAME,
                              password=static.PASSWORD,
                              client_id=static.CLIENT_ID,
                              client_secret=static.CLIENT_SECRET,
                              user_agent=static.USER_AGENT)
        log.info("Successfully logged in!")

        return session

    # If no session could be retrieved, return None
    except (RedditAPIException, Exception):
        log.warning("Couldn't log in!")
        log.error(traceback.format_exc())
        return None
