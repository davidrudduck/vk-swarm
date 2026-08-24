---
id: "019"
phase: 4
title: "Add the deployed verify script with an executable test that proves its positive and negative cases"
status: ready
depends_on: ["013","014"]
parallel: false
conflicts_with: []
files:
  - "scripts/verify-local-node-browser-oauth.sh"
  - "scripts/test-verify-local-node-browser-oauth.sh"
siblings: ["scripts/clean-cargo-cache.sh","scripts/check-i18n.sh","scripts/assert-dockerfile-node-match.sh","scripts/dev-swarm-setup.sh"]
irreversible: false
scope_test: "scripts/test-verify-local-node-browser-oauth.sh"
allowed_change: create
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
File: `scripts/test-verify-local-node-browser-oauth.sh` — create. It is the FAILING TEST for the verifier: written first, it fails until `verify-local-node-browser-oauth.sh` exists and behaves.

It stands up a temporary fake node with `node -e` (guaranteed present — this is a pnpm repo; no python, no jq, no network), runs the verifier against it, and asserts the exit code. The fake server is parameterised by a FIXTURE name so one server script serves every case:

```bash
#!/usr/bin/env bash
# Executable test for scripts/verify-local-node-browser-oauth.sh.
set -euo pipefail

FAKE_PID=''
FAKE_URL=''
FAKE_READY=''

cleanup_fake() {
  if [ -n "${FAKE_PID:-}" ]; then
    kill "$FAKE_PID" 2>/dev/null || true
    wait "$FAKE_PID" 2>/dev/null || true
    FAKE_PID=''
  fi
  if [ -n "${FAKE_READY:-}" ]; then
    rm -f "$FAKE_READY"
    FAKE_READY=''
  fi
}
trap cleanup_fake EXIT

start_fake() {   # $1 = fixture; sets FAKE_URL and FAKE_PID in THIS shell
  local ready=0 port
  cleanup_fake
  FAKE_READY=$(mktemp "${TMPDIR:-/tmp}/vlnbo-ready.XXXXXX")
  FIXTURE="$1" READY_FILE="$FAKE_READY" node -e '
    const fs = require("fs");
    const http = require("http");
    const f = process.env.FIXTURE;
    const server = http.createServer((req, res) => {
      const url = req.url.split("?")[0];
      const json = (code, body) => { res.writeHead(code, {"content-type":"application/json"});
                                     res.end(body); };
      if (url === "/api/health") return json(200, JSON.stringify({status:"ok"}));
      if (url === "/api/auth/state") {
        const data = {authorized:false, oauth_available:true};
        if (f === "leaky_state") data.access_token = "leaked";
        if (f === "extra_state") data.credentials_path = "/tmp/private";
        return json(200, JSON.stringify({success:true, data,
              error_data:null, message:null}));
      }
      if (url === "/api/info" || url === "/api/projects" || url === "/api/auth/status")
        return json(f === "open_info" ? 200 : 401, "{}");
      if (url === "/api/events") return json(401, "{}");
      if (/^\/api\/logs\/.*\/live$/.test(url)) return json(401, "{}");
      if (f === "spa_fallback") { res.writeHead(200, {"content-type":"text/html"});
                                   return res.end("<!DOCTYPE html><html></html>"); }
      return json(404, JSON.stringify({success:false}));
    });
    server.listen(0, "127.0.0.1", () =>
      fs.writeFileSync(process.env.READY_FILE, String(server.address().port)));
  ' &
  FAKE_PID=$!

  for _ in $(seq 1 50); do
    if [ -s "$FAKE_READY" ]; then ready=1; break; fi
    if ! kill -0 "$FAKE_PID" 2>/dev/null; then break; fi
    sleep 0.1
  done
  if [ "$ready" -ne 1 ]; then
    echo "fake server did not become ready" >&2
    return 1
  fi
  port=$(<"$FAKE_READY")
  FAKE_URL="http://127.0.0.1:$port"
}

run_case() {   # $1 = fixture, $2 = expected exit status, $3 = label
  local status
  start_fake "$1"
  set +e
  bash scripts/verify-local-node-browser-oauth.sh "$FAKE_URL" \
    >"${TMPDIR:-/tmp}/vlnbo.$$.out" 2>&1
  status=$?
  set -e
  cleanup_fake
  if [ "$status" -ne "$2" ]; then
    echo "FAIL $3: expected exit $2, got $status"
    cat "${TMPDIR:-/tmp}/vlnbo.$$.out"
    exit 1
  fi
  echo "PASS $3"
}

run_case compliant    0 'compliant node passes'
run_case open_info    1 'NEGATIVE: /api/info returning 200 must FAIL the verifier'
run_case spa_fallback 1 'NEGATIVE: unknown /api falling through to SPA html must FAIL'
run_case leaky_state  1 'NEGATIVE: auth state leaking a token field must FAIL'
run_case extra_state  1 'NEGATIVE: auth state carrying any extra field must FAIL'
rm -f "${TMPDIR:-/tmp}/vlnbo.$$.out"
echo 'ALL VERIFIER TESTS PASSED'
```

The four negative cases are the point: a verifier that exits 0 on all five fixtures proves nothing, and `open_info` is exactly what a node built from `main` looks like.


## Change
**File:** `scripts/verify-local-node-browser-oauth.sh` — create, `chmod +x`.
**Anchor:** new file. The spec's `verify_cmd` is `bash scripts/verify-local-node-browser-oauth.sh`, so this path is fixed.
**Before:** (does not exist)
**After:** a `set -euo pipefail` script in the house style. Contract:
```bash
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
  got=$(curl -s -o /dev/null -w '%{http_code}' "$BASE_URL$2" || echo 000)
  if [ "$got" = "$3" ]; then echo "PASS $1"; else
    echo "FAIL $1 (expected $3, got $got)"; failures=$((failures + 1)); fi
}

# 1. Public surface reachable without a session.
check_status 'health is public'      /api/health     200
check_status 'auth state is public'  /api/auth/state 200
#    ...and the auth state is structurally MINIMAL. ApiResponse serializes all four wrapper
#    fields; its data object must contain exactly two Boolean fields and nothing else.
state=$(curl -s "$BASE_URL/api/auth/state")
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
unknown_status=$(curl -s -o /dev/null -w '%{http_code}' "$BASE_URL/api/__does_not_exist__")
unknown_ctype=$(curl -s -o /dev/null -w '%{content_type}' "$BASE_URL/api/__does_not_exist__")
if [ "$unknown_status" = "404" ]; then echo 'PASS unknown api path is 404'; else
  echo "FAIL unknown api path (expected 404, got $unknown_status)"; failures=$((failures + 1)); fi
case "$unknown_ctype" in text/html*) echo 'FAIL unknown api path fell through to SPA html';
    failures=$((failures + 1));; *) echo 'PASS unknown api path is not SPA html';; esac

if [ "$failures" -gt 0 ]; then
  echo "$failures check(s) failed"; exit 1
fi
echo 'All browser-authorization boundary checks passed'
```

**File:** `scripts/test-verify-local-node-browser-oauth.sh` — create, `chmod +x`, exactly as in the Failing-test section. `run_case` calls `start_fake` directly, never through command substitution, so `FAKE_PID` remains in the parent shell and the EXIT trap can always reap it. The Node process binds port 0 itself and publishes the selected port through a temporary readiness file, avoiding a free-port probe/rebind race.

**Sibling alignment (rubric 9).** Read `scripts/clean-cargo-cache.sh` and `scripts/check-i18n.sh` before writing: they are the house bash conventions in this directory (shebang, `set -euo pipefail`, plain PASS/FAIL lines, non-zero exit on any failure, no GNU-only flags). Match them; do not introduce a new output format or a dependency on jq.

**Symbol grounding:** This task introduces the shell functions `check_status()` in `verify-local-node-browser-oauth.sh` and `start_fake()` / `run_case()` in `test-verify-local-node-browser-oauth.sh`. It introduces no Rust or TypeScript symbols and calls no symbol defined by another task.


## Allowed moves
[
  "Create exactly the two script files and make both executable.",
  "The verifier must be read-only against the node: GETs only, no login, no mutation.",
  "Use only Bash, curl, Node and standard host utilities already used by repository scripts. No jq, no python, no network access beyond BASE_URL.",
  "No production code, doc or configuration changes."
]


## STOP triggers
[
  "The verifier requiring jq, a session cookie, or any credential — it is an outside-observer check.",
  "The unknown-path check asserting a status without also asserting the content-type — a 200 text/html SPA fallback is exactly the failure this check exists to catch.",
  "The test script passing all four fixtures — if the three negative cases do not FAIL the verifier, the verifier certifies nothing; fix the verifier, never the fixture.",
  "Replacing the readiness poll with a bare `sleep` — that is how this test becomes flaky in CI.",
  "Leaving the fake server process running after a case (each run_case must kill and wait).",
  "Writing the doc page or touching docs/docs.json here — that is task 020.",
  "Calling `url=$(start_fake ...)` — command substitution runs the function in a subshell and loses FAKE_PID, so cleanup cannot kill or wait for the server.",
  "Probing an ephemeral port with one process and reopening it with another — let the fake HTTP server bind port 0 and report its actual port through the readiness file.",
  "Validating auth state with a field-name blacklist or substring search — parse JSON and require the exact ApiResponse wrapper and exact two-Boolean data shape."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `bash -n scripts/verify-local-node-browser-oauth.sh && bash -n scripts/test-verify-local-node-browser-oauth.sh` exits 0.
2. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="true" WAI_TEST_CMD="bash {scope}" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 019` exits 0. The runner MUST be pinned: a `.sh` scope_test has no auto-detected runner.
3. `bash scripts/test-verify-local-node-browser-oauth.sh` — prints five fixture PASS lines and `ALL VERIFIER TESTS PASSED`.
4. Against the REAL feature-branch node (`pnpm run dev`): `bash scripts/verify-local-node-browser-oauth.sh http://127.0.0.1:<BACKEND_PORT>` exits 0. Paste the output into the ledger. ORCHESTRATOR-ONLY — the implementer must not start a dev server.
5. Against a node built from `main` (no boundary), the same command MUST exit non-zero. Paste that too — the `open_info` fixture is a stand-in for this, and this step confirms the stand-in is faithful. ORCHESTRATOR-ONLY.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 019` exits 0
