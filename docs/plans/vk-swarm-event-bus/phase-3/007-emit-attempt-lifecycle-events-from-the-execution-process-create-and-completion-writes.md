---
id: "007"
phase: 3
title: "Emit attempt lifecycle events from the execution-process create and completion writes"
status: ready
depends_on: ["006"]
parallel: false
conflicts_with: []
files:
  - "crates/db/src/models/execution_process/queries.rs"
  - "crates/db/src/models/execution_process/lifecycle.rs"
irreversible: false
scope_test: "crates/db"
allowed_change: edit
covers_criteria: ["SC2"]
covers_tests: []
---
## Failing test (write first)
**File:** `crates/db/src/models/execution_process/` colocated tests, using
`db::test_utils::create_test_pool_with_migrations()`.

1. `create_emits_attempt_started_with_identity` — assert exactly one `attempt_started` row carrying
   task id, attempt id, execution-process id, and executor identity. SC2 names all four; a row
   missing executor identity fails.
2. `completion_success_emits_attempt_finished` — drive the completion write to a successful terminal
   state; assert one `attempt_finished` carrying the exit code.
3. `completion_failure_emits_attempt_failed` — assert one `attempt_failed` carrying a reason.
4. `non_terminal_update_emits_nothing` — an intermediate update (e.g. setting a pid) must emit no
   event. Guards against emitting on every UPDATE.
5. `rolled_back_create_journals_nothing` — proves the shared transaction here too.


## Change
Same D2 emission-ownership shape as task 006: the model function owns the transaction around
its own discrete write statement.

**File:** `crates/db/src/models/execution_process/queries.rs`
**Anchor:** `ExecutionProcess::create` at L361 — a single `INSERT … RETURNING` (L371-391) taking
`pool: &SqlitePool`.
**After:** wrap in a transaction, append `NodeEvent::AttemptStarted { .. }`, commit, broadcast.

CRITICAL — why this is safe here: the git I/O that computes `before_head_commit` runs in the CALLER
at `crates/services/src/services/container.rs:1516-1523` and its result is passed in as a plain
`Option<&str>`. The transaction therefore spans only the INSERT, never git or filesystem I/O. Do NOT
widen the transaction into `start_execution` — holding SQLite's single writer lock across that git
call would stall every writer on the node.

Executor identity: source it from the row's `executor_action` / the associated `ExecutorSession`.
If it is not reachable at this layer without an extra query inside the transaction, do that extra
read INSIDE the transaction and record the cost in the ledger — do not emit an event without it.

**File:** `crates/db/src/models/execution_process/lifecycle.rs`
**Anchor:** the completion write — the function that transitions an execution process to a terminal
`ExecutionProcessStatus` (Completed / Failed / Killed) and sets `completed_at`. Locate it with
`git grep -n "completed_at" crates/db/src/models/execution_process/`.
**After:** same shape; emit `attempt_finished` on success and `attempt_failed` otherwise, keyed off
the terminal status being written. Emit ONLY on the terminal transition (failing test 4).


## Allowed moves
ONLY transaction wrapping, journal append, and post-commit broadcast at the create
and terminal-completion writes. Do NOT modify `crates/services/src/services/container.rs` — no
transaction may be opened there. Do NOT change any signature.


## STOP triggers
- The completion write is NOT in `crates/db/src/models/execution_process/lifecycle.rs` — find the
  real file, update `files:` via a plan amendment, and STOP rather than editing an unlisted file.
- The completion path is not a discrete statement (it interleaves other I/O) — STOP; the D2 rule
  only permits a transaction around a discrete write.
- Executor identity is genuinely unreachable at this layer — STOP and escalate; SC2 requires it.


## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p db execution_process"

Live SC2 check (record in the ledger): start an attempt on a running node and let it finish, then
`sqlite3 $VK_DATABASE_PATH "select seq, event_type from event_journal where event_type like 'attempt_%' order by seq"`
shows `attempt_started` then `attempt_finished` (or `attempt_failed`), each payload carrying task id,
attempt id, and executor identity.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 007` exits 0
