import sys

from helpers import logger
from helpers.reddit.reddit_comment_generator import generate_reply
from models.link import Canonical
from models.resultcode import ResultCode

log = logger.get_log(sys)


def generate_flashmessage(result_code, links=None) -> str:
    # Initialize all variables
    canonical_text_latest, canonical_text, cached_note, summoned_note, alt_link, who, what = "", "", "", "", "", "", ""
    alt_link_html, canonical_text_latest_html, canonical_text_html, add_s = "", "", "", ""
    n_amp_urls, n_can, n_cached = 0, 0, 0

    if result_code == ResultCode.SUCCESS:
        for link in links:
            if link.origin and link.origin.is_amp:
                if link.canonical:
                    n_amp_urls += 1
                    n_can += 1
                    c_alt: Canonical = next(
                        (c for c in link.canonicals if c.domain != link.canonical.domain and not c.is_amp), None)

                    if c_alt is not None:
                        alt_link = f" | {c_alt.domain.capitalize()} canonical: {c_alt.url}"
                        alt_link_html = f" | {c_alt.domain.capitalize()} canonical: " \
                                        f"<a href='{c_alt.url}' class='canonical-link'>{c_alt.url}</a>"

                    canonical_text_latest = f"{link.canonical.url}{alt_link}"
                    canonical_text_latest_html = f"<a href='{link.canonical.url}' class='canonical-link'>" \
                                                 f"{link.canonical.url}</a>{alt_link_html}"
                    canonical_text += f"{link.canonical.url}{alt_link}, "
                    canonical_text_html += f"{canonical_text_latest_html}, "

                elif link.amp_canonical:
                    n_amp_urls += 1
                    n_can += 1
                    amp_tho = " (Still AMP, but no longer cached - unable to process further)"
                    canonical_text_latest = f"{link.amp_canonical.url}{amp_tho}"
                    canonical_text_latest_html = f"<a href='{link.amp_canonical.url}' class='canonical-link'>" \
                                                 f"{link.amp_canonical.url}</a>{amp_tho}"
                    canonical_text += f"{link.amp_canonical.url}{amp_tho}, "
                    canonical_text_html += f"{canonical_text_latest_html}, "

        if n_can == 1:
            canonical_text = canonical_text_latest
            canonical_text_html = canonical_text_latest_html
        else:
            canonical_text = canonical_text[:-2]
            canonical_text_html = canonical_text_html[:-2]
            add_s = "s"

        message = f"<b>Found canonical link{add_s}: </b><span class='canonicals'>{canonical_text_html}</span>" \
                  f"<div class='copy-container'><input id='copy-input' value='{canonical_text}'>" \
                  f"<button id='copy-button' onclick='clickToCopy(this.id, document.getElementById(`copy-input`))'>" \
                  f"Click to copy</button></div>"

    elif result_code == ResultCode.ERROR_NO_AMP:
        message = "Error: <b>That doesn't look like a valid AMP URL!</b> Make sure to include a link with a string " \
                  "in it like <b>amp</b>. Links need to be prefixed with <b>http://</b> or <b>https://</b>."

    elif result_code == ResultCode.ERROR_NO_CANONICALS:

        message = f"Error: <b>Couldn't find any canonicals</b>. The most common cause for " \
                  f"this error is that the website blocks bots or users from certain countries (aka geo-blocking). " \
                  f"Other common causes are websites that implemented AMP specs incorrectly, privacy- and " \
                  f"cookiewalls and so on. We'll do our best to fix this going forward."

    elif result_code == ResultCode.ERROR_PROBLEMATIC_DOMAIN:
        message = f"Error: <b>Couldn't scrape the page, no canonicals found. This is a known issue specific to this " \
                  f"domain and a good fix is currently not possible because the reasons for this error are beyond " \
                  f"our control. The most common cause for this error " \
                  f"The most common cause for this error is that the website blocks bots or users from certain " \
                  f"countries (aka geo-blocking). Other common causes are websites that implemented AMP specs " \
                  f"incorrectly, privacy- and cookiewalls and so on. We'll do our best to fix this going forward. "
    else:
        message = f"Error: <b>Unknown error</b>. Couldn't complete it's processes because of an unknown error " \
                  f"('{result_code.value}') - that's all we know. This error has been automatically logged and will " \
                  f"be investigated as soon as possible. Apologies for the inconvenience."

    return message


# Generate a simplified comment for the web
def generate_simplified_comment(links) -> str:
    try:
        reply_text, reply_canonical_text = generate_reply(links=links, from_online=True)

        reddit_message = f"<span><b>Generated Reddit comment text:</b></span>" \
                         f"<div class='copy-container copy-container-reddit'><textarea id='copy-container-reddit-textarea' rows='3'>{reply_text}</textarea>" \
                         f"<button id='copy-button-reddit' onclick='clickToCopy(this.id, document.getElementById(`copy-container-reddit-textarea`))'>" \
                         f"Click to copy</button></div>"

        return reddit_message

    except Exception:
        log.warning("Couldn't generate Reddit comment")
        return "Couldn't generate Reddit comment"
