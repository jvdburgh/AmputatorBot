import enum

from models.urlmeta import UrlMeta


class Link:
    def __init__(self, amp_canonical=None, canonical=None, canonicals=None, origin=None):
        self.amp_canonical: Canonical = amp_canonical
        self.canonical: Canonical = canonical
        self.canonicals: [Canonical] = canonicals
        self.origin: UrlMeta = origin


class Canonical(UrlMeta):
    def __init__(self, url=None, domain=None, is_amp=None, is_cached=False,
                 is_alt=False, type=None, url_similarity=None):
        super().__init__(url, domain, is_amp, is_cached)
        self.is_alt = is_alt
        self.type: CanonicalType = type
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
    META_REDIRECT = "meta_redirect"
    GUESS_AND_CHECK = "guess_and_check"
    DATABASE = "database"
