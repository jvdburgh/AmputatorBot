# URLConversions test fixtures

CSV exports from the legacy MySQL `URLConversions` table on PythonAnywhere.
These drive the M2 parity tests for canonical-finding and the AMP-detection
unit tests.

## Files

| File | Rows | What it is |
|---|---|---|
| `10000_conversions_unfiltered.csv` | ~10,000 | **Random sample** of the production table — includes successful conversions, failed lookups, and false positives. Use this as the **default parity-test fixture**. |
| `10000_conversions_with_canonical.csv` | ~7,000 | Only rows where the legacy bot recorded a non-null `canonical_url`. Use for a **successes-only benchmark** — the new impl should match or beat the old result on every row here. |

## Schema (header row present)

```
entry_id,entry_type,handled_utc,original_url,canonical_url,canonical_type,note
```

| Col | Field | Notes |
|---|---|---|
| 0 | `entry_id` | DB primary key. |
| 1 | `entry_type` | `COMMENT`, `SUBMISSION`, `ONLINE` (web form), `API`, … |
| 2 | `handled_utc` | `YYYY-MM-DD HH:MM:SS` UTC. |
| 3 | `original_url` | The AMP / suspected-AMP URL the bot saw. |
| 4 | `canonical_url` | What the bot resolved it to. **Empty for failures and false positives** in the unfiltered file. |
| 5 | `canonical_type` | The `CanonicalType` that found it: `REL`, `CANURL`, `OG_URL`, `GOOGLE_MANUAL_REDIRECT`, `GOOGLE_JS_REDIRECT`, `BING_ORIGINAL_URL`, `SCHEMA_MAINENTITY`, `TCO_PAGETITLE`, `META_REDIRECT`, `GUESS_AND_CHECK`, `DATABASE`. **Empty for failures.** |
| 6 | `note` | Free-text annotation. Usually empty. |

## How M2 tests use these

- **AMP detection false-positive tests** — rows in the unfiltered file where `original_url` contains an `amp` substring but `canonical_url` is empty are candidates for "the legacy bot shouldn't have flagged this as AMP." See `canonical/amp_detect.rs` for the regression tests around the `amputeestore.com`-shape false positive.
- **Parity tests** — `cargo nextest run --test parity` (or `just parity`) runs each row's `original_url` through the new Rust resolver and compares the result to the recorded `canonical_url`. Drift surfaces in `tests/parity-report.md` with per-bucket counts (matched / mismatch_url / legacy_only / new_only / skipped).
- **Snapshot tests** — a hand-curated set of ~10 representative scenarios lives in `tests/snapshots.rs` as inline-HTML cases; the JSON output is pinned in `tests/snapshots/*.snap`.

## Run the parity test against these fixtures

```bash
cd backend
just record-fixtures   # ~hour for the full 10k set; idempotent on re-run
just parity            # ~20 sec — runs resolver against recorded HTML
cat tests/parity-report.md
```

By default `just record-fixtures` reads `10000_conversions_unfiltered.csv`.
Override with `just record-fixtures tests/fixtures/urlconversions/10000_conversions_with_canonical.csv`
to record the successes-only set instead.

## Don't

- Don't commit additional dumps that leak user data — these exports contain only URLs + metadata, no usernames or content beyond what was already public on Reddit.
- Don't hit live publishers from CI by running the recorder against every row on every PR. The recorder is a manual, occasional step; the parity test then runs offline against the saved HTML.
