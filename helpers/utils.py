import sys
import traceback
from difflib import SequenceMatcher
from random import choice

import requests
import validators
from bs4 import BeautifulSoup
from newspaper import ArticleException, Article
from praw.exceptions import RedditAPIException
from prawcore import Forbidden, NotFound
from tldextract import tldextract
from urlextract import URLExtract
from validators import ValidationFailure

from helpers import logger
from helpers.canonical_methods import get_canonical_with_soup
from models.link import Link, Canonical, CanonicalType
from models.page import Page
from static import static

log = logger.get_log(sys)


# Get the body of the submission
def get_submission_body(submission):
    if submission.selftext != "":
        return submission.selftext
    else:
        return submission.url


# Get all the URLS from the body
def get_urls(body):
    # Set up an extractor to extract the URLS
    extractor = URLExtract()
    # Update the TLD list if it is older than x days
    extractor.update_when_older(7)
    # Run the extractor and remove any duplicates
    urls = extractor.find_urls(body, only_unique=True)

    return urls


# Loop through all the URLs, run get_url_info for each url, append Link instance to links
def get_urls_info(urls, guess_and_check=False):
    links = []
    for url in urls:
        link = get_url_info(url, guess_and_check)
        links.append(link)

    # Return the links
    return links


# Get and save all the info on the URLS as a Link instance, including the original and canonical URL and more
def get_url_info(url, guess_and_check):
    # Create a new Link instance
    link = Link()
    # Save the extracted URL
    link.url = url
    # Remove markdown and other artifacts from the URL
    link.url_clean = remove_markdown(url)
    # Check if the clean URL is valid, if so continue with the next steps
    link.url_clean_is_valid = check_if_valid_url(link.url_clean)
    if link.url_clean_is_valid:
        link.is_amp = check_if_amp(link.url_clean)
        if link.is_amp:
            link.is_cached = check_if_cached(link.url_clean)
            link.domain = tldextract.extract(link.url_clean).domain
            link = get_canonical(link, guess_and_check)

    return link


# Remove markdown and other artifacts from the URL
def remove_markdown(url):
    markdown_chars = ("?", "(", ")", "[", "]", "\\", ",", ".", "\"", "”",
                      "`", "^", "*", "|", ">", "<", "{", "}", "~", ":", ";")
    while url.endswith(markdown_chars):
        for char in markdown_chars:
            if url.endswith(char):
                url = url[:-len(char)]
    return url


# Check if URL is valid
def check_if_valid_url(url):
    try:
        return validators.url(url)
    except (ValidationFailure, Exception):
        log.error(traceback.format_exc())
        log.warning("Failed to validate URL")
        return False


# Check if the string contains an AMP link
def check_if_amp(string):
    # Make string lowercase
    string = string.lower()

    # Detect if the string contains links
    protocols = ["https://", "http://"]
    if not any(map(string.__contains__, protocols)):
        return False

    # Detect if the string contains common AMP keywords
    amp_keywords = ["/amp", "amp/", ".amp", "amp.", "?amp", "amp?", "=amp", "amp=",
                    "&amp", "amp&", "%amp", "amp%", "_amp", "amp_"]
    return any(map(string.__contains__, amp_keywords))


# Check if the page is hosted by Google, Ampproject or Bing
def check_if_cached(url):
    # Make string lowercase
    url = url.lower()

    # Check if the page is hosted by Google, Ampproject or Bing
    return ("/amp/" in url and "www.google." in url) or ("ampproject.net" in url or "ampproject.org" in url) \
           or ("/amp/" in url and "www.bing." in url)


# Get the canonical of the URL
def get_canonical(link, guess_and_check=False):
    next_url = link.url_clean
    depth = 0

    while depth < static.MAX_DEPTH:
        log.info(f"\nNEXT UP = {next_url}")
        # Get the HTML content of the URL
        response = get_page(next_url)
        if not response:
            log.warning("Invalid response, the page could not be fetched!")
            return link

        # Try every soup method
        for method in CanonicalType:
            # Create a new canonical instance of the specified method
            canonical = Canonical(CanonicalType=method)
            # Assign the found url and note if it is still amp
            canonical.url, canonical.is_amp = get_canonical_with_soup(response, next_url, method, guess_and_check)

            # If a canonical url was returned, check if it is valid
            if canonical.url:
                canonical.is_valid = check_if_valid_url(canonical.url)
                # If it is valid, calculate the similarity ratio and append it to the link instance
                if canonical.is_valid:
                    canonical.url_similarity = SequenceMatcher(None, canonical.url, link.url_clean).ratio()
                    canonical.domain = tldextract.extract(canonical.url).domain
                    link.canonicals.append(canonical)
                    # Don't guess if there is already one that works, since guess_and_check is heavy and unreliable
                    guess_and_check = False
                else:
                    log.warning("Found url is not valid")

        # Return 'empty' link if no canonicals were found
        if len(link.canonicals) == 0:
            log.warning(f"No canonicals found")
            return link

        # Sort the canonicals based on their similarity score
        link.canonicals.sort(key=lambda c: c.url_similarity, reverse=True)

        # Filter out the canonicals that are still amp
        link.canonicals_solved = [c for c in link.canonicals if c.is_amp is False]

        # If there are one or more solved canonicals, pick the best one and assign it to the canonical attribute
        if len(link.canonicals_solved) > 0:
            log.info(f"There are solved canonicals ({len(link.canonicals_solved)})")
            best_canonical = link.canonicals_solved[0]
            log.info(f"Best canonical: {best_canonical.url}")
            if len(link.canonicals_solved) > 1:
                canonical_alt = next((c for c in link.canonicals_solved if c.domain != best_canonical.domain), None)
                if canonical_alt is not None:
                    log.info(f"Alternative canonical: {canonical_alt.url}")
                    link.canonical_alt = canonical_alt.url
                    link.canonical_alt_domain = canonical_alt.domain
            link.canonical = best_canonical.url
            return link

        # If there a no solved canonicals, return no canonical, the amp canonical or run again
        else:
            # If the found URL is the same as the one before, return no canonical or the amp canonical
            if next_url == link.canonicals[0].url:
                log.warning("Didn't find a new canonical!")
                if len(link.canonicals) > 0:
                    amp_canonical = link.canonicals[0].url
                    log.info(f"Could find a canonical, but it is still amp: {amp_canonical}")
                    link.amp_canonical = amp_canonical
                return link
            # If the found URL is different from before, but still amp, run again with the found URL
            else:
                log.info("Canonical url found, but still amp! Running again\n")
                next_url = link.canonicals[0].url

        depth += 1

    # If there are no solved canonicals and we're out of depth, return the amp canonical or no canonical
    if len(link.canonicals_solved) == 0:
        if len(link.canonicals) > 0:
            amp_canonical = link.canonicals[0].url
            log.info(f"Could find a canonical, but it is still amp: {amp_canonical}")
            link.amp_canonical = amp_canonical

    log.warning("\nMax depth reached, no canonicals found!")
    return link


# Make a request to the page with randomized headers, make a soup, save the current url, page title and status code
def get_page(url):
    try:
        # Fetch amp page
        log.info(f"Started fetching..")
        req = requests.get(url, headers=random_headers())
        current_url = req.url

        # Make the received data searchable
        soup = BeautifulSoup(req.text, features="html.parser")
        title = soup.title.string if soup.title else "Error: Title not found"
        log.info(f"Made a soup of {current_url} ({req.status_code})")
        log.info(f"Page title = {title}")
        page = Page(current_url, req.status_code, title, soup)
        return page

    # If the submitted page couldn't be fetched, throw an exception
    except (ConnectionError, Exception, AttributeError):
        log.warning(traceback.format_exc())
        log.warning("the page could not be fetched")
        return None


# Get randomized user agent, set default accept and request English page
# This is done to prevent 403 errors.
def random_headers():
    return {
        'User-Agent': choice(static.HEADERS),
        'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8',
        'Accept-Language': 'en-US'
    }


# return True if the file is connected to a terminal device
def check_if_ide():
    try:
        return "venv" in sys.executable
    except (SyntaxError, Exception):
        return False


# Check how similar the article text is
def get_article_similarity(url1, url2, log_articles=False):
    try:
        # Download and parse first article
        article1 = Article(url1, browser_user_agent=choice(static.HEADERS))
        article1.download()
        article1.parse()
        article1_text = article1.text

        # Download and parse second article
        article2 = Article(url2, browser_user_agent=choice(static.HEADERS))
        article2.download()
        article2.parse()
        article2_text = article2.text

        if log_articles:
            log.debug(f"Article 1: {article1_text}\n\nArticle 2: {article2_text}")

        # Compare the two articles and return the ratio (0-1)
        return SequenceMatcher(None, article1_text, article2_text).ratio()

    except (ArticleException, Exception):
        log.error(traceback.format_exc())
        log.warning("Couldn't compare articles")
        return None


# Check if AmputatorBot is banned in subreddit X
def check_if_banned(subreddit):
    try:
        if subreddit.user_is_banned:
            log.info(f"Ban by {subreddit} has been validated")
            return True

        else:
            log.warning("Couldn't validate ban, user isn't banned!")
            return False

    except (Forbidden, NotFound, RedditAPIException, Exception):
        log.error(traceback.format_exc())
        log.warning("Couldn't validate ban, an error was raised!")
        return False
