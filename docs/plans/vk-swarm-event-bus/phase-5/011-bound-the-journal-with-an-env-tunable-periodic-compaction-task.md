---
id: "011"
phase: 5
title: "Bound the journal with an env-tunable periodic compaction task"
status: ready
depends_on: ["004","009"]
parallel: false
conflicts_with: ["005","009"]
files:
  - "crates/services/src/services/event_compaction.rs"
  - "crates/services/src/services/mod.rs"
  - ".env.example"
irreversible: false
scope_test: "crates/services"
allowed_change: mixed
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
**File:** `crates/services/src/services/event_compaction.rs` colocated tests. The compaction
PREDICATE is already proved by task 004's TS1 tests; what is new here is the loop's configuration and
scheduling, so test that:

1. `reads_retention_defaults_when_env_absent` — asserts 168 hours, 10000 min rows, and 100000 max
   rows, the spec's D6 defaults.
2. `env_overrides_are_respected` — `VK_EVENT_RETENTION_HOURS` / `VK_EVENT_MIN_ROWS` /
   `VK_EVENT_MAX_ROWS` parse and win.
3. `invalid_env_falls_back_to_default_and_warns` — a non-numeric value must not panic the node at
   startup.
4. `compaction_run_is_a_no_op_on_an_empty_journal`.
5. `max_rows_below_min_rows_is_clamped_with_a_warning` — a misconfiguration where the hard cap sits
   below the retention floor is contradictory. **The behaviour is DICTATED, not chosen (amended
   2026-08-12): CLAMP `max_rows` up to `min_rows` and emit a `tracing::warn!` naming both values.**
   Do not refuse to start — an unreachable node is worse than a wider journal. (The earlier wording
   said "pick one behaviour … and pin it", which is an undictated choice; a task in this run was
   already rejected for exactly that, so the choice is made here instead.)
6. `max_rows_of_zero_is_clamped_to_at_least_one` — NEW (added 2026-08-12). `VK_EVENT_MAX_ROWS=0`
   would let stage 2 empty the journal completely, and `compact`'s post-delete
   `MIN(seq)` then returns NULL and falls back to `0`
   (`crates/db/src/models/event_journal/queries.rs:169-174`), so the
   `WHERE last_processed_seq < 0` flagging matches NOTHING — every consumer loses every event and
   NONE is marked `needs_rebootstrap`. That is silent data loss with no signal, the same
   Option-collapsed-to-zero shape as the bug task 004 was rejected for. Clamp `min_rows` and
   `max_rows` to a floor of 1 before calling `compact`, warn, and assert here that a configured `0`
   does not produce an empty journal.

## Change
**File:** `crates/services/src/services/event_compaction.rs`
**Anchor:** new file
**Sibling to read FIRST:** the existing WAL-monitor loop named by the spec (find it with
`git grep -n "VK_WAL_CHECK_INTERVAL_SECS"`). Follow its spawn shape, interval handling, and shutdown
behaviour; justify divergence in the ledger.
**After:** a periodic task that reads `VK_EVENT_RETENTION_HOURS` (default 168), `VK_EVENT_MIN_ROWS`
(default 10000) and `VK_EVENT_MAX_ROWS` (default 100000), and calls
`event_journal::compact(pool, retention_hours, min_rows, max_rows)` on an interval.

**Sanitise the three values BEFORE every `compact` call (dictated 2026-08-12).** In order: clamp
`min_rows` to a floor of 1; clamp `max_rows` to a floor of 1; then clamp `max_rows` up to `min_rows`
if it is below it. Emit one `tracing::warn!` naming the offending variable and both the configured
and effective values whenever any clamp fires. `compact` itself does NOT defend against these — task
004 takes the numbers as given, so this loop is the only place the invariant can hold, and a
configured `0` would otherwise let stage 2 empty the journal while flagging nobody. Both the cursor
floor and the hard cap that overrides it are enforced inside `compact` (task 004), not re-implemented
here.

This task creates the loop; task 014 spawns it. A loop that is never spawned leaves the journal
unbounded no matter how correct its predicate is, which is why the wiring is its own tracked task
rather than an aside here.

**File:** `crates/services/src/services/mod.rs`
**Change:** add `pub mod event_compaction;` in alphabetical position.

**File:** `.env.example`
**Anchor:** the VK_* documentation block — follow the exact commented-default style used by
`VK_BACKUP_RETENTION` at L102 and the WAL settings.
**After:** append a documented block:
```bash
# Event journal retention (see docs — ADR-0017). Compaction normally never deletes rows at or above
# the minimum persisted trigger cursor, so a lagging consumer is never starved of events. The hard
# cap overrides that floor: above VK_EVENT_MAX_ROWS, older rows are deleted regardless and any
# trigger cursor they passed is flagged for rebootstrap, so a dead consumer cannot pin the journal
# and grow it without bound.
# VK_EVENT_RETENTION_HOURS=168
# VK_EVENT_MIN_ROWS=10000
# VK_EVENT_MAX_ROWS=100000
```

## Allowed moves
ONLY the new compaction module, its module declaration, and the .env.example block. Do
NOT re-implement the compaction SQL — call task 004's `compact`. Do NOT change the retention
defaults from the spec's D6 values.

## STOP triggers
- Spawning the loop requires editing a file not in `files:` (e.g. the deployment startup) — STOP and
  amend the plan.
- The WAL-monitor sibling does not exist / uses a pattern that cannot be followed — record what was
  used instead.

## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p services event_compaction"

1. `grep -n VK_EVENT_ .env.example` shows both variables documented and commented out.
2. Start the node with `VK_EVENT_RETENTION_HOURS=0` against a scratch DB holding backdated journal
   rows and a `trigger_cursors` row; confirm rows below the cursor are removed and rows at or above
   it survive. Record the before/after row counts in the ledger.

## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 011` exits 0
