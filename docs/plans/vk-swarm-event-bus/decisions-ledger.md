# vk-swarm-event-bus — decisions ledger

## 2026-08-07 precheck: anchor-check false positive (documented per CLAUDE.md no-deferred-remediation)

`wai-precheck.sh` assert 3 flagged `src/services/event_bus.rs` as "referenced as existing
but ABSENT on main". Doubly a false positive:

1. The spec cites `crates/services/src/services/event_bus.rs` as a **new file the design
   creates** (Design section, "Bus core"), not as an existing anchor.
2. The extractor truncated the `crates/services/` prefix before probing main.

Evidence: `git grep -c 'crates/services/src/services/event_bus.rs' docs/superpowers/specs/2026-08-07-vk-swarm-event-bus.md` → the only form the spec uses; no bare `src/services/event_bus.rs` reference exists in the doc.

Precheck re-run with `--no-anchor-check` per the skill's false-positive instruction; all
other asserts pass unmodified.

## 2026-08-11 decompose HALTED on four frozen-spec contradictions; spec + ADR-0017 amended

`/wai:decompose` halted before authoring any plan/phase/task file. Four contradictions between
the spec (frozen at `spec_sha=c9a1dc75de0760d4160333b3eb84cdd1515aae12`) and merged `main`:

| # | Contradiction | Evidence |
|---|---|---|
| C1 | Spec says create `crates/server/src/routes/events.rs` and mount `GET /api/events`; both already exist, serving the record-patch SSE stream | `crates/server/src/routes/events.rs` (28 lines); mounted `crates/server/src/routes/mod.rs:20,72`; nested under `/api` at `:85` |
| C2 | SC7/D5/TS5 assume a board poll loop to replace and react-query-backed board state; neither is true | `frontend/src/pages/ProjectTasks.tsx:290` → `useProjectTasks`; local projects stream over `/api/tasks/stream/ws` (`useProjectTasks.ts:80-92`; route `crates/server/src/routes/tasks/mod.rs:66`); `refetchInterval` only when `isRemote` (`:107`) |
| C3 | Spec anchors connectivity events at `HiveSyncService` connect/disconnect/reconcile; that service has no such transitions | `hive_sync.rs` exposes only `sync_once` (`:167`) and `sync_local_projects` (`:697`); real transitions at `hive_client.rs:819,907`; reconcile leg in `node_runner.rs:812,1150` |
| C4 | D2/ADR-0017 pin `emit(&mut tx, …)` "same transaction as the state change"; NO emission site had a transaction | `task/queries.rs:290,327` `fetch_one(pool)` then post-write `enqueue_task_upsert_op`; `container.rs` has no `begin()`/`Transaction` at all; connectivity is an mpsc send with no DB write |

C1 and C3 slipped past `/wai:precheck` because the 2026-08-07 run used `--no-anchor-check`
(entry above). Root cause of the false positive, now pinned: `wai-precheck.sh:241` extracts path
anchors with `grep -noE '(src|extensions|ui|packages|apps)/…'` — no `crates/` alternative and no
prefix capture, so every monorepo path (`crates/*/src/…`, `frontend/src/…`) is truncated before
being probed against `main`. This is a WAI plugin limitation, not a spec defect, and it will
misfire on every vk-swarm spec.

**Compensating control adopted this run:** rather than blanket-suppress again, all 19 distinct
path anchors in the amended spec were verified individually against `main` — 18 EXIST as
referenced, 1 (`crates/services/src/services/event_bus.rs`) is ABSENT and is the file the design
creates. All 15 line-number citations added to the spec were also read back and confirmed.
Only then was precheck re-run with `--no-anchor-check`.

### Decisions taken (spec owner, 2026-08-11)

- **C1 → repurpose `/api/events`, deleting the old route first** (spec D7). Chosen over a new
  path so the public contract P7 and external tools bind to is unambiguous, and so dead code is
  removed while still cheap. Blast radius measured before deciding: the route file (28 lines),
  two lines in `routes/mod.rs`, and the orphaned `Deployment::stream_events` default trait method
  (`crates/deployment/src/lib.rs:197-205`, whose only caller is that route). `EventService` is
  NOT touched — it continues to back `/api/tasks/stream/ws`. Repo-wide grep for `api/events`
  found zero consumers outside the route's own definition and the spec.
- **C2 → drop the UI surface entirely.** SC7 deleted, D5 withdrawn, US1 restated, TS5/TS6
  narrowed. Rationale recorded in the spec's Out of scope: the WS patch stream is a read-model
  push (view state) and the bus is a domain event log (facts); re-plumbing the board onto the log
  would need either a fetch per event or a fattened event schema that loses typed-lifecycle
  purity. Any future unification should drive the patch channel FROM the bus server-side.
- **C3 → anchors repointed** to `hive_client.rs` and `node_runner.rs`.
- **C4 → the DB model function owns the transaction** around its own discrete write statement
  (spec D2, Design "Emission ownership"). Caller signatures stay `&SqlitePool`, so nothing is
  threaded through callers — which is why the `node_outbox` precedent's stated objection
  (`task/queries.rs:337`, "threading a shared txn through all `Task::create` callers is OUT of
  scope") does not apply. Deliberately excludes wrapping `ContainerService::start_execution`,
  which performs git I/O at `container.rs:1516-1523`; holding SQLite's single writer lock across
  it would be a node-wide liveness hazard. Verified that `ExecutionProcess::create`
  (`execution_process/queries.rs:361`) is a single `INSERT … RETURNING` with that git I/O
  *preceding* it, so `attempt_started` is fully transactional rather than best-effort.

Two design gaps the collisions exposed were closed in the same amendment: the
`crates/db` → `crates/services` layering of the broadcast sender (D8 — sender lives in the db
layer, `crates/db/Cargo.toml:20` already has tokio) and the non-contiguous `seq` contract
(D9 — a rolled-back transaction may consume a value; a seq hole is not a missed event).

### Accepted risk (recorded, not mitigated)

With the UI dropped, the `/api/events` SSE endpoint ships with **no first-party in-repo
consumer** — its named consumer is P7 (MCP/ACP connectivity), a later phase of the same program
(`docs/superpowers/specs/2026-06-25-vk-swarm-refactor.md:36,38`). This is a deliberate
ship-the-contract-ahead-of-its-consumer choice, accepted rather than mitigated: adding an
artificial first-party consumer purely to exercise the wire format would be the actual
redundancy. The hard part of the contract (replay-to-live cursor handoff) IS exercised
first-party in-process by the trigger-hook runner (SC6), and SC4's live acceptance proves the
wire end-to-end with a real external client. Residual: SSE framing drift would be caught only by
SC4/TS5 until P7 lands.

### ADR-0017 amended in place

Amend-in-place with a dated `## Amendment` section, not superseded: the ADR was four days old,
unreleased, and no code depended on it. `dev-docs/adr/` had no prior amendment or superseding
convention (all 16 ADRs `accepted`, no amendment precedent), so this establishes one. The
irreversible core — journal-first, in-process broadcast, monotonic seq cursors, at-least-once,
one typed enum, no external broker — is unchanged.

Spec re-frozen: `spec_sha=8b2c864b5b8679acfd0e278d2728731e3b720ba4`.

## 2026-08-11 tournament 1 HALTED the handoff; spec + ADR-0017 amended a second time

The adversarial breakdown review (`reviews/tournament-1.md`) found the 12-task breakdown at
`ae3e807e` **substantially defective**. Codex (23 findings) and grok-4.5 (13) converged
independently on seven identical defects at identical anchors; kimi-k3 timed out, so the round
closed with two competitors plus mutual non-self peer judging. Full scoreboard and the five findings
the peer gate *killed* are in the review file.

Two findings were contradictions internal to the frozen spec, escalated per ADR-0001 rather than
patched into tasks. Both were decided by the spec owner:

| # | Collision | Decision | Why this option |
|---|---|---|---|
| E1 | Design required publish-inside-the-model AND sender-in-db-layer AND unchanged `&SqlitePool` signatures — jointly unsatisfiable; a pool has no back-reference to its `DBService` | **Journal tailer** (D10; D2/D8 revised) | Dissolves E2 as a side effect, zero caller churn, makes journal-first structural, kills the broadcast-before-commit bug class |
| E2 | `Task::delete` is generic over `E: Executor` and its route owns an outer txn, so "the model opens its own transaction" could never apply | resolved BY E1 — a generic `append` composes with a caller-owned txn | no separate decision needed |
| E3 | Bounded-journal Constraint vs "compaction never deletes at/above the min trigger cursor" — a dead hook pins the journal forever | **`VK_EVENT_MAX_ROWS` hard cap overriding the floor, marking passed cursors for rebootstrap** (D6 revised) | only option that satisfies the Constraint absolutely; makes loss explicit rather than silent |

Rejected for E1, with the evidence that killed each:

- **Process-global `OnceLock` in `crates/db`** — production builds `DBService::bootstrap()` before the
  live service (`crates/local-deployment/src/lib.rs:155-166`), so a single-assignment global would
  permanently capture the *bootstrap* sender. Independently, `create_test_pool()` gives each test its
  own DB inside one shared process, so a single global cross-publishes between tests. The first
  argument came from the peer judge, not from either finder — the judge round earned its cost here.
- **`DBService`-owned emitting wrappers** — architecturally sound, but forces migration of every
  caller of the six emission functions, which is exactly the churn D2 exists to avoid.

Accepted cost of the tailer: publication latency is tail-interval-bounded rather than immediate, plus
one supervised background task per node. Immaterial — every named consumer (P6 triggers, P7 MCP/ACP
observability, the SSE endpoint) is non-interactive. Reversible: restoring synchronous publish later
needs only a sender path at the emission sites and changes none of the seq/cursor/at-least-once
contracts P6/P7 bind to.

Two corrections needed no spec-owner decision because the spec had already deferred them or was
silent:

- **D9's stated mechanism was factually wrong.** It claimed a rolled-back transaction "may consume a
  seq value". Direct probe: committed `seq=1`; allocation `2` inside a rolled-back txn; the next
  commit **reused** `2`; `sqlite_sequence` read back `('j', 2)` — it is itself transactional. The
  consumer-facing contract stands unchanged because it is the conservative direction. The spec had
  already said this was "asserted by a test rather than assumed", so only task 004's assertion
  direction was wrong, not the decision.
- **`ExecutionProcess::mark_orphaned_as_failed`** (`crates/db/src/models/execution_process/queries.rs:115-131`)
  is a bulk running→failed transition invoked from startup recovery — a real terminal outcome SC2
  requires, which the Design's emission-site list had omitted entirely.

D11 was added: the hook runner persists its cursor after every consumed event, not only after a fire.
Advancing only on a fire leaves non-matching events unacknowledged forever, causing infinite replay
across restarts and pinning compaction at the first non-matching event.

Spec re-frozen: `spec_sha=5b2ce1af399d459da9789c4d46b709ff40351d61` (was
`8b2c864b5b8679acfd0e278d2728731e3b720ba4`). Precheck anchor-check was again suppressed only after
all 19 path anchors were verified by hand against `main` (18 exist; `event_bus.rs` is the file the
design creates) — same compensating control as the first amendment, same upstream cause
(`agent-plugins` issue #86, which this run confirmed also truncates `crates/*/src/lib.rs` to
`src/lib.rs`).

## 2026-08-11 decompose: sibling-advisory acknowledgement (plan-lint SC6 `W:` lines)

`wai-plan-lint.sh` emits a same-directory sibling advisory per created file. It names the
alphabetically-first unlisted neighbour, which in every case here is NOT the pattern sibling. Each
`W:` is acknowledged below; the REAL pattern siblings are declared in the tasks' `siblings:` fields
(the decomposer's cross-directory judgement, which the lint structurally cannot see).

| `W:` names | Task | Verdict | Real sibling declared in `siblings:` |
|---|---|---|---|
| `crates/db/migrations/20250617183714_init.sql` | 002 | Not a pattern sibling — the initial schema dump, not a durable-log migration | `20260201000400_add_node_outbox.sql` |
| `crates/db/src/models/activity_dismissal.rs` | 003 | Not a pattern sibling — a simple CRUD model, not a serde-tagged TS-exported enum | none needed; the pattern is the ts-rs `decl()` registration in `generate_types.rs` |
| `crates/services/src/services/approvals.rs` | 005, 009, 011 | Not a pattern sibling — alphabetically first in a 40-file directory; unrelated domain | 005 → `events.rs` (the cross-directory `EventService` naming-collision risk); 009 → none; 011 → the WAL-monitor loop, named in the task body |
| `crates/services/tests/electric_task_sync.rs` | 008 | Weak sibling — it IS the nearest integration-test harness precedent, and the task body already directs reading the existing test conventions | none declared; harness shape is followed, not its domain logic |
| `crates/db/src/models/activity_dismissal.rs` | 009 (`trigger_cursor.rs`) | Not a pattern sibling — `trigger_cursor` is a single-row-per-key cursor table; closest precedent is `shared_activity_cursor.rs` | noted here rather than in `siblings:` so the gate does not treat it as a creation target |

Note for a future run: `shared_activity_cursor.rs` is the closer precedent for task 009's
`trigger_cursor.rs` and should be read during execution even though the lint did not name it.
