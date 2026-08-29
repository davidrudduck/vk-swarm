---
id: "001"
phase: 1
title: "Live repro script: two-leg WAL-unlink harness (harness mechanics; red proved at 002)"
status: ready
depends_on: []
parallel: false
conflicts_with: ["002"]
files:
  - "scripts/live/wal-unlink-durability-repro.sh"
siblings: ["scripts/verify-local-node-browser-oauth.sh"]
irreversible: false
scope_test: "N/A"
allowed_change: create
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
N/A — the deliverable is a bash script. Amendment 2026-08-29 (operator-approved re-sequence): the red proof MOVED to task 002, because the incident's lock-state trigger window must be pinned empirically (002 VERDICT 1) before the harness can reliably fire the unlink on current code — on a fresh scratch node the pool holds the wal-index and the external close does NOT unlink (spec line 16 predicted exactly this window). 001's gate is harness MECHANICS only: MODE=baseline green on both legs (lifecycle, two-boot seeding, project/task setup, timings, journal_mode assert all work end-to-end) and a full-mode run that exits 1 with both legs COMPLETED (no ABORTED) and per-leg assertion counts in the summary (legs will report FAIL on current code — expected; not a defect here). Gate env for this task: WAI_TYPECHECK_CMD="true" WAI_TEST_CMD="true" WAI_LINT_CMD="true".


## Change
Create `scripts/live/wal-unlink-durability-repro.sh` (new directory), mode 0755, `set -euo pipefail`, matching the house style of scripts/verify-local-node-browser-oauth.sh (check_status helper, PASS/FAIL counter, exit 1 on any failure). Read that sibling first and mirror its structure; justify divergences in the decisions ledger.

The script runs TWO legs against isolated scratch nodes, each in its own mktemp -d subdir under ${SCRATCH_ROOT:-$(mktemp -d /tmp/wal-repro.XXXXXX)}:

RUN SELECTORS: `LEGS=A|B|AB` (default AB) selects which legs run. `MODE=full` (default) runs the leg contracts below; `MODE=baseline` runs a REDUCED contract — boot, seed, 5 timing writes, graceful stop, offline journal_mode assert — with NO external unlink and NO fixed-code assertions, so the SAME script measures an unfixed main-built binary (task 040's SC3 baseline needs comparable SUCCESSFUL runs from both binaries).

PREFLIGHT (exit 2 on any violation): port 9012 free (`! (exec 3<>/dev/tcp/127.0.0.1/9012) 2>/dev/null`); `command -v sqlite3`; `command -v curl`; node binary exists at ${BINARY:-target/release/vks-node-server} (print `cargo build --release -p server --bin vks-node-server` hint if absent); `unset VK_HIVE_URL VK_NODE_API_KEY VK_NODE_NAME VK_NODE_PUBLIC_URL VK_WAL_GUARD` at script start (a caller's stray VK_WAL_GUARD=off must not silently flip leg A); NEVER echo VK_NODE_API_KEY / VK_CONNECTION_TOKEN_SECRET / credentials.json contents.

PER-LEG NODE LIFECYCLE (function run_node <legdir> <extra-env...>): launch `HOST=0.0.0.0 BACKEND_PORT=9012 VK_ASSET_DIR=<legdir> VK_DATABASE_PATH=<legdir>/db.sqlite VK_BACKUP_DIR=<legdir>/backup VK_WORKTREE_DIR=<legdir>/worktrees VK_LOG_DIR=<legdir>/logs "$BINARY" > <legdir>/node.log 2>&1 &`; capture `PID=$!` ONLY and append it to a script-scope NODE_PIDS array; ONE script-scope cleanup trap — `trap 'for p in "${NODE_PIDS[@]:-}"; do kill "$p" 2>/dev/null || true; done' EXIT` (a `trap ... RETURN` inside a function is unreliable under set -e; exact-PID management still applies — never pkill/killall); wait_health: poll GET http://127.0.0.1:9012/api/health until `database_ready":true` (max 30s).

SESSION SEEDING (all /api routes except /api/health require a browser session cookie — routes/mod.rs L52-95, session.rs L54-70; NO loopback exemption): two-boot pattern per leg. Boot 1: run node to healthy, graceful stop (kill $PID, wait). Then offline: `RAW=$(head -c 32 /dev/urandom | sha256sum | cut -d' ' -f1)`; `HASH=$(printf '%s' "$RAW" | sha256sum | cut -d' ' -f1)` (matches hash_token, seams.rs L77-85); `ID=$(head -c 16 /dev/urandom | od -An -tx1 | tr -d ' \n')`; `HIVE=$(head -c 16 /dev/urandom | od -An -tx1 | tr -d ' \n')`; `sqlite3 <legdir>/db.sqlite "INSERT INTO browser_sessions (id, token_hash, hive_user_id, created_at, revoked_at) VALUES (X'$ID', '$HASH', X'$HIVE', $(date +%s%3N), NULL);"` (migration 20260821000000_add_browser_auth.sql L41-48; no expiry check — revoked_at IS NULL = live). Boot 2: relaunch; use `curl -b <legdir>/jar -H "Cookie: vks_browser_session=$RAW"` (cookie name from cookies.rs L11) for all API calls.

PROJECT SETUP (once per leg, after boot2): `REPO_DIR=$(mktemp -d)/repo && git init -q "$REPO_DIR"`; POST /api/projects with CreateProject{name:"repro-<leg>", git_repo_path:$REPO_DIR, use_existing_repo:true} (models/project/mod.rs L78-88). Expect http **200** — create_project returns 200 (there is NO StatusCode::CREATED anywhere in crates/server/src) — AND `.success==true` in the body (a duplicate git_repo_path ALSO returns 200 with an ApiResponse error envelope); capture PROJECT_ID from `.data.id`. API WRITE HELPER (api_write <marker>): POST /api/tasks with CreateTask{project_id:$PROJECT_ID, title:<marker>} (models/task/mod.rs L104-112) expecting http 200 + `.success==true`; echo the task id.

TRIP DETECTOR (O7): poll `ls -l /proc/$PID/fd` for `db.sqlite-wal (deleted)` every 0.5s up to 30s (do NOT rely on fixed sleeps).

LEG A — guard-on durability (SC1): leg-A node boots with `VK_WAL_GUARD=on` exported explicitly (guard active once Phase 4 wires it; on current code a no-op). Steps: seed+boot2; api_write "marker-A-pre"; external CLI read `sqlite3 <legdir>/db.sqlite 'SELECT count(*) FROM tasks;'`; wait trip-detector (on current code the WAL unlinks ONLY under the VERDICT-1 trigger window that 002 encodes — before that encoding a detector timeout is UNINFORMATIVE, not a pass; after the fix the guard prevents the unlink and a timeout is the EXPECTED pass-signal); api_write "marker-A-post"; graceful stop; offline: `sqlite3 <legdir>/db.sqlite "SELECT count(*) FROM tasks WHERE title='marker-A-post';"` must be 1, and `PRAGMA journal_mode;` must print `wal`.

LEG B — guard-off detection+refusal (SC2): relaunch leg-B node with `VK_WAL_GUARD=off`; seed+boot2; api_write "marker-B-pre"; CLI read; trip-detector MUST fire (fail leg if timeout after O8 retry: one retry of the CLI step on a fresh scratch DB); assert `grep -c 'wal_unlinked_externally' <legdir>/node.log` >= 1 and the log line names the db path; api_write "marker-B-post" MUST be rejected: expect a DB-failure signal — non-2xx status OR `.success==false` envelope (capture BOTH curl %{http_code} and the body); assert `grep -c 'wal_write_refusal_active' <legdir>/node.log` >= 1; assert node process still alive (`kill -0 $PID`); graceful stop; offline: `SELECT count(*) FROM tasks WHERE title='marker-B-post';` MUST be 0 (a post-trip write that 'succeeds' but never lands is the incident's exact failure shape).

TIMINGS (SC3 raw material): in leg A, wrap 5 api_write "timing-N" calls with `date +%s%N` deltas, append `write_latency_ms=<delta>` lines to $SCRATCH_ROOT/timings.txt.

Print a final PASS/FAIL summary per leg; exit 1 if any assertion failed. The summary MUST print per-leg assertion counts (PASS/FAIL/total). A leg that aborts mid-run must be reported as ABORTED (distinct from failed assertions) — for 001's gate an ABORTED leg is a defect: full-mode must exit 1 (assertion failures), never 2 (preflight) and never die silently under set -e. On CURRENT code the legs report failures (that becomes the RED state once 002 encodes the trigger window); 001's gate is: baseline mode exit 0, full-mode exit 1, both legs COMPLETED (no ABORTED) with per-leg assertion counts recorded in the ledger.


## Allowed moves
Read scripts/verify-local-node-browser-oauth.sh for house style. Create ONLY scripts/live/wal-unlink-durability-repro.sh. Do not modify any other file. Do not echo secrets. Kill only captured PIDs.


## STOP triggers
Port 9012 is occupied by a process you did not spawn — STOP and report (never touch the production node on :9002 either). The two-boot seeding flow cannot produce an authenticated session (API writes return 401/403 despite the seeded cookie) — STOP; the auth contract has drifted from routes/session.rs. (Amendment 2026-08-29: the "external CLI read never unlinks the WAL" STOP trigger MOVED to task 002 — on current code without the VERDICT-1 window the unlink is EXPECTED not to fire; that is 002's evidence hunt, not a 001 halt.)


Declared decision points (from the spec; do not edit here):
- DP1: T1 evidence shows the backup subsystem shares the unlink hazard, crossing this spec's out-of-scope boundary; continuing requires scope renegotiation with the operator, not silent scope growth.  [codes: human_gate_required]
- DP2: T1 refutes the guard-connection prevention (an external close still unlinks the WAL while the guard holds the wal-index lock), so D4 cannot be adopted as designed and the route must be re-settled with the operator.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
Build the release binary (`cargo build --release -p server --bin vks-node-server`). Run `MODE=baseline bash scripts/live/wal-unlink-durability-repro.sh` — MUST exit 0 (both legs green: boot, seed, 5 timing writes, graceful stop, offline journal_mode=wal). Then run `bash scripts/live/wal-unlink-durability-repro.sh` (full mode) on CURRENT code — expected exit 1 with both legs COMPLETED (no ABORTED); paste the summary block (including per-leg assertion counts) into the decisions ledger. The full-mode red proof against an encoded trigger window is task 002's deliverable, not this task's.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh wal-unlink-durability 001` exits 0
