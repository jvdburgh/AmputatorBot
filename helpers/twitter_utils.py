import sys

from helpers import logger
from helpers.utils import check_if_cached

log = logger.get_log(sys)


# Check if the tweet is a retweet (either official or non-official)
def check_if_retweet(tweet, body):
    if hasattr(tweet, 'retweeted_status') or "RT @" in body:
        return True

    return False


# Get all URLs of the tweet that are cached
def get_cached_tweet_urls(tweet):
    try:
        urls = tweet.entities['urls']
        expanded_urls = []
        for url in urls:
            expended_url = url['expanded_url']
            if check_if_cached(expended_url):
                expanded_urls.append(expended_url)
        return expanded_urls
    except AttributeError:
        return None


# Get the screen_name of the author of the tweet
def get_twitterer_name(tweet):
    try:
        return tweet.author.screen_name
    except AttributeError:
        log.info(f"Couldn't get user screen_name of tweet {tweet.id}")
        return None


# Generate a reply tweet
def generate_tweet(links):
    # Initialize all variables
    alt_link, canonical_text_latest, canonical_text_list, canonical_text = "", "", "", ""
    n_amp_urls, n_canonicals = 0, 0

    # Loop through all links, and generate the canonical link part of the comment
    for link in links:
        if link.is_amp:
            if link.canonical:
                n_canonicals += 1
                canonical_text_latest += link.canonical
                canonical_text_list += f"[{n_canonicals}] {link.canonical}\n"

    if n_canonicals >= 1:
        if n_canonicals == 1:
            intro_who_wat = "It looks like you tweeted a cached AMP link. "
            canonical_text = canonical_text_latest
            intro_maybe = "You might want to visit the non-cached page instead: "

        else:
            intro_who_wat = "It looks like you tweeted some cached AMP links. "
            canonical_text = canonical_text_list
            intro_maybe = "You might want to visit the non-cached pages instead: "

        tweet_text = intro_who_wat + intro_maybe + canonical_text

        return tweet_text

    log.warning("Couldn't generate reply")
    return None
