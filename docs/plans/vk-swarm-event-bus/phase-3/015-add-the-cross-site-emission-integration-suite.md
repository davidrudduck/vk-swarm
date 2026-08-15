---
id: "015"
phase: 3
title: "Add the cross-site emission integration suite"
status: ready
depends_on: ["006","007","008"]
parallel: false
conflicts_with: []
files:
  - "crates/services/tests/event_emission.rs"
siblings: ["crates/services/tests/electric_task_sync.rs", "crates/services/tests/event_bus_end_to_end.rs"]
irreversible: false
scope_test: "crates/services"
allowed_change: create
covers_criteria: []
covers_tests: ["TS3"]
---
## Failing test (write first)
**File:** `crates/services/tests/event_emission.rs` — a new integration suite. This IS TS3.

It is separate from tasks 006/007/008 deliberately: those instrument three different failure domains
in two different crates, while this asserts the ONE property that spans all of them —
exactly-one-event-per-state-change. Fusing it into 008 (as the original breakdown did) meant a
connectivity bug and a cross-site assertion bug blocked each other's revert.

1. `task_crud_emits_exactly_one_event_each` — create/move/delete; assert exactly one journal row per
   operation, correctly typed.
2. `attempt_lifecycle_emits_exactly_one_event_each` — start and terminate an attempt; assert one
   `attempt_started` and one terminal event.
3. `connectivity_transitions_emit_exactly_one_event_each` — one row per genuine transition, none for
   repeated failed retries from an already-disconnected state.
4. `no_duplicate_events_for_a_single_state_change` — the regression guard against double-emission if
   a site is ever instrumented at two layers at once.
5. `every_emitted_event_type_round_trips_from_the_journal` — read each journaled payload back into
   `NodeEvent` and assert `event_type()` matches the stored `event_type` column. Catches a site that
   journals a hand-written type string instead of the typed contract.

## Change

**Query form for any NEW SQL you write (amended 2026-08-12).** Use the runtime sqlx API —
`sqlx::query(...)`, `sqlx::query_as::<_, Row>(...)`, `sqlx::query_scalar::<_, T>(...)` plus
`.bind()`. Do NOT write a NEW `sqlx::query!` / `query_as!` / `query_scalar!` macro call (re-using an
EXISTING macro query verbatim is fine — it is already cached). Reason, established by probe and
recorded in full in task 004's Change section: this crate's `.sqlx` offline query cache is tracked,
compile-time verification is active, and a new macro query would require `cargo sqlx prepare` whose
`query-<hash>.json` output cannot be declared in `files:` — the committer would leave it unstaged, so
the build would work here and nowhere else. STOP if you find yourself needing `cargo sqlx prepare`.
**Sibling to read FIRST:** `crates/services/tests/electric_task_sync.rs` — the nearest
integration-test harness precedent in this crate. Follow its setup shape (how it builds a pool, a
deployment/service graph, and drives the system under test); its domain logic is irrelevant here.
Record any divergence in the ledger.

Build the suite around `db::test_utils::create_test_pool_with_migrations()` per CLAUDE.md, never
hand-written `CREATE TABLE`.

The assertions all take the same form: perform a state change, then read `event_journal` directly and
assert on the SET and COUNT of rows. Query the journal rather than subscribing to the bus — this
suite is about EMISSION, and going through the tailer would make a publication bug look like an
emission bug.

## Allowed moves
ONLY this one new test file. Do NOT modify any emission site to make a test pass —
if a site is wrong, that is a defect in task 006/007/008 and belongs there. Do NOT assert on
broadcast delivery; that is task 013's and task 005's territory.

## STOP triggers
- A test here fails because an emission site is wrong — STOP and fix it in the owning task rather
  than weakening the assertion.
- Driving a real hive connection from an integration test is not feasible — assert connectivity
  emission through whatever seam `node_runner`'s own tests use, and record the substitution.
- Task 008 escalated on its clean-close ambiguity STOP trigger — then test 3 here
  (`connectivity_transitions_emit_exactly_one_event_each`) has no implementation to assert against
  and is blocked on THAT escalation, not on this task. Say so explicitly rather than weakening the
  test to pass; this suite inherits 008's risk and must not disguise it.

## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p services event_emission"

All five tests green. Record in the ledger the `electric_task_sync.rs` sibling comparison.

## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 015` exits 0
