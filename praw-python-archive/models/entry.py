class Entry:
    def __init__(self, entry_id=None, entry_type=None, handled_utc=None, original_url=None,
                 canonical_url=None, canonical_type=None, note=None):
        self.entry_id = entry_id
        self.entry_type = entry_type
        self.handled_utc = handled_utc
        self.original_url = original_url
        self.canonical_url = canonical_url
        self.canonical_type = canonical_type
        self.note = note
