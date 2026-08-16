---
id: "023"
phase: 3
title: "Convert Task::update and Task::update_status to BEGIN IMMEDIATE (latent 517 fix)"
status: passed
depends_on: ["022"]
parallel: false
conflicts_with: []
files:
  - "crates/db/src/models/task/queries.rs"
  - "crates/db/src/models/task/hierarchy.rs"
irreversible: false
scope_test: "crates/db"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Why this task exists (found by task 022's panel B, fixed in-run — not deferred)
`Task::update` (`crates/db/src/models/task/queries.rs:341-347`) and `Task::update_status`
(`crates/db/src/models/task/hierarchy.rs:43-47`) — both instrumented by task 006 and living
unmerged on this branch — open a DEFERRED transaction whose FIRST statement is a `SELECT status`,
then UPDATE. Under WAL with a concurrent writer landing between the SELECT (which takes a read
snapshot) and the UPDATE (which must upgrade it), SQLite returns `SQLITE_BUSY_SNAPSHOT` (517),
which the busy handler does NOT retry — the caller gets a hard error. This exact shape caused real
failures twice in this workstream (`Task::delete` pool path, `mark_orphaned_as_failed`) and is why
task 022 forbade it. A single-writer test suite can never surface it, which is why 006's suite is
green over it. Task 022 (attempt 2) establishes the fix pattern; this task aligns the two
siblings to it.

## Failing test (write first)
Colocated with each function's existing tests. These are CONCURRENCY regression tests — single-
writer tests cannot catch this defect class.

1. `concurrent_updates_serialize_without_errors` (in `queries.rs`'s test module) — create one
   task; spawn N=16 concurrent `Task::update` calls against it (multi-thread runtime, distinct
   titles, same task) through the test pool; join all; assert ZERO errors and that exactly the
   expected `task_status_changed` rows exist (status unchanged in all 16 → ZERO such rows; assert
   the title equals one of the 16 written values). Against the current DEFERRED shape this test
   is EXPECTED to be flaky-red under contention (517s); if it happens to pass on your machine,
   that does NOT mean the defect is absent — proceed with the conversion regardless and record
   the observed pass/fail count in the ledger.
2. `concurrent_status_updates_serialize_without_errors` (in `hierarchy.rs`'s test module) — same
   harness against `Task::update_status`, alternating two statuses; assert zero errors and that
   the journal's `task_status_changed` rows equal the number of ACTUAL transitions observed (read
   final status; the count is deterministic per the edge-gating? It is NOT deterministic under
   concurrency — so assert instead: zero errors AND every journaled row has old_status !=
   new_status AND the final status equals the last committed writer's value read back). Keep the
   assertions to what IS deterministic: zero errors; no row with old_status == new_status.

## Change
In BOTH functions, exactly one line each: replace `pool.begin()` with
`pool.begin_with("BEGIN IMMEDIATE")`. Nothing else changes — the SELECT-then-UPDATE body is
CORRECT under IMMEDIATE (the write lock is held from BEGIN, so there is no snapshot to upgrade,
and the TOCTOU comment at `hierarchy.rs:41-42` still holds). Do not reorder statements, do not
convert the SELECT to a write, do not touch the emission logic.

Mirror task 022 attempt 2's shape exactly (read it first — `sync.rs`, `upsert_remote_task`): same
`begin_with` literal, same reasoning comment style. Record in the ledger that the three sites
(update, update_status, upsert_remote_task) now share one transaction discipline: IMMEDIATE
begin, read probe, write, append, commit.

## Allowed moves
ONLY: the two `begin_with` conversions, one short comment above each explaining WHY (517 /
snapshot-upgrade, citing this task id), and the two new tests. Do NOT touch `Task::create`,
`Task::delete` (already write-first — no snapshot to upgrade, no change needed), any sync.rs
function, or any emission logic.

## STOP triggers
- `begin_with` does not exist on `Pool` in the workspace's sqlx — STOP (it does: sqlx-core-0.8.6
  `src/pool/mod.rs:391`; if the resolved version differs, report it).
- Either function's body differs from the SELECT-first shape described above — STOP with the
  actual code.
- The concurrency tests hang or deadlock rather than erroring — STOP and report; do not add
  timeouts to mask it.

## Manual verification (record in decisions-ledger)
Gate invocation: Rust crate — override the runner. Use
WAI_TYPECHECK_CMD="cargo fmt --all -- --check && cargo check --workspace" with
WAI_TEST_CMD="cargo test -p db".
Record: the pre-conversion observed behavior of test 1 (pass/fail counts over 4 runs), and the
post-conversion 4/4 green.

## Done when
`WAI_TYPECHECK_CMD="<typecheck>" WAI_TEST_CMD="<test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 023` exits 0
