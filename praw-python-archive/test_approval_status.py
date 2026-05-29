from helpers import login
import traceback
should_run = True

# Run the program
while should_run:
    subreddits = ['amputatorbot', 'joris', 'iets', 'worldnews']
    approved_subreddits = []
    #disallowed_mods = ['dummy01','dummy01']
    praw_session = login.get_praw_session()

    for subreddit in praw_session.user.contributor_subreddits(limit=None):
        print(str(subreddit))
        print("lol idk")
    for subreddit in subreddits:
    # Check if AmputatorBot is a contributor (approved user) in subreddit X
        print(f"checking status of {subreddit}")
        try:
            if subreddit in praw_session.user.contributor_subreddits(limit=None):
                print("Contributor status has been confirmed")

        except (Exception):
            print(traceback.format_exc())
            print("Failed to confirm contributor status, an error was raised!")

        print("Failed to confirm contributor status")

    should_run = False