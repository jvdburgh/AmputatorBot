# AmputatorBot — Claude Code conventions

## Source of truth

The migration plan lives at `docs/amputatorbot-devvit-migration-plan-v7.md`. Read it before making architectural decisions. It supersedes anything memory or instinct suggests.

## Working style

- **Consult Joris before any architectural decision** — library/framework choice, schema design, module structure, cost/complexity tradeoffs, plan deviations. Mechanical implementation work doesn't need consultation.
- Push back honestly if you disagree with his instinct. The plan went through many revisions specifically because pushback produced better answers.

### External actions Joris does manually, not Claude

- Queries against production databases (write the SQL, hand it over)
- Signing up for services / OAuth logins (`devvit login`, `scw init`, billing changes)
- DNS changes in Cloudflare
- Stopping/starting the PythonAnywhere bot
- `git push` to production-affecting remotes
- Publishing the Devvit app (`devvit upload --publish`)

Claude prepares exact commands and tells Joris when to run them.

## Repo layout (post-M1)

- `backend/` — Rust (Axum). The API, canonical-finding, static-file serving. Single binary.
- `devvit-app/` — TypeScript Devvit app. Reddit triggers (comment/post/modmail).
- `website/` — Astro site. Built into the Rust container.
- `archive/` — old Python bot + Flask site. **Read-only reference.** Don't run, modify, or import from it. Preserved for canonical-finding logic reference and history.
- `docs/` — plan + reference docs.

## Tooling commands

Use `just` rather than raw cargo/pnpm:

- `just test` — run all tests for changed projects
- `just lint` — format + lint check
- `just fmt` — apply formatting
- `just dev` — local dev (per-project)

Each subproject has its own `justfile` with the same recipe names; the root `justfile` fans out.

Toolchain managed by `mise` — `mise install` reproduces the pinned Rust + Node versions.

## Tech stack (locked)

- Rust 1.80+, Axum 0.8, `sqlx`, `scraper`, `reqwest`, `dom_smoothie` (Mozilla Readability port for article-similarity scoring)
- TypeScript with `@devvit/web` (modern Devvit server model — not legacy Blocks)
- Astro 5 + Tailwind 4 + shadcn/ui
- Postgres 16 on Scaleway Managed Database (smallest tier — DB is 42.56 MB)
- Hosting: Scaleway Serverless Containers (Paris/AMS, EU)
- Lint + format: **Biome** (JS/TS), `rustfmt` + `clippy` (Rust)
- Type check: **`tsgo`** (TypeScript Native Preview, Go port), `astro check`
- Test: **`cargo nextest`** + `insta` (Rust), **Vitest** (TS)
- CI: GitHub Actions, path-filtered

## Don't

- Don't commit credentials. The `archive/` tree has stale ones (Reddit OAuth, MySQL, SSH, Twitter) — they need rotation, not propagation.
- Don't run code from `archive/`.
- Don't add backwards-compat shims to bridge old ↔ new. The old bot keeps running in parallel as fallback; no bridge needed.
- Don't replace `dom_smoothie` or the canonical-finding methods/order without explicit discussion — these are tuned.
- Don't change the public `GET /api/v1/convert` contract or response shape (`AmputatorBotCom/main.py:161+` is the reference). Both encoded and unencoded URLs must continue to work; for unencoded URLs, `q` must be the last query param.

## Memory

Persistent memory lives at `~/.claude/projects/-Users-jvdb-Projects-other-AmputatorBot/memory/`. Key entries: user background, hosting recommendation criteria, consult-on-architecture rule, v7 project context.

## Per-milestone session pattern

Start a fresh Claude Code session per milestone (M1–M5) to keep context tight. Open with: *"Continue M*N* of `docs/amputatorbot-devvit-migration-plan-v7.md`."* Memory carries the project decisions across sessions.
