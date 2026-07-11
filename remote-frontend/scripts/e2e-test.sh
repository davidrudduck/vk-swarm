#!/usr/bin/env bash
# e2e-test.sh — Spin up Docker environment, seed database, run Playwright tests, tear down
#
# Usage:
#   ./scripts/e2e-test.sh                    # Full E2E (Docker + Playwright)
#   ./scripts/e2e-test.sh --skip-docker      # Skip Docker (use existing)
#   ./scripts/e2e-test.sh --keep             # Keep Docker running after tests
#   ./scripts/e2e-test.sh --seed-only        # Just seed the database
#
# Prerequisites:
#   - Docker and Docker Compose installed
#   - pnpm installed
#   - Playwright browsers installed (pnpm -C remote-frontend exec playwright install chromium)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
COMPOSE_DIR="$REPO_ROOT/crates/remote"
COMPOSE_FILE="$COMPOSE_DIR/docker-compose.dev.yml"
ENV_FILE="$COMPOSE_DIR/.env.dev"
SEED_FILE="$COMPOSE_DIR/scripts/seed-e2e-db.sql"

SKIP_DOCKER=false
KEEP_RUNNING=false
SEED_ONLY=false

for arg in "$@"; do
    case "$arg" in
        --skip-docker) SKIP_DOCKER=true ;;
        --keep) KEEP_RUNNING=true ;;
        --seed-only) SEED_ONLY=true ;;
        *) echo "Unknown arg: $arg"; exit 1 ;;
    esac
done

# =============================================================================
# FUNCTIONS
# =============================================================================

log() { echo -e "\033[1;34m[e2e]\033[0m $*"; }
err() { echo -e "\033[1;31m[e2e]\033[0m $*" >&2; }
ok()  { echo -e "\033[1;32m[e2e]\033[0m $*"; }

cleanup() {
    if [ "$KEEP_RUNNING" = false ]; then
        log "Tearing down Docker environment..."
        cd "$COMPOSE_DIR"
        docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down -v 2>/dev/null || true
        ok "Docker environment stopped."
    else
        log "Docker environment kept running (--keep)."
        log "To stop: cd crates/remote && docker compose -f docker-compose.dev.yml --env-file .env.dev down -v"
    fi
}

# Register cleanup EARLY so it runs on any early failure (set -e exits before
# the late trap definition if Docker fails to start or migrations time out).
trap cleanup EXIT

wait_for_health() {
    local url="$1"
    local max_wait="${2:-60}"
    local elapsed=0
    log "Waiting for $url ..."
    while [ $elapsed -lt $max_wait ]; do
        if curl -sf "$url" >/dev/null 2>&1; then
            ok "Server healthy at $url"
            return 0
        fi
        sleep 2
        elapsed=$((elapsed + 2))
    done
    err "Server did not become healthy within ${max_wait}s"
    return 1
}

seed_database() {
    log "Seeding database with E2E test data..."
    cd "$COMPOSE_DIR"
    # Wait for migrations to complete (users table exists)
    local max_wait=60
    local elapsed=0
    while [ $elapsed -lt $max_wait ]; do
        if docker compose -f "$COMPOSE_FILE" exec -T remote-db \
            psql -U postgres -d vibe_remote -c "SELECT 1 FROM users LIMIT 1" >/dev/null 2>&1; then
            break
        fi
        sleep 2
        elapsed=$((elapsed + 2))
    done

    docker compose -f "$COMPOSE_FILE" exec -T remote-db \
        psql -U postgres -d vibe_remote -f /dev/stdin < "$SEED_FILE"
    ok "Database seeded."
}

# =============================================================================
# MAIN
# =============================================================================

cd "$REPO_ROOT"

# Step 1: Docker
if [ "$SKIP_DOCKER" = false ]; then
    log "Starting Docker environment..."
    cd "$COMPOSE_DIR"
    docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d --build 2>&1 | tail -5
    cd "$REPO_ROOT"

    # Wait for server
    wait_for_health "http://localhost:9000/v1/health" 120

    # Seed with comprehensive E2E data
    seed_database
else
    log "Skipping Docker (--skip-docker). Assuming server at localhost:9000."
    if ! curl -sf "http://localhost:9000/v1/health" >/dev/null 2>&1; then
        err "Server not healthy at localhost:9000. Run without --skip-docker first."
        exit 1
    fi
fi

if [ "$SEED_ONLY" = true ]; then
    ok "Seed complete (--seed-only). Docker environment is running."
    exit 0
fi

# Step 2: Run Playwright tests against Docker environment
log "Running Playwright E2E tests against http://localhost:9000 ..."
cd "$REPO_ROOT/remote-frontend"

# Set baseURL to Docker environment
export PLAYWRIGHT_BASE_URL="http://localhost:9000"

# Run Playwright with Docker config — temporarily disable set -e so we can
# capture the exit code and print a meaningful failure message before cleanup.
set +e
npx playwright test --config=playwright.docker.config.ts --reporter=list 2>&1
E2E_EXIT=$?
set -e

cd "$REPO_ROOT"

# (trap set at top of file — runs on any exit path)

if [ $E2E_EXIT -eq 0 ]; then
    ok "All E2E tests passed!"
else
    err "E2E tests failed (exit code: $E2E_EXIT)"
fi

exit $E2E_EXIT
