# 2026-09-03 — Next workstream selection for the vk-swarm refactor

Reviewed `main` @ `492e8e94` from the `cosmic-aardvark` worktree. Inputs: `dev-docs/MASTER.md`,
`dev-docs/BACKLOG.md`, every `dev-docs/workstreams/*/README.md`, the two governing specs
(`docs/superpowers/specs/2026-06-25-vk-swarm-refactor.md`,
`docs/superpowers/specs/2026-08-21-local-node-first-use-roadmap-design.md`), `wai-next.sh`,
`wai-followups.sh --open`, `wai-status.sh --check`, and GitHub (`gh issue list`, `gh pr list`).
An adversarial panel reviewed the draft (`dev-docs/2026-09-03-next-workstream-selection-reviews/`);
this document is the post-review revision.

## 1. Decision

**Next workstream: M2 `local-execution-lifecycle-correctness`** (roadmap §7), preceded by two small
units that unblock it. Bundle into M2 only what is literally in its contract. Everything else stays
out.

Execution order:

| # | Unit | Kind | Why it is in this position |
|---|---|---|---|
| 1 | M0 closure: tracker + backlog truth (§5 below) | docs PR | Roadmap §9.1 lists M0 as a release-gate item; `/wai:next` must be truthful before decomposition. |
| 2 | `local-deployment-test-orphan-cleanup-safety` (already `ready`), expanded to own F-2026-08-19-02 | `bugfix` micro-workstream | M2's deterministic suite (roadmap TS2) constructs `LocalDeployment`/`LocalContainerService` repeatedly; today those constructors can sweep real clean worktrees and dial the live hive. Its own reachability gate ("test/scratch construction has no operator-owned side effects") is coherent and distinct from M2's. Panel vote 4:1 for a predecessor over "M2 phase 0". |
| 3 | **M2 `local-execution-lifecycle-correctness`** | `behaviour` workstream | Earliest unmet dependency on the first-release path `M0 → (M1 ∥ M2) → M3 → release`. M1 shipped (PR #478, live-verified 2026-08-26). M3 is defined as consuming M2's contract. |
| ∥ | `claude-executor-hooks-hang` (F-2026-08-09-01, high) | `bugfix`, parallel worktree | Not release-gating (first release excludes Claude, roadmap §Out of scope + §11.1), but it blocks every server-spawned Claude run on nodes with hooks and is a self-contained executor fix (`crates/executors/src/executors/claude.rs:172-185`). Disjoint files from M2. |
| ∥ | `mcp-server-dev-loop-rebuild` (F-2026-08-09-02) | tooling, parallel worktree | `package.json` dev scripts only. |
| 4 | M3 `opencode-attached-server` | after M2's spec settles | Host OpenCode is **1.18.26**; roadmap §8.2 certifies **1.18.19**. M3's spec must re-pin. |
| 5 | First-release certification (roadmap §9.5–9.7) | umbrella step with an owner | Not implied by M3's normal close: full gate on one commit, 16-step LAN run, integrated M1–M3 adversarial review, reruns. |

## 2. Program state (verified)

| Item | State | Evidence |
|---|---|---|
| P1 analysis | done | `docs/specs/2026-06-25-vk-swarm-phase1-analysis.md` |
| P2a/b/c | shipped | PRs #447–#451, #464, #466, #467 |
| P3 task breakdown | shipped | PR #475; README `status: shipped` |
| P4 event bus | shipped | PR #477; README `status: shipped` |
| M1 `local-node-browser-oauth` | shipped, live-verified 2026-08-26 | PR #478; README "Acceptance evidence (TS7)" |
| `wal-unlink-durability` (unplanned) | shipped | PRs #479/#480; `dev-docs/releases/2026-08-31-wal-unlink-durability.md` |
| M2, M3, M4–M6 children | do not exist | `ls dev-docs/workstreams/` |
| P5–P8 | paused behind M7 | roadmap D1 |
| GitHub | 0 open issues, 0 open PRs | `gh issue list` / `gh pr list` |
| `/wai:next` | 1 ready task: `hive-oauth-sw-bypass` 301 | see §5 item 1 |
| `wai-followups.sh --open` | empty | — |
| Backlog | 48 findings; 15 open; 1 open high (F-2026-08-09-01) | `dev-docs/BACKLOG.md` |

Roadmap M2 anchors re-verified on `main` (all hold):

| Roadmap claim | Code |
|---|---|
| Create & Start reports success after start failure | `crates/server/src/routes/tasks/handlers/core.rs:486-511` (`start_attempt(...).is_ok()` folded into `has_in_progress_attempt`; unconditional `Ok`) |
| Stop failures suppressed | `crates/services/src/services/container.rs:714-748` (`debug!` on error; timeout "proceeding anyway") |
| MsgStore broadcasts before recording; subscribes after snapshot | `crates/utils/src/msg_store.rs:56` vs `:67`; `:128` (`get_history()` then `get_receiver()`) |
| Historical logs migrated before batcher/normalizer complete | `crates/local-deployment/src/container.rs:657` before `:836-858` |
| Only `Running/Completed/Failed/Killed` states | `crates/db/src/models/execution_process/mod.rs:56-61` |
| No execution-generation identity | `crates/db/migrations/` has `server_instance_id` (20260118) and `resume_state` (20260201000100), no generation |

## 3. M2 scope — what is in, what the panel added

### 3.1 In-contract bundle (these are M2, not ride-alongs)

| Item | Why | Files |
|---|---|---|
| `normalize-fast-execution-lost-logs-flake` (ready; F-2026-08-04-02) | Is roadmap §7.5 bullet 1 and AC6. Confirmed product race: handle completes in 0.49–0.52s with zero patches (README). Fixing it early also makes `cargo test -p services` deterministic for every later M2 gate (currently fails ~1 in 6–8 full-crate runs). | `crates/utils/src/msg_store.rs`, `crates/services/tests/normalize_sync_test.rs` |
| `services-normalize-flaky-test` (active) | Contradictory tracker for the same test (still records the disproved H1). Close as superseded; re-point F-2026-08-04-02 at M2. | docs |
| F-2026-09-02-01 (serve `Err` skips `perform_cleanup_actions`) | M2 owns "acknowledged batcher shutdown" (§7.5) and the startup barrier (§7.6), both in `crates/server/src/main.rs`; the skipped cleanup is where the batcher's ack'd shutdown runs (`main.rs:273-280`, `:339-345`). Make it an explicit task, not optional. | `crates/server/src/main.rs` |

### 3.2 Design forks the M2 spec must settle (found by the panel; not in the roadmap text)

1. **ADR-0001 fence-then-resume vs roadmap D8 "no automatic continuation before M6."**
   `cleanup_orphan_executions` (`crates/services/src/services/container.rs:326-553`) already
   auto-resumes when a `session_id` exists (`:468-530`, `resume_execution` at `:500-508`,
   `set_resume_state("resumed")` at `:512`). ADR-0001 is `accepted`. Resolution is an ADR
   (supersede-or-defer, not a `decisions[]` row): gate the resume branch behind an M6 capability,
   keep the fence half live, mark `interrupted`, expose explicit retry. Two couplings: the resume
   path falls back to `ClaudeCode` when `executor_action` is unparseable (`:481-488`), which can
   route a non-Claude run into the hooks hang; and `main.rs:126-146` sequences recovery → queued
   drain in one detached spawn, so the startup barrier must own that ordering and the shipped
   resume tests (`container.rs:2209`, `:2292`) will invert.
2. **P4 event semantics.** `ExecutionProcess::create` inserts `status = running` and appends
   `AttemptStarted` in the same transaction (`crates/db/src/models/execution_process/queries.rs:443-507`).
   §7.1 forbids `running` before ownership/PID are durable, so `AttemptStarted` cannot keep its
   current meaning at creation. `NodeEvent` is a shipped TS-exported surface
   (`crates/db/src/models/event.rs:20-51`, `shared/types.ts`), fixed by the P4 spec. Decide:
   new `starting` event + move `AttemptStarted` to the durable `running` transition, or version
   the event. Update P4 emission/conformance tests.
3. **Generation identity must evolve, not duplicate, existing ownership fields.**
   `server_instance_id`, `resume_state`, `fence_attempt_count`, `executor_sessions.session_id`,
   and the node outbox `fencing_token` (`crates/db/migrations/20260201000400_add_node_outbox.sql:19`;
   hive `ADR-0009`) already exist. D5/SC7 forbid a second ownership model; name and relate them
   explicitly before the migration lands.
4. **Output durability has five mechanisms, not one.** Beyond the snapshot/subscribe gap:
   the live leg drops `Lagged` silently (`msg_store.rs:131-148`, `res.ok()`); `LogBatcher::finish`
   returns `()` with no ack (`crates/services/src/services/log_batcher.rs:84-99`); terminal
   completion + migration run before batcher finish and normalizer await
   (`local-deployment/container.rs:620-673` vs `:835-864`); incremental migration skips an
   execution when *any* entry exists (`crates/services/src/services/log_migration.rs:192-212`), the
   opposite of resumable; migration completion is guessed by three stable 50 ms polls
   (`log_migration.rs:307-384`). AC6–AC9 need each enumerated with failure injection. Note the
   `stop_execution` path in the same file (~`:2020-2145`) already has the correct
   flush → finish → migrate order; use it as the reference.
5. **Startup barrier belongs at the shared service boundary, not the router.** Hive assignments
   (`assignment_handler.rs:115-154`), queued drafts (`drafts.rs:211-279`), breakdown
   (`container.rs:1378-1495`), setup/review/follow-up all reach `start_execution`. M2 must define
   how every run reason participates in generation ownership and the barrier; add breakdown and
   hive-assignment compatibility tests. Breakdown-specific Draft repair stays in
   `task-breakdown-followups` behind a named reconciliation hook.
6. **Frontend + hive projection surface.** New statuses ripple into the generated
   `ExecutionProcessStatus` TS enum (`frontend/src/lib/types.ts`,
   `frontend/src/hooks/useConversationHistory.ts`,
   `frontend/src/components/tasks/TaskDetails/ProcessesTab.tsx`), four-locale i18n, and the hive
   mirror (`crates/remote/src/db/node_execution_processes.rs`). Budget tasks; several sites key
   "is live" off `=== running` and will mis-handle `starting`.
7. **M3–M6 compatibility fixtures.** M2 proves its schema is permanent with fixtures for attached
   OpenCode, direct CLI, Claude, and an M6 claim — no M3 behaviour, just shape.

### 3.3 Size and the overload question

Seat estimates for M2 as scoped: opus 25–32, glm 26–34, gpt-sol 41–53 (the last counts UI, hive
projection, and fault-injection tasks explicitly). Prior workstreams: node-ui-localize 19, M1
browser-oauth 22, design-system 24, hive-redesign 37. **M2 will be the largest workstream so far.**

All five seats agree it stays **one workstream, one spec, one identity model** (D5/SC7; roadmap
§4 "each milestone is a separate WAI child"; §9.1 "no M1–M3 acceptance criterion deferred").
Overload is managed by phase order and per-phase PRs (roadmap §4 permits small PRs per milestone):

1. Phase 1 — §7.5 output durability including the lost-log race. Introduces no identity; lands
   first so every later Stage-1 gate on `crates/services` is deterministic.
2. Phase 2 — §7.1 states + generation/ownership schema + P4 event decision (ADR).
3. Phase 3 — §7.2 Create & Start compensation and callers.
4. Phase 4 — §7.3 cancellation + §7.4 retry.
5. Phase 5 — §7.6 startup barrier + ADR-0001 gating + F-2026-09-02-01.
6. Phase 6 — UI/i18n/hive projection, integration + fault-injection suite, live evidence.

Fallback rule: if `/wai:decompose` yields more than ~40 tasks, split **phase 1 only** into its own
child (`local-execution-output-durability`). That split is SC7-safe because §7.5 introduces no
identity; no other split line is.

## 4. Kept out of M2 (and where each goes)

| Item | Goes to | Reason |
|---|---|---|
| M3 attached OpenCode | own workstream after M2 spec | D4/SC8; consumes M2's contract; re-certify OpenCode 1.18.26 |
| `claude-executor-hooks-hang` | parallel `bugfix` worktree | not release-gating (Claude is M5); disjoint files |
| `mcp-server-dev-loop-rebuild` | parallel tooling worktree | dev scripts only |
| `task-breakdown-followups` items 1–5 | M7 tail | P3 hygiene; item 6 consumes M2's reconciliation hook later |
| `browser-auth-terminal-row-retention` | later; M1 follow-up | product/storage decision |
| `remote-services-doctest-revival`, `terminal-session-pty-tests` | downgrade to `draft` | test debt, zero progress, not gating |
| F-2026-08-28-02 (hive tailwind), F-2026-07-29-03 | M7 hive usability | hive UI |
| F-2026-08-31-01/-02 (WAL error code, central write gate) | backlog | WAL domain, low |
| F-2026-08-31-03 (Node 26 vitest) | any PR, one-line `engines` pin | trivial |
| P5–P8 | paused until M7 | roadmap D1 |

## 5. M0 closure — the hygiene PR, mapped to roadmap M0 acceptance criteria

**AC1 — `/wai:next` lists only genuinely unfinished ready work**

1. `hive-oauth-sw-bypass` task 301 is **6/7 complete, not stale**. Manual verification item 7
   (SC4: abandon the popup, confirm `/api/auth/status` polling ceases after 120 s and the timeout
   error renders) was never exercised live; the ledger says so
   (`decisions-ledger.md:287-302`). Either record that ~3-minute operator check in the ledger and
   flip 301 to `passed`, or amend the task's acceptance with an explicit decision. Do not flip it
   on the strength of the vitest pins alone.
2. `node-task-delete-dangling-shared-id` README `active` → `shipped` (PR #468; ledger "Deploy
   verification (2026-08-28) VERDICT: PASS").
3. `sqlite-busy-snapshot-calibration-stability` README `active` → `shipped` (merged via PR #478;
   control test at `crates/db/src/models/execution_process/queries.rs:1369`).
4. `frontend-test-debt-2026-08` README `active` → `shipped` (README "Outcome — COMPLETE").
5. `services-normalize-flaky-test` → closed as superseded by M2.

**AC3 — every first-release blocker has exactly one owner**

6. BACKLOG reconciliation: F-2026-08-04-01 → `fixed` (commit `e3688d32`, `OAuthDialog.tsx`
   deadline + error branch); F-2026-08-03-01/-02 `triaged` → `fixed`; F-2026-08-05-01 `promoted`
   → `fixed`; F-2026-08-04-02 workstream → M2; F-2026-08-19-01 and -02 workstream →
   `local-deployment-test-orphan-cleanup-safety` (and fix -01's stale `lib.rs:514-531` location:
   the remaining exposure is `new_for_drain_test`, `crates/local-deployment/src/container.rs:168-190`).
7. `local-deployment-test-orphan-cleanup-safety` README: absorb F-2026-08-19-02 into its scope.
8. `dev-docs/2026-08-05-next-session-handoff.md`: mark superseded **with a re-point** (WS-A–D
   shipped; WS-E's F-2026-08-04-02 moved to M2). Do not describe it as fully consumed.

**AC2, AC4, AC5 — umbrella state**

9. `vk-swarm-refactor` README: P3/P4 → shipped; add the 2026-08-21 roadmap as the governing
   sequencing doc; P5–P8 "paused behind local-node-first-use M7"; refresh "Recommended order".
10. `local-node-first-use` README: add the M0–M7 table with child slugs and states, the
    first-release gate, and two durable statements: "no M1–M3 child closes with live/E2E evidence
    outstanding" (AC4) and "each child has one canonical spec; no umbrella-wide decomposition"
    (AC5).
11. MASTER.md "Open questions": mark the P3–P8 mapping answered; action the doctest/PTY downgrade.
12. `staging_pointers` in several READMEs point at `docs/plans/<ws>/…`, which is empty; ledgers
    live at `dev-docs/workstreams/<ws>/plans/<ws>/`. Repoint.

## 6. Commands, in order

```bash
# 1. hygiene PR (docs only) — items in §5

# 2. predecessor micro-workstream (README exists; needs spec → precheck → decompose)
/wai:prd-new local-deployment-test-orphan-cleanup-safety   # change_kind: bugfix
/wai:spec local-deployment-test-orphan-cleanup-safety
/wai:precheck local-deployment-test-orphan-cleanup-safety
/wai:decompose local-deployment-test-orphan-cleanup-safety
/wai:execute local-deployment-test-orphan-cleanup-safety   # implementer subagent-glm-flash → subagent-gpt-terra → subagent-gpt-sol

# 3. M2
/wai:prd-new local-execution-lifecycle-correctness         # change_kind: behaviour; seed from roadmap §7 + §3 of this doc
/wai:spec local-execution-lifecycle-correctness            # forks in §3.2; ADR for ADR-0001 deferral; ADR/decision for P4 event semantics
/wai:precheck local-execution-lifecycle-correctness
/wai:decompose local-execution-lifecycle-correctness       # phases per §3.3; apply the >40-task fallback rule
/wai:execute local-execution-lifecycle-correctness         # same implementer ladder

# ∥ parallel worktrees, only if capacity
/wai:prd-new claude-executor-hooks-hang                    # bugfix
/wai:prd-new mcp-server-dev-loop-rebuild                   # tooling
```

## 7. Panel record

Target: draft proposal (`…-reviews/proposal-draft-v1.md`). Governing intent: the two specs above.
Push gate: N/A (planning doc). Seats requested by the operator: panel = opus, gpt-sol, grok;
challengers = grok, kimi, gpt-sol.

| Seat | Role | Result |
|---|---|---|
| subagent-opus (claude-code/opus) | panel | ran; REVISE; 12 findings (1 blocking) |
| subagent-gpt-sol (gpt-5.6-sol) | panel | ran; REVISE; 6 findings (2 blocking) |
| subagent-grok (xai/grok-4.6) | panel | **failed**: provider spending limit ("run out of credits or need a Grok subscription") |
| subagent-grok | challenger | **failed**: same |
| subagent-kimi (kimi-k3) | challenger | **failed**: provider account suspended, insufficient balance |
| subagent-gpt-sol | challenger | ran; REVISE; 8 findings (5 blocking) |
| subagent-glm (glm-5.3) | replacement challenger (Z.AI, family not otherwise on the roster) | ran; REVISE; 11 findings (0 blocking) |
| subagent-sonnet (sonnet 5) | replacement challenger | ran; REVISE; 5 findings (1 blocking) |

Five of eight seats returned real, cited reports across three families. The grok and kimi
seats cannot be re-run until the operator restores credit on those providers.

Consolidated findings (deduped; all cited; nothing discarded for lack of citation):

| Finding | Seats | Accepted | Resolution |
|---|---|---|---|
| M2 is the right next unit; nothing open outranks it | all 5 | yes | §1 |
| Test-seam fixes as M2 "phase 0" violate one-spec/one-gate | gpt-sol×2, glm, sonnet (opus: keep in M2 but transfer ownership) | yes (4:1) | predecessor micro-workstream, §1 row 2 |
| ADR-0001 auto-resume conflicts with D8; needs an ADR, gate not delete; ClaudeCode fallback; startup sequencing | all 5 | yes | §3.2.1 |
| Handoff doc is not "fully consumed" (WS-E open) | opus, gpt-sol, glm | yes | §5.8 |
| Task 301 has a genuinely unexercised live item (SC4) | gpt-sol panel (opus/glm/sonnet called it stale) | yes — the task file requires it | §5.1 |
| BACKLOG statuses stale (F-2026-08-04-01 fixed; -03-01/-02; -05-01; -19-01 location) | opus, gpt-sol, glm, sonnet | yes | §5.6 |
| P4 `AttemptStarted` at creation collides with `starting` | gpt-sol challenger | yes (verified `queries.rs:443-507`) | §3.2.2 |
| Generation identity vs `fencing_token`/`resume_state`/`server_instance_id` | opus, gpt-sol | yes | §3.2.3 |
| Output durability: Lagged drop, unacked finish, skip-if-any migration, polled completion | gpt-sol, glm | yes | §3.2.4 |
| Startup barrier at service boundary; breakdown + hive assignment callers | gpt-sol | yes | §3.2.5 |
| Frontend enum fan-out + i18n + hive projection unscoped | opus, gpt-sol, glm | yes | §3.2.6 |
| Task estimate 20–28 too low | all 5 (25–32 / 26–34 / 41–53) | yes | §3.3 |
| F-2026-09-02-01 should be mandatory (gpt-sol) vs leave out (glm) | split | include | §3.1 — M2 owns the shutdown ack path |
| Post-M3 release certification step missing | gpt-sol | yes | §1 row 5 |
| Anchor `container.rs:322-469` too short | opus, gpt-sol, glm, sonnet | yes | corrected to `:326-553` / `:468-530` |
| "M2 is the only unstarted critical-path item" overstated | gpt-sol | yes | reworded "earliest unmet dependency" |
| Comparator counts uncited | glm | partly | M1 = 22 added; hive-redesign 37 per MASTER |

### Lessons learned

- A single reviewer read the roadmap and confirmed its anchors; the cross-family pass found what
  the roadmap itself does not mention — that ADR-0001's auto-resume and P4's `AttemptStarted`
  emission are *shipped, tested* behaviours M2 must override, not gaps it fills. Those are the two
  items most likely to have surfaced mid-execution as "discovered scope".
- Three seats called task 301 "stale"; one read the task file's manual-verification list and found
  a real unexercised item. Truthful `/wai:next` means checking the task contract, not the ledger's
  headline verdict.
- Two of the six requested seats were dead on provider billing. Replacing them with unused
  families kept cross-family coverage; the roster should be re-verified before the M2 breakdown
  review.
