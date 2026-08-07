---
id: "301"
phase: 3
title: "Full repo gates (exit 0) + live deploy verification evidence (SW registered sign-ins, cache inspection, stalled-flow timeout)"
status: ready
depends_on: ["102","202"]
parallel: false
conflicts_with: []
files:
  - "docs/plans/hive-oauth-sw-bypass/decisions-ledger.md"
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: ["SC1","SC2","SC3","SC4"]
covers_tests: ["TS3"]
---
## Failing test (write first)
N/A — covered by existing tests: the full gate set below; this task adds no code.


## Change
**File:** `docs/plans/hive-oauth-sw-bypass/decisions-ledger.md`

Append a `## Deploy verification` section containing the fenced command outputs enumerated in Manual verification. No source files change in this task.


## Allowed moves
Only appending evidence sections to the decisions ledger.


## STOP triggers
Any gate exits non-zero (fix belongs in the owning task — reopen it; if the failure is PRE-EXISTING on the baseline, follow CLAUDE.md: fix now, create a tracked named scope-split workstream in THIS session, or escalate — merely noting it is NOT permitted); live verification shows sign-in still failing with the SW registered (escalate: the out-of-scope node-handoff hypothesis may need promotion — spec change requires re-precheck).


## Manual verification (record in decisions-ledger)
Record ALL of the following in the decisions-ledger, fenced; every gate MUST exit 0:
1. `cargo clippy --all --all-targets --all-features -- -D warnings` — exit 0.
2. `cargo test --workspace` — exit 0.
3. `cd frontend && npm run lint && npx tsc --noEmit && npx vitest run` — exit 0.
4. `cd remote-frontend && npm run lint && npx tsc --noEmit && npx vitest run` — exit 0.
5. SC1 (both legs): deploy the feature-branch hive build to the running hive (docker compose per `crates/remote/docker-compose.dev.yml` or the operator's deploy path), then (a) `curl -fsS http://127.0.0.1:9000/sw.js | grep -c 'v1/oauth'` >= 1; (b) after the SC3 sign-in below, open DevTools → Application → Cache Storage → `api-cache` and record that it contains ZERO `/v1/oauth/*` entries (paste the entry list or a screenshot reference).
6. SC2/SC3 (operator evidence): with the hive SW REGISTERED in a normal window (no unregister, no incognito), complete GitHub sign-in from the node (node `/api/auth/status` flips to authenticated — paste the JSON) and on the hive (authenticated session). If the Network tab from a PRE-update SW trace is available, record which failure mechanism dominated (SW redirected-response rejection vs stale cache hit); if not reproducible post-fix, record 'mechanism: indeterminate — fix correct under both'.
7. SC4 (operator evidence, running node): start a sign-in and abandon the popup (do not complete auth); after POLL_DEADLINE_MS (120 s) record that `/api/auth/status` requests CEASE in the Network tab and the localized timeout error with the tryAgain button renders (screenshot reference + observation note).


## Done when
`WAI_ROOT="$(ls -d ~/.claude/plugins/cache/agent-plugins/wai/[0-9]*/ | sort -V | tail -1)"; WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit && cd ../remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cargo test --workspace" bash "$WAI_ROOT/scripts/task-gate.sh" hive-oauth-sw-bypass 301` exits 0
