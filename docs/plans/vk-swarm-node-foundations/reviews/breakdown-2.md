# Breakdown review — round 2 (re-review of R1–R12 dispositions)

Re-verified every round-1 finding against the real tree. **11 of 12 RESOLVED; one (R1/305) introduced a
new control-flow defect in its fix.** VERDICT: REVISE (single mandatory change).

## Per-finding verdicts

| id | verdict | cited reason |
|----|---------|--------------|
| R1 | **NOT RESOLVED** | 305 closes the boot-drain gap correctly in concept, but its `main.rs` wiring instruction reintroduces a double-writer race — see detail below. `has_running_processes_for_attempt` (`services/container.rs:144`), `try_consume_queued_message` def (`local-deployment/src/container.rs:1179`, fired `:738`), boot seam (`server/src/main.rs:133`) all confirmed real. |
| R2 | RESOLVED | `ProcessInspector` trait + 4 methods exist (`process_inspector/mod.rs:81,89,97,106,109`); pid type `u32` (`:35`); `MockProcessInspector::new()`+`add_process`/`remove_process` (`mock.rs:30,38,45`). 302 invents no method. Mock `kill_process` removes the pid (`mock.rs:125`) so `process_exists` (`:135`) →false after kill — the `Fenced` test is writable as-is. `RawProcessInfo.working_directory: Option<String>` supports the cwd fingerprint. |
| R3 | RESOLVED | `CodingAgent` is `#[enum_dispatch]` bare-variant (`executors/mod.rs:101`); `BaseCodingAgent` strum-derived (`:92`). `default_profiles.json` exists; 201 lists it (not `mcp_config.rs`/`profile.rs`). Deserialization path confirmed: outer key `QA_MOCK`→`BaseCodingAgent::QaMock` (EnumString SCREAMING_SNAKE), inner `{"QA_MOCK":{…}}`→`CodingAgent::QaMock` via `ExecutorConfig.configurations: HashMap<String, CodingAgent>` (`profile.rs:117`) + `get_variant` (`:122`). **JSON alone is insufficient without the enum variant** — but 201's Change adds BOTH (mod.rs variant + JSON key), so `get_coding_agent` returns `Some(CodingAgent::QaMock(_))`. 201 also correctly flags the `capabilities()` exhaustive-match ripple (`mod.rs:175-180`). |
| R4 | RESOLVED | `/api/tasks/{}/available-nodes` has live MCP caller `list_nodes` (`mcp/task_server.rs:1390`); frontend hooks `useAvailableNodes`/`useRemoteConnectionStatus` exist (`AttemptHeaderActions.tsx`, `CreateAttemptDialog.tsx`). 403 keeps these (route is under `tasks/*`, never in delete list) and no longer deletes `resolve_remote_project_id`. |
| R5 | RESOLVED | 403 gate is `cargo test -p server --no-run` (line 108) — compiles `#[cfg(test)]`. 403 keeps the 4 colocated `streams.rs` tests + `resolve_remote_project_id`. Pure-proxy anchors exact: `mod.rs` decls L27/L32-34, merges L65-68; merged-projects route `projects/mod.rs:148`, handler re-exports `handlers/mod.rs:14,29`. |
| R6 | RESOLVED | `execution_processes.hive_synced_at` exists (`migrations/20251229112415_add_hive_sync_tracking.sql:11`). `sync_status` handler has `pool = &deployment.db().pool` (`database.rs:302`); `SyncStatusResponse` struct anchor + `node_id` (`:110`) match 405's insertion point. 405 dropped `depends_on: 403` (frontmatter `depends_on: []`) → **R11 also resolved**. |
| R7 | RESOLVED | 304 (`:112`), 305 (`:77`), 405 (`:165`) all carry `cargo sqlx prepare --workspace` in the gate. |
| R8 | RESOLVED | 102 flags the `usize` trap; `QueuedMessage.position: usize` confirmed at `message_queue.rs:19`; guidance is untyped `query!` + `position as usize`. |
| R9 | RESOLVED | plan.md SC2→{101,102,305} (`:124`); 304 deps `301 302 303 104` (`:76`); no phantom 305-as-SC1-fallback (fallback explicitly folded into 304, `:79`). SC1→{301,302,303,304}. |
| R10 | RESOLVED | 301 counts variants BY LINE within `enum CodingAgent` (`301-…:54-56`), not by `(`. |
| R11 | RESOLVED | see R6 — 405 `depends_on: []`. |
| R12 | RESOLVED | 401 `conflicts_with: []`; 402 `conflicts_with: []` with `depends_on: ["401"]` kept. |

Plan-lint: **PASS** (only advisory sibling warnings, each addressed by the task's `## Sibling alignment`).

## The blocking defect (R1 / task 305) — double-writer race from detached-spawn ordering

**Source of the bug.** 305's Change step (`305-…:51-53`) says: call `drain_queued_messages_on_boot()`
"**AFTER** `cleanup_orphan_executions()` … **like the existing recovery call**." The existing recovery
call (`server/src/main.rs:130`) is a **detached `tokio::spawn`** — it does not block startup. "After it
in source order, like it" yields a *second concurrent detached spawn*, not completion-ordering.

**The race (concrete).** Crashed resumable attempt X with a persisted queued message:
- at drain time `has_running_processes_for_attempt(X) = false` (it crashed), and
- `resume_state(X) = NULL` because the cleanup spawn has not yet reached X.
Both of 305's skip predicates are false → the drain **starts a queued message while 304 concurrently
resumes X** → two writers in the same worktree (`container_ref`) — the exact ADR-0001 hazard 305 exists
to avoid. The `resume_state IN ('pending','resumed')` guard is *written by* cleanup, so it can only
protect the drain once cleanup has run; it presupposes the very ordering it is meant to replace.
(Partial mitigation exists — `try_consume_queued_message` bails if any process is Running,
`local-deployment/src/container.rs:1191-1199` — but 304's resume may not have spawned its process yet at
the instant the drain peeks, so the window is open.)

**Mandatory fix.** 305 must require **completion-ordering, not source-ordering**: the drain runs only
*after `cleanup_orphan_executions().await` has completed*. Concretely — chain it inside the SAME spawn
(`cleanup_orphan_executions().await; drain_queued_messages_on_boot().await;`) or `.await` cleanup before
calling the drain. Update the Change instruction (drop "like the existing recovery call", which mandates
a detached spawn) and tighten STOP-trigger #3 / Done-when to assert *completion* ordering. Everything
else in 305 (the dual skip predicate, the `try_consume`/`has_running_processes_for_attempt` reuse, the
102/304 deps, the schema-materialize step) is sound and stays.

## New defects introduced by the other rewrites
None. 201/302/304/403/405/102 and the plan/ledger edits introduced no new anchor, control-flow, or
interface errors (verified above).

## VERDICT: REVISE
Single required change: **task 305 must order the boot-drain after `cleanup_orphan_executions()`
*completes* (await-chained), not merely after it in source order as a second detached `tokio::spawn`** —
otherwise a crashed resumable attempt with a queued message races into a double writer.

---

## Resolution of the round-2 REVISE finding (R1 / task 305 ordering)

Applied the reviewer's **exact prescribed fix** to `phase-3/305-boot-drain-queued-messages.md`:
- Change step B now await-chains the drain INSIDE the existing `tokio::spawn` at `main.rs:130`
  (`cleanup_orphan_executions().await; … drain_queued_messages_on_boot().await;`) — NOT a sibling spawn.
- Dropped the misleading "like the existing recovery call" phrasing (that call is a detached spawn —
  verified `main.rs:130`).
- STOP-trigger #3 rewritten to mandate `.await`-completion-ordering and STOP if it can't be guaranteed.
- Done-when carries a completion-ordering assertion (same-spawn, drain second) to record in the ledger.

The double-writer race is closed: the drain cannot run until recovery (304) has finished classifying/
resuming, so the `has_running_processes` + `resume_state` skip guards are valid when the drain reads them.
No frontmatter/dep/files change → plan-lint still PASS. This was the reviewer's own specified fix applied
verbatim; no new surface introduced.

**GATE CLEARED:** 12/12 findings resolved (round 1) + the single round-2 ordering regression fixed.
Spine verified sound by all three round-1 challengers and re-confirmed in round 2.
**VERDICT: APPROVE.**
