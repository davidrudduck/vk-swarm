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
| `crates/services/tests/filesystem_repo_discovery.rs` | 015 | Not the pattern sibling — alphabetically first integration test, unrelated domain | `electric_task_sync.rs`, declared in `siblings:` — the nearest integration-test harness precedent |
| `crates/services/src/services/approvals.rs` | 013 (`event_bus/tailer.rs`) | Not a pattern sibling — alphabetically first in a 40-file directory | none; 013 restructures 005's own module, whose conventions it inherits directly |
| `crates/server/tests/harness_smoke.rs` | 010 (`crates/server/tests/events.rs`) | Weak sibling — it IS a real harness user; the task body directs reading `crates/server/tests/common/mod.rs` and the neighbouring `*_routes.rs` suites, which is the same convention | none declared; the harness module is named in the task body |
| `crates/db/src/models/activity_dismissal.rs` | 009 (`trigger_cursor.rs`) | Not a pattern sibling — `trigger_cursor` is a single-row-per-key cursor table; closest precedent is `shared_activity_cursor.rs` | noted here rather than in `siblings:` so the gate does not treat it as a creation target |

Note for a future run: `shared_activity_cursor.rs` is the closer precedent for task 009's
`trigger_cursor.rs` and should be read during execution even though the lint did not name it.

## 2026-08-11 tournament 1 remediation: task-level fixes applied via envelope resubmit

The breakdown was rebuilt from the submit envelope and re-promoted atomically — no promoted file was
hand-edited to clear a gate. 12 tasks became 15. Task ids 001-012 were deliberately NOT renumbered so
the tournament reports stay legible against the tree they reviewed; the three new tasks take the next
free numbers and sit in their correct phases.

| New task | Phase | Why it exists |
|---|---|---|
| 013 | 2 | The journal tailer. The component that makes journal-first structural and dissolves the sender-reachability collision. |
| 014 | 5 | Startup wiring. Tasks 009/011/013 each correctly STOPPED rather than editing `local-deployment`, so without this every one of them shipped as dead code — the tailer never runs, the hook is never registered, compaction never spawns. |
| 015 | 3 | The TS3 cross-site suite, split out of 008. Connectivity emission and a cross-crate assertion suite are different failure domains; fused, a bug in either blocked the other's revert. |

Coverage moves, both forced by the exactly-one-claimant gate: **TS3** 008 → 015, and **SC1** 006 →
012. SC1's second clause ("observable via the subscription endpoint") was covered-but-hollow — task
006 verified only journal rows via sqlite, and the endpoint does not exist until task 010. Task 012
now proves both halves together on a live node, which is the only place both exist at once.

Task-level fixes applied (all peer-validated and independently verified against the repo first):

- **004** `allowed_change: create` → `mixed` + `crates/db/src/models/mod.rs`. `pub mod event_journal;`
  is now a first-class step, not an "only if cargo check demands it" aside — the file-set gate rejects
  the latter outright, and task 009 already declared the identical edit correctly.
- **004** the rollback assertion was inverted. It asserted seq values are never reused; SQLite reuses
  them. Now asserts only that COMMITTED seqs strictly increase.
- **004** `append` cannot return `Result<_, sqlx::Error>` while calling `serde_json::to_string(..)?` —
  no `From` impl exists. Added `EventJournalError` with `#[from]` variants per CLAUDE.md.
- **004** `append` is now explicitly generic over `sqlx::Executor`. This is load-bearing: it is what
  lets task 006's delete append onto the route's own transaction.
- **003** the serde-tag test covered one of nine variants; table-driven across all nine with a length
  assertion so adding a variant without extending the table fails.
- **003** `executor` added to `AttemptFinished`/`AttemptFailed`. SC2 requires executor identity on
  terminal events; the field was absent from the contract, so no amount of work in task 007 could
  have satisfied it.
- **003** pinned the `TaskStatus` string form. It has two (`serde` → `inprogress`, `strum` → 
  `in-progress`); emission sites must use serde's.
- **006** `Task::delete` appends on the executor it is GIVEN and does not commit. Its route already
  owns a transaction spanning child nullification.
- **006** `Task::update_status` calls a pool-taking activity-dismissal helper; generalized to take an
  executor and moved inside the transaction, with a test that exercises a task that HAS a dismissal.
- **007** added `mark_orphaned_as_failed`, restructured to SELECT-then-UPDATE in one transaction so
  "one event per transitioned process" is exact rather than inferred from `rows_affected`.
- **008** re-anchored from `hive_client.rs` to `node_runner.rs` (L353/L375), where a `DBService` is
  actually in scope, and given an explicit `was_connected` transition gate. The original anchor was
  doubly broken: a clean close takes the `Ok(())` arm and emitted NOTHING, while every failed retry
  took the `Err` arm and emitted ANOTHER disconnect from an already-disconnected state.
- **008** `ReconcileCompleted` anchored at one completion point with `entity_count` defined; the
  digest-heal pull at L1150 is explicitly NOT a second anchor for the same variant.
- **009** the cursor now advances on every consumed event, not only after a fire (D11).
- **010** declared `crates/server/tests/events.rs` — TS5 requires route tests and the gate rejects
  writes to undeclared paths.
- **010** corrected the SSE precedent: `stream_raw_stream` does not exist (it is `stream_raw_logs`),
  and `routes/logs.rs` is REST/WebSocket, not SSE. The real precedent is the route task 001 deletes,
  read via `git show`.
- **012** no longer requires post-merge `main` evidence in a commit that is part of the PR being
  merged. Deployed feature-branch build plus its SHA.
- **005** `subscribe_from` is now fallible (`Result` of stream of `Result`) and specified as an
  explicit loop rather than five linear steps, pinning fresh-mark capture on refill, that `Lagged(n)`
  is a COUNT not a seq, and that recovery re-enters the live loop.
- **005** dropped `broadcast_only_after_commit`, which tried to broadcast inside a transaction and
  expected rollback to retract the message — impossible, `broadcast::Sender::send` is not
  transactional. Task 013 replaces it with a structural equivalent: an uncommitted row is unreadable,
  so it cannot be tailed.

### One deliberate deviation from a judge's corrected fix (task 006, delete)

Judging grok's delete finding, codex ruled the ISSUE real but the FIX unsound, and supplied its own:
add `delete_with_event(pool, id)` that opens a transaction, loads identity, nullifies children,
deletes, appends, commits and broadcasts, then route every real deletion through it, adding `core.rs`
and `remote.rs` to the file set.

We implemented something different — `Task::delete` appends on the executor it is given and never
commits — so the reasoning is recorded rather than left implicit.

Codex's specific objection was to grok's *conditional* formulation: "append onto that same executor
**when the executor is a transaction**, else open one." That is genuinely not expressible — a generic
`E: Executor` cannot be interrogated at runtime for transaction-ness. The objection is correct.

Our version has no conditional. `Task::delete` ALWAYS appends on the passed executor and NEVER
commits, which is well-typed for every `E: Executor`, and the caller commits both the deletion and
its journal row together. That satisfies D2's actual invariant (the journal write shares the state
write's transaction) while preserving the route's existing atomic unit exactly.

Preferred over codex's fix because that one moves child-nullification — a route-level composition
concern — down into the DB model, edits two route handlers, and creates a second delete entry point
whose existence invites a caller to use the wrong one. Ours changes no caller at all. Codex's remains
the correct fallback if `append` cannot in practice be made executor-generic; task 004 carries a STOP
trigger for exactly that case.

## Task 002

- [Task 002] `event_journal` uses `INTEGER PRIMARY KEY AUTOINCREMENT` (not scalar subquery) — sibling `node_outbox` (20260201000400) assigns `seq` via scalar subquery with `UNIQUE` guard because its PK is `id BLOB` and `seq` is not a rowid alias; here `seq` IS the primary key, so AUTOINCREMENT is both correct and cheaper, and guarantees no reuse after deletion (required for compaction) — `crates/db/migrations/20260812000000_add_event_journal.sql`

## 2026-08-12 execute: orchestrator amendments (sqlx query form)

- [Task 004 orchestrator] Amended the Change section to require the RUNTIME sqlx API
  (`query`/`query_as::<_, T>`/`query_scalar::<_, T>` + `.bind()`) and forbid the `query!` macro
  family; rewrote STOP trigger 3 (it named the macro form the Change section now forbids) and added
  a STOP trigger for `cargo sqlx prepare` — established by probe, not assumption: `crates/db/.sqlx`
  is a TRACKED per-crate offline cache (235 files, `git ls-files`), `DATABASE_URL`/`SQLX_OFFLINE` are
  both unset, and substituting an unknown table into an existing `query_scalar!` fails with
  `error: set DATABASE_URL to use query macros online, or run cargo sqlx prepare to update the query cache`.
  A new macro query therefore needs `cargo sqlx prepare`, whose `crates/db/.sqlx/query-<hash>.json`
  output cannot be declared in `files:` — `task-gate.sh`'s `is_declared()` skips any declared entry
  whose basename contains a dot when expanding directory scopes, so `crates/db/.sqlx` covers nothing
  beneath it. Worst case is silent: `wai-committer.sh` stages only declared files, so the regenerated
  cache is left unstaged, the gate passes on a machine whose cache is warm, and every other machine
  fails to compile. Not a spec divergence — the spec constrains only that `append` be generic over
  `sqlx::Executor` (spec L87, L106) and says nothing about query form, so ADR-0001 does not apply.
  Consistent with the declared sibling `crates/db/src/models/node_outbox.rs:81,100,126`, which already
  uses the runtime form — `docs/plans/vk-swarm-event-bus/phase-1/004-*.md`
- [Task 006/007/015 orchestrator] Widened the same directive after an exhaustive re-scan of every
  task file: 006's and 015's tests read `event_journal` directly, and 007 adds a new `SELECT` of the
  rows about to transition inside its transaction — all three author NEW SQL in a crate with a
  tracked `.sqlx` cache. The first scoping pass (004/009 only) was too narrow. The inserted block is
  deliberately narrower than 004's: re-using an EXISTING macro query verbatim stays allowed, because
  its text is already cached — this matters for 006, which re-runs `Task::create`'s existing
  `query_as!` against `&mut *tx` — `docs/plans/vk-swarm-event-bus/phase-3/{006,007,015}-*.md`
- [Task 009 orchestrator] Same directive applied to `crates/db/src/models/trigger_cursor.rs`, which
  authors the `trigger_cursors` UPSERT and the `MIN(last_processed_seq)` read — the only other task
  that writes NEW SQL. 010/011/013 call task 004's model rather than authoring SQL and were left
  untouched — `docs/plans/vk-swarm-event-bus/phase-4/009-*.md`
- [Run orchestrator] Gate runner override supplied via the `.wai-test-cmd` file channel
  (`cargo test -p "$(basename {scope})"`) with `WAI_TYPECHECK_CMD="cargo check --workspace --all-targets"`.
  Without it the gate auto-detects vitest from the root `pnpm-lock.yaml` and would have run a
  TypeScript runner against Rust crates for all 15 tasks. The channel path is gitignored (commit
  `ccb09d98`) because `task-gate.sh` refuses a tracked channel file and an untracked one could
  otherwise be swept into a task commit — `.gitignore`, `docs/plans/vk-swarm-event-bus/.wai-test-cmd`
- [Task 010 note, raised by the task-003 panel] `SequencedEvent.seq` and
  `NodeEvent::ReconcileCompleted.entity_count` generate as TypeScript `bigint` (`shared/types.ts:33,35`),
  following the project-wide ts-rs `i64` mapping that `Task.remote_version` already uses; no
  `#[ts(type = "number")]` override exists anywhere in `crates/`. An SSE consumer, however, receives a
  plain `number` from `JSON.parse` — the declared type and the runtime value disagree on the INBOUND
  direction, and `frontend/src/lib/api/utils.ts:92` `jsonBody()` only addresses the outbound one.
  Pre-existing convention, NOT introduced by task 003 and not a defect in it; carried here so task
  010's implementer and panel see it when the SSE endpoint gains a real consumer.
- [Task 003 orchestrator] Attempt 1 (`4f244ae4`) passed the deterministic gate but the adversarial
  panel returned DEVIATES on two undeclared type choices, both verified against the repo before acting:
  `exit_code: i32` and `executor: String` were named-but-untyped by the task, so the implementer chose
  silently and declared nothing. Resolved by DICTATING the types in the task rather than letting a
  second implementer re-choose. (a) `exit_code: i64` — `shared/types.ts:854` already types the same
  datum `bigint | null`, so `i32` made one generated file describe one field two ways; it also spares
  task 007 a narrowing `as i32`. (b) `executor: String`, not the TS-registered `BaseCodingAgent`.
  My first rationale for (b) — "closed enums break replay" — was SELF-REFUTING and the expedited
  amendment review caught it: `TaskStatus` is equally closed (same `EnumString`, no `#[serde(other)]`)
  yet is correctly typed. The property that actually discriminates is vocabulary stability:
  `TaskStatus` is pinned to the persisted `tasks.status` column
  (`crates/db/src/models/task/mod.rs:24`) so no variant can be dropped without a migration that can
  also rewrite journal payloads, whereas `BaseCodingAgent`
  (`crates/executors/src/executors/mod.rs:103-117`) is a vendor list that churns with no migration ever
  touching journal rows — and one undeserializable row fails an entire `(cursor, mark]` window
  (ADR-0017 L36-40), wedging every consumer below that seq permanently —
  `docs/plans/vk-swarm-event-bus/phase-1/003-*.md`
- [Task 003 orchestrator] The same review found `old_status`/`new_status` were still undictated inside
  a block titled "Field types are DICTATED", AND that the task's own test skeleton contradicted its
  Change section: `old_status: "todo".into()` compiles only against `String`, so attempt 1 had to
  silently rewrite those constructors to `TaskStatus::Todo`. Both fixed — the `String` escape hatch is
  withdrawn and the skeleton's constructors corrected. This was a defect in the ORIGINAL task text that
  the breakdown tournament and the attempt-1 panel both missed —
  `docs/plans/vk-swarm-event-bus/phase-1/003-*.md`
- [Task 007 orchestrator] Added the `Option<i64>` sourcing clause: dictating a non-optional `i64` on
  the event only MOVED the silent choice downstream, and `unwrap_or(0)` would report a clean exit that
  never happened. 007 now requires emitting `attempt_failed` with a reason naming the missing exit code
  instead of substituting a value — `docs/plans/vk-swarm-event-bus/phase-3/007-*.md`
- [Task 007 orchestrator] Removed stale pre-tournament wording: the `ExecutionProcess::create`
  paragraph ended "commit, broadcast", contradicting the SAME file's Allowed-moves paragraph
  ("**Nothing broadcasts** — the tailer publishes (task 013)") and the journal-tailer decision the
  spec was re-frozen on. Left as written it would have had the implementer either STOP on the
  contradiction or wire a broadcast sender into a DB model — the precise coupling D10 removed. Found
  by sweeping every task file for `broadcast|publish|sender`; 007:56 was the only stale instance
  (006 and 008 state the rule correctly). No expedited review dispatched for this one: it deletes a
  contradiction rather than introducing a decision, and the surviving text is the file's own already-
  reviewed statement — `docs/plans/vk-swarm-event-bus/phase-3/007-*.md`

## 2026-08-12 task 004 execute: sibling comparison and SQLite AUTOINCREMENT observation

### Sibling `crates/db/src/models/node_outbox.rs` structural choices and justified divergences

Sibling `node_outbox.rs` uses:
1. **Separate public/private struct pattern**: `OutboxOp` (public) vs `OutboxOpRow` (private with `sqlx::FromRow`)
2. **Conversion trait for deserialization**: `From<OutboxOpRow>` with best-effort JSON parsing (warns, continues with `Null` on error)
3. **Repository pattern**: `OutboxRepository` struct with associated functions (not instantiated)
4. **Best-effort error posture**: Failed enqueue is logged and swallows the error, allowing partial success
5. **Error type scoped to sqlx**: All functions return `Result<_, sqlx::Error>`

**Event journal divergences** (all justified by the D2 SC1 guarantee):

1. **`EventJournalError` with serde composition**: Append MUST propagate `serde_json::Error` because a silently dropped event breaks SC1/SC2. The contract is SC1 (read replays all events in order) and SC2 (executor identity is durable); dropping an event breaks both. `node_outbox` can be best-effort because a missed outbound op is a liveness issue (the hive retries), not a data loss.

2. **Generic over `sqlx::Executor` not just `&SqlitePool`**: Required to compose with caller-owned transactions. The spec (L87, L106) names this as load-bearing for task 006 (`Task::delete` already owns an outer transaction spanning child nullification). `node_outbox` is a stateless repository with no such requirement.

3. **Direct `FromRow` on `EventJournalEntry`**: Simplified pattern vs `node_outbox`'s separate row struct. The event_journal schema has no BLOB UUID PKs or nullable complex payloads requiring conversion; the single structure is sufficient.

4. **No Repository struct**: Functions are module-level (`append`, `read_range`, `compact`). This is a stylistic choice not dictated by the task; `node_outbox`'s pattern is equally valid and was considered.

### SQLite AUTOINCREMENT rollback observation

Test 3 (`committed_seqs_are_strictly_increasing_across_rollback`) directly probed SQLite's `AUTOINCREMENT` behavior under rollback:

- Appended event, committed: `seq = 1`
- Appended event, rolled back: allocation consumed `seq = 2`
- Appended event, committed: reused `seq = 2` (**REUSE CONFIRMED**)
- `sqlite_sequence` table read directly: `('event_journal', 2)` after the committed second insert

**Finding**: SQLite **does reuse** allocations when a transaction rolls back. The `sqlite_sequence` internal table is itself transactional; the rollback reverts the allocation. This is the conservative direction for the consumer contract (D9 says "consumers must tolerate holes"), so the guarantee holds either way — but the observation confirms SQLite's documented behavior rather than an assumption.
- [Task 008 orchestrator] The expedited review of the `entity_count` amendment found a far bigger
  PRE-EXISTING defect, verified independently before acting: 008's connectivity anchors `L353`/`L375`
  sit in `NodeRunnerHandle::process_event` (`node_runner.rs:349`), whose struct (`:334-343`) holds
  only `event_rx`, `command_tx`, `state`, `_join_handle` — **no `DBService` and no pool**. That is the
  identical defect the task diagnoses about `hive_client.rs`: tournament 1 moved the anchor off
  `hive_client.rs` but landed one layer up, still with no database handle. The loop that actually
  holds `db: DBService` is in `spawn_node_runner` (`:697-701`, loop `:804-1175`) — and it has NO
  `Disconnected` arm at all (verified: arms are Connected `:806`, TaskAssigned, TaskCancelled,
  TaskSyncResponse, LabelSync, BackfillRequest, OpAck, LeaseRevoked, DigestResult, then `Some(_)`
  `:1166`), so hive disconnects currently fall through and are ignored. Anchors corrected and the new
  arm made an explicit, authorised requirement. Left unfixed this task was unimplementable as written
  — `docs/plans/vk-swarm-event-bus/phase-3/008-*.md`
- [Task 008 orchestrator] Three further amendment defects the same review caught, all accepted:
  (a) my `i64::try_from(n).unwrap_or(i64::MAX)` dictate rested on a FALSE premise — `n` is
  `Vec::len()`, bounded by `isize::MAX`, so `as i64` is lossless and the fallback unreachable;
  worse, reporting `i64::MAX` is the same meaningful-looking-lie class that task 007's `unwrap_or(0)`
  ban exists to prevent, so I had imported a rule to justify the thing it forbids. Repo precedent is
  15 `.len() as i64` sites (including `node_runner.rs:1118`) against zero `i64::try_from`.
  (b) step 3 dictated only the `Ok`/`Err` branches and left `remote_client == None` (`:701`) to the
  implementer — the exact undictated-choice pattern that got task 003 rejected, reintroduced by the
  amendment meant to eliminate it. All three branches are now dictated.
  (c) Allowed moves did not authorise the call-site restructure the step requires — the same
  paragraph-vs-Allowed-moves contradiction just found in task 007. Also fixed STOP trigger 1, which
  named `hive_client.rs`'s private `ConnectionState` (`:761`) in a file the task forbids touching,
  contradicting the Change section's own local `was_connected: bool` —
  `docs/plans/vk-swarm-event-bus/phase-3/008-*.md`

## 2026-08-12 execute: task 004 attempt 1 REJECTED (mutation testing found a live bug)

- [Task 004 orchestrator] Attempt 1 (`826545fd`) passed the deterministic gate with 10/10 tests green
  and was REJECTED by the adversarial panel. The decisive evidence was mutation testing: deleting the
  cursor-floor logic from `compact` entirely left the whole suite GREEN, including the test named for
  that guarantee. Three findings, each verified independently against the task text before acting:
  (1) **Live bug.** The task dictated `COALESCE((SELECT MIN(last_processed_seq) FROM trigger_cursors),
  <high_water>)`. COALESCE substitutes only on NULL. The implementation wrote `.unwrap_or(0)` then
  `if cursor_floor == 0 { high_water }`, which ALSO fires on a legitimate `0` — and the migration
  declares `last_processed_seq INTEGER NOT NULL DEFAULT 0`, so a freshly-registered hook that has
  processed nothing is stripped of all compaction protection. Probed live by the panel: 5 rows should
  have survived, 1 did.
  (2) **Hollow test.** `compact_never_crosses_min_trigger_cursor` asserted
  `all(|r| r.0 >= 3)` — the CONVERSE of "every row with seq >= N survives" — and with `min_rows = 1`
  one row survives on the unrelated min-rows floor, making `all()` vacuously true over a single
  element.
  (3) `hard_cap_overrides_cursor_floor_and_flags_rebootstrap` shipped clause (b) as a bare comment
  with no assertion.
  Task sharpened rather than left as-is, because the phrasing PERMITTED the weak reading: exact
  surviving-set assertions, all three clauses of test 10 required in code, COALESCE spelled out as
  NULL-only, and a new test 11 for the `last_processed_seq = 0` boundary that nothing covered —
  `docs/plans/vk-swarm-event-bus/phase-1/004-*.md`
- [Run orchestrator] No task in this plan declares `red_proof`, so `task-gate.sh`'s mutation-proof
  check never runs. The adversarial panel is therefore the ONLY defence against a hollow test on the
  covered tasks, and task 004 is the proof that this is not theoretical — a fully green gate shipped
  a broken cursor floor. Panels for tasks with non-empty `covers_tests` are dispatched with an
  explicit mutation-testing instruction for the remainder of this run.

## 2026-08-12 task 004 attempt 2: fixed cursor floor and hollow tests

Three fixes applied to address the attempt-1 failures:

1. **Cursor floor bug (live): decide on Option, not the value 0.** Rewrote lines 96-115 of 
   `queries.rs` to fetch `cursor_floor_option: Option<i64>` from `MIN(last_processed_seq)`, then 
   `unwrap_or(high_water)`. Deleted the `if cursor_floor == 0 { high_water }` block entirely. This 
   correctly distinguishes "no cursors exist" (NULL) from "cursor at zero" (a real floor) — the 
   migration declares `last_processed_seq INTEGER NOT NULL DEFAULT 0`, so a freshly-registered hook 
   sits at exactly that value.

2. **Hollow test 7: assert the exact surviving seq set, not a loose predicate.** Changed 
   `compact_never_crosses_min_trigger_cursor` from `assert!(rows.iter().all(|r| r.0 >= 3))` to 
   `assert_eq!(surviving_seqs, vec![3, 4, 5])`. With min_rows=1 and all rows old, cursor floor at 3 
   protects seqs 3–5; without that logic, only seq 5 survives (the min_rows floor), which the old 
   assertion would incorrectly pass.

3. **Hollow test 10, clause (b): add a real assertion.** Added 
   `assert!(row_below_floor.is_none())` to confirm that seq 4 (below cursor floor 5) is absent 
   after the hard cap forces deletion. Previously this was a bare comment.

4. **New test 11: regression guard for the zero-cursor boundary.** Added 
   `compact_treats_a_zero_cursor_as_a_real_floor` — one cursor at 0, all rows backdated beyond 
   retention, min_rows=1; asserts all 5 rows survive (seq < 0 is never true). This catches the bug 
   if someone re-introduces the `if == 0` logic.

Mutation proof: deleted cursor floor logic entirely, reran tests; both 7 and 11 failed:
- Test 7: expected [3, 4, 5], got [5]
- Test 11: expected [1, 2, 3, 4, 5], got [5]
Restored from backup; both tests pass. All 11 event_journal tests green, cargo check and clippy 
clean.
- [Task 004 orchestrator] Attempt 2 (`46cb7c62`) drew CONFORMS from both challengers: a 12-mutation
  sweep killed 9, and each of the three attempt-1 findings was re-verified by REINTRODUCING the exact
  bug as a mutation and confirming the matching test now fails. Of the three uncaught mutations, two
  are genuinely unreachable by unit test — SQLite returns rows in index order for this query shape so
  dropping `ORDER BY` is observationally a no-op, and crash-atomicity of the stage-2 flagging cannot
  be observed without fault injection (both verified correct by inspection instead). The THIRD is a
  real coverage gap and is being closed rather than banked: test 10 inserts a single
  `trigger_cursors` row, so "flag only cursors the deletion passed" and "flag EVERY cursor" are
  indistinguishable to it. The shipped predicate is correct, but nothing would catch a later refactor
  breaking it — and the failure mode is healthy consumers being forced to rebootstrap. Task amended to
  require two cursors, one either side of the new minimum —
  `docs/plans/vk-swarm-event-bus/phase-1/004-*.md`
- [Task 011 orchestrator] Two defects of the already-proven classes, found by following task 004's
  `.unwrap_or(0)` shape downstream rather than waiting for 011 to be implemented: (a) test 5 said
  "pick one behaviour (clamp … is preferred) and pin it" — an undictated choice of exactly the kind
  that got task 003 rejected; now DICTATED as clamp-with-warning. (b) nothing covered
  `VK_EVENT_MAX_ROWS=0`, which would let stage 2 empty the journal, after which `compact`'s
  post-delete `MIN(seq)` returns NULL and falls back to `0`
  (`crates/db/src/models/event_journal/queries.rs:169-174`), so `WHERE last_processed_seq < 0` flags
  NOTHING — every consumer loses every event and none is marked for rebootstrap. Silent loss with no
  signal, the same Option-collapsed-to-zero shape as the 004 bug. 011 must now sanitise `min_rows` and
  `max_rows` to a floor of 1 (and `max_rows` up to `min_rows`) before every `compact` call, with a
  warning; task 004 deliberately takes its arguments as given, so the loop is the only place the
  invariant can hold — `docs/plans/vk-swarm-event-bus/phase-5/011-*.md`
- [Task 004 orchestrator] Attempt 3 (`6f1dc922`) is test-only and was NOT sent to a third panel
  round. Recording the call rather than leaving it implicit: both challengers had already CONFORMED
  the production code on attempt 2; attempt 3 touches one test file, only ADDS assertions (verified
  by reading the diff — clause (c) is preserved and re-asserted, (a) and (b) untouched), and carries
  its own mutation proof (flagging every cursor unconditionally makes the new assertion (d) fail with
  `left: 1, right: 0`, then restores green). A challenger would have re-derived exactly that.
- [Task 005 orchestrator] Corrected a flat-vs-directory-module contradiction before dispatch: the
  "Failing test" header named `crates/services/src/services/event_bus.rs` while `files:` and the
  Change section both require `event_bus/mod.rs`. An implementer following the test header would have
  created an undeclared path and the file-set gate would have rejected the commit. This is the THIRD
  instance of the class (013 at decompose time, 007's "commit, broadcast", now 005), so every task
  file was swept programmatically — comparing every `crates/**.rs|.sql` path mentioned in the body
  against the `files:` frontmatter for directory-module collisions. No further instances exist —
  `docs/plans/vk-swarm-event-bus/phase-2/005-*.md`
- [Task 005 orchestrator] Pre-flight for its STOP triggers: `async-stream` is NOT a dependency of
  `crates/services`; `tokio-stream 0.1.17`, `futures 0.3.31` and `futures-util 0.3` ARE. The task
  already sanctions "return a boxed stream" in that case and requires the concrete return type in the
  ledger, so this is a declared choice rather than a STOP. Cargo.toml is NOT in the task's `files:`,
  so adding a dependency is out of bounds — STOP if one seems necessary
