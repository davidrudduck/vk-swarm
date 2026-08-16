# vk-swarm-event-bus Plan

## Spec
docs/superpowers/specs/2026-08-07-vk-swarm-event-bus.md

## Approach
Journal first, broadcast second (ADR-0017, amended twice on 2026-08-11). A new event_journal
table takes one typed row per covered lifecycle change, appended in the same transaction as the
discrete state-write statement. EventJournal::append is generic over sqlx Executor, so it composes
both with a model function that opens its own transaction (the pool-taking sites) and with one handed
a caller-owned transaction (Task::delete, whose route already owns an outer transaction spanning
child nullification). Caller signatures are unchanged either way.

Publication is DECOUPLED from emission. Model functions append only; a per-DBService background
tailer reads rows above its last published seq and sends them on an in-process tokio broadcast
channel. Because a row is only readable once its transaction has committed, journal-first ordering is
structural rather than a convention an implementer can forget, and a rolled-back transaction cannot
produce a phantom broadcast. The Sender and the tailer both live in
crates/services/src/services/event_bus.rs, which also owns the subscribe_from replay-to-live handoff
every consumer shares. crates/db needs no sender at all — that is precisely what makes the
unchanged-signature rule satisfiable (tournament 1 proved the alternative unbuildable: a &SqlitePool
has no back-reference to its owning DBService).

Two consumers ship: an internal TriggerHook registry with per-hook persisted cursors (the P6 seam,
proven with one real hook) and a GET /api/events SSE endpoint with cursor resume. That path is
currently occupied by a consumer-less record-patch SSE route, which is deleted first as a standalone
irreversible step so the new route is a clean create.

Emission is instrumented at four choke points: task CRUD in crates/db/src/models/task/, the
execution-process create and completion writes, the bulk orphan-recovery failure transition, and hive
connectivity transitions — anchored in node_runner.rs, where a DBService is actually in scope, rather
than in HiveClient, which holds no database handle. A periodic compaction task bounds the journal,
with a hard row cap that overrides the trigger-cursor floor so a dead consumer cannot pin it forever.
UI is explicitly out of scope — the board already streams live over /api/tasks/stream/ws.

## Phases
- **Phase 1: Foundation** — Clear the /api/events path, then land the durable substrate: schema, typed event contract, and the journal model with an executor-generic append.
- **Phase 2: Bus core** — The broadcast channel, the journal tailer that feeds it, and the shared replay-to-live contract every consumer binds to: subscribe_from with journal catch-up, live handoff, dedupe, and Lagged refill.
- **Phase 3: Emission** — One journal row per state change at every choke point, appended in the same transaction as its discrete write statement, then proved end to end by one cross-site suite.
- **Phase 4: Consumers** — The P6 trigger-hook seam with persisted cursors, and the external SSE subscription endpoint on the freed /api/events path.
- **Phase 5: Bounding, wiring and acceptance** — Bound the journal with env-tunable compaction and a hard cap, start every background loop on a real node, then prove the SC list live including restart durability and offline coverage.

## Tasks

| id | phase | title | deps | conflicts |
|---|---:|---|---|---|
| 001 | 1 | Delete the consumer-less /api/events record-patch route and its orphaned trait method | dep: none | conflicts: none |
| 002 | 1 | Add the event_journal and trigger_cursors migration | dep: none | conflicts: none |
| 003 | 1 | Define the NodeEvent and SequencedEvent typed contract and export it via ts-rs | dep: none | conflicts: none |
| 004 | 1 | Add the event_journal model with append, cursor range-read, and compaction | dep: 002 003 | conflicts: none |
| 005 | 2 | Add the EventBus with the broadcast channel and the subscribe_from replay-to-live contract | dep: 004 | conflicts: none |
| 013 | 2 | Add the journal tailer that publishes committed rows onto the broadcast channel | dep: 005 | conflicts: none |
| 016 | 2 | Make the tailer give-up defect unrepresentable, and the tailer observable | dep: 013 | conflicts: none |
| 017 | 2 | Add the end-to-end bus seam suite that hand-drives nothing | dep: 013 016 | conflicts: none |
| 018 | 2 | Close the EventBus startup race by awaiting tailer readiness | dep: 013 016 017 | conflicts: none |
| 019 | 2 | Make the seam suite catch tailer cursor defects by giving the tailer a non-zero start mark | dep: 017 018 | conflicts: none |
| 006 | 3 | Emit task lifecycle events from the task model inside its own transaction | dep: 005 | conflicts: none |
| 007 | 3 | Emit attempt lifecycle events from the execution-process create and completion writes | dep: 006 | conflicts: none |
| 008 | 3 | Emit hive connectivity events from the node_runner event loop | dep: 007 | conflicts: none |
| 020 | 3 | Emit TaskCreated for the child tasks a breakdown acceptance creates | dep: 004 006 | conflicts: none |
| 022 | 3 | Emit task_created / task_status_changed from Task::upsert_remote_task | dep: 006 | conflicts: none |
| 021 | 3 | Add the emission conformance guard (architecture fitness test) | dep: 006 007 020 022 | conflicts: none |
| 015 | 3 | Add the cross-site emission integration suite | dep: 006 007 008 020 022 | conflicts: none |
| 009 | 4 | Add the TriggerHook seam with persisted per-hook cursors and one real hook | dep: 005 | conflicts: none |
| 010 | 4 | Add the GET /api/events SSE endpoint with cursor resume on the freed path | dep: 001 005 | conflicts: none |
| 011 | 5 | Bound the journal with an env-tunable periodic compaction task | dep: 004 009 | conflicts: none |
| 014 | 5 | Wire the EventBus, tailer, trigger-hook runner and compaction loop into deployment startup | dep: 009 011 013 | conflicts: none |
| 012 | 5 | Record live acceptance for task-lifecycle observability, restart durability and full offline coverage | dep: 010 014 015 | conflicts: none |
