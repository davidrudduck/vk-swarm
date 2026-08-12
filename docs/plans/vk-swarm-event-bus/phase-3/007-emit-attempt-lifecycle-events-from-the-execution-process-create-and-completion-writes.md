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
6. `orphan_recovery_emits_one_attempt_failed_per_process` — seed three `running` execution processes
   with a stale `server_instance_id`, run `mark_orphaned_as_failed`, assert exactly three
   `attempt_failed` rows, one per transitioned process, each carrying task id, attempt id,
   execution-process id, and executor identity. Also seed one row that must NOT transition (a
   `resume_state` of `pending`) and assert it produces no event.
7. `terminal_events_carry_executor_identity` — assert the `executor` field is populated and non-empty
   on BOTH `attempt_finished` and `attempt_failed`, not only on `attempt_started`. SC2 names all
   three events.

## Change

**Query form for any NEW SQL you write (amended 2026-08-12).** Use the runtime sqlx API —
`sqlx::query(...)`, `sqlx::query_as::<_, Row>(...)`, `sqlx::query_scalar::<_, T>(...)` plus
`.bind()`. Do NOT write a NEW `sqlx::query!` / `query_as!` / `query_scalar!` macro call (re-using an
EXISTING macro query verbatim is fine — it is already cached). Reason, established by probe and
recorded in full in task 004's Change section: this crate's `.sqlx` offline query cache is tracked,
compile-time verification is active, and a new macro query would require `cargo sqlx prepare` whose
`query-<hash>.json` output cannot be declared in `files:` — the committer would leave it unstaged, so
the build would work here and nowhere else. STOP if you find yourself needing `cargo sqlx prepare`.
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
the terminal status being written. Emit ONLY on the terminal transition (failing test 4). Load the
owning `TaskAttempt` INSIDE the transaction to source `executor` for the payload — SC2 requires
executor identity on the terminal event, and task 003 now carries the field on both terminal
variants so it can actually be serialized.

**`exit_code` is `Option<i64>` at the source and non-optional `i64` on the event — do NOT paper over
the gap (added 2026-08-12).** `update_completion` takes `exit_code: Option<i64>`
(`crates/db/src/models/execution_process/lifecycle.rs:27`) while `NodeEvent::AttemptFinished.exit_code`
is a plain `i64` (task 003, dictated to match `ExecutionProcess.exit_code: Option<i64>`'s width and
`shared/types.ts:854`'s `bigint | null`). `unwrap_or(0)` is FORBIDDEN here: it would report a clean
exit that never happened, which is worse than the narrowing cast this width was chosen to avoid.
`attempt_finished` is emitted only on the success transition; if `exit_code` is `None` at that point,
emit `attempt_failed` with a `reason` naming the missing exit code instead of substituting a value.

**Anchor:** `ExecutionProcess::mark_orphaned_as_failed` at L115-131 — a bulk
`UPDATE execution_processes SET status = 'failed' … WHERE status = 'running' AND …`, invoked from
startup recovery (`crates/services/src/services/container.rs:539-549`).
**After:** this is a REAL terminal-failure path that the original breakdown missed entirely — after a
node crash, every orphaned process transitions to `failed` here and, as written, emitted nothing.
SC2 covers terminal outcomes, so this must emit.

Restructure as: open one transaction; SELECT the exact rows about to transition (with their task,
attempt, execution-process ids and executor identity) using the same predicate; UPDATE them; append
one `AttemptFailed` per selected row with a reason identifying orphan recovery; commit. Selecting
before updating inside the same transaction is what makes "one event per transitioned process"
exact — counting `rows_affected` after the fact cannot tell you WHICH rows moved.

The function keeps its `pool: &SqlitePool` signature and its `Result<u64, …>` return.

## Allowed moves
ONLY transaction wrapping and journal append at the create write, the
terminal-completion write, and `mark_orphaned_as_failed`. **Nothing broadcasts** — the tailer
publishes (task 013). Do NOT modify `crates/services/src/services/container.rs` — no transaction may
be opened there. Do NOT change any signature.

## STOP triggers
- The completion write is NOT in `crates/db/src/models/execution_process/lifecycle.rs` — find the
  real file, update `files:` via a plan amendment, and STOP rather than editing an unlisted file.
- The completion path is not a discrete statement (it interleaves other I/O) — STOP; the D2 rule
  only permits a transaction around a discrete write.
- Executor identity is genuinely unreachable at this layer — STOP and escalate; SC2 requires it and
  task 003's schema now has a field that would serialize as empty.
- `git grep -n "SET status" crates/db/src/models/execution_process/` finds a status writer beyond the
  three instrumented here — every such path is a missed terminal event; enumerate and STOP.

## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p db execution_process"

Live SC2 check (record in the ledger): start an attempt on a running node and let it finish, then
`sqlite3 $VK_DATABASE_PATH "select seq, event_type from event_journal where event_type like 'attempt_%' order by seq"`
shows `attempt_started` then `attempt_finished` (or `attempt_failed`), each payload carrying task id,
attempt id, and executor identity.

## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 007` exits 0
