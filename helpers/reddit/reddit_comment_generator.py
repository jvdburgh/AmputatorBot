import sys
from typing import Optional, Tuple

from helpers import logger
from models.link import Canonical, Link
from models.type import Type
from static import static

log = logger.get_log(sys)


# Generate a comment based on the types, links and more
def generate_reply(links: [Link], stream_type=None, np_subreddits=None, item_type=None,
                   subreddit=None, summoned_link=None, from_online=False) -> Optional[Tuple[str, str]]:

    canonical_text_latest, canonical_text, cached_note, summoned_note, alt_link, who, what = "", "", "", "", "", "", ""
    n_amp_urls, n_can, n_cached = 0, 0, 0
    bot_or_human = "human" if from_online else "bot"
    from_online_note = "| Generated with AmputatorBot" if from_online else ""

    if not from_online:
        # Check the type of the item, set the correct variables
        if item_type == Type.SUBMISSION:
            who = "OP"
            what = "posted"
        else:
            who = "you"
            what = "shared"

        # If it is a mention, add a summoned note
        if stream_type == Type.MENTION:
            summoned_note = f"^( | **Summoned by** )[^(**this good human!**)](https://reddit.com{summoned_link})"

    else:
        who = "you"
        what = "shared"

    # Loop through all links, and generate the canonical link part of the comment
    for link in links:
        if link.origin and link.origin.is_amp:
            if link.canonical:
                n_amp_urls += 1
                n_can += 1
                c_alt: Canonical = next(
                    (c for c in link.canonicals if c.domain != link.canonical.domain and not c.is_amp), None)

                if c_alt is not None:
                    alt_link = f" | {c_alt.domain.capitalize()} canonical: **[{c_alt.url}]({c_alt.url})**"

                canonical_text_latest = f"**[{link.canonical.url}]({link.canonical.url})**{alt_link}"
                canonical_text += f"\n\n- **[{link.canonical.url}]({link.canonical.url})**{alt_link}"

            elif link.amp_canonical:
                n_amp_urls += 1
                n_can += 1
                amp_tho = " ^(Still AMP, but no longer cached - unable to process further)"
                canonical_text_latest = f"**[{link.amp_canonical.url}]({link.amp_canonical.url})**{amp_tho}"
                canonical_text += f"\n\n- **[{link.amp_canonical.url}]({link.amp_canonical.url})**{amp_tho}"

            if link.origin.is_cached:
                n_cached += 1

    # If there is at least one canonical detected, continue
    if n_can >= 1:
        intro_why = f"These should load faster, but AMP is controversial because of " \
                    f"[concerns over privacy and the Open Web]({static.FAQ_LINK})."
        outro = f"\n\n*****\n\n ^(I'm a {bot_or_human} {from_online_note}| )[^(Why & About)]({static.FAQ_LINK})^" \
                f"( | )[^(Summon: u/AmputatorBot)]({static.SUMMON_INFO_LINK})"

        # If there is only 1 canonical url, write a comment in singular
        if n_amp_urls == 1:
            canonical_text = canonical_text_latest
            intro_who_wat = f"It looks like {who} {what} an AMP link. "
            intro_maybe = "\n\nMaybe check out **the canonical page** instead: "

        # If there are more than 1 canonical urls, write a comment in plural
        else:
            intro_who_wat = f"It looks like {who} {what} some AMP links. "
            intro_maybe = "\n\nMaybe check out **the canonical pages** instead: "

        # Set-up the cached_note note
        if n_cached > 0:
            if n_amp_urls == 1 and n_cached == 1:
                n_note = "the one"
            elif 1 < n_amp_urls == n_cached:
                n_note = "the ones"
            else:
                n_note = "some of the ones"
            cached_note = f" Fully cached AMP pages (like {n_note} {who} {what}), " \
                          f"are [especially problematic]({static.FAQ_LINK})."

        reply_text = f"{intro_who_wat}{intro_why}{cached_note}{intro_maybe}{canonical_text}{outro}{summoned_note}"

        if not from_online:
            if subreddit.display_name.casefold() in list(sub.casefold() for sub in np_subreddits):
                reply_text = reply_text.replace("www.reddit", "np.reddit")

        return reply_text, canonical_text

    log.warning("Couldn't generate reply")
    return None
