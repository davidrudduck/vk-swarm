# vk-swarm-event-bus Plan

## Spec
docs/superpowers/specs/2026-08-07-vk-swarm-event-bus.md

## Approach
Journal first, broadcast second (ADR-0017, amended 2026-08-11). A new event_journal table
takes one typed row per covered lifecycle change, written in the same transaction as the discrete
state-write statement — the DB model function owns that transaction, so no caller signature changes.
After commit, the event is published on an in-process tokio broadcast channel whose Sender lives in
crates/db (crates/db cannot depend on crates/services); crates/services/src/services/event_bus.rs
wraps it and owns the subscribe_from replay-to-live handoff that every consumer shares.

Two consumers ship: an internal TriggerHook registry with per-hook persisted cursors (the P6 seam,
proven with one real hook) and a GET /api/events SSE endpoint with cursor resume. That path is
currently occupied by a consumer-less record-patch SSE route, which is deleted first as a standalone
irreversible step so the new route is a clean create.

Emission is instrumented at three choke points: task CRUD in crates/db/src/models/task/, the
execution-process create/completion writes, and hive connectivity transitions in hive_client.rs plus
the reconcile leg in node_runner.rs. A periodic compaction task bounds the journal. UI is explicitly
out of scope — the board already streams live over /api/tasks/stream/ws.


## Phases
- **Phase 1: Foundation** — Clear the /api/events path, then land the durable substrate: schema, typed event contract, journal model, and the broadcast sender in the db layer.
- **Phase 2: Bus core** — The shared replay-to-live contract every consumer binds to: subscribe_from with journal catch-up, live handoff, dedupe, and Lagged refill.
- **Phase 3: Emission** — One event per state change at all three choke points, each journaled in the same transaction as its discrete write statement.
- **Phase 4: Consumers** — The P6 trigger-hook seam with persisted cursors, and the external SSE subscription endpoint on the freed /api/events path.
- **Phase 5: Bounding and acceptance** — Bound the journal with env-tunable compaction, then prove the SC list on a live node including restart durability and offline coverage.

## Tasks

| id | phase | title | deps | conflicts |
|---|---:|---|---|---|
| 001 | 1 | Delete the consumer-less /api/events record-patch route and its orphaned trait method | dep: none | conflicts: none |
| 002 | 1 | Add the event_journal and trigger_cursors migration | dep: none | conflicts: none |
| 003 | 1 | Define the NodeEvent and SequencedEvent typed contract and export it via ts-rs | dep: none | conflicts: none |
| 004 | 1 | Add the event_journal model with append, cursor range-read, and compaction | dep: 002 003 | conflicts: none |
| 005 | 2 | Add the broadcast sender to DBService and the EventBus wrapper with subscribe_from | dep: 004 | conflicts: none |
| 006 | 3 | Emit task lifecycle events from the task model inside its own transaction | dep: 005 | conflicts: none |
| 007 | 3 | Emit attempt lifecycle events from the execution-process create and completion writes | dep: 006 | conflicts: none |
| 008 | 3 | Emit hive connectivity events and add the cross-site emission integration suite | dep: 007 | conflicts: none |
| 009 | 4 | Add the TriggerHook seam with persisted per-hook cursors and one real hook | dep: 005 | conflicts: none |
| 010 | 4 | Add the GET /api/events SSE endpoint with cursor resume on the freed path | dep: 001 005 | conflicts: none |
| 011 | 5 | Bound the journal with an env-tunable periodic compaction task | dep: 004 009 | conflicts: none |
| 012 | 5 | Record live acceptance for restart durability and full offline coverage | dep: 006 007 008 009 010 011 | conflicts: none |
