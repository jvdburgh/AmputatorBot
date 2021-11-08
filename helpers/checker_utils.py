import sys
import traceback

import validators
from validators import ValidationFailure

from helpers import logger
from static import static

log = logger.get_log(sys)


# Check if URL is valid
def check_if_valid_url(url) -> bool:
    try:
        return validators.url(url)
    except (ValidationFailure, Exception):
        log.error(traceback.format_exc())
        log.warning("Failed to validate URL")
        return False


# Check if the string contains an AMP link
def check_if_amp(string) -> bool:
    # Make string lowercase
    string = string.lower()

    # Detect if the string contains links
    protocol_keywords = static.PROTOCOL_KEYWORDS
    if not any(map(string.__contains__, protocol_keywords)):
        return False

    # Detect if the string contains common AMP keywords
    amp_keywords = static.AMP_KEYWORDS

    return any(map(string.__contains__, amp_keywords))


# Check if the page is hosted by Google, Ampproject or Bing
def check_if_cached(url) -> bool:
    # Make string lowercase
    url = url.lower()

    # Check if the page is hosted by Google, Ampproject or Bing
    return ("/amp/" in url and "www.google." in url) or ("ampproject.net" in url or "ampproject.org" in url) or \
           ("/amp/" in url and "www.bing." in url)
