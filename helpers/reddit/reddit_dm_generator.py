from typing import Tuple

from models.link import Link
from models.resultcode import ResultCode


def dm_generator(result_code, reply_link=None, parent_subreddit=None, parent_type=None,
                 parent_link=None, links=None, canonical_text=None) -> Tuple[str, str]:

    first_amp_link: Link = next((link for link in links if link.origin and link.origin.is_amp), None)
    first_amp_origin_url = first_amp_link.origin.url

    outro = "\n\nFeel free to leave feedback by contacting u/Killed_Mufasa, " \
            "by posting on [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/) or by " \
            "[opening an issue on GitHub](https://github.com/KilledMufasa/AmputatorBot/issues/new).\n\n" \
            "Tip: With the online version of AmputatorBot it's easier than ever to remove AMP from your URLs. " \
            f"Check out an example with your AMP link here: https://www.amputatorbot.com/?q={first_amp_origin_url}"

    good_human_for_trying = "PS: you're a very good human for trying! <3"

    if result_code == ResultCode.SUCCESS:
        subject = "Thx for summoning AmputatorBot!"
        message = f"u/AmputatorBot has [successfully replied](https://www.reddit.com{reply_link}?context=1) to " \
                  f"[the {parent_type} you summoned it for](https://www.reddit.com{parent_link}).\n\n" \
                  f"Thanks for summoning me, I couldn't do this without you (no but literally!). " \
                  f"Good human! <3{outro}"

    elif result_code == ResultCode.ERROR_DISALLOWED_MOD:
        subject = "AmputatorBot ran into an error: Disallowed subreddit"
        message = f"u/AmputatorBot can't reply to [the {parent_type} you summoned it for](https://www.reddit.com" \
                  f"{parent_link}) because r/{parent_subreddit} is being moderated by an anti-bot-bot. Because of " \
                  f"this, u/AmputatorBot will be banned once it comments something, and the comment will be removed " \
                  f"immediately. Unless r/{parent_subreddit} makes an exception for u/AmputatorBot, it won't be able " \
                  f"to interact with this subreddit. But that doesn't stop us! Here are the canonical URLs you " \
                  f"requested: {canonical_text}\n\nMaybe _you_ can post it instead? o_0 You can easily generate a " \
                  f"comment to post, by [clicking here]" \
                  f"(https://www.amputatorbot.com/?gc=true&q={first_amp_origin_url}).\n\n" \

    elif result_code == ResultCode.ERROR_DISALLOWED_SUBREDDIT:
        subject = "AmputatorBot ran into an error: Disallowed subreddit"
        message = f"u/AmputatorBot can't reply to [the {parent_type} you summoned it for](https://www.reddit.com" \
                  f"{parent_link}) because u/AmputatorBot is disallowed and/or banned in r/{parent_subreddit}." \
                  f"But that doesn't stop us! Here are the canonical URLs you requested: {canonical_text}\n\n" \
                  f"Maybe _you_ can post it instead? o_0 You can easily generate a " \
                  f"comment to post, by [clicking here]" \
                  f"(https://www.amputatorbot.com/?gc=true&q={first_amp_origin_url}).\n\n" \
                  f"{good_human_for_trying}{outro}"

    elif result_code == ResultCode.ERROR_NO_CANONICALS:
        subject = "AmputatorBot ran into an error: No canonicals found"
        message = f"u/AmputatorBot didn't reply to [the {parent_type} you summoned it for](https://www.reddit.com" \
                  f"{parent_link}) because AmputatorBot couldn't find any canonicals. The most common cause for " \
                  f"this error is that the website blocks bots or users from certain countries (aka geo-blocking). " \
                  f"Other common causes are websites that implemented AMP specs incorrectly, privacy- and " \
                  f"cookiewalls and so on. We'll do our best to fix this going forward. In the meantime, maybe " \
                  f"you can try to find the canonical(s) manually and post it for the others? o_0\n\n" \
                  f"{good_human_for_trying}{outro}"

    elif result_code == ResultCode.ERROR_PROBLEMATIC_DOMAIN:
        subject = "AmputatorBot ran into an error: Couldn't scrape website"
        message = f"u/AmputatorBot didn't reply to [the {parent_type} you summoned it for](https://www.reddit.com" \
                  f"{parent_link}) because AmputatorBot couldn't scrape [this page]({first_amp_origin_url}) and thus " \
                  f"it couldn't find the canonical link. This is a known issue specific to this domain and a good " \
                  f"fix is currently not possible because the reasons for this error are beyond our control. " \
                  f"The most common cause for this error is that the website blocks bots or users from certain " \
                  f"countries (aka geo-blocking). Other common causes are websites that implemented AMP specs " \
                  f"incorrectly, privacy- and cookiewalls and so on. We'll do our best to fix this going forward. " \
                  f"In the meantime, maybe you can try to find the canonical(s) manually and post it for the others? " \
                  f"o_o\n\n{good_human_for_trying}{outro}"

    elif result_code == ResultCode.ERROR_REPLY_FAILED:
        subject = "AmputatorBot ran into an error: Couldn't post reply"
        message = f"u/AmputatorBot couldn't reply to [the {parent_type} you summoned it for](https://www.reddit.com" \
                  f"{parent_link}) because the reply couldn't be posted. The {parent_type} might have been " \
                  f"removed or deleted, or " \
                  f"But that doesn't stop us! Here are the canonical URLs you requested: {canonical_text}\n\n" \
                  f"Maybe _you_ can post it instead? o_0 You can easily generate a " \
                  f"comment to post, by [clicking here]" \
                  f"(https://www.amputatorbot.com/?gc=true&q={first_amp_origin_url}).\n\n" \
                  f"{good_human_for_trying}{outro}"

    elif result_code == ResultCode.ERROR_USER_OPTED_OUT:
        subject = "AmputatorBot ran into an error: User opted out"
        message = f"u/AmputatorBot can't reply to [the {parent_type} you summoned it for](https://www.reddit.com" \
                  f"{parent_link}) because the user who posted the {parent_type} has opted out of AmputatorBot. " \
                  f"This feature allows people to prevent AmputatorBot from replying to their comments and posts. " \
                  f"Not everyone likes bots and that's okay!\n\n Here are the canonical URLs you requested: " \
                  f"{canonical_text}\n\n{good_human_for_trying}{outro}"

    else:
        subject = "AmputatorBot ran into an error: Unknown error"
        message = f"u/AmputatorBot couldn't reply to [the {parent_type} you summoned it for](https://www.reddit.com" \
                  f"{parent_link}) because of an unknown error ('{result_code.value}') - that's all we know." \
                  f"We'll do our best to fix this going forward. In the meantime, maybe " \
                  f"you can try to find the canonical(s) manually and post it for the others? o_0\n\n" \
                  f"{good_human_for_trying}{outro}"

    return subject, message
