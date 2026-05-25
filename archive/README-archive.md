# Archive — legacy AmputatorBot (Python + PRAW + Flask)

Everything in this directory is the **legacy AmputatorBot codebase** as it ran on PythonAnywhere from 2019 until the Devvit migration. It is preserved here for reference during the v7 migration and as historical record.

## Why archived, not deleted

- The 10 canonical-finding methods in `helpers/canonical_methods.py` and the AMP-detection logic in `static/static.py` + `helpers/checker_utils.py` are the authoritative reference the new Rust impl ports. Keeping them readable matters more than tidying them away.
- `amputatorbotcom/main.py:161+` defines the existing `/api/v1/convert` HTTP contract that the new Rust backend must match exactly. The Rust port is measured against this code.
- The legacy bot **continues to run** on PythonAnywhere as a parallel fallback throughout the migration. If anything goes wrong with the new stack, the old bot is still answering AMP comments on Reddit.

## Read-only — do not run, modify, or import

- Don't run anything in here. The dependencies (`requirements.txt`) target Python 3.8-ish; the bot relies on `praw`, `tweepy`, `newspaper3k`, `mysql-connector`, etc.
- Don't import from `archive/` in any new code. The `backend/`, `devvit-app/`, and `website/` projects are independent.
- Don't change the canonical-finding logic here. If a bug surfaces, fix it in the Rust port — the Python is frozen.

## What's here

| Path | What it is |
|---|---|
| `check_comments.py`, `check_submissions.py`, `check_inbox.py` | PRAW stream loops (every 120 s) for comments, posts, and inbox. |
| `check_tweets.py` (was present, dropped during M1) | Inactive Twitter bot, not archived. |
| `helpers/` | Canonical-finding methods, URL extraction, AMP detection, reply formatting. |
| `helpers/canonical_methods.py` | The 10 `CanonicalType` methods. Reference for the Rust port. |
| `helpers/article_comparer.py` | `newspaper3k` + `SequenceMatcher` similarity scoring used by `GUESS_AND_CHECK`. Replaced by `dom_smoothie` in the Rust port. |
| `datahandlers/` | MySQL (via SSH tunnel) + local-file state. |
| `models/` | SQLAlchemy ORM + domain types (`Link`, `Canonical`, `CanonicalType`, …). |
| `static/static.py` | Constants + secrets (gitignored, local only). The 14 AMP keywords are at line 8-9. |
| `static/static.txt` | Placeholder/template version of `static.py` — checked into git. |
| `amputatorbotcom/` | Flask website + `/api/v1/convert` endpoint. The contract the Rust impl must match. |
| `data/` | Bot state on disk (gitignored). Allowlist/denylist subreddits, processed-id lists, etc. Personal data — not in git. |
| `img/` | Logos and screenshots. |
| `test*.py` | Old test scripts. |
| `requirements.txt` | Legacy Python deps. |

## Migration plan

See `../docs/amputatorbot-devvit-migration-plan-v7.md` for the full migration. Highlights:

- The new Rust backend in `../backend/` replaces this entire archived bot's API + canonical-finding logic.
- The new Devvit app in `../devvit-app/` replaces the PRAW streams.
- The new Astro site in `../website/` replaces the Flask website in `amputatorbotcom/`.
- The legacy `URLConversions` table (~1.7M rows, 42.56 MB) gets migrated to Scaleway Managed Postgres in M3. CSV exports live in `../backend/tests/fixtures/urlconversions/` for parity testing.

## Credentials

The credentials in `static/static.py` (gitignored, on Joris's machine only) are the *live* production credentials for `u/AmputatorBot` Reddit OAuth, the legacy Twitter API, the PythonAnywhere SSH/MySQL accounts, etc. **Once the migration is complete and the old bot is retired (some indeterminate future date)**, these should be rotated as part of decommissioning. Until then they remain valid so the legacy bot keeps running.
