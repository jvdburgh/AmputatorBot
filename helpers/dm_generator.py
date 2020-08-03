from models.resultcode import ResultCode
from static import static


def dm_generator(result_code, reply_link=None, parent_subreddit=None, parent_type=None,
                 parent_link=None, first_amp_url=None, canonical_text=None):
    outro = "\n\nFeel free to leave feedback by contacting u/Killed_Mufasa, " \
            "by posting on [r/AmputatorBot](https://www.reddit.com/r/AmputatorBot/) or by " \
            "[opening an issue on GitHub](https://github.com/KilledMufasa/AmputatorBot/issues/new).\n\n" \
            "TIP: With the online version of AmputatorBot it's easier than ever to remove AMP from your URLS. " \
            f"Check out an example with your AMP link here: https://www.amputatorbot.com/?{first_amp_url}"

    if result_code == ResultCode.SUCCESS:
        subject = "Thx for summoning AmputatorBot!"
        message = f"u/AmputatorBot has [successfully replied](https://www.reddit.com{reply_link}?context=1) to " \
                  f"[the {parent_type} you summoned it for](https://www.reddit.com{parent_link}).\n\n" \
                  f"Thanks for summoning me, I couldn't do this without you (no but literally!). " \
                  f"You've a very good human <3{outro}"

    elif result_code == ResultCode.ERROR_DISALLOWED_MOD:
        subject = "AmputatorBot ran into an error: Disallowed subreddit"
        message = f"u/AmputatorBot couldn't reply to [the {parent_type} you summoned it for](https://www.reddit.com" \
                  f"{parent_link}) because r/{parent_subreddit} is being moderated by an anti-bot-bot. Because of " \
                  f"this, u/AmputatorBot will be banned once it comments something, and the comment will be removed " \
                  f"immediately. Most subreddits using an anti-bot service have a strict no-bot policy. Sometimes an " \
                  f"exception can be made, but until that's the case, u/AmputatorBot won't be able to interact with " \
                  f"this subreddit, [(just like it can't in these)]({static.FAQ_LINK}).\n\n" \
                  f"But that doesn't stop us! Here are the canonical URLs you requested: {canonical_text}\n\n" \
                  f"Maybe _you_ can post it instead? o_0\n\nPS: you're a very good human for trying <3{outro}"

    elif result_code == ResultCode.ERROR_DISALLOWED_SUBREDDIT:
        subject = "AmputatorBot ran into an error: Disallowed subreddit"
        message = f"u/AmputatorBot couldn't reply to [the {parent_type} you summoned it for](https://www.reddit.com" \
                  f"{parent_link}) because u/AmputatorBot is disallowed and/or banned in r/{parent_subreddit}, " \
                  f"just like it is [some others]({static.FAQ_LINK}).\n\n And as long as that's the case, " \
                  f"u/AmputatorBot won't be able to interact with this subreddit.\n\n" \
                  f"But that doesn't stop us! Here are the canonical URLs you requested: {canonical_text}\n\n" \
                  f"Maybe _you_ can post it instead? o_0\n\nPS: you're a very good human for trying <3{outro}"

    elif result_code == ResultCode.ERROR_NO_CANONICALS:
        subject = "AmputatorBot ran into an error: No canonicals found"
        message = f"u/AmputatorBot didn't reply to [the {parent_type} you summoned it for](https://www.reddit.com" \
                  f"{parent_link}) because AmputatorBot couldn't find any canonicals. The most common cause for " \
                  f"this error is that the website blocks bots or users from certain countries (aka geo-blocking). " \
                  f"Other common causes are websites that implemented AMP specs incorrectly, privacywalls, " \
                  f"cookiewalls and so on. This error has been automatically logged and will be investigated as soon " \
                  f"as possible.\n\nMaybe you can try to find it manually and post it for the others? o_0\n\n" \
                  f"PS: you're a very good human for trying <3{outro}"

    elif result_code == ResultCode.ERROR_PROBLEMATIC_DOMAIN:
        subject = "AmputatorBot ran into an error: Couldn't scrape website"
        message = f"u/AmputatorBot couldn't reply to [the {parent_type} you summoned it for](https://www.reddit.com" \
                  f"{parent_link}) because AmputatorBot couldn't scrape [this page]({first_amp_url}) and thus " \
                  f"it couldn't find the canonical link. This is a known issue specific to this domain and a good " \
                  f"fix is currently not possible because the reasons for this error are beyond our control. " \
                  f"The most common cause for this error is that the website blocks bots or users from certain " \
                  f"countries (aka geo-bocking). Other common causes are websites that implemented AMP specs " \
                  f"incorrectly, privacywalls, cookiewalls and so on. This error has been automatically logged and " \
                  f"will be investigated as soon as possible.\n\nMaybe you can try to find it manually and post it " \
                  f"for the others? o_0\n\nPS: you're a very good human for trying <3{outro}"

    elif result_code == ResultCode.ERROR_REPLY_FAILED:
        subject = "AmputatorBot ran into an error: Couldn't post reply"
        message = f"u/AmputatorBot couldn't reply to [the {parent_type} you summoned it for](https://www.reddit.com" \
                  f"{parent_link}) because the reply didn't got through. This happens when the bot got banned while " \
                  f"handling the {parent_type}, if the banned-list was outdated or, and this is much more likely, " \
                  f"the {parent_type} has been removed or deleted. But that doesn't stop us! Here are the canonical " \
                  f"URLs you requested: {canonical_text}\n\n Maybe _you_ can post it instead? o_0\n\nThis error has " \
                  f"been automatically logged and will be investigated as soon as possible.\n\nPS: you're a very " \
                  f"good human for trying <3{outro}"

    elif result_code == ResultCode.ERROR_USER_OPTED_OUT:
        subject = "AmputatorBot ran into an error: User opted out"
        message = f"u/AmputatorBot didn't reply to [the {parent_type} you summoned it for](https://www.reddit.com" \
                  f"{parent_link}) because the user who posted the {parent_type} has opted out of AmputatorBot. " \
                  f"This feature allows people to prevent AmputatorBot from replying to their comments and posts. " \
                  f"Apologies for the inconvenience. Here are the canonical URLs you requested: {canonical_text}\n\n" \
                  f"Feel free to share it yourself, but please don't make fun of and/or hate on OP, because not " \
                  f"everyone likes bots and that is okay.\n\nPS: you're a very good human for trying <3{outro}"

    else:
        subject = "AmputatorBot ran into an error: Unknown error"
        message = f"u/AmputatorBot couldn't reply to [the {parent_type} you summoned it for](https://www.reddit.com" \
                  f"{parent_link}) because of an unknown error ('{result_code.value}') - that's all we know. This " \
                  f"error has been automatically logged and will be investigated as soon as possible. Apologies for " \
                  f"the inconvenience.\n\nMaybe you can try to find it manually and post it for the others? o_0\n\n" \
                  f"PS: you're a very good human for trying <3{outro}"

    return subject, message
