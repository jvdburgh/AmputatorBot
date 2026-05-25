# AmputatorBot root tasks — fan out to per-project justfiles.

# Default: list recipes
default:
    @just --list

# Run all checks across all projects (CI equivalent locally)
check: backend-check devvit-check website-check

# Format every project
fmt: backend-fmt devvit-fmt website-fmt

# Lint every project
lint: backend-lint devvit-lint website-lint

# Run all tests
test: backend-test devvit-test website-test

# --- backend ---
backend-check:
    cd backend && just check

backend-fmt:
    cd backend && just fmt

backend-lint:
    cd backend && just lint

backend-test:
    cd backend && just test

backend-dev:
    cd backend && just dev

# --- devvit-app ---
devvit-check:
    cd devvit-app && just check

devvit-fmt:
    cd devvit-app && just fmt

devvit-lint:
    cd devvit-app && just lint

devvit-test:
    cd devvit-app && just test

devvit-dev:
    cd devvit-app && just dev

# --- website ---
website-check:
    cd website && just check

website-fmt:
    cd website && just fmt

website-lint:
    cd website && just lint

website-test:
    cd website && just test

website-dev:
    cd website && just dev

# --- one-time setup after clone ---
setup:
    mise install
    pnpm install -r
    lefthook install
    @echo "Setup complete. Try: just check"
