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

## REQUIRED — panel remediation (attempt 2, 2026-08-17)

Attempt 1 (60cf4dd6 + dd115c03) passed the Stage-1 gate but was REJECTED by the Stage-2 panel with
two verified contract violations and one coverage hole. Every fix below is dictated exactly; the
attempt-1 code is the starting point (do NOT revert it — amend it).

### F1 — rebootstrap resume is off by one (loses the surviving low-water event)

Compaction flags cursors with `last_processed_seq < new_min_seq`
(`crates/db/src/models/event_journal/queries.rs:174`), so a flagged hook's cursor is STRICTLY BELOW
the smallest surviving seq — the event at `MIN(seq)` is present and unprocessed. But the rebootstrap
branch sets `cursor = new_min.unwrap_or(0)` (`trigger_hooks.rs:127`) and `subscribe_from` replays
`seq > cursor` EXCLUSIVE (`event_bus/mod.rs:187`), so the event at `MIN(seq)` is silently skipped —
and `trigger_cursor::set` then records it as processed.

**Fix (trigger_hooks.rs, rebootstrap branch):**
```rust
cursor = new_min.map(|m| m - 1).unwrap_or(0);
```
Update the `info!` line's comment/field naming if needed so `resumed_from_seq` is understood as the
EXCLUSIVE cursor (replay starts at the next seq).

### F2 — decouple flag clearing from cursor advance

`trigger_cursor::set` writes `needs_rebootstrap = 0` in BOTH its INSERT and DO UPDATE branches
(`trigger_cursor.rs:56-70`), and the runner reads the flag exactly once before its loop. So a flag
raised by compaction while the runner is LIVE is erased by the runner's next cursor write before any
restart can act on it — the flag's only consumer can never see it in the live case.

**Fix (trigger_cursor.rs):**
1. `set()` must NOT touch `needs_rebootstrap` in the `DO UPDATE` branch (keep `0` in the INSERT
   `VALUES` — a fresh row starts unflagged). Update its doc comment.
2. Add `pub async fn clear_rebootstrap(pool: &SqlitePool, hook_name: &str) -> Result<(), sqlx::Error>`
   — a single `UPDATE trigger_cursors SET needs_rebootstrap = 0 WHERE hook_name = ?`.
3. Add `pub async fn ensure_row(pool: &SqlitePool, hook_name: &str) -> Result<(), sqlx::Error>` —
   `INSERT OR IGNORE` a row with `last_processed_seq = 0, needs_rebootstrap = 0`. (Task 014 will call
   this at hook registration: a hook with NO row contributes nothing to the compaction floor and can
   have the journal deleted underneath it without ever being flagged.)

**Fix (trigger_hooks.rs, rebootstrap branch):** after the existing `trigger_cursor::set(...)` call,
add `trigger_cursor::clear_rebootstrap(&pool, hook_name).await?;`. A crash between the two writes is
safe: the flag survives, the next start re-runs the (idempotent) rebootstrap.

**Fix (db-side tests, trigger_cursor.rs):** INVERT `test_cursor_set_clears_rebootstrap_flag`
(`:149`) — rename to `test_cursor_set_preserves_rebootstrap_flag`: raise the flag, call `set()`,
assert the flag is STILL 1. Add `test_clear_rebootstrap_clears_flag` and `test_ensure_row_is_noop_on_existing`
(existing cursor value survives `ensure_row`).

### F1 test — restore test 7's dictated expectation

Attempt 1 weakened `rebootstrap_flag_is_surfaced_and_cleared` from the dictated 2 firings to 1 to
fit the off-by-one (ledger claim "this is correct behavior" was FALSE). Restore the original
expectation against the fixed code, keeping the existing setup shape (event1 at seq1, stale cursor 0
with flag=1, event2 at seq2 — nothing deleted, so BOTH survive):
- assert fired seqs == `[seq1, seq2]` (both fire, in order),
- assert cursor == seq2,
- keep `assert!(!flag)` — with F2's decoupling this assertion is now load-bearing: `set()` no longer
  clears the flag, so it passes ONLY if `clear_rebootstrap` actually ran.

### F3 — rewrite test 4 as a REAL crash simulation (D11 ordering coverage)

The attempt-1 test is a tautology (clears a Vec, pushes two events by hand, asserts len 2 — zero
coupling to `run_hook`; the persist-then-fire mutation left ALL SEVEN tests green). Rewrite
`at_least_once_tolerates_duplicate_delivery`:

1. Journal exactly ONE matching event (`task_status_changed`, seq 1). No others.
2. Fault-inject the cursor persist DETERMINISTICALLY with poison triggers (chmod/pool-close inject
   nothing — established this run):
   ```sql
   CREATE TRIGGER poison_cursor_insert BEFORE INSERT ON trigger_cursors
     BEGIN SELECT RAISE(ABORT, 'injected cursor-write failure'); END;
   CREATE TRIGGER poison_cursor_update BEFORE UPDATE ON trigger_cursors
     BEGIN SELECT RAISE(ABORT, 'injected cursor-write failure'); END;
   ```
   (`get_with_flag` is a SELECT — unaffected; the runner reaches the replay.)
3. Run `run_hook` and AWAIT its JoinHandle — it must return `Err` (the post-fire `set()` failed).
   No sleep needed for this phase. Assert fired seqs == `[seq1]`: the hook FIRED before the persist
   failed. This is the D11 ordering pin — under a persist-then-fire mutation the set() fails BEFORE
   the fire and this assertion sees `[]`.
4. `DROP TRIGGER` both. Start a NEW runner (fresh cursor load: the poisoned INSERT never landed, so
   cursor is 0). Sleep per the existing 300ms pattern; assert TOTAL fired seqs == `[seq1, seq1]` —
   the same event delivered twice (at-least-once, duplicate tolerated), and assert the cursor now
   equals seq1. Abort the runner handle.

### Red proofs (record command output in the ledger — mandatory)

- **RP1:** temporarily revert F1 (`cursor = new_min.unwrap_or(0)`) → test 7 must go RED. Restore.
- **RP2:** temporarily swap the matching branch to persist-then-fire → test 4 must go RED. Restore.
Backups via `cp` into `.wai-scratch/` with diff-verified restores. NEVER git checkout/restore/stash/
reset in any form.

### Verify

`cargo test -p services --lib trigger_hooks` (7/7) AND `cargo test -p db --lib trigger_cursor` (all
green — Stage-1's `crates/services` scope does not run these; run them yourself). Then the usual
gate commands (fmt by EXIT CODE + `cargo check --workspace --all-targets`).

Ledger: append a "Task 009 remediation (attempt 2)" section — the attempt-1 false claim is corrected
APPEND-ONLY (never edit the old entry). Also correct the `futures::stream::StreamExt` →
`futures_util::stream::StreamExt` ledger inaccuracy there.
