class Item:
    def __init__(self, type=None, id=None, subreddit=None, author=None, context=None,
                 parent_link=None, parent_type=None, summoner=None, body=None):
        self.type = type
        self.id = id
        self.subreddit = subreddit
        self.author = author
        self.context = context
        self.parent_link = parent_link
        self.parent_type = parent_type
        self.summoner = summoner
        self.body = body
        self.links = []
