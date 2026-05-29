//! Query-string parsing for `/api/v1/convert`.
//!
//! Refines `praw-python-archive/AmputatorBotCom/main.py:109-141` — same two strategies
//! the legacy bot used, but dispatched on whether the URL is actually
//! encoded rather than the legacy's `%20`-presence guess (which broke for
//! every encoded URL that didn't happen to contain a space).
//!
//! The contract:
//!
//! 1. **Encoded URL**: always works, regardless of where `q` sits in the
//!    param list. `?q=https%3A%2F%2F...&gac=true` and
//!    `?gac=true&q=https%3A%2F%2F...` both resolve cleanly.
//! 2. **Unencoded URL**: only required to work when `q` is the last param.
//!    `?gac=true&q=https://example.eu/article?id=1` works; `q` between
//!    other params is best-effort and may glue stray params onto the URL.
//!
//! How: try the raw-strip path first (removes known params from the raw
//! query string, leaves whatever's left as the URL). If the result contains
//! a literal `://`, the caller sent an unencoded URL — use that, with any
//! URL-internal `?id=...&ref=...` tail preserved. Otherwise the URL was
//! percent-encoded; the args-decoded `q` is correct.
//!
//! ### Known limitation faithfully preserved
//!
//! The legacy `md=\w` regex only matches a single character, so `md=10`
//! survives the strip and gets glued to the URL. Real-world traffic only
//! ever uses `md=3`, so the bug is dormant; the new impl keeps it for
//! parity. Fix lands as a follow-up commit once parity is verified.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

/// Parsed `/api/v1/convert` request parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertParams {
    /// The URL (or text containing URLs) to resolve.
    pub q: String,
    /// `gac` — guess-and-check enabled? Default `true`.
    pub use_gac: bool,
    /// `md` — max depth. Default `3`.
    pub max_depth: u32,
    /// `r` — redirect mode? Default `false`. When true and a canonical is
    /// found, the handler returns a 303 redirect instead of JSON.
    pub redirect: bool,
}

/// Reason parsing failed. Maps 1:1 to an HTTP status code in the handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// No `q` param, or `q` was empty.
    MissingQ,
}

/// Default `md` (max depth) when the param is absent. Ports
/// `praw-python-archive/static/static.py:MAX_DEPTH = 3`.
const DEFAULT_MAX_DEPTH: u32 = 3;

/// Parse the raw query string. `raw` is whatever follows `?` in the request
/// URI (no leading `?`, may be empty).
pub fn parse(raw: &str) -> Result<ConvertParams, ParseError> {
    let params = parse_params(raw);

    let q_arg = params.get("q").map(String::as_str).unwrap_or("");
    if q_arg.is_empty() {
        return Err(ParseError::MissingQ);
    }

    // Pick whichever of the two parsing strategies actually saw a URL:
    // the strip path keeps URL-internal `?` and `&` intact, but produces
    // garbage when the URL was percent-encoded (the scheme delimiter
    // becomes `%3A%2F%2F`). The presence of a literal `://` in the strip
    // output is exactly the signal we need.
    let stripped = strip_known_params(raw);
    let q = if stripped.contains("://") {
        stripped
    } else {
        q_arg.to_string()
    };

    Ok(ConvertParams {
        q,
        use_gac: bool_param(&params, "gac", true),
        max_depth: int_param(&params, "md", DEFAULT_MAX_DEPTH),
        redirect: bool_param(&params, "r", false),
    })
}

/// Decode the query string into a flat `name → value` map.
///
/// Last-value-wins for duplicate names — matches Flask's `request.args[key]`
/// behavior (which returns the first value, but the legacy bot never uses
/// duplicates so the tie-break direction doesn't matter for parity).
fn parse_params(raw: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if raw.is_empty() {
        return out;
    }
    for pair in raw.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let key = percent_decode(k);
        let value = percent_decode(v);
        out.insert(key, value);
    }
    out
}

/// Ports the regex strip pass from `praw-python-archive/AmputatorBotCom/main.py:116-127`.
///
/// Removes `md=<single-char>`, `gac=true|false`, and the literal `q=` from
/// the raw query string. The single-character limitation on `md` is
/// intentional — see the module-level "Known limitation" note.
fn strip_known_params(raw: &str) -> String {
    static PATTERNS: LazyLock<[Regex; 7]> = LazyLock::new(|| {
        [
            Regex::new(r"&md=\w").unwrap(),
            Regex::new(r"md=\w&").unwrap(),
            Regex::new(r"&gac=(?:true|false)").unwrap(),
            Regex::new(r"gac=(?:true|false)&").unwrap(),
            // `r` is a v7-era param; the legacy strip pass didn't know about
            // it. Strip both orderings so unencoded callers can put `r=true`
            // anywhere relative to `q`.
            Regex::new(r"&r=(?:true|false)").unwrap(),
            Regex::new(r"r=(?:true|false)&").unwrap(),
            Regex::new(r"q=").unwrap(),
        ]
    });
    let mut s = raw.to_string();
    for pat in PATTERNS.iter() {
        s = pat.replace_all(&s, "").into_owned();
    }
    s
}

/// Parse a boolean param. Ports `distutils.util.strtobool` behavior:
/// `y`/`yes`/`t`/`true`/`on`/`1` → true; `n`/`no`/`f`/`false`/`off`/`0`
/// → false. Anything else (including missing or empty) → `default`.
fn bool_param(params: &HashMap<String, String>, key: &str, default: bool) -> bool {
    match params.get(key).map(|s| s.to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "y" | "yes" | "t" | "true" | "on" | "1") => true,
        Some(v) if matches!(v.as_str(), "n" | "no" | "f" | "false" | "off" | "0") => false,
        _ => default,
    }
}

/// Parse an integer param. Missing, empty, or unparseable → `default`.
/// Legacy `int(...)` would raise on non-integer values; we soften that to
/// the default since the surrounding code (`max_depth`) already has a sane
/// fallback.
fn int_param(params: &HashMap<String, String>, key: &str, default: u32) -> u32 {
    params
        .get(key)
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Percent-decode a query-string component. Tolerant of malformed inputs:
/// stray `%` at the end or an invalid hex digit leaves the bytes as-is
/// rather than erroring. `+` becomes a space (legacy URL-encoded form data
/// shape — Flask does this implicitly via `request.args`).
fn percent_decode(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'+' {
            out.push(b' ');
            i += 1;
        } else if b == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push((h * 16 + l) as u8);
                i += 3;
            } else {
                out.push(b);
                i += 1;
            }
        } else {
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ──────────────────────────────────────────────────────────────
    //  Encoded-q path
    // ──────────────────────────────────────────────────────────────

    #[test]
    fn parses_encoded_url_with_q_last() {
        let q = "gac=true&md=3&q=https%3A%2F%2Fwww.google.com%2Famp%2Fs%2Fexample.eu%2Famp%2F";
        let p = parse(q).expect("should parse");
        assert_eq!(p.q, "https://www.google.com/amp/s/example.eu/amp/");
        assert!(p.use_gac);
        assert_eq!(p.max_depth, 3);
    }

    #[test]
    fn parses_encoded_url_with_q_first() {
        let q = "q=https%3A%2F%2Fexample.eu%2Famp%2Farticle&gac=false&md=5";
        let p = parse(q).expect("should parse");
        assert_eq!(p.q, "https://example.eu/amp/article");
        assert!(!p.use_gac);
        assert_eq!(p.max_depth, 5);
    }

    #[test]
    fn parses_encoded_url_no_spaces_no_special_chars() {
        // Regression: the legacy `%20` heuristic broke this case. A well-
        // encoded URL with no literal `%20` in it (because the URL has no
        // spaces) used to fall through to the raw-strip path, which left
        // the URL still percent-encoded and unparseable by linkify. The
        // `://` check fixes it.
        let q = "q=https%3A%2F%2Fabcnews.com%2Famp%2FPolitics%2Fstory%3Fid%3D1";
        let p = parse(q).expect("should parse");
        assert_eq!(p.q, "https://abcnews.com/amp/Politics/story?id=1");
    }

    // ──────────────────────────────────────────────────────────────
    //  Unencoded-q path (the load-bearing pragmatic fallback)
    // ──────────────────────────────────────────────────────────────

    #[test]
    fn parses_unencoded_url_q_last() {
        // Human-pastes-into-browser case.
        let q = "gac=true&md=3&q=https://www.google.com/amp/s/example.eu/amp/";
        let p = parse(q).expect("should parse");
        assert_eq!(p.q, "https://www.google.com/amp/s/example.eu/amp/");
        assert!(p.use_gac);
        assert_eq!(p.max_depth, 3);
    }

    #[test]
    fn parses_unencoded_url_with_q_marker_and_amp_in_url() {
        // The URL itself has `?` and `&` in it. The strip pass removes
        // `gac=true&` and `q=` but leaves the URL's internal `?`/`&`.
        let q = "gac=true&md=3&q=https://www.google.com/amp/s/example.eu/article?ref=abc&tag=xyz";
        let p = parse(q).expect("should parse");
        assert_eq!(
            p.q,
            "https://www.google.com/amp/s/example.eu/article?ref=abc&tag=xyz"
        );
    }

    #[test]
    fn unencoded_with_only_q_param() {
        let q = "q=https://www.google.com/amp/s/example.eu/article";
        let p = parse(q).expect("should parse");
        assert_eq!(p.q, "https://www.google.com/amp/s/example.eu/article");
        assert!(p.use_gac); // default
        assert_eq!(p.max_depth, 3); // default
    }

    // ──────────────────────────────────────────────────────────────
    //  Missing q
    // ──────────────────────────────────────────────────────────────

    #[test]
    fn missing_q_returns_error() {
        assert_eq!(parse("gac=true&md=3"), Err(ParseError::MissingQ));
    }

    #[test]
    fn empty_q_returns_error() {
        // Legacy treats `q=` (empty value) same as missing.
        assert_eq!(parse("q=&gac=true"), Err(ParseError::MissingQ));
    }

    #[test]
    fn empty_query_string_returns_error() {
        assert_eq!(parse(""), Err(ParseError::MissingQ));
    }

    // ──────────────────────────────────────────────────────────────
    //  Bool parsing — strtobool compatibility
    // ──────────────────────────────────────────────────────────────

    #[test]
    fn gac_accepts_legacy_true_values() {
        for v in ["true", "True", "yes", "y", "1", "on", "t"] {
            let p = parse(&format!("q=https://amp.example/x&gac={v}")).unwrap();
            assert!(p.use_gac, "expected gac=true for value `{v}`");
        }
    }

    #[test]
    fn gac_accepts_legacy_false_values() {
        for v in ["false", "False", "no", "n", "0", "off", "f"] {
            let p = parse(&format!("q=https://amp.example/x&gac={v}")).unwrap();
            assert!(!p.use_gac, "expected gac=false for value `{v}`");
        }
    }

    #[test]
    fn gac_unknown_falls_back_to_default() {
        let p = parse("q=https://amp.example/x&gac=banana").unwrap();
        assert!(p.use_gac, "unknown gac value should use default (true)");
    }

    // ──────────────────────────────────────────────────────────────
    //  Int parsing
    // ──────────────────────────────────────────────────────────────

    #[test]
    fn md_parses_int() {
        let p = parse("q=https://amp.example/x&md=5").unwrap();
        assert_eq!(p.max_depth, 5);
    }

    #[test]
    fn md_invalid_falls_back_to_default() {
        let p = parse("q=https://amp.example/x&md=not-a-number").unwrap();
        assert_eq!(p.max_depth, 3);
    }

    #[test]
    fn md_missing_falls_back_to_default() {
        let p = parse("q=https://amp.example/x").unwrap();
        assert_eq!(p.max_depth, 3);
    }

    // ──────────────────────────────────────────────────────────────
    //  r (redirect) param — new in v7
    // ──────────────────────────────────────────────────────────────

    #[test]
    fn r_defaults_to_false() {
        let p = parse("q=https://amp.example/x").unwrap();
        assert!(!p.redirect);
    }

    #[test]
    fn r_true_sets_redirect() {
        let p = parse("q=https://amp.example/x&r=true").unwrap();
        assert!(p.redirect);
    }

    // ──────────────────────────────────────────────────────────────
    //  Known limitations (documented for parity)
    // ──────────────────────────────────────────────────────────────

    #[test]
    fn unencoded_md_multidigit_glues_to_url_legacy_bug() {
        // `md=10` is not stripped because the legacy regex `md=\w` only
        // matches a single character. Anything past the first digit gets
        // glued to the URL.
        let q = "gac=true&md=10&q=https://amp.example/x";
        let p = parse(q).unwrap();
        // The `md=` substring is stripped by the `q=` regex too? No — only
        // literal `q=` is stripped. So we get the URL prefixed with the
        // un-stripped `0&` from `md=10&`. Faithful to legacy.
        assert!(
            p.q.contains("0&") || p.q.starts_with("0"),
            "legacy bug: md=10 should leak into q; got {}",
            p.q
        );
    }

    #[test]
    fn unencoded_url_with_q_not_last_has_residual() {
        // Documented limitation: `q` must be the last param in unencoded
        // mode, otherwise residual params get glued to the URL.
        let q = "q=https://amp.example/x&gac=true&md=3";
        let p = parse(q).unwrap();
        // The strip pass removes `&gac=true` and `&md=3`, leaving just the
        // URL — in this particular case it actually works! But: if the
        // tail param weren't a known one, it would leak through.
        // Demonstrate the failure mode with an unknown tail param:
        let q_unknown = "q=https://amp.example/x&unknown=value";
        let p_unknown = parse(q_unknown).unwrap();
        assert!(
            p_unknown.q.contains("unknown=value"),
            "unknown trailing param leaks into q (documented limitation): {}",
            p_unknown.q
        );
        // Sanity that the known-tail case works.
        assert_eq!(p.q, "https://amp.example/x");
    }

    // ──────────────────────────────────────────────────────────────
    //  Percent-decoding edge cases
    // ──────────────────────────────────────────────────────────────

    #[test]
    fn percent_decode_handles_plus_as_space() {
        let p = parse("q=https://amp.example/x?title=hello+world").unwrap();
        // No %20 → unencoded path. `+` is preserved in the strip path
        // because we're working with the raw query string there.
        assert!(p.q.contains("hello+world"));
    }

    #[test]
    fn percent_decode_tolerates_malformed_percent() {
        // Standalone `%` at end of string — should not panic.
        let p = parse("q=hello%").unwrap();
        assert!(p.q.contains("hello"));
    }
}
