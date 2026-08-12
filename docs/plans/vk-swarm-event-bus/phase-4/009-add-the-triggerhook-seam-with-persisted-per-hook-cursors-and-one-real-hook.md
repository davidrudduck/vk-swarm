---
id: "009"
phase: 4
title: "Add the TriggerHook seam with persisted per-hook cursors and one real hook"
status: ready
depends_on: ["005"]
parallel: false
conflicts_with: []
files:
  - "crates/services/src/services/trigger_hooks.rs"
  - "crates/services/src/services/mod.rs"
  - "crates/db/src/models/trigger_cursor.rs"
  - "crates/db/src/models/mod.rs"
irreversible: false
scope_test: "crates/services"
allowed_change: mixed
covers_criteria: ["SC6"]
covers_tests: ["TS4"]
---
## Failing test (write first)
**File:** `crates/services/src/services/trigger_hooks.rs` colocated tests, using a RECORDING test
hook that appends every fired event to a `Vec` behind a mutex. These ARE TS4.

1. `hook_fires_only_on_matching_events` — a hook matching `task_status_changed` must not fire on
   `task_created`.
2. `cursor_is_persisted_after_each_fire` — assert `trigger_cursors.last_processed_seq` advances.
3. `restart_resumes_from_persisted_cursor_without_loss` — the SC6 core: run the hook over events
   1..3, drop the runner, journal events 4..6 while it is DOWN, start a NEW runner for the same hook
   name, and assert it sees 4,5,6. Losing 4..6 is the exact failure this seam exists to prevent.
4. `at_least_once_tolerates_duplicate_delivery` — a crash between firing and persisting the cursor
   re-delivers; assert that is accepted (hook is idempotent), not an error.
5. `unknown_hook_starts_at_cursor_zero` — a hook with no `trigger_cursors` row replays from the
   beginning of the journal rather than silently starting live.
6. `cursor_advances_past_non_matching_events` — journal one matching event followed by five
   NON-matching ones; assert the persisted cursor reaches the LAST seq, not the seq of the last
   match. Then drop the runner, restart it, and assert it replays nothing. Without this the hook
   re-reads those five events on every restart forever, and — because compaction floors on
   `MIN(last_processed_seq)` — pins the journal at the first non-matching event permanently.
7. `rebootstrap_flag_is_surfaced_and_cleared` — set `needs_rebootstrap = 1` on the hook's cursor row
   (as the hard cap does), start the runner, and assert it observes the flag, resumes from the
   journal's current minimum rather than its stale cursor, and clears the flag. A hook that ignores
   the flag silently resumes mid-gap.

## Change
**File:** `crates/db/src/models/trigger_cursor.rs`
**Anchor:** new file
**After:** load/upsert accessors over the `trigger_cursors` table created in task 002 —
`get(pool, hook_name) -> Option<i64>`, `set(pool, hook_name, seq)` (UPSERT on the `hook_name`
primary key, updating `updated_at`), and `min_cursor(pool) -> Option<i64>` used by compaction.

**Query form — use the RUNTIME API, not the `query!` macro family (amended 2026-08-12).** Write every
statement here with `sqlx::query(...)`, `sqlx::query_as::<_, Row>(...)` or
`sqlx::query_scalar::<_, T>(...)` plus `.bind()`. Do NOT use `sqlx::query!`, `query_as!` or
`query_scalar!`. Same reason as task 004: `crates/db/.sqlx` is a tracked offline query cache and
compile-time verification is active, so a new macro query would demand `cargo sqlx prepare`, whose
`crates/db/.sqlx/query-<hash>.json` output cannot be declared in `files:` (the gate treats `.sqlx` as a
file, not a directory scope) and would be left unstaged by the committer — compiling here and nowhere
else. The sibling `crates/db/src/models/node_outbox.rs:81,100,126` uses this runtime form.
STOP if you find yourself needing `cargo sqlx prepare`.

**File:** `crates/db/src/models/mod.rs`
**Change:** add `pub mod trigger_cursor;` in alphabetical position.

**File:** `crates/services/src/services/trigger_hooks.rs`
**Anchor:** new file
**After:** the trait exactly as the spec's Design specifies:
```rust
#[async_trait]
pub trait TriggerHook: Send + Sync {
    fn name(&self) -> &'static str;
    fn matches(&self, event: &NodeEvent) -> bool;
    async fn fire(&self, event: SequencedEvent);
}
```
Plus a registry holding `Vec<Arc<dyn TriggerHook>>` and a per-hook runner task that:
1. loads the hook's cursor (0 when absent),
2. consumes `EventBus::subscribe_from(cursor)` — the SAME contract as every other consumer, not a
   bespoke poll loop,
3. calls `fire` for events where `matches` is true,
4. persists the cursor for EVERY consumed event — after firing when it matched, immediately when it
   did not.

**Point 4 is the correction, and it matters twice.** Advancing only after a fire (as the original
breakdown said) leaves non-matching events permanently unacknowledged: on every restart the hook
replays them again, and because compaction floors on `MIN(last_processed_seq)` across
`trigger_cursors`, the journal is pinned at the first non-matching event forever. Spec D11 now states
this explicitly. Ordering within a matching event is unchanged and still deliberate:
fire-then-persist gives at-least-once, while persist-then-fire would give at-most-once and could LOSE
events, which SC6 forbids.

Also honour `needs_rebootstrap`: when the runner loads a cursor whose flag is set, the hard cap has
deleted events it never saw. It must resume from the journal's current low-water mark rather than its
stale cursor, log that it lost events, and clear the flag.

Ship ONE real hook as the SC6 proof: a hook matching `task_status_changed` whose `fire` emits a
structured `tracing::info!` carrying task id, old status, new status, and seq. That log line is the
observable side-effect SC6 requires — it must be greppable in the node log.

**File:** `crates/services/src/services/mod.rs`
**Change:** add `pub mod trigger_hooks;` in alphabetical position.

## Allowed moves
ONLY the four files listed. Registration of the hook on the deployment at startup is
in scope ONLY if it can be done without editing an unlisted file; if it requires touching
`crates/local-deployment/src/lib.rs`, STOP and amend the plan rather than editing outside `files:`.
Do NOT build rule engines, priority selection, or any policy — the spec puts those in P5/P6.

## STOP triggers
- Registering the hook requires editing a file not in `files:` — STOP and amend.
- The runner needs its own DB connection held for the process lifetime — check it against the pool
  size (`VK_SQLITE_MAX_CONNECTIONS`, default 10) and record the reasoning.
- `async_trait` is not already a dependency of `crates/services` — do NOT add a new dependency
  without recording it; prefer whatever async-trait mechanism the crate already uses.

## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p services trigger_hooks"

Live SC6 check (record BOTH halves in the ledger):
1. On a running node, move a task; `grep` the node log for the hook's structured line carrying the
   task id and the new status.
2. Restart the node, move a task again, and confirm the hook fires again AND that
   `sqlite3 $VK_DATABASE_PATH "select hook_name, last_processed_seq from trigger_cursors"` shows the
   cursor advancing across the restart rather than resetting.

## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 009` exits 0
