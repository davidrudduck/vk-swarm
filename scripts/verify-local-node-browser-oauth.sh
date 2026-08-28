#!/usr/bin/env bash
# Deployed observation for the local-node-browser-oauth boundary.
#
# Usage: bash scripts/verify-local-node-browser-oauth.sh [BASE_URL]
#   BASE_URL defaults to http://127.0.0.1:${BACKEND_PORT:-8080}
#
# Asserts the boundary as an OUTSIDE observer with NO session cookie. Read-only: it never logs
# in, never mutates node state, and needs no credentials. Uses Bash, curl and Node (already required by this pnpm repo) -- NO jq and no Python.
# Node structurally parses auth-state JSON so an unknown extra field cannot evade a blacklist.
set -euo pipefail

BASE_URL="${1:-http://127.0.0.1:${BACKEND_PORT:-8080}}"
failures=0

check_status() {   # $1 label, $2 path, $3 expected status
  local got
  got=$(curl -s --connect-timeout 5 --max-time 15 -o /dev/null -w '%{http_code}' "$BASE_URL$2" || echo 000)
  if [ "$got" = "$3" ]; then echo "PASS $1"; else
    echo "FAIL $1 (expected $3, got $got)"; failures=$((failures + 1)); fi
}

# 1. Public surface reachable without a session.
check_status 'health is public'      /api/health     200
check_status 'auth state is public'  /api/auth/state 200
#    ...and the auth state is structurally MINIMAL. ApiResponse serializes all four wrapper
#    fields; its data object must contain exactly two Boolean fields and nothing else.
state=$(curl -s --connect-timeout 5 --max-time 15 "$BASE_URL/api/auth/state")
if printf '%s' "$state" | node -e '
  const body = JSON.parse(require("fs").readFileSync(0, "utf8"));
  const exact = (value, keys) => value && typeof value === "object" &&
    !Array.isArray(value) && JSON.stringify(Object.keys(value).sort()) ===
      JSON.stringify([...keys].sort());
  if (!exact(body, ["success", "data", "error_data", "message"]) ||
      body.success !== true || body.error_data !== null || body.message !== null ||
      !exact(body.data, ["authorized", "oauth_available"]) ||
      typeof body.data.authorized !== "boolean" ||
      typeof body.data.oauth_available !== "boolean") process.exit(1);
'; then
  echo 'PASS auth state has the exact minimal shape'
else
  echo 'FAIL auth state is malformed or carries unexpected fields'
  failures=$((failures + 1))
fi

# 2. Protected surface denied without a session.
check_status 'info is protected'      /api/info        401
check_status 'projects are protected' /api/projects    401
check_status 'status is protected'    /api/auth/status 401

# 3. Streams reject anonymously.
check_status 'events SSE is protected' /api/events 401
check_status 'live logs are protected' \
  /api/logs/00000000-0000-0000-0000-000000000000/live 401

# 4. Unknown API paths terminate INSIDE the api boundary. A status check alone is not enough:
#    the SPA catch-all answers 200 text/html for anything, so the content-type is the real
#    signal.
unknown_status=$(curl -s --connect-timeout 5 --max-time 15 -o /dev/null -w '%{http_code}' "$BASE_URL/api/__does_not_exist__")
unknown_ctype=$(curl -s --connect-timeout 5 --max-time 15 -o /dev/null -w '%{content_type}' "$BASE_URL/api/__does_not_exist__")
if [ "$unknown_status" = "404" ]; then echo 'PASS unknown api path is 404'; else
  echo "FAIL unknown api path (expected 404, got $unknown_status)"; failures=$((failures + 1)); fi
case "$unknown_ctype" in text/html*) echo 'FAIL unknown api path fell through to SPA html';
    failures=$((failures + 1));; *) echo 'PASS unknown api path is not SPA html';; esac

if [ "$failures" -gt 0 ]; then
  echo "$failures check(s) failed"; exit 1
fi
echo 'All browser-authorization boundary checks passed'
