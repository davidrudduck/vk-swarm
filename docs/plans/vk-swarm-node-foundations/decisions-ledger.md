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

### Trap 2 — SQLx compile-time queries need the schema materialized
This repo uses `sqlx::query_as!`/`query!` (compile-time-checked against a live schema). The moment a
query references a new table/column/view (tasks 102, 104, and the recovery queries in 303/304), the
build needs that schema present: either the dev DB has the migration **applied**, or the offline
`.sqlx` cache is **regenerated** (`cargo sqlx prepare --workspace`). A migration-add + query-use in the
**same commit** will NOT compile otherwise. Each such task's `## Done when` includes the apply/prepare
step. Migration-only tasks (101, 103) that add no query are safe alone.

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

### Decompose decision — fence fingerprint is a cmdline/worktree heuristic, NOT persisted start-time (302/304)
ADR-0001 names the PID-reuse fingerprint as "process start-time / pgid" (by example). A persisted
start-time would require capturing it at spawn + a new column + a spawn-path edit — work not in this
plan. **Decision:** the fence uses sysinfo (`services` already depends on `sysinfo = "0.37"`, which can
both inspect AND kill a PID) to read the *live* process's command line / exe and confirm it references
the expected worktree (`task_attempts.container_ref`) or executor before killing. A live PID whose
cmdline does NOT match is treated as a reused PID → NOT killed. This defeats PID reuse with no schema
change, within the ADR's intent (the ADR's mechanism was illustrative). If the heuristic proves
insufficient at implementation (e.g. cmdline not introspectable on the target OS), 302 STOPs and
escalates — persisting start-time becomes a spec-scope question, not a silent addition. `nix` (in
`local-deployment`) is NOT needed; the fence lives entirely in `services`.

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
