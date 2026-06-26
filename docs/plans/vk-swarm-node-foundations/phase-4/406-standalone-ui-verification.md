---
id: "406"
phase: 4
title: Verify the node builds + serves its UI standalone with no hive env (SC6)
status: ready
depends_on: ["402","404","405"]
parallel: false
conflicts_with: []
files: []
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC6]
---
## Failing test (write first)

`N/A — this is a verification task, no code.` SC6 ("the node runs fully standalone and always serves
its scoped local UI; no headless flag") is asserted by the `## Manual verification` section below and a
recorded smoke result, not by a unit test. `files:` is empty: this task changes nothing, it observes.

## Change

None. This task runs commands and records evidence. If any check fails, do NOT patch here — open a
finding and route the fix to the responsible phase-4 task (402/403/404/405).

## Allowed moves

- Run the verification commands below.
- Append the recorded smoke result to the decisions-ledger.
- No source edits whatsoever (empty `files:`; the gate rejects any tracked-file change).

## STOP triggers

- The backend fails to start with the hive env UNSET (means standalone is broken — a real SC6 failure;
  record it and route to the owning task, do not work around it here).
- The frontend route does not return 200 (UI not served — SC6 failure).
- A headless/no-UI flag is discovered gating UI serving (contradicts ADR-0002 "no headless flag" — must
  not exist; record as a defect).

## Manual verification (record in decisions-ledger)

Run from a clean build of the phase-4 branch. Use a throwaway port to avoid clobbering a running dev
instance (`pnpm run stop --list` first; never `pkill`).

1. **No-hive env is truly unset** (standalone precondition):
   ```text
   unset VK_HIVE_URL VK_NODE_API_KEY VK_NODE_NAME VK_NODE_PUBLIC_URL
   env | grep -E '^VK_HIVE_URL=|^VK_NODE_API_KEY=' || echo "hive env unset OK"
   ```

   Expected: `hive env unset OK`.

2. **Build + run standalone** (release binary, no hive config):
   ```text
   pnpm run prod:build
   BACKEND_PORT=8099 HOST=127.0.0.1 ./target/release/vks-node-server &
   ```

   Expected: process starts and stays up (no panic, no "hive required" error). The node runner simply
   does not spawn (it returns `None` from `NodeRunnerContext::from_env` when `VK_HIVE_URL` is absent).

3. **UI is served unconditionally** (SC6 core):
   ```text
   curl -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:8099/
   curl -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:8099/projects
   ```

   Expected: `200` for both. The frontend root (`serve_frontend_root`) and SPA fallback
   (`serve_frontend`) are registered at `crates/server/src/routes/mod.rs:84-85`, OUTSIDE any
   conditional and with no headless gate — confirm by inspection that mod.rs:82-86 is:
   ```text
       Router::new()
           .nest("/api", base_routes)
           .route("/", get(frontend::serve_frontend_root))
           .route("/{*path}", get(frontend::serve_frontend))
           .into_make_service()
   ```

   and that no `headless`/`NO_UI`/`SERVE_UI` flag exists in `crates/` (only `remote/`'s unrelated
   "headless node sync" doc comments):
   ```text
   git grep -niE 'headless|no_?ui|serve_ui' crates/server crates/services crates/deployment
   ```

   Expected: no flag gating UI serving.

4. **Sync-status endpoint reports standalone** (cross-checks 405):
   ```text
   curl -s http://127.0.0.1:8099/api/database/sync-status | jq '{is_connected, hive_url, node_name}'
   ```

   Expected: `{"is_connected": false, "hive_url": null, "node_name": null}`.

5. **Tear down:** `kill %1` (the exact backgrounded PID — never `pkill`).

**Record** in `decisions-ledger.md` under a "Task 406 smoke result" heading: the two HTTP codes from
step 3, the JSON from step 4, the git-grep result (no headless flag), and PASS/FAIL for SC6.

## Done when

`WAI_TYPECHECK_CMD="true" WAI_TEST_CMD="true" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 406` exits 0

> Verification-only task: no compile/test runner applies (empty `files:` → nothing to type-check or
> test). The `true` overrides keep the gate honest (it still enforces "no tracked file changed");
> the real evidence is the recorded smoke result above (per schema: a task with `scope_test: N/A` is
> valid only with a `## Manual verification` section, which this has).
