# URLConversions test fixtures

CSV exports from the legacy MySQL `URLConversions` table on PythonAnywhere. These drive the M2 parity tests for canonical-finding and the AMP-detection unit tests.

## Files

| File | Rows | What it is |
|---|---|---|
| `URLConversions_500_including_failed_and_false_positives.csv` | 500 | **Random sample** of the production table — includes successful conversions, failed lookups, and false positives. Use this as the **fast smoke-test fixture** for parity runs. |
| `URLConversions_7000_successful.csv` | 7000 | All successful conversions in the export window. Use for the **full parity benchmark** to ensure the new Rust impl finds at least as many canonicals as the old Python bot did. |

## Schema (no header row)

| Col | Field | Notes |
|---|---|---|
| 0 | `id` | DB row id (entry_id) |
| 1 | `type` | `COMMENT`, `SUBMISSION`, `ONLINE` (web form), `API`, … |
| 2 | `timestamp_utc` | `YYYY-MM-DD HH:MM:SS` |
| 3 | `original_url` | The AMP / suspected-AMP URL the bot saw |
| 4 | `canonical_url` | What the bot resolved it to. **Empty for failures and false positives.** |
| 5 | `canonical_type` | The `CanonicalType` that found it: `REL`, `CANURL`, `OG_URL`, `GOOGLE_MANUAL_REDIRECT`, `GOOGLE_JS_REDIRECT`, `BING_ORIGINAL_URL`, `SCHEMA_MAINENTITY`, `TCO_PAGETITLE`, `META_REDIRECT`, `GUESS_AND_CHECK`, `DATABASE`. **Empty for failures.** |
| 6 | `note` | Free-text annotation. Usually empty. |

## How M2 tests use these

- **AMP detection false-positive tests** — rows in the 500-sample where `original_url` contains an AMP substring like `amp` but `canonical_url` is empty are candidates for "should not have been flagged as AMP." Examples in the 500 include `amputeestore.com` URLs, where `amp` appears in the domain but the URL isn't an AMP page.
- **Parity tests** — the Rust impl runs each row's `original_url` through canonical-finding and compares against the recorded `canonical_url`. The new impl should match or beat the old result; drift is reported with a diff.
- **Snapshot tests** — a hand-curated subset of ~20 representative rows is checked into `backend/tests/snapshots/` as golden JSON files.

## Don't

- Don't commit additional dumps that leak user data — these exports contain only URLs + metadata, no usernames or content beyond what was already public on Reddit.
- Don't hit the live API for every row of the 7000 in CI — that'd hammer publishers. The parity test reads the recorded `canonical_url` as the expected value; only the new Rust impl runs against live URLs, and even that should be rate-limited or run as a one-off benchmark, not on every PR.
