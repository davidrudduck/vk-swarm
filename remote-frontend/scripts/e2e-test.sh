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
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
COMPOSE_DIR="$REPO_ROOT/crates/remote"
COMPOSE_FILE="$COMPOSE_DIR/docker-compose.dev.yml"
ENV_FILE="$COMPOSE_DIR/.env.dev"
SEED_FILE="$COMPOSE_DIR/scripts/seed-e2e-db.sql"

# -----------------------------------------------------------------------------
# Isolation from any OTHER stack on this machine.
#
# Compose derives its project name from the directory name, so this script used
# to run as project "remote" — the SAME project a deployed hive checkout
# (`.../vk-swarm/crates/remote`) uses. Two consequences, both bad:
#
#   * `up -d --build` ADOPTS that project's running `remote-server`/`remote-db`
#     containers and recreates them from this checkout's config.
#   * the EXIT trap's `down -v` then DESTROYS them — and the trap is registered
#     before the first command, so even an early failure (a missing .env.dev)
#     tears down the other stack on the way out.
#
# Pinning the project name and using non-default ports keeps an E2E run
# completely disjoint from a deployment, so the two can coexist.
# -----------------------------------------------------------------------------
export COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-vkswarm-e2e}"
# Not 9100/5434: 9100 is Prometheus node_exporter's default and 5434 is the
# deployed hive's Postgres. These are picked to avoid both.
export SERVER_PORT="${SERVER_PORT:-9210}"
export POSTGRES_PORT="${POSTGRES_PORT:-5540}"
E2E_BASE_URL="http://localhost:${SERVER_PORT}"
export SERVER_PUBLIC_BASE_URL="${SERVER_PUBLIC_BASE_URL:-$E2E_BASE_URL}"
export VITE_API_BASE_URL="${VITE_API_BASE_URL:-$E2E_BASE_URL}"
export VITE_APP_BASE_URL="${VITE_APP_BASE_URL:-$E2E_BASE_URL}"

# `--env-file` on a missing file is a hard compose error. Every variable the
# compose file reads has an inline default, so an absent .env.dev is fine —
# just omit the flag rather than failing (and triggering the teardown trap).
COMPOSE_ENV_ARGS=()
if [ -f "$ENV_FILE" ]; then
    COMPOSE_ENV_ARGS=(--env-file "$ENV_FILE")
fi

dc() { docker compose -f "$COMPOSE_FILE" "${COMPOSE_ENV_ARGS[@]}" "$@"; }

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
    if [ "$SKIP_DOCKER" = true ] || [ "$SEED_ONLY" = true ]; then
        return 0
    fi
    if [ "$KEEP_RUNNING" = false ]; then
        log "Tearing down Docker environment (project: $COMPOSE_PROJECT_NAME)..."
        cd "$COMPOSE_DIR"
        dc down -v 2>/dev/null || true
        ok "Docker environment stopped."
    else
        log "Docker environment kept running (--keep)."
        log "To stop: cd crates/remote && COMPOSE_PROJECT_NAME=$COMPOSE_PROJECT_NAME docker compose -f docker-compose.dev.yml down -v"
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
    local found=0
    while [ $elapsed -lt $max_wait ]; do
        if dc exec -T remote-db \
            psql -U postgres -d vibe_remote -c "SELECT 1 FROM users LIMIT 1" >/dev/null 2>&1; then
            found=1
            break
        fi
        sleep 2
        elapsed=$((elapsed + 2))
    done
    if [ "$found" -eq 0 ]; then
        err "Migrations did not complete within ${max_wait}s — users table not found"
        return 1
    fi

    dc exec -T remote-db \
        psql -U postgres -d vibe_remote -f /dev/stdin < "$SEED_FILE"
    ok "Database seeded."
}

# =============================================================================
# MAIN
# =============================================================================

cd "$REPO_ROOT"

# Step 1: Docker
if [ "$SKIP_DOCKER" = false ]; then
    # A port already in use means something else is listening — quite possibly a
    # deployed hive. Abort BEFORE `up`, while the trap has nothing to tear down,
    # rather than colliding and then cleaning up someone else's stack.
    for port in "$SERVER_PORT" "$POSTGRES_PORT"; do
        if ss -ltn "sport = :$port" 2>/dev/null | grep -q LISTEN; then
            err "Port $port is already in use — refusing to start the E2E stack."
            err "Something else (a deployed hive?) is listening. Free the port, or"
            err "re-run with SERVER_PORT/POSTGRES_PORT set to unused values."
            SKIP_DOCKER=true   # neutralise the teardown trap; we started nothing
            exit 1
        fi
    done

    log "Starting Docker environment (project: $COMPOSE_PROJECT_NAME, port: $SERVER_PORT)..."
    cd "$COMPOSE_DIR"
    dc up -d --build
    cd "$REPO_ROOT"

    # Wait for server
    wait_for_health "$E2E_BASE_URL/v1/health" 120

    # Seed with comprehensive E2E data
    seed_database
else
    log "Skipping Docker (--skip-docker). Assuming server at $E2E_BASE_URL."
    if ! curl -sf "$E2E_BASE_URL/v1/health" >/dev/null 2>&1; then
        err "Server not healthy at $E2E_BASE_URL. Run without --skip-docker first."
        exit 1
    fi
fi

if [ "$SEED_ONLY" = true ]; then
    ok "Seed complete (--seed-only). Docker environment is running."
    exit 0
fi

# Step 2: Run Playwright tests against Docker environment
log "Running Playwright E2E tests against $E2E_BASE_URL ..."
cd "$REPO_ROOT/remote-frontend"

# Set baseURL to Docker environment
export PLAYWRIGHT_BASE_URL="$E2E_BASE_URL"

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
