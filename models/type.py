import enum


class Type(enum.Enum):
    COMMENT = "comment"
    MENTION = "mention"
    ONLINE = "online"
    SUBMISSION = "submission"
    TEST = "test"
