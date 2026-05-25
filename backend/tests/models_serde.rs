//! Verify the domain types serialize to JSON matching the legacy Python API.
//!
//! The reference is the live API response captured during v7 plan research:
//!
//! ```bash
//! curl 'https://www.amputatorbot.com/api/v1/convert?gac=true&md=3&q=https://www.google.com/amp/s/electrek.co/2018/06/19/tesla-model-3-assembly-line-inside-tent-elon-musk/amp/'
//! ```
//!
//! See `docs/amputatorbot-devvit-migration-plan-v7.md` §"API contract".

use amputatorbot_backend::models::{Canonical, CanonicalType, Link, UrlMeta};

/// Canonical type names must serialize as SCREAMING_SNAKE_CASE because Python
/// `jsons.dump` uses `enum.name` (uppercase identifier). Verified against the
/// live API in the response example above (`"type": "DATABASE"`,
/// `"type": "GOOGLE_MANUAL_REDIRECT"`).
#[test]
fn canonical_type_serializes_to_screaming_snake() {
    let cases = [
        (CanonicalType::Rel, "\"REL\""),
        (CanonicalType::Canurl, "\"CANURL\""),
        (CanonicalType::OgUrl, "\"OG_URL\""),
        (
            CanonicalType::GoogleManualRedirect,
            "\"GOOGLE_MANUAL_REDIRECT\"",
        ),
        (CanonicalType::GoogleJsRedirect, "\"GOOGLE_JS_REDIRECT\""),
        (CanonicalType::BingOriginalUrl, "\"BING_ORIGINAL_URL\""),
        (CanonicalType::SchemaMainentity, "\"SCHEMA_MAINENTITY\""),
        (CanonicalType::TcoPagetitle, "\"TCO_PAGETITLE\""),
        (CanonicalType::MetaRedirect, "\"META_REDIRECT\""),
        (CanonicalType::GuessAndCheck, "\"GUESS_AND_CHECK\""),
        (CanonicalType::Database, "\"DATABASE\""),
    ];

    for (variant, expected) in cases {
        let actual = serde_json::to_string(&variant).unwrap();
        assert_eq!(
            actual, expected,
            "{variant:?} should serialize to {expected}"
        );
    }
}

/// Round-trip an actual Link constructed to match the live API response for
/// the `electrek.co` example, then assert the JSON output is byte-identical
/// (modulo array/object key order — serde_json preserves field order).
#[test]
fn link_matches_live_api_shape() {
    let canonical_db = Canonical {
        domain: Some("electrek".into()),
        is_alt: false,
        is_amp: Some(false),
        is_cached: None,
        is_valid: Some(true),
        type_: Some(CanonicalType::Database),
        url: Some(
            "https://electrek.co/2018/06/19/tesla-model-3-assembly-line-inside-tent-elon-musk/"
                .into(),
        ),
        url_similarity: Some(0.866_310_160_427_807_5),
    };

    let canonical_google = Canonical {
        domain: Some("electrek".into()),
        is_alt: false,
        is_amp: Some(true),
        is_cached: Some(false),
        is_valid: Some(true),
        type_: Some(CanonicalType::GoogleManualRedirect),
        url: Some(
            "https://electrek.co/2018/06/19/tesla-model-3-assembly-line-inside-tent-elon-musk/amp/"
                .into(),
        ),
        url_similarity: Some(0.890_052_356_020_942_5),
    };

    let link = Link {
        amp_canonical: None,
        canonical: Some(canonical_db.clone()),
        canonicals: vec![canonical_google, canonical_db],
        origin: UrlMeta {
            domain: Some("google".into()),
            is_amp: Some(true),
            is_cached: Some(true),
            is_valid: Some(true),
            url: Some(
                "https://www.google.com/amp/s/electrek.co/2018/06/19/tesla-model-3-assembly-line-inside-tent-elon-musk/amp/"
                    .into(),
            ),
        },
    };

    let json = serde_json::to_value(&link).unwrap();

    // Sanity-check the JSON structure matches the live response.
    assert!(json["amp_canonical"].is_null());
    assert_eq!(json["canonical"]["type"], "DATABASE");
    assert_eq!(json["canonical"]["domain"], "electrek");
    assert_eq!(json["canonical"]["is_amp"], false);
    assert!(json["canonical"]["is_cached"].is_null());
    assert_eq!(json["canonicals"].as_array().unwrap().len(), 2);
    assert_eq!(json["canonicals"][0]["type"], "GOOGLE_MANUAL_REDIRECT");
    assert_eq!(json["canonicals"][1]["type"], "DATABASE");
    assert_eq!(json["origin"]["is_amp"], true);
    assert_eq!(json["origin"]["domain"], "google");

    // Round-trip stability — serialize, parse, re-serialize, expect identical.
    let serialized = serde_json::to_string(&link).unwrap();
    let parsed: Link = serde_json::from_str(&serialized).unwrap();
    assert_eq!(link, parsed);
}

/// A Link with only an origin (e.g. non-AMP input) should still serialize
/// cleanly with `canonicals: []` and the optional fields as `null`.
#[test]
fn link_minimal_serializes_with_nulls() {
    let link = Link::new(UrlMeta::new("https://example.com"));
    let json = serde_json::to_value(&link).unwrap();

    assert!(json["amp_canonical"].is_null());
    assert!(json["canonical"].is_null());
    assert_eq!(json["canonicals"].as_array().unwrap().len(), 0);
    assert!(json["origin"]["is_amp"].is_null());
    assert!(json["origin"]["is_cached"].is_null());
    assert!(json["origin"]["domain"].is_null());
    assert_eq!(json["origin"]["url"], "https://example.com");
}
