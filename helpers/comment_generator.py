import sys

from helpers import logger
from models.type import Type
from static import static

log = logger.get_log(sys)


# Generate a comment based on the types, links and more
def generate_reply(stream_type, np_subreddits, item_type, subreddit, links, summoned_link=None):
    # Initialize all variables
    canonical_text_latest, canonical_text, google_note, summoned_note, alt_link, who, what = "", "", "", "", "", "", ""
    n_amp_urls, n_canonicals, n_cached = 0, 0, 0

    # Check the type of the item, set the correct variables
    if item_type == Type.COMMENT:
        who = "you"
        what = "shared"
    elif item_type == Type.SUBMISSION:
        who = "OP"
        what = "posted"
    else:
        who = "you"
        what = "shared"

    # If it is a mention, add a summoned note
    if stream_type == Type.MENTION:
        summoned_note = f"^( | **Summoned by a** )[^(**good human here!**)](https://reddit.com{summoned_link})"

    # Loop through all links, and generate the canonical link part of the comment
    for link in links:
        if link.is_amp:
            if link.canonical:
                n_amp_urls += 1
                n_canonicals += 1
                if link.canonical_alt:
                    alt_link = f" | {link.canonical_alt_domain.capitalize()} canonical: " \
                               f"**[{link.canonical_alt}]({link.canonical_alt})**"
                canonical_text_latest = f"**[{link.canonical}]({link.canonical})**{alt_link}"
                canonical_text += f"\n\n[{n_canonicals}] **[{link.canonical}]({link.canonical})**{alt_link}"

            elif link.amp_canonical:
                n_amp_urls += 1
                n_canonicals += 1
                amp_tho = " ^(Still AMP, but no longer cached - unable to process further)"
                canonical_text_latest = f"**[{link.amp_canonical}]({link.amp_canonical})**{amp_tho}"
                canonical_text += f"\n\n[{n_canonicals}] **[{link.amp_canonical}]({link.amp_canonical})**{amp_tho}"

            if link.is_cached:
                n_cached += 1

    # If there is at least one canonical detected, continue
    if n_canonicals >= 1:
        intro_why = f"These should load faster, but Google's AMP is controversial because of " \
                    f"[concerns over privacy and the Open Web]({static.FAQ_LINK})."
        outro = f"\n\n*****\n\n ^(I'm a bot | )[^(Why & About)]({static.FAQ_LINK})^" \
                f"( | )[^(Summon me with u/AmputatorBot)]({static.SUMMON_INFO_LINK})"

        # If there is only 1 canonical url, write a comment in singular
        if n_amp_urls == 1:
            canonical_text = canonical_text_latest
            intro_who_wat = f"It looks like {who} {what} an AMP link. "
            intro_maybe = "\n\nYou might want to visit **the canonical page** instead: "

        # If there are more than 1 canonical urls, write a comment in plural
        else:
            intro_who_wat = f"It looks like {who} {what} some AMP links. "
            intro_maybe = "\n\nYou might want to visit **the canonical pages** instead:"

        # Set-up the google_hosted note
        if n_cached > 0:
            if n_amp_urls == 1 and n_cached == 1:
                n_note = "the one"
            elif 1 < n_amp_urls == n_cached:
                n_note = "the ones"
            else:
                n_note = "some of the ones"
            google_note = f"Fully cached AMP pages (like {n_note} {who} {what})," \
                          f" are [especially problematic]({static.FAQ_LINK}). "

        reply_text = f"{intro_who_wat}{google_note}{intro_why}{intro_maybe}{canonical_text}{outro}{summoned_note}"

        if subreddit.display_name.casefold() in list(sub.casefold() for sub in np_subreddits):
            reply_text = reply_text.replace("www.reddit", "np.reddit")

        return reply_text, canonical_text

    log.warning("Couldn't generate reply")
    return None
