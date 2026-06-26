---
topic: vk-swarm-node-foundations
doc_type: decisions-ledger
---

# Decisions ledger — vk-swarm-node-foundations

The executor appends per-task decisions here. Below are **pre-empted traps** (decomposer-resolved
conventions) every implementer must follow, so they never have to re-decide them. Recorded at
decompose time from the spec, ADRs, and an advisor review of the breakdown.

## Pre-empted traps (read before executing ANY task)

### Trap 1 — WAI gate is TypeScript-shaped; this is a Cargo workspace
The shared task-gate skips the TS type-check for non-TS repos and its `scope_test` runner detection
has **no native `cargo test` path** (`.py`→pytest, `.mjs/.cjs/.test.*`→node, dir→vitest/node). Every
Rust task therefore sets BOTH overrides in its `## Done when` line:
- `WAI_TYPECHECK_CMD="cargo check -p <crate>"` (or `--workspace` when arms span crates)
- `WAI_TEST_CMD="cargo test -p <crate> <test_name>"`
A Rust task that omits these silently runs the wrong (or no) check and the gate's "pass" is hollow.

### Trap 2 — SQLx is OFFLINE-mode here; build against a live migrated DB, do NOT `cargo sqlx prepare` in a task
This repo commits a `.sqlx` offline cache (**211 tracked `.sqlx/query-*.json`**) and leaves
`DATABASE_URL` unset (`.env:24` commented) — so by default `query!`/`query_as!` validate against the
cache, NOT a live DB. A task that adds a migration + a new query (102, 104, 304, 305, 405) will FAIL to
compile against the stale cache.

**Do NOT run `cargo sqlx prepare` inside a task.** It rewrites the tracked `.sqlx/*.json`, and those are
NOT in any task's `files:`. `task-gate.sh`'s dir-scope trick can't cover them: `.sqlx`'s basename has a
leading dot, so `${d##*/}` matches `*.*` and the gate treats `.sqlx` as a *file*, not a directory — the
regenerated cache files are rejected as "outside files:" (verified against task-gate.sh `is_declared`).

**Instead: execute schema/query tasks against a LIVE migrated dev DB.** Export
`DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` and apply migrations (`sqlx migrate run --source
crates/db/migrations`, or let the dev server auto-migrate on startup) so `query!` macros check the live
schema. `SQLX_OFFLINE` is NOT forced anywhere, so a set `DATABASE_URL` takes precedence over the cache.
The gate's `cargo check`/`test` then pass WITHOUT touching `.sqlx`. **`DATABASE_URL` exported to a
migrated dev DB is a precondition of executing these tasks** (the opencode runner must set it).

**`.sqlx` regeneration is a single closeout step, intentionally OUTSIDE the per-task gates:** after all
schema/query tasks land, run `cargo sqlx prepare --workspace` ONCE and commit the `.sqlx` delta as a
standalone housekeeping commit at `/wai:close` (so offline builds/CI work again). Do NOT try to fold it
into a gated task. Migration-only tasks (101, 103) add no query and are safe alone.

### Trap 3 — Rust module registration + `enum_dispatch` exhaustiveness
`CodingAgent` is a hand-written enum with **explicit match arms** (capabilities @ `mod.rs:211`,
mcp_config @ `mcp_config.rs:410`, follow-up @ `coding_agent_follow_up.rs:56`, initial @
`coding_agent_initial.rs:53` — upstream line refs). Adding a variant requires editing EVERY arm in the
same commit or the workspace won't compile under `-D warnings`. This is why task 201 is a single
`mixed` task, not a create+wire split. Mirror the upstream registration diff exactly.

### Trap 4 — Recovery lives in the `services` trait, NOT `local-deployment`
Verified against `main`: `cleanup_orphan_executions`, `start_execution`, and `start_execution_inner`
are **all methods on the `ContainerService` trait** in `crates/services/src/services/container.rs`
(lines 239, 1064, 445). Recovery reaches relaunch via `self.start_execution_inner(&task_attempt,
&execution_process, &executor_action)` — same-trait dispatch into the `local-deployment` impl, **no
cross-crate inversion**. Therefore tasks 303 and 304 BOTH edit `services/.../container.rs` and
genuinely `conflicts_with` each other (same file, sequenced by `depends_on`). Do not try to put
recovery code in `local-deployment` — `services` does not depend on it.

### Trap 5 — Anchor-checker strips `crates/*/` (precheck false-positive)
`wai-precheck.sh`'s anchor regex extracts bare `src/...` suffixes and looks for them at repo root; in
this workspace everything is under `crates/<crate>/src/`. Precheck was run with `--no-anchor-check`
for this reason (all four flagged files were verified to exist at full paths). Not a task constraint —
recorded so a re-run of precheck is not mistaken for a real contradiction.

### Convention — sync plumbing is OFF-LIMITS in Phase 4 (SC5d, the negative clause)
Tasks 401–405 filter at the read/API/frontend layer ONLY. They must NOT touch `upsert_remote_task`,
the share publisher, the WS node runner, or the `remote_*`/`shared_task_id` columns. This is a
*negative* success clause (SC5d): verified by those files being ABSENT from every task's `files:` list
(the gate rejects edits to unlisted files), not by a positive task. ADR-0002 is the authority.

### Decompose decision — resume-intent column accessed via dedicated scalar queries (103/104/304)
`ExecutionProcess` is a `FromRow` struct and every `query_as!(ExecutionProcess, …)` lists ALL columns
explicitly (queries.rs, sync.rs, lifecycle.rs). Adding the resume-intent field to the struct would
force editing **every** SELECT — large blast radius, easy to miss one, fails the "one concern" rubric.
**Decision:** task 103 adds the column via migration but does NOT add it to the `ExecutionProcess`
struct. Recovery reads/writes it through dedicated scalar queries (e.g. `set_resume_state(pool, id,
state)`, `find_resumable_running(pool)`) added in 304. The assembling view (104) exposes it for the
queryable surface (SC3b). This keeps each task surgical and the struct-mapped queries untouched.

### Decompose decision + OPEN judgment call — resume prompt semantics (301/303) ⚠ FLAG FOR REVIEW
Verified mechanism: resume re-entry builds a `CodingAgentFollowUpRequest { prompt, session_id,
executor_profile_id }` (`crates/executors/src/actions/coding_agent_follow_up.rs:14-55`) wrapped in an
`ExecutorAction`, then calls `self.start_execution_inner(&task_attempt, &execution_process,
&executor_action)`. `executor_profile_id` comes from the stored `execution_processes.executor_action`;
`session_id` from `find_latest_session_id_by_task_attempt` (`queries.rs:~208`). **The one genuine
judgment call:** what `prompt` to pass when resuming a *crashed mid-run* process. `spawn_follow_up`
requires a prompt, but a crash-resume has no new user instruction. This depends on per-executor
`--resume` semantics (replay vs continue) — which is exactly what the **301 capability audit** must
determine. Task 303 specifies the mechanism and carries a STOP-and-decide point: default to a minimal
continuation prompt, but 301's findings may override. This is the spec's only underspecified point;
it is surfaced to the user and the adversarial review rather than papered over.

### Decompose decision — fence builds on the EXISTING `process_inspector`; fingerprint = worktree-cwd match (302/304)
ADR-0001 names the PID-reuse fingerprint as "process start-time / pgid" (by example). A persisted
start-time would require a spawn-path edit + new column — not in this plan. **Decision (revised after
breakdown-review R2/F4):** do NOT reimplement liveness/kill on raw sysinfo — the repo ALREADY has
`crates/services/src/services/process_inspector/` (`ProcessInspector` trait + `SysinfoProcessInspector`
+ `MockProcessInspector`) providing `process_exists`, `kill_process` (SIGTERM→SIGKILL),
`get_process_tree`, and **`find_processes_by_cwd_prefix`**. Task 302 builds a thin `fence()`
orchestration ATOP that trait. The PID-reuse fingerprint is the **worktree-cwd match**
(`find_processes_by_cwd_prefix(container_ref)` — a stronger guard than a cmdline heuristic): a live PID
whose cwd is not under the attempt's worktree is a reused PID → NOT killed. No schema change, no new
dependency, and the existing `MockProcessInspector` makes 302 hermetically testable. (Reinventing the
inspector was the original 302's mistake — the exact sibling-duplication antipattern the review caught.)

### Sibling-advisory acknowledgement (wai-plan-lint `W:` lines, SC6)
The lint emits advisory `W:` warnings for new files placed beside unlisted same-directory siblings.
Acknowledged here per the decompose contract:
- **Migration files (101/103/104 beside `…_init.sql`)** — migrations are independent forward-only DDL,
  NOT reimplementations of a pattern. Each task's Change gives exact SQL and follows the house
  conventions confirmed against recent migrations (BLOB UUID PKs, `datetime('now','subsec')`,
  `CREATE … IF NOT EXISTS`, partial indexes). No sibling-divergence risk; not pattern siblings.
- **New leaf modules (104 `workstream_state.rs`, 201 `qa_mock.rs`, 302 `process_fence.rs`)** — each
  carries an explicit `## Sibling alignment` step naming a real sibling to read and match (style, error
  type, trait surface, tests). These are the genuine pattern-sibling cases and are handled in-task.
- **503 `check-npm-runtime-vulns.mjs` beside `check-i18n.sh`** — different language/purpose; the
  Phase-5 author read `check-i18n.sh` and justified divergence in 503's Sibling-alignment section.
- **405 `HiveSyncStatusCard.tsx` beside `BackupsSection.tsx`** — same settings-section shape;
  `BackupsSection.tsx` IS a structural sibling. The task 405 implementer MUST read it and match
  its Card/section layout, prop types, and query hook conventions. Missing from the original `files:`
  list but recorded here per W: policy.

### USER-APPROVED SCOPE SPLIT — entangled remote-display removal → `vk-swarm-node-ui-localize` (Phase 4)
Decompose discovered that ADR-0002's "remove remote-display surfaces" is NOT a clean delete: the
remote card badges, `useMergedProjects` (which `ProjectList`/`ProjectSwitcher` are *typed on*), and the
remote stream/diff hooks (`useNodeLogStream`, `useDiffStream`, `useRemoteConnectionStatus`,
`useAvailableNodes`) are **entangled with live local UI** (dual-purpose hooks) — a multi-component
frontend repoint, not deletes. Escalated to the user (frozen-spec contradiction protocol). **User
decision (2026-06-26): carve this into its own workstream `vk-swarm-node-ui-localize`**, spec'd /
prechecked / decomposed separately and sequenced AFTER node-foundations.
- **What node-foundations DOES deliver for SC5:** the backend visibility discriminator (401), removal
  of the request-time remote merge (402), removal of the node-surface remote API proxies (403),
  deletion of the self-contained Nodes-management feature (404), and the read-only hive-sync view (405).
  This makes the node local-only at the **data / API / dedicated-feature** layer.
- **What is deferred to `vk-swarm-node-ui-localize`:** repointing the dual-purpose display hooks +
  `useMergedProjects` + remote card badges so the *local* views render local-only state. Until that
  ships, the local UI may still surface some remote state through those shared hooks — an accepted,
  tracked gap, not an oversight. SC5's lint-coverage (claimed by 401–405) reflects the structural core;
  this note records the honest residual so the Round-2 fidelity lens is satisfied transparently.
- Tracker seeded at `dev-docs/workstreams/vk-swarm-node-ui-localize/README.md` (status: draft, no spec
  yet — a future `/wai:prd-new`+`/wai:spec`). The entanglement map lives there as the seed.

## Per-task decisions (executor appends below)

### Task 101 — Add queued_messages table migration
- **Timestamp selection:** Verified `20260201000000` sorts AFTER latest migration (`20260131000000_add_webhooks.sql`). No collisions detected.
- **FK column type:** Confirmed `task_attempts.id` uses `BLOB PRIMARY KEY` (verified: `20250617183714_init.sql:43`). Migration uses `BLOB` for both `id` and `task_attempt_id`.
- **Migration file location:** `crates/db/migrations/20260201000000_add_queued_messages.sql` created successfully.
- **Schema validation:** File exists: `ls crates/db/migrations/20260201000000_add_queued_messages.sql` ✓
- **DB test pool:** `cargo test -p db --lib` exit status: 0 (180 tests passed, 0 failed).
- **Table schema:** `sqlite3 dev_assets/db.sqlite ".schema queued_messages"` output:
  ```sql
  CREATE TABLE queued_messages (
      id              BLOB PRIMARY KEY,
      task_attempt_id BLOB NOT NULL,
      content         TEXT NOT NULL,
      variant         TEXT,
      position        INTEGER NOT NULL,
      created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
      FOREIGN KEY (task_attempt_id) REFERENCES task_attempts(id) ON DELETE CASCADE
  );
  CREATE INDEX idx_queued_messages_attempt_position
      ON queued_messages(task_attempt_id, position);
  ```
- **Commit:** `9945d61feca236ebba6c11a01677e647ce6ee125`

### Task 102 — Back MessageQueueStore with queued_messages table + boot drain
- **Scope statement:** Task implements durability only (SC2-survive); boot drain is owned by task 305. No boot-drain logic added (verified: `try_consume_queued_message` called only at `container.rs:738` inside exit monitor).
- **File identification:** Task touches only `crates/local-deployment/src/message_queue.rs` (store impl + tests) and `crates/local-deployment/src/container.rs` (constructor call site).
- **Constructor site verification:** Single call at `container.rs:127`; grep shows all test invocations are within message_queue.rs test module. All sites updated to pass pool.
- **Pool access:** Used `db.pool.clone()` from `DBService` parameter already in scope at `LocalContainerService::new`.
- **Type annotations for sqlx::query!():** 
  - Used `as "field!: Uuid"` (non-null) for id and task_attempt_id fields
  - Used `as "created_at!: DateTime<Utc>"` to parse TEXT created_at into DateTime
  - Position stored as SQLite INTEGER (i64) cast to usize on SELECT, cast back on INSERT
  - Variant and content remain TEXT/nullable as in schema
- **Error handling degradation:** All DB errors logged and degraded to return signature (Vec::default(), None, false) so method signatures unchanged.
- **Position re-packing:** Implemented in `remove()` and `pop_next()` via `UPDATE queued_messages SET position = position - 1 WHERE task_attempt_id = ? AND position > X` to maintain 0-based contiguity.
- **Transaction usage:** `reorder()` uses `pool.begin()` and wraps all position updates in a transaction for atomicity.
- **Test helper:** Created local `seed_task_attempt()` in test module to insert minimal parent rows (projects, tasks, task_attempts) for FK constraint satisfaction.
- **Existing test migration:** All 7 in-memory tests updated to:
  - Call `create_test_pool()` from `db::test_utils`
  - Seed task_attempt via helper
  - Create store with `MessageQueueStore::new(pool.clone())`
  - Keep all assertions unchanged
- **Persistence test:** Added `test_queue_persists_across_store_recreation()` matching spec exactly:
  - Seeds task_attempt
  - Adds 2 messages to store1
  - Drops store1, creates store2 over same pool
  - Verifies list() returns 2 messages with correct content and position
  - Verifies pop_next() reindexes remaining message to position 0
- **Library build:** `cargo build --lib -p local-deployment` exits cleanly (Finished dev profile).
- **Removed Default impl:** `impl Default for MessageQueueStore` deleted (cannot default SqlitePool); constructor call at container.rs:127 updated to pass pool parameter explicitly.
- **Commit:** `e7d96cad020e49c46a2b716e9fc1dc55e074bb4d`
- **Verification stop (2026-06-26):** Re-ran the task-file command exactly:
  `DATABASE_URL=sqlite:///home/david/Code/vk-swarm/dev_assets/db.sqlite cargo check -p local-deployment`.
  It failed before task tests because Cargo checked the `remote` dependency and SQLx expanded remote
  Postgres queries against the SQLite dev DB. First failure:
  `crates/remote/src/db/auth.rs:50:9: error returned from database: (code: 1) no such table: auth_sessions`,
  followed by the known `E0282 type annotations needed` cascade in `crates/remote`. Worktree status was
  clean before verification; no files outside task 102's declared files were modified. Outcome: stopped.

### Task 103 — Add resume-intent column migration on execution_processes
- **Timestamp selection:** Verified `20260201000100` sorts AFTER task 101's `20260201000000_add_queued_messages.sql`. No collisions detected.
- **Migration file existence:** `ls crates/db/migrations/20260201000100_add_resume_state_to_execution_processes.sql` ✓
- **No prior column:** `grep -rn "resume_state" crates/db/migrations/` (no output) — column does not exist in any migration.
- **Schema validation:** Applied cleanly via `sqlx migrate run --source crates/db/migrations`. Output: `Applied 20260201000100/migrate add resume state to execution processes (7.15378ms)`.
- **DB schema inspection:** `sqlite3 dev_assets/db.sqlite ".schema execution_processes"` confirms:
  - `resume_state TEXT` column present
  - `idx_execution_processes_resume_state` index created with `WHERE status = 'running'` condition ✓
- **No Rust struct changes:** `git diff --stat` shows only the migration file added (no ExecutionProcess struct modification) ✓
- **DB test pool:** `cargo test -p db --lib` exit status: 0 (180 tests passed, 0 failed).
- **Type check:** `cargo check -p db` completed successfully.
- **Commit:** `cb0bfaf092872fce3c6189c34a04a3568c66dc12`

### Task 104 — Add assembling view + read accessor
- **Migration timestamp:** `20260201000200` sorts after task 103's `20260201000100`. No collisions detected.
- **Migration application:** `sqlx migrate run --source crates/db/migrations --database-url "sqlite:dev_assets/db.sqlite"` completed successfully. Output: `Applied 20260201000200/migrate add workstream state view (6.856464ms)`.
- **View schema validation:** `sqlite3 dev_assets/db.sqlite "SELECT sql FROM sqlite_master WHERE type='view' AND name='v_workstream_state';"` confirms LEFT JOIN to executor_sessions (nullable session_id) ✓
- **Sibling alignment:** Matched `executor_session.rs` style (17 fields):
  - Uses `query_as!` with type aliases `as "id!: Uuid"` for BLOB→Uuid conversion
  - Imports: `FromRow`, `SqlitePool`, `Uuid`, `Serialize`/`Deserialize`, `TS` (no TS export on view model)
  - Module doc header: `//!` with description of the view's role
  - Test module: `#[cfg(test)]` with test harness seeding parent rows
- **Schema columns verified:** All 13 projected columns confirmed to exist in underlying tables (task_attempts: container_ref, branch, target_branch; execution_processes: run_reason, status, resume_state, pid, before_head_commit, after_head_commit, created_at; executor_sessions: session_id via LEFT JOIN).
- **Struct field types:** Decided on plain `String`/`Option<String>`/`Option<i64>` (not typed enums) for the view model, matching scalar-query pattern stated in decompose decisions (resume-intent column accessed via dedicated scalar queries, not struct-mapped). This keeps the view a projection surface without enum baggage.
- **UUID handling:** ExecutorSession uses `query_as!` with `as "id!: Uuid"` type casts; WorkstreamState follows same pattern for `execution_process_id` and `task_attempt_id` fields (BLOB→Uuid automatic via macro).
- **Test helper:** `seed_running_attempt_with_session()` inserts full triple (project → task → task_attempt → execution_process → executor_session) to test assembling. Seeding parameters:
  - `executor_action` field (NOT NULL, required) set to `"{}"` (empty JSON object, matches factory defaults)
  - `run_reason` set to `"codingagent"` (matches enum CheckConstraint on execution_processes)
  - Status set to `"running"` (target state for resumable query)
- **Test execution:** `cargo test -p db --lib workstream_state 2>&1` passed: `test models::workstream_state::tests::test_workstream_state_assembles_the_triple ... ok` ✓
- **Compilation:** `cargo check -p db 2>&1` completed without errors (no sqlx prepare invoked) ✓
- **Module registration:** Added `pub mod workstream_state;` to `crates/db/src/models/mod.rs` after `webhook` in alphabetical order.
- **Commit:** (pending)
