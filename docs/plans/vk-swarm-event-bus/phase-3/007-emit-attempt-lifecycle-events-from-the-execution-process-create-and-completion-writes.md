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
  - "crates/db/.sqlx/query-1e339e959f8d2cdac13b3e2b452d2f718c0fd6cf6202d5c9139fb1afda123d29.json"
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
**After:** wrap in a transaction, append `NodeEvent::AttemptStarted { .. }`, commit. Nothing is
broadcast here — see "Nothing broadcasts" below; the tailer (task 013) publishes what it reads back
from the journal. (Amended 2026-08-12: this line previously ended "commit, broadcast", stale wording
from the pre-tournament design that contradicted this same file's own Allowed-moves paragraph.)

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

---

## SECONDARY — delete one orphaned `.sqlx` cache entry (panel 16, F16-1)

Unrelated to attempt lifecycle events; here because CLAUDE.md forbids carrying a finding into a
later session and this task's `files:` can legitimately declare the path.

Task 006 replaced `sqlx::query!("DELETE FROM tasks WHERE id = $1", id)` with a runtime-API
`DELETE ... RETURNING`. The tracked offline-cache entry that macro generated is now orphaned — no
source references that query text:

```text
crates/db/.sqlx/query-1e339e959f8d2cdac13b3e2b452d2f718c0fd6cf6202d5c9139fb1afda123d29.json
  -> 'DELETE FROM tasks WHERE id = $1'
```

**`git rm` it.** Confirm first that nothing references that exact query text
(`grep -rn 'DELETE FROM tasks WHERE id = \$1' --include='*.rs' crates/`) — `sync.rs:382` has a
DIFFERENT delete-by-`shared_task_id` query whose entry must NOT be touched.

It cannot break the build (nothing runs `cargo sqlx prepare --check`), so if removing it turns
anything red, STOP — that means something still needs it and the orphan analysis is wrong.

**Why this is declarable at all**, since the run has recorded the opposite about `.sqlx`:
`task-gate.sh`'s `is_declared()` checks `DECL[path]` for an EXACT match *before* the
directory-expansion loop whose dotted-basename heuristic breaks on `.sqlx` (agent-plugins #105). A
specific `query-<hash>.json` file can therefore be declared; only the directory scope cannot. Do not
generalise this into declaring `crates/db/.sqlx` — that still covers nothing.

---

## REQUIRED — attempt 2, after panels 17A and 17B

Two panels reviewed attempt 1 with disjoint remits. **Both rejected it, on different defects.** The
emission logic, the executor sourcing, the identity fields and the orphan predicate are all correct
and stay — every one was attacked and held. What follows is three real defects and five corrections.

**READ THIS FIRST: the two panels' remediations CONFLICT, and resolving that is the substance of
this attempt.**

### THE CONFLICT

- **17A** says: `update_completion` emits on every terminal WRITE rather than on a terminal
  TRANSITION. Its proposed fix is to fold `ep.status` into the owner JOIN and **move that SELECT
  before the UPDATE**.
- **17B** proves: a deferred transaction that **reads then upgrades to a write** acquires
  `SQLITE_BUSY_SNAPSHOT` (517), which SQLite's busy handler **does not retry**, so `busy_timeout`
  never applies. Measured 6/200 vs 0/200 for the pre-007 shape.

`update_completion` today is **write-first** — UPDATE at `lifecycle.rs:64`, SELECT at `:80` — so it
has NO snapshot exposure. **Applying 17A's fix literally would introduce 17B's defect into a function
that does not currently have it.**

**You must satisfy both constraints: gate emission on a real transition AND never read-then-upgrade.
I am not dictating the shape.** Candidates, none pre-blessed:

- **(a) Gate in the UPDATE's own WHERE clause** — e.g. `UPDATE ... WHERE id = ? AND status = 'running'
  RETURNING task_attempt_id`, treating "no row returned" as "no transition, no event". Write-first and
  atomic, no TOCTOU. **But it changes write behaviour**: an already-terminal row would no longer be
  overwritten at all. Assess whether any caller depends on that overwrite before choosing it, and say
  what you found either way.
- **(b) Capture the prior status in the same statement**, e.g. a CTE reading `status` before the
  UPDATE within one statement. Keeps both properties if SQLite evaluates it as you expect — prove it
  rather than assuming.
- **(c) Take the write lock up front** (`BEGIN IMMEDIATE`) so the read is not an upgrade. 17B flagged
  that it did **not** verify this sqlx version exposes it — check before adopting.
- **(d) Something better.** If you find one, say why.

**REQUIRED regardless of choice: a test proving the chosen shape does NOT read-then-upgrade.** 17B's
harness is the pattern — two connections, prod-like pool config (WAL, `busy_timeout`,
`max_connections(10)`), an independent writer committing between the read and the write. The pre-007
shape scored 0/200; yours must too.

### 1. BLOCKING — gate `update_completion` on a real transition (17A-1, 17B-4)

Three identical `Completed` writes emit three `attempt_finished`. `Completed → Killed` emits **both**
`attempt_finished` and `attempt_failed` for one process. SC2 names a singular terminal outcome.

Task 006's `Task::update` is the sibling that does this right (`task/queries.rs:340-386`): it reads
`old_status` inside the transaction and gates on `old_status != task.status`. Your doc comment
already invokes "same reasoning as `Task::update`'s prior-status read" while not doing the read — fix
the code, and fix that comment.

**Boundary tests REQUIRED** (17A-2 proved the guard is entirely untested — mutating `is_terminal` to
`true` leaves all 254 green): a `Running` write emits nothing; a repeated identical terminal write
emits once; `Completed → Killed` emits once, not two contradictory events.

**Note test 4 is misnamed for what it does.** `non_terminal_update_emits_nothing` drives
`update_pid`, which never enters `update_completion`. **That is my task file's example being wrong,
not your implementation.** Keep it (it guards a real thing) but add the real ones.

### 2. BLOCKING — remove the read-then-upgrade from `mark_orphaned_as_failed` (17B-1)

It reads the rows about to transition, then UPDATEs — the shape that scores 517. The sweep runs as a
background task at startup (`server/src/main.rs:126-146`) alongside a sibling that writes the same
table, and the error is swallowed into `tracing::warn!` at `:135`, so the whole batch stays `running`
and emits **nothing** — the exact SC2 hole this task exists to close.

**`UPDATE ... RETURNING id` first, then load identities for exactly those ids.** Write-first, and it
makes `rows_affected` and the event count structurally identical rather than merely equal by
argument — which also closes item 3. This is the same fix `DELETE ... RETURNING` gave task 006 for
the same shape; it is the second time this pattern has bitten this run.

### 3. The identity JOIN can miss rows the state write hit (17B-2)

The SELECT has `JOIN task_attempts`; the UPDATE does not. A row whose parent attempt is absent
transitions but emits nothing. Zero live occurrences across all three local DBs, so this is latent —
but item 2's remediation closes it structurally, and **two shipped statements assert the opposite**
and must be corrected: `lifecycle.rs`'s comment that "`owner` is None only when `id` did not match any
row", and the ledger's "same predicate" in Undictated choice 4.

### 4. NULL executor silently emits `"executor": ""` (17A-3, 17B-3 — found independently by both)

`task_attempts.executor` is nullable and sqlx decodes SQL NULL into `String` as `""` rather than
erroring. **Your tests assert `!executor.is_empty()` with the message "SC2 requires non-empty executor
identity" — but the code never enforces it**; the assertion passes only because every fixture sets it.

Decode as `Option<String>` and handle NULL explicitly. **You choose** whether that means refusing to
emit, emitting a sentinel, or failing the write — argue it in the ledger. Reachability is
legacy-data-only (the typed enum prevents new NULLs), so this is about the code meaning what its
tests claim.

### 5. Corrections, no behaviour change

- **Breakdown is enumerated wrongly** in the ledger and commit message. `ExecutorAction::base_executor()`
  returns `None` only for `ScriptRequest`; Breakdown is constructed with `CodingAgentInitialRequest`,
  which carries a profile. The decision stands (three of four still lack a source, and the JOIN is
  needed for `task_id` regardless) — fix the enumeration.
- **Add the missing rollback tests.** 007 ships none for `update_completion` or
  `mark_orphaned_as_failed`; 006 ships one per site. 17B probed all three and they behave correctly —
  pin them.
- **Record the `(Completed, None)` bus/table contradiction** (17A-4) as a ledger residual: the row's
  `status` column reads `completed` while the journal says failed. Unreachable from every production
  caller today, so a design note, not a defect.
- **Record the ~3.1x `update_completion` slowdown** (17B-5: ≈1.54ms vs 0.50ms) as a declared residual.

## Verification for attempt 2

`cargo test -p db`, `cargo fmt --all -- --check`, `cargo clippy -p db --all-targets --all-features
-- -D warnings`, `cargo check --workspace --all-targets` — all exit 0. Plus, verbatim: the
no-read-then-upgrade test result, and a bite proof for the transition guard (mutate it away, the new
boundary tests must fail).
