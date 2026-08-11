---
id: "012"
phase: 5
title: "Record live acceptance for restart durability and full offline coverage"
status: ready
depends_on: ["006","007","008","009","010","011"]
parallel: false
conflicts_with: []
files:
  - "docs/plans/vk-swarm-event-bus/decisions-ledger.md"
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: ["SC5","SC8"]
covers_tests: ["TS6"]
---
## Failing test (write first)
N/A — this task records observations of a DEPLOYED node, which no unit test can assert.
Verification is the `## Manual verification` protocol below, per schema/task.frontmatter.md's
scope_test-OR-manual-verification rule. The underlying behaviour is already covered by the tests in
tasks 004-011; what is proved HERE is that it holds on real hardware, which is the whole point of the
two-phase live-evidence rule.


## Change
**File:** `docs/plans/vk-swarm-event-bus/decisions-ledger.md`
**Anchor:** append a new section `## Live acceptance — <date>`
**After:** the pasted, fenced transcripts of every check below. Paste real output — a summary is not
evidence. Deploy the FEATURE BRANCH build to the live node first; merging is not a prerequisite for
deploying a branch.


## Allowed moves
ONLY append to the decisions-ledger. Do NOT edit code to make a check pass — a failing
check means an earlier task is wrong; STOP and fix it there, then re-run.


## STOP triggers
- Any check below fails — STOP and fix the responsible task; do NOT record a partial pass.
- Seq REGRESSES or a value is REUSED after restart — that breaks the ADR-0017 cursor contract
  outright; STOP and escalate.
- The node cannot be started with the hive unreachable — that breaks the offline-first constraint;
  STOP.


## Manual verification (record in decisions-ledger)
**SC5 — restart durability**
1. On the running node, generate several events; record the highest seq:
   `sqlite3 $VK_DATABASE_PATH "select max(seq) from event_journal"`.
2. Hard-kill the node process (`kill -9 <pid>` — not a graceful stop; the point is crash durability).
3. Restart it. Confirm the journal still holds the pre-kill rows and that they are REPLAYABLE:
   `curl -N "http://<node>/api/events?cursor=0"` returns them.
4. Emit a new event and assert its seq is strictly greater than the pre-kill maximum — no reuse, no
   regression. Paste all four outputs.

**SC8 — offline coverage (the full matrix, with the hive unreachable)**
Make the hive unreachable (stop it, or point `VK_HIVE_URL` at a dead address), then re-verify each of
SC1, SC2, SC4, and SC6 exactly as their own tasks specify:
- SC1: create / move / delete a task → three `task_%` rows in seq order.
- SC2: run an attempt to completion → `attempt_started` then a terminal event, carrying task id,
  attempt id, and executor identity.
- SC4: subscribe, disconnect, emit N, resubscribe with the last-seen cursor → nothing skipped.
- SC6: move a task → the trigger hook's log line appears; restart → the cursor advances rather than
  resetting.
Paste each transcript. SC8 passes only when all four hold offline.

**TS6 — post-deploy verify_cmd**
After merge and deploy of `main`, run `wai-verify.sh vk-swarm-event-bus` and paste the result. The
spec's `verify_cmd` is:
`sqlite3 ${VK_DATABASE_PATH:-$HOME/.local/share/vibe-kanban/db.sqlite} "select 1 from event_journal where event_type like 'task_%' limit 1" | grep -q 1`


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 012` exits 0
