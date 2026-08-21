---
id: "021"
phase: 4
title: "Run the real trusted-LAN acceptance and the full repository gates, and record the WAI close evidence"
status: ready
depends_on: ["015","017","018","019","020"]
parallel: false
conflicts_with: []
files:
  - "dev-docs/workstreams/local-node-browser-oauth/README.md"
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: []
covers_tests: ["TS7"]
---
## Failing test (write first)
N/A — TS7 is a REAL non-loopback browser acceptance run against the feature-branch build. It cannot be automated inside this repository (sidecar assumption A6 is `unprobed` precisely because it needs a real Hive deployment and a second physical browser on the LAN). Verification is the `## Manual verification` block below; the deliverable is the recorded evidence in the workstream README.


## Change
**File:** `dev-docs/workstreams/local-node-browser-oauth/README.md`
**Anchor:** end of the file, after the existing `# local-node-browser-oauth` heading and body.
**Before:**
```markdown
# local-node-browser-oauth

local-node-browser-oauth
```
**After:** the same, followed by an `## Acceptance evidence (TS7)` section containing, verbatim, the results of every numbered check in Manual verification below: the command run, the observed output or browser observation, and the date.

The workstream README is the IMPLEMENTATION record. It is NOT the WAI close evidence — that lives in `docs/plans/local-node-browser-oauth/decisions-ledger.md` and is written as part of Manual verification section D below. The ledger sits inside the task directory, which the gate excludes from the commit scan, so writing it does not violate this task's `files:` list.

No code changes. This task's job is to prove the whole feature on a real deployment and to leave the evidence where both the workstream README and the WAI close step can find it.

**Symbol grounding:** This task introduces no symbols. It runs the verifier `check_status()` chain shipped by task 019 and records the result; every other symbol it exercises is introduced by tasks 001-018.

**Ledger convention.** Do not add `docs/plans/local-node-browser-oauth/decisions-ledger.md` to `files:` merely because this task appends evidence. WAI explicitly permits append-only writes under `docs/plans/$TOPIC/*`. During execution append only the required `## Reachability gate` and `## Deploy verification` sections; no other ledger section is authored by task 021.



## Allowed moves
[
  "Append the acceptance-evidence section to the workstream README.",
  "No code, test, doc or configuration changes of any kind."
]


## STOP triggers
[
  "Any gate in section A failing — STOP and fix it in THIS session in the owning task (CLAUDE.md: no deferred remediation). Do not record a failing gate as 'known'.",
  "Substituting a loopback browser for the required non-loopback LAN browser — assumption A6 is exactly what this task exists to probe, and 127.0.0.1 does not exercise the trusted-LAN path.",
  "Recording an acceptance step as passed without pasting the observed evidence.",
  "Treating the workstream README as the close evidence — the `## Reachability gate` and `## Deploy verification` sections in docs/plans/local-node-browser-oauth/decisions-ledger.md are mandatory and wai-evidence.sh must pass.",
  "Running wai-evidence.sh before the deploy output has been pasted into a fenced code block — an empty or placeholder section is a failed gate, not a formality.",
  "Discovering a behaviour gap that needs code — STOP, amend the owning task and re-run its gate; do not patch production code from this task.",
  "Any edit to the frozen spec or its .decisions.json sidecar — their shas are pinned in .precheck.passed; a change there requires a deliberate amendment plus a /wai:precheck re-freeze.",
  "Adding decisions-ledger.md to task files solely for evidence append — preserve the WAI docs/plans append-only exemption and write only Reachability gate / Deploy verification."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
**A. Full repository gates** (all must be green; paste output):
1. `cargo fmt --all -- --check`
2. `cargo clippy --all --all-targets --all-features -- -D warnings`
3. `cargo test --workspace`
4. `npm run generate-types:check`
5. `cd frontend && npm run lint && npm run format:check && npx tsc --noEmit && npx vitest run`
6. `cd remote-frontend && npm run lint && npx tsc --noEmit && npx vitest run`
7. `bash scripts/check-i18n.sh`
8. `bash scripts/test-verify-local-node-browser-oauth.sh`

**B. Real non-loopback trusted-LAN acceptance (TS7)** — feature-branch build, served on `HOST=0.0.0.0`, reached from a DIFFERENT machine's browser by LAN IP, against a real Hive:
1. **Two-browser isolation:** browser A completes Hive OAuth and reaches the board. Browser B (clean profile, different machine or a private window) shows only the login shell and 401s on `/api/info`. Copy A's callback URL into B — it must not authorize B. Record both observations.
2. **Restart persistence:** `pnpm run stop`, then restart the node. Browser A reloads and is still signed in with no re-auth. Record.
3. **Transient Hive outage:** block or stop Hive; confirm A can still list projects, open a task, view live logs and receive SSE updates, and that a NEW sign-in from B fails. Restore Hive and confirm nothing was revoked. Record.
4. **Logout scope:** sign out in A; A returns to the login shell, a third signed-in browser C is unaffected, and the node is still synchronising with Hive. Record.
5. **Disconnect scope:** Settings -> Swarm -> Disconnect from Hive; every browser returns to the login shell, sync stops, and a DIFFERENT Hive account attempting to sign in is refused (owner retained). Record.
6. **Deployed verifier:** `bash scripts/verify-local-node-browser-oauth.sh http://<lan-ip>:<port>` from the other machine — every check PASS, exit 0. KEEP THIS OUTPUT: section D requires it verbatim.
7. **Non-disclosure spot check:** with devtools open on browser A, confirm `document.cookie` does not show `vks_browser_session` (it is HttpOnly) and that no request/response body, URL or storage entry contains a Hive access or refresh token. Record.

**C. Sidecar assumption A6** — RECORD the outcome of section B as the probe evidence for A6 ("a real Hive deployment and non-loopback browser can be made available") in the workstream README and the decisions ledger. Do NOT edit `docs/superpowers/specs/2026-08-21-local-node-browser-oauth.decisions.json`: it is frozen and its sha is pinned in `docs/plans/local-node-browser-oauth/.precheck.passed`. If the sidecar's `status` field itself must change from `unprobed`, that is a deliberate spec amendment requiring a `/wai:precheck` re-freeze — escalate rather than editing it from this task.

**D. WAI close evidence — REQUIRED BEFORE THE WORKSTREAM CAN CLOSE.** The workstream README alone does NOT satisfy it. In `docs/plans/local-node-browser-oauth/decisions-ledger.md`, add two NON-EMPTY sections:
- `## Reachability gate` — for each of SC1..SC10 and TS1..TS7, the owning task id and the specific test name or observation that discharges it, plus an explicit statement that every new route, middleware and frontend component added by this plan is reachable from a real entry point (name the entry point: the served router for the Rust paths, `App.tsx` -> `AuthBoundary` for the frontend). Any row without evidence blocks the close.
- `## Deploy verification` — the section-B6 deployed verifier run, with the exact command and its full output inside a fenced code block, plus the node's LAN URL, the build's commit sha and the date.
Then run `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/wai-evidence.sh"; bash "$WAI_ROOT/scripts/wai-evidence.sh" local-node-browser-oauth` and confirm it exits 0. If it reports a missing or empty section, fix the ledger and re-run — do not proceed to close with a failing evidence gate.

**E. Criterion roll-call** — the `## Reachability gate` table from section D is that roll-call; confirm it lists SC1..SC10 and TS1..TS7 with no gaps.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 021` exits 0
