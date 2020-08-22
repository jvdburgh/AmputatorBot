import enum


class Type(enum.Enum):
    API = "api"
    COMMENT = "comment"
    MENTION = "mention"
    ONLINE = "online"
    SUBMISSION = "submission"
    TEST = "test"
    TWEET = "tweet"
