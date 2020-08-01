import re
import sys
import traceback
from urllib.parse import urlparse

from helpers import logger, utils
from models.link import CanonicalType

log = logger.get_log(sys)


# Try to find the canonical url by scanning for the specified tag
def get_canonical_with_soup(r, url, method, guess_and_check=False):
    log.debug(f"Trying {method.value}")

    can_urls = None
    # Find the canonical urls with method rel
    if method == CanonicalType.REL:
        can_urls = get_can_urls(r.soup.find_all(rel='canonical'), 'href', url=url)

    # Find the canonical urls with method amp-canurl
    elif method == CanonicalType.CANURL:
        can_urls = get_can_urls(r.soup.find_all(a='amp-canurl'), 'href', url=url)

    # Find the canonical urls with method og_url
    elif method == CanonicalType.OG_URL:
        can_urls = get_can_urls(r.soup.find_all('meta', property='og:url'), 'content', url=url)

    # Find the canonical urls with method google manual_redirect
    elif method == CanonicalType.GOOGLE_MANUAL_REDIRECT:
        if 'url?q=' in r.current_url and 'www.google.' in r.current_url:
            can_urls = get_can_urls(r.soup.find_all('a'), 'href', url=url)

    # Find the canonical urls with method google js redirect_url
    elif method == CanonicalType.GOOGLE_JS_REDIRECT:
        if 'url?' in r.current_url and 'www.google.' in r.current_url:
            js_redirect_url = get_can_url_with_regex(r.soup, re.compile("var\\s?redirectUrl\\s?=\\s?([\'|\"])(.*?)\\1"))
            if js_redirect_url:
                can_urls = [js_redirect_url]

    # Find the canonical urls with method bing cururl
    elif method == CanonicalType.BING_ORIGINAL_URL:
        if '/amp/s/' in r.current_url and 'www.bing.' in r.current_url:
            cururl = get_can_url_with_regex(r.soup, re.compile("([\'|\"])originalUrl\\1\\s?:\\s?\\1(.*?)\\1"))
            if cururl:
                can_urls = [cururl]

    # Find the canonical urls with method schema_mainentity
    elif method == CanonicalType.SCHEMA_MAINENTITY:
        main_entity = get_can_url_with_regex(r.soup, re.compile("\"mainEntityOfPage\"\\s?:\\s?([\'|\"])(.*?)\\1"))
        if main_entity:
            can_urls = [main_entity]

    # Find the canonical urls with method page tco_pagetitle
    elif method == CanonicalType.TCO_PAGETITLE:
        if 'https://t.co' in r.current_url and 'amp=1' in r.current_url:
            pagetitle = r.title
            if pagetitle:
                can_urls = [pagetitle]

    elif method == CanonicalType.GUESS_AND_CHECK:
        if guess_and_check:
            guessed_and_checked_url = get_can_url_with_guess_and_check(url)
            if guessed_and_checked_url:
                can_urls = [guessed_and_checked_url]
        else:
            log.debug("Guess and check is disabled")

    # Detect unknown canonical methods
    else:
        log.error(traceback.format_exc())
        log.warning("Unknown canonical method type!")
        return None, None

    if can_urls:
        # Loop through every can_url found
        for can_url in can_urls:
            # If the canonical is the same as before, it's a false positive
            if can_url != url:
                # Return the canonical and if it is still AMP or not
                still_amp = utils.check_if_amp(can_url)
                if still_amp:
                    log.warning(f"AMP canonical found with {method.value}: {can_url}")
                    return can_url, True
                else:
                    log.info(f"Canonical found with {method.value}: {can_url}")
                    return can_url, False

            else:
                log.warning(f"False positive!: {can_url}")

    return None, False


def get_can_url_with_regex(soup, regex):
    scripts = soup.find_all("script", {"src": False})
    if scripts:
        for script in scripts:
            if script.string is not None:
                result = regex.search(script.string)
                if result:
                    url = result.group(2)
                    url = re.sub('\\\/', '/', url)
                    return url

    return None


def get_can_url_with_guess_and_check(url):
    amp_keywords = ["/amp", "amp/", ".amp", "amp.", "?amp", "amp?", "=amp", "amp=",
                    "&amp", "amp&", "%amp", "amp%", "_amp", "amp_"]
    for keyword in amp_keywords:
        if keyword in url:
            guessed_url = url.replace(keyword, "")
            if utils.check_if_valid_url(guessed_url):
                guessed_page = utils.get_page(guessed_url)
                if guessed_page:
                    if guessed_page.status_code == 200:
                        article_similarity = utils.get_article_similarity(url, guessed_url)
                        log.info(f"Article similarity: {article_similarity}")
                        if article_similarity > 0.6:
                            log.info(f"{url} and {guessed_url} are probably the same article")
                            return guessed_url
                        elif article_similarity > 0.35:
                            found_urls = get_can_urls(guessed_page.soup.find_all(rel='amphtml'), 'href', url=url)
                            if found_urls:
                                if found_urls[0] == url:
                                    log.info(f"{url} and {guessed_url} are probably the same article")
                                    return guessed_url

    return None


def get_can_urls(tags, target_value, url=None):
    can_values = []
    for can_tag in tags:
        value = can_tag.get(target_value)
        if value.startswith("//"):
            parsed_uri = urlparse(url)
            value = f"{parsed_uri.scheme}:{value}"
        elif value.startswith("/"):
            parsed_uri = urlparse(url)
            value = f"{parsed_uri.scheme}://{parsed_uri.netloc}{value}"
        can_values.append(value)
    return can_values
