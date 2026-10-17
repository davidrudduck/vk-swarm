# Proposal: next workstream for the vk-swarm refactor (draft for panel review)

Date: 2026-09-03. Repo: davidrudduck/vk-swarm, `main` @ `492e8e94`. Reviewed from the
`cosmic-aardvark` worktree at
`/home/david/.local/share/opencode/worktree/864023a7bea1094222edb02741f5b7e3b07c3f4d/cosmic-aardvark`.

## 1. Where the program actually is

Two planning documents govern the refactor:

- `docs/superpowers/specs/2026-06-25-vk-swarm-refactor.md` — the 8-phase umbrella (P1 analysis, P2
  foundations, P3 task breakdown, P4 event bus, P5 conflict/priority, P6 management agent, P7
  MCP/ACP, P8 WednesdayAI adapter).
- `docs/superpowers/specs/2026-08-21-local-node-first-use-roadmap-design.md` — the 2026-08-21
  re-sequencing (D1: "local usable loop first"). Milestones M0..M7; **M0–M3 gate the first
  release**. It explicitly pauses P5–P7 behind local usability (M7).

State on `main` today:

| Item | State | Evidence |
|---|---|---|
| P1 analysis | done | `docs/specs/2026-06-25-vk-swarm-phase1-analysis.md` |
| P2a/P2b/P2c (node foundations, hive redesign, hive UI) | shipped | PRs #447–#451, #464, #466, #467 |
| P3 task breakdown | shipped | PR #475 (README `status: shipped`) |
| P4 event bus | shipped | PR #477 (README `status: shipped`) |
| M1 `local-node-browser-oauth` | shipped + live-verified 2026-08-26 | PR #478; README "Acceptance evidence (TS7)" |
| `wal-unlink-durability` (unplanned, from live verification) | shipped | PRs #479, #480; `dev-docs/releases/2026-08-31-wal-unlink-durability.md` |
| M2 `local-execution-lifecycle-correctness` | **does not exist** (no PRD, no README) | `ls dev-docs/workstreams/` |
| M3 `opencode-attached-server` | does not exist | same |
| M4/M5/M6 children | do not exist | same |
| P5–P8 | not started, paused by roadmap D1 | umbrella README |

Other facts gathered:

- GitHub: **0 open issues, 0 open PRs** (`gh issue list` / `gh pr list` on davidrudduck/vk-swarm).
- `/wai:next` (`wai-next.sh`) lists exactly **one** ready task: `hive-oauth-sw-bypass` 301. That
  workstream is `status: shipped`, its other 5 tasks are `passed`, and the ledger records the
  round-2 live deploy verification PASS (2026-08-06, operator statement "sign in now works"). Task
  301 is stale-ready, not real work. With it flipped, **`/wai:next` is empty** — nothing is
  decomposed, so `/wai:execute` has nothing to consume until a PRD → spec → precheck → decompose
  cycle runs.
- `wai-followups.sh --open` is empty. `wai-status.sh --check` passes (MASTER table matches
  frontmatter), but several READMEs carry stale `status:` values (see §5).
- Backlog: 48 findings, 8 high (all `fixed`/`triaged`/`promoted` except **F-2026-08-09-01**
  claude hooks hang, `open`, high). 15 findings are `open`; all low/medium except that one.

## 2. The critical path is M2

The roadmap's dependency graph is `M0 → (M1 ∥ M2) → M3 → FIRST RELEASE`. M1 shipped. M2 is the
only unstarted item on the critical path, and M3 is defined as "decomposed against [M2's] settled
execution-lifecycle contract" (roadmap §5). Therefore **the next workstream is M2:
`local-execution-lifecycle-correctness`**, created via `/wai:prd-new` (the slug is already named
in roadmap §5 item 2).

The roadmap's M2 claims were re-verified against `main` @ `492e8e94` today; every anchor still
holds:

| Roadmap claim (§Design / §7) | Verified on main |
|---|---|
| `core.rs` reports Create & Start success after executor start failure | `crates/server/src/routes/tasks/handlers/core.rs:486-511` — `start_attempt(...).is_ok()` is folded into `has_in_progress_attempt`; handler returns `Ok(ApiResponse::success(...))` regardless |
| `container.rs` suppresses stop failures | `crates/services/src/services/container.rs:714-748` — `try_stop` logs at `debug!` via `unwrap_or_else` and on timeout "proceeding anyway" |
| `msg_store.rs` broadcasts before recording history; subscribes after snapshot | `crates/utils/src/msg_store.rs:55-71` (`sender.send` at :56, history push at :67) and `:125-135` (`get_history()` then `get_receiver()`) — a message pushed between the two calls is neither in the snapshot nor on the receiver |
| local-deployment migrates historical logs before normalizer + batcher complete | `crates/local-deployment/src/container.rs:657` (`migrate_execution_logs`) runs before `:836-837` (`log_batcher.finish`) and `:844-858` (`push_finished` + await normalization handle) |
| No `starting/stopping/stop_failed/interrupted` states exist | `crates/db/src/models/execution_process/mod.rs:56-61` — enum is `Running, Completed, Failed, Killed` only |
| No execution-generation identity | `crates/db/migrations/` has `server_instance_id` (20260118) but no generation column |

One **design tension the M2 spec must resolve** (not in the roadmap text): shipped ADR-0001
"fence-then-resume" crash recovery (`crates/services/src/services/container.rs:322-469`,
`cleanup_orphan_executions`) already **automatically resumes** a fenced process when a
`session_id` exists. Roadmap D8 / §7.6 says pre-M6 restart must "launch no automatic replacement"
and leave the run `interrupted` with explicit retry. M2 cannot satisfy §7.6 without either gating
the existing resume path or re-scoping D8. This is the first `decisions[]` fork for `/wai:spec`.

## 3. What to bundle INTO M2 (same files, same contract)

| Candidate | Why it belongs in M2 | Files |
|---|---|---|
| **`normalize-fast-execution-lost-logs-flake`** (README `status: ready`; F-2026-08-04-02) | It is not a ride-along, it IS roadmap §7.5 "make MsgStore history-plus-live subscription atomic". The README proves a real lost-log race (handle completes in 0.49–0.52s with zero patches). The mechanism matches `msg_store.rs:56/67/128`. M2 acceptance criterion 6 ("history/live boundary stress loses and duplicates zero messages") is this bug's regression guard. | `crates/utils/src/msg_store.rs`, `crates/services/tests/normalize_sync_test.rs`, normalizer subscription in `crates/services/src/services/container.rs` |
| **`services-normalize-flaky-test`** (README `status: active`) | Same test, contradictory tracker: it still records hypothesis H1 ("test harness race, check the timeout") which the lost-logs README disproved on 2026-08-14. Roadmap M0 says "consolidate contradictory normalizer trackers under the confirmed product defect". Close it as superseded, pointing at M2; do not carry two owners for one bug. | docs only |
| **`local-deployment-test-orphan-cleanup-safety`** (README `status: ready`; F-2026-08-19-01) + **F-2026-08-19-02** (`NodeRunnerConfig::from_env` reads ambient hive env in tests) | M2's deterministic suite (roadmap TS2) must construct `LocalContainerService`/`LocalDeployment` many times in `crates/local-deployment` and `crates/services` tests. Today those constructors can sweep real clean worktrees (`container.rs:320`) and connect to the LIVE hive (`node_runner.rs:62`). This is a **phase-0 prerequisite** for M2's tests, and both fixes are constructor-level opt-outs in the same crates M2 edits. Bundle as M2 phase 0 ("test seam hardening"). | `crates/local-deployment/src/{container.rs,lib.rs}`, `crates/services/src/services/node_runner.rs` |
| **F-2026-09-02-01** (serve `Err` skips `perform_cleanup_actions`) | Low, one-file, in the shutdown path M2 already changes for "acknowledged batcher shutdown" (§7.5). Ride-along only if the shutdown ordering task touches `main.rs`; otherwise leave. | `crates/server/src/main.rs:273-275` |

Estimated size: roadmap M2 alone is 6 sub-areas / 13 acceptance criteria; with phase 0 and the
lost-log race (already in-spec) this is roughly 20–28 surgical tasks — comparable to
`vk-swarm-node-ui-localize` (19) and `vk-swarm-design-system` (24), below `hive-redesign` (37).

## 4. What NOT to bundle (and where it goes)

| Item | Decision | Why |
|---|---|---|
| M3 `opencode-attached-server` | separate workstream, after M2 spec settles | Roadmap D4/SC8: M3 consumes M2's contract; one spec per milestone. Note for M3: host `opencode --version` = **1.18.26**, roadmap certifies **1.18.19** (§8.2). M3's spec must re-pin or re-certify. |
| `claude-executor-hooks-hang` (draft; F-2026-08-09-01, **high**) | separate small `bugfix` workstream, may run in a **parallel worktree** | Roadmap assigns hooks policy to M5 §11.1, but the fix it prescribes ("do not merge global hooks", `claude.rs:172-185` `get_hooks_merged`) is a self-contained executor change with a known A/B repro. It blocks every server-spawned Claude run on nodes with hooks today. Different files from M2 (`crates/executors/src/executors/claude.rs`). Per the 2026-08-05 handoff rule: parallel worktrees, never one spec for unrelated bugs. |
| `mcp-server-dev-loop-rebuild` (draft; F-2026-08-09-02) | separate tooling workstream, parallel-able | `package.json`/dev scripts only; no overlap with M2. |
| `task-breakdown-followups` (draft, 6 items) | leave; M7 tail | P3 hygiene. Item 6 (stuck Draft after restart) touches restart reconciliation, but it is breakdown-specific and can consume M2's reconciliation hook afterwards rather than expand M2 now. |
| `browser-auth-terminal-row-retention` (ready) | leave | Product/storage decision, M1 follow-up, not on critical path. |
| `remote-services-doctest-revival`, `terminal-session-pty-tests` (active, 0 progress) | leave; downgrade to `draft` per MASTER open question | Test debt, not release-gating. |
| F-2026-08-28-02 (hive tailwind `theme.extend` empty), F-2026-07-29-03 | leave; M7 hive usability | Hive UI. |
| F-2026-08-31-01/-02 (WAL refusal error code; central write gate) | leave | WAL domain, low; not lifecycle. |
| F-2026-08-31-03 (Node 26 vitest) | leave; trivial pin | Could be a 1-line `engines` change in any PR. |
| P5–P8 | paused by roadmap D1 until M7 | — |

## 5. Tracker hygiene owed by roadmap M0 (do before/with `/wai:prd-new`)

M0 acceptance criterion 1 ("`/wai:next` lists only genuinely unfinished ready work") and the
"update the `vk-swarm-refactor` tracker to show P3 and P4 shipped" instruction are unmet:

1. `hive-oauth-sw-bypass` task 301 → `passed` (ledger round-2 deploy PASS 2026-08-06; SC4 pinned by vitest, not live — record that in the task file).
2. `node-task-delete-dangling-shared-id` README `active` → `shipped` (PR #468; ledger "Deploy verification (2026-08-28) VERDICT: PASS").
3. `sqlite-busy-snapshot-calibration-stability` README `active` → `shipped` (its only open box was "branch merged"; the control test is on main at `crates/db/src/models/execution_process/queries.rs:1369` via PR #478).
4. `frontend-test-debt-2026-08` README `active` → `shipped` (README "Outcome — COMPLETE (2026-08-01)"; F-2026-07-31-01/02/03 `fixed`).
5. `services-normalize-flaky-test` → closed/superseded into M2; re-point F-2026-08-04-02 `workstream` at M2.
6. `vk-swarm-refactor` README: P3/P4 rows → shipped; add the 2026-08-21 roadmap as the governing sequencing doc; mark P5–P8 "paused behind local-node-first-use M7"; refresh the "Recommended order" (items 1–2 are done).
7. `local-node-first-use` README: add the M0–M7 table with child slugs and state (M1 shipped, M2 next).
8. MASTER.md "Open questions": the P3–P8 mapping question is answered; the doctest/PTY question should be actioned (downgrade to draft).
9. `dev-docs/2026-08-05-next-session-handoff.md` is fully consumed (all five WS-A..E shipped); mark superseded or delete.

## 6. Recommended sequence

1. Tracker hygiene PR (§5) — small, docs-only, restores `/wai:next` truth.
2. `/wai:prd-new local-execution-lifecycle-correctness` seeded from roadmap §7 + §3 of this doc
   (`change_kind: behaviour`; success criteria = roadmap M2 AC 1–13 + lost-log race regression +
   test-seam safety).
3. `/wai:spec` — first decision fork: ADR-0001 auto-resume vs roadmap D8 no-auto-continuation.
4. `/wai:precheck` → `/wai:decompose` → `/wai:execute local-execution-lifecycle-correctness`
   (implementer subagent-glm-flash, escalate subagent-gpt-terra → subagent-gpt-sol).
5. In parallel worktrees if capacity: `claude-executor-hooks-hang` (bugfix) and
   `mcp-server-dev-loop-rebuild` (tooling).
6. After M2 settles: `/wai:prd-new opencode-attached-server` (M3), re-certifying OpenCode
   1.18.26.

## Questions for the panel

- Q1. Is M2 the right next unit, or is there a smaller release-gating item I have missed?
- Q2. Is folding the test-seam safety items (F-2026-08-19-01/-02) into M2 phase 0 correct, or
  should they precede M2 as their own micro-workstream?
- Q3. Should `claude-executor-hooks-hang` stay out of M2 (as proposed) even though it is the only
  open high-severity finding?
- Q4. Is M2 as scoped (6 sub-areas + phase 0) overloaded? If so, where is the clean split line
  that does not create two lifecycle identity models (roadmap D5/SC7 forbid that)?
- Q5. Any anchor above that is wrong on `main` @ `492e8e94`?
