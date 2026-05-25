from datahandlers.local_datahandler import get_data_by_filename
from helpers import login
from helpers.utils import check_if_banned

should_run = True

# Run the program
while should_run:
    duplicates = []
    banned_subreddits = []
    unbanned_subreddits = []
    praw_session = login.get_praw_session()
    allowed_subreddits = get_data_by_filename("allowed_subreddits")
    allowed_subreddits_lowered = [x.lower() for x in allowed_subreddits]
    disallowed_subreddits = get_data_by_filename("disallowed_subreddits")
    disallowed_subreddits_lowered = [x.lower() for x in disallowed_subreddits]
    #subreddits = ['programming', 'nottheonion', 'Libertarian', 'test', 'LeopardsAteMyFace']
    #disallowed_mods = ['dummy01','dummy01']

    # Check for dupplicates in allowed subreddits list
    for subreddit in allowed_subreddits:
        subreddit = praw_session.subreddit(subreddit)
        subreddit_name = subreddit.display_name
        if subreddit.display_name in disallowed_subreddits_lowered:
            duplicates.append(subreddit_name)

    # Check for dupplicates in disallowed subreddits list
    for subreddit in disallowed_subreddits:
        subreddit = praw_session.subreddit(subreddit)
        subreddit_name = subreddit.display_name

        if subreddit.display_name in allowed_subreddits_lowered:
            duplicates.append(subreddit_name)

    print(f"Duplicates:{duplicates}")

    # Check if bans are correct
    for subreddit in disallowed_subreddits:
        subreddit = praw_session.subreddit(subreddit)
        subreddit_name = subreddit.display_name
        is_banned = check_if_banned(subreddit)

        if is_banned:
            print(f"Banned from: {subreddit_name}")
            banned_subreddits.append(subreddit_name)
        else:
            print(f"Not banned from: {subreddit_name}")
            unbanned_subreddits.append(subreddit_name)

    print(f"Banned from:{banned_subreddits}")
    print(f"Not banned from:{unbanned_subreddits}")

    should_run = False