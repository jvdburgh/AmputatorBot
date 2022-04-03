import sys
import traceback
from random import choice
from typing import Optional, List, Dict

import requests
from bs4 import BeautifulSoup
from requests import Timeout
from tldextract import tldextract
from urlextract import URLExtract

from helpers import logger
from helpers.canonical_methods import get_canonical_with_soup
from helpers.checker_utils import check_if_valid_url, check_if_cached, check_if_amp
from models.link import Link, CanonicalType, Canonical
from models.page import Page
from models.urlmeta import UrlMeta
from static import static

log = logger.get_log(sys)


# Get all the URLs from the body
def get_urls(body) -> List[str]:
    # Set up an extractor to extract the URLs
    extractor = URLExtract()
    # Update the TLD list if it is older than x days
    extractor.update_when_older(7)
    # Run the extractor and remove any duplicates
    urls = extractor.find_urls(body, only_unique=True)

    return urls


# Loop through all the URLs, run get_url_info for each url, append Link instance to links
def get_urls_info(urls, use_gac=False, max_depth=static.MAX_DEPTH) -> [Link]:
    links = []
    for url in urls:
        link = get_url_info(url, use_gac, max_depth)
        links.append(link)

    # Return the links
    return links


# Get and save all the info on the URLs as a Link instance
def get_url_info(url, use_gac, max_depth) -> Link:
    link = Link(canonicals=[])

    origin = UrlMeta(url=remove_markdown(url))
    origin.is_valid = check_if_valid_url(origin.url)
    origin.is_amp = check_if_amp(origin.url)

    if origin.is_valid:
        if origin.is_amp:
            origin.is_cached = check_if_cached(origin.url)
            origin.domain = tldextract.extract(origin.url).domain
            link.origin = origin
            link = get_canonicals(
                link=link,
                max_depth=max_depth,
                use_gac=use_gac
            )

    return link


# Remove markdown and other artifacts from the URL
def remove_markdown(url) -> str:
    markdown_chars = ("?", "(", ")", "[", "]", "\\", ",", ".", "\"", "”",
                      "`", "^", "*", "|", ">", "<", "{", "}", "~", ":", ";")
    while url.endswith(markdown_chars):
        for char in markdown_chars:
            if url.endswith(char):
                url = url[:-len(char)]
    return url


# Get the canonical of the URL
def get_canonicals(link: Link, max_depth=static.MAX_DEPTH, use_db=True, use_gac=True, use_mr=True) -> Link:
    next_url = link.origin.url
    depth = 0

    while depth < max_depth:
        log.info(f"\nNEXT UP = {next_url}")

        # Get the HTML content of the URL
        response = get_page(next_url)
        if not response:
            log.warning("Invalid response, the page could not be fetched!")
            return link

        # Try every soup method
        for method in CanonicalType:
            # Create a new canonical instance of the specified method
            canonical = Canonical(type=method)

            # Assign the found url and note if it is still amp
            canonical = get_canonical_with_soup(
                r=response,
                url=next_url,
                meta=canonical,
                original_url=link.origin.url,
                use_db=use_db,
                use_gac=use_gac,
                use_mr=use_mr,
            )

            if canonical and canonical.is_valid:
                link.canonicals.append(canonical)

                # Disable resource-heavy and untrustworthy methods if a canonical without AMP was found
                if not canonical.is_amp:
                    use_db = False
                    use_gac = False
                    use_mr = False

        # Return 'empty' link if no canonicals were found
        if len(link.canonicals) == 0:
            log.warning(f"No canonicals found")
            return link

        # Sort the canonicals based on their url similarity score
        link.canonicals.sort(key=lambda c: c.url_similarity, reverse=True)

        # Filter out the canonicals that are still amp
        canonicals_solved = [c for c in link.canonicals if c.is_amp is False]

        # If there are one or more solved canonicals, pick the best one and assign it to the canonical attribute
        if len(canonicals_solved) > 0:
            log.info(f"Solved canonicals: {len(canonicals_solved)}")
            link.canonical = canonicals_solved[0]
            log.info(f"Best canonical: {link.canonical.url}")

            # Assign alternative canonicals if solved canonicals have different domains
            if len(canonicals_solved) > 1:
                c_alt: Canonical = next((c for c in canonicals_solved if c.domain != link.canonical.domain), None)

                if c_alt is not None:
                    for c in link.canonicals:
                        if c.domain == c_alt.domain and c.url_similarity == c_alt.url_similarity:
                            c.is_alt = True
                            log.info(f"Alternative canonical: {c_alt.url}")

            return link

        # If there a no solved canonicals, return no canonical, the amp canonical or run again
        else:
            # If the found URL is the same as the one before, return no canonical or the amp canonical
            if next_url == link.canonicals[0].url:
                log.warning("Didn't find a new canonical!")
                if len(link.canonicals) > 0:
                    amp_canonical = link.canonicals[0]
                    log.info(f"Found a canonical, but it's still AMP: {amp_canonical.url}")
                    if link.origin.is_cached:
                        log.info("No longer cached, using it")
                        link.amp_canonical = amp_canonical
                return link
            # If the found URL is different from before, but still amp, run again with the found URL
            else:
                log.info("Canonical found, but still AMP! Running again\n")
                next_url = link.canonicals[0].url

        depth += 1

    # If there are no solved canonicals and depth has maxed out, return the amp canonical or no canonical
    if len([c for c in link.canonicals if c.is_amp is False]) == 0:
        if len(link.canonicals) > 0:
            amp_canonical = link.canonicals[0]
            log.info(f"Canonical found, but still AMP: {amp_canonical}")
            if link.origin.is_cached:
                log.info("No longer cached, using it")
                link.amp_canonical = amp_canonical

    log.warning("\nMax depth reached, no canonicals found!")
    return link


# Make a request to the page with randomized headers, make a soup, save the current url, page title and status code
def get_page(url) -> Optional[Page]:
    try:
        log.info(f"Started fetching..")
        req = requests.get(url, headers=get_randomized_headers(), timeout=15)

        # Make the received data searchable
        soup = BeautifulSoup(req.text, features="html.parser")
        log.info(f"Souped {req.url} ({req.status_code})")
        title = soup.title.string if soup.title and soup.title.string else "Error: Title not found"
        log.info(f"Title: {title}")
        page = Page(req.url, req.status_code, title, soup)
        return page

    # If the submitted page couldn't be fetched, throw an exception
    except (ConnectionError, Timeout, AttributeError, Exception):
        log.debug(traceback.format_exc())
        log.warning(f"Unable to fetch {url}")
        return None


# Get randomized user agent, set default accept and request English page
# This is done to prevent 403 errors.
def get_randomized_headers() -> Dict[str, str]:
    return {
        'User-Agent': choice(static.HEADERS),
        'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8',
        'Accept-Language': 'en-US'
    }


# return True if the file is connected to a terminal device
def check_if_ide() -> bool:
    try:
        return "venv" in sys.executable
    except (SyntaxError, Exception):
        return False
