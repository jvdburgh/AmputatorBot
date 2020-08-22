import enum


class Link:
    def __init__(self, url=None, url_clean=None, url_clean_is_valid=None, is_amp=None, is_cached=None,
                 domain=None, canonical=None, canonical_alt=None, canonical_alt_domain=None, amp_canonical=None):
        self.url = url
        self.url_clean = url_clean
        self.url_clean_is_valid = url_clean_is_valid
        self.is_amp = is_amp
        self.is_cached = is_cached
        self.domain = domain
        self.canonical = canonical
        self.canonical_alt = canonical_alt
        self.canonical_alt_domain = canonical_alt_domain
        self.amp_canonical = amp_canonical
        self.canonicals = []
        self.canonicals_solved = []


class Canonical:
    def __init__(self, type=None, url=None, domain=None, article_text=None,
                 is_amp=None, is_valid=None, url_similarity=None):
        self.type = type
        self.url = url
        self.domain = domain
        self.article_text = article_text
        self.is_amp = is_amp
        self.is_valid = is_valid
        self.url_similarity = url_similarity


class CanonicalType(enum.Enum):
    REL = "rel"
    CANURL = "canurl"
    OG_URL = "og_url"
    GOOGLE_MANUAL_REDIRECT = "google_manual_redirect"
    GOOGLE_JS_REDIRECT = "google_js_redirect"
    BING_ORIGINAL_URL = "bing_original_url"
    SCHEMA_MAINENTITY = "schema_mainentity"
    TCO_PAGETITLE = "tco_pagetitle"
    GUESS_AND_CHECK = "guess_and_check"
