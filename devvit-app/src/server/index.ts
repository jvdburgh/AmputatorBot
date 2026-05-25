// M1 hello-world entry point.
// Triggers (comment-submit, post-submit, modmail, on-app-install) are wired in M4
// per docs/amputatorbot-devvit-migration-plan-v7.md.

// Intentionally empty — Devvit's bundler picks up handlers as they're added.
// Adding triggers later means: declare them in devvit.json + export the matching
// HTTP route handlers under src/server/routes/.
