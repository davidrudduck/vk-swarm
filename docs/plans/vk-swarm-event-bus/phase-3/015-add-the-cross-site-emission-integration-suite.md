---
id: "015"
phase: 3
title: "Add the cross-site emission integration suite"
status: ready
depends_on: ["006","007","008","020","022"]
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
1b. `breakdown_acceptance_emits_one_event_per_child` — **added 2026-08-15 with task 020.** Accept a
   proposal with multiple items and assert one `TaskCreated` per child, with the child ids as a SET.
   This suite's whole claim is CROSS-SITE completeness, so omitting the breakdown site would let it
   report full coverage while a routed, user-initiated creation path emits nothing — the exact gap
   that forced task 020 into existence. If task 020 has not landed, this test is blocked on 020, not
   on this task; say so rather than dropping it.
1c. `remote_upsert_emits_exactly_one_event_each` — **added 2026-08-16 with task 022.** Drive
   `Task::upsert_remote_task` three ways: fresh `shared_task_id` (assert exactly one
   `task_created`), version-bumped upsert with a changed status (assert exactly one
   `task_status_changed` with correct old/new), and a version-stale upsert (assert zero new rows).
   The remote write path is a first-class lifecycle site (user-driven status changes on
   remote-project tasks flow through it); a cross-site completeness suite without it repeats the
   exact omission that forced task 022 into existence.
2. `attempt_lifecycle_emits_exactly_one_event_each` — start and terminate an attempt; assert one
   `attempt_started` and one terminal event.
3. CONNECTIVITY — DELEGATED, not tested here (amended 2026-08-16; supersedes the earlier
   `connectivity_transitions_emit_exactly_one_event_each`). The connectivity gate
   (`ConnectivityJournal`) is intentionally PRIVATE to `node_runner.rs`; an integration test in
   `crates/services/tests/` cannot reach it, and exposing it publicly for a test would invert the
   design. Single-emission-per-transition is already pinned by node_runner's EIGHT colocated
   `connectivity_event_tests` (edge gating both directions, ordering, fault-injected append,
   clean-close), and the upstream `hive_client.rs` clean-close send is provable ONLY live — that
   obligation sits with task 012's SC3 check, NOT this suite. State exactly this delegation in the
   suite's module doc comment so a reader of the "cross-site" claim knows where connectivity is
   proven, and record it in the ledger.
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
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo fmt --all -- --check && cargo check --workspace --all-targets" with the WAI_TEST_CMD given below (fmt is checked by EXIT CODE — the nightly-config warnings are noise; two attempts this run shipped fmt-red claiming green).
WAI_TEST_CMD="cargo test -p services --test event_emission"

All tests green (1, 1b, 1c, 2, 4, 5 — connectivity delegated per item 3). Record in the ledger the
`electric_task_sync.rs` sibling comparison AND the connectivity delegation.

## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 015` exits 0
