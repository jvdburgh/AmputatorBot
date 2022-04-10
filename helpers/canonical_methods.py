import re
import sys
import traceback
from difflib import SequenceMatcher
from typing import Optional
from urllib.parse import urlparse

from tldextract import tldextract

import datahandlers.remote_datahandler
from helpers import logger, utils
from helpers.article_comparer import get_article_similarity
from helpers.checker_utils import check_if_amp, check_if_cached, check_if_valid_url
from models.link import CanonicalType, Canonical
from static import static

log = logger.get_log(sys)


# Try to find the canonical url by scanning for the specified tag
def get_canonical_with_soup(r, url, meta: Canonical, original_url,
                            use_db=False, use_gac=False, use_mr=False) -> Optional[Canonical]:

    log.debug(f"Trying {meta.type.value}")

    can_urls = None

    # Find canonical urls with method rel
    if meta.type == CanonicalType.REL:
        can_urls = get_can_urls_by_tags(r.soup.find_all(rel='canonical'), 'href', url=url)

    # Find canonical urls with method amp-canurl
    elif meta.type == CanonicalType.CANURL:
        can_urls = get_can_urls_by_tags(r.soup.find_all(a='amp-canurl'), 'href', url=url)

    # Find canonical urls with method og_url
    elif meta.type == CanonicalType.OG_URL:
        can_urls = get_can_urls_by_tags(r.soup.find_all('meta', property='og:url'), 'content', url=url)

    # Find canonical urls with method google manual_redirect
    elif meta.type == CanonicalType.GOOGLE_MANUAL_REDIRECT:
        if 'url?q=' in r.current_url and 'www.google.' in r.current_url:
            can_urls = get_can_urls_by_tags(r.soup.find_all('a'), 'href', url=url)

    # Find canonical urls with method google js redirect_url
    elif meta.type == CanonicalType.GOOGLE_JS_REDIRECT:
        if 'url?' in r.current_url and 'www.google.' in r.current_url:
            js_redirect_url = get_can_url_with_regex(r.soup, re.compile("var\\s?redirectUrl\\s?=\\s?([\'|\"])(.*?)\\1"))
            if js_redirect_url:
                can_urls = [js_redirect_url]

    # Find canonical urls with method bing cururl
    elif meta.type == CanonicalType.BING_ORIGINAL_URL:
        if '/amp/s/' in r.current_url and 'www.bing.' in r.current_url:
            cururl = get_can_url_with_regex(r.soup, re.compile("([\'|\"])originalUrl\\1\\s?:\\s?\\1(.*?)\\1"))
            if cururl:
                can_urls = [cururl]

    # Find canonical urls with method schema_mainentity
    elif meta.type == CanonicalType.SCHEMA_MAINENTITY:
        main_entity = get_can_url_with_regex(r.soup, re.compile("\"mainEntityOfPage\"\\s?:\\s?([\'|\"])(.*?)\\1"))
        if main_entity:
            can_urls = [main_entity]

    # Find canonical urls with method page tco_pagetitle
    elif meta.type == CanonicalType.TCO_PAGETITLE:
        if 'https://t.co' in r.current_url and 'amp=1' in r.current_url:
            pagetitle = r.title
            if pagetitle:
                can_urls = [pagetitle]

    # Find canonical urls with method meta http-equiv redirect
    elif meta.type == CanonicalType.META_REDIRECT:
        if use_mr:
            can_urls = get_can_urls_with_meta_redirect(r.soup.find_all('meta'), url=url)
        else:
            log.debug("use_mr is disabled")

    # Find canonical urls by 'guessing' a canonical link and checking the article content for similarity
    elif meta.type == CanonicalType.GUESS_AND_CHECK:
        if use_gac:
            guessed_and_checked_url = get_can_url_with_guess_and_check(url)
            if guessed_and_checked_url:
                can_urls = [guessed_and_checked_url]
        else:
            log.debug("use_gac is disabled")

    # Find canonical urls by checking the database
    elif meta.type == CanonicalType.DATABASE:
        if use_db:
            entry = datahandlers.remote_datahandler.get_entry_by_original_url(url)
            if entry and entry.canonical_url:
                can_urls = [entry.canonical_url]
        else:
            log.debug("use_db is disabled")

    # Catch unknown canonical methods
    else:
        log.error(traceback.format_exc())
        log.warning("Unknown canonical method type!")
        return None

    if can_urls:
        # Loop through every can_url found
        for can_url in can_urls:
            # If the canonical is the same as before, it's a false positive
            if can_url != url:
                # Return the canonical and if it is still AMP or not
                meta.is_valid = check_if_valid_url(url)
                if meta.is_valid:
                    meta.url = can_url
                    meta.url_similarity = SequenceMatcher(None, meta.url, original_url).ratio()
                    meta.domain = tldextract.extract(meta.url).domain
                    meta.is_amp = check_if_amp(meta.url)
                    if meta.is_amp:
                        log.warning(f"AMP canonical found with {meta.type.value}: {meta.url}")
                        meta.is_cached = check_if_cached(meta.url)
                    else:
                        log.info(f"Canonical found with {meta.type.value}: {meta.url}")
                    return meta

                else:
                    log.warning("Found url is invalid")

            else:
                log.warning(f"False positive!: {can_url}")

    return None


def get_can_url_with_regex(soup, regex) -> Optional[str]:
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


def get_can_url_with_guess_and_check(url) -> Optional[str]:
    for keyword in static.AMP_KEYWORDS:
        if keyword in url:
            guessed_url = url.replace(keyword, "")
            if check_if_valid_url(guessed_url):
                guessed_page = utils.get_page(guessed_url)
                if guessed_page and guessed_page.status_code == 200:
                    article_similarity = get_article_similarity(url, guessed_url)
                    log.info(f"Article similarity: {article_similarity}")
                    if article_similarity > 0.6:
                        log.info(f"{url} and {guessed_url} are probably the same article")
                        return guessed_url
                    elif article_similarity > 0.35:
                        found_urls = get_can_urls_by_tags(guessed_page.soup.find_all(rel='amphtml'), 'href', url=url)
                        if found_urls and found_urls[0] == url:
                            log.info(f"{url} and {guessed_url} are probably the same article")
                            return guessed_url

    return None


def get_can_urls_by_tags(tags, target_value, url=None) -> [str]:
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


def get_can_urls_with_meta_redirect(tags, url=None) -> [str]:
    can_values = []
    for can_tag in tags:
        if can_tag.get('http-equiv') != 'refresh':
            continue
        if 'content' not in can_tag.attrs:
            continue
        value = can_tag.get('content')
        if "url=" not in value:
            continue
        value = value.partition("url=")[2]
        if value.startswith("//"):
            parsed_uri = urlparse(url)
            value = f"{parsed_uri.scheme}:{value}"
        elif value.startswith("/"):
            parsed_uri = urlparse(url)
            value = f"{parsed_uri.scheme}://{parsed_uri.netloc}{value}"
        can_values.append(value)
    return can_values
