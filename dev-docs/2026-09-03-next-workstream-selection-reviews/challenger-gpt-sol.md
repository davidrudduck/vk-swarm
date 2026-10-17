# Challenger review: next vk-swarm workstream

Reviewed against `main` at `492e8e94746327e1b39f2531d9f84afb6d8a3d9f` on 2026-09-03. Read-only commands also returned no open issues and no open pull requests for `davidrudduck/vk-swarm`.

## Findings

### 1. [BLOCKING] The product milestone is right, but the proposed first executable unit has an incoherent phase 0

M2 is the correct next **product milestone**. The roadmap orders `M0 -> (M1 || M2) -> M3 -> FIRST RELEASE`, says M3 consumes M2's contract, and names `local-execution-lifecycle-correctness` as the M2 child (`docs/superpowers/specs/2026-08-21-local-node-first-use-roadmap-design.md:167-200`, `:210-224`). M1 is shipped (`dev-docs/workstreams/local-node-browser-oauth/README.md:1-16`), while no M2 child exists.

The constructor-safety findings should not, however, become M2 success criteria merely because M2 needs safe tests. One WAI workstream has one incident-level spec and reachability gate; the repository's explicit rule says unrelated bugs under one spec make that gate meaningless (`dev-docs/2026-08-05-next-session-handoff.md:15-30`). M2's gate asks whether launch, stop, retry, output, and restart behavior is truthful. It cannot simultaneously prove that a scratch deployment did not delete an operator worktree or connect to an ambient live Hive.

These two safety findings do have a coherent **predecessor** boundary: construction of a test/scratch `LocalDeployment` must have zero operator-owned external side effects. The same `from_parts` path constructs `LocalContainerService` (`crates/local-deployment/src/lib.rs:256-265`) and reads ambient Hive configuration (`crates/local-deployment/src/lib.rs:272-279`). The container constructor starts the worktree sweep (`crates/local-deployment/src/container.rs:143-165`, `:514-520`), while `NodeRunnerConfig::from_env` reads `VK_HIVE_URL` and `VK_NODE_API_KEY` (`crates/services/src/services/node_runner.rs:50-65`). The existing tracker already requires dependency-injected test policy and a sentinel-worktree regression (`dev-docs/workstreams/local-deployment-test-orphan-cleanup-safety/README.md:19-53`).

Required revision: run tracker hygiene first, then expand/rename the existing safety tracker to a constructor-side-effect-isolation micro-workstream owning F-2026-08-19-01 and F-2026-08-19-02. Its single reachability test should construct every test/scratch entry point under ambient production-looking Hive variables and an isolated sentinel worktree, proving no outbound connection and no deletion. If one gate cannot exercise both, keep F-2026-08-19-02 as a second micro-workstream. Make this work a dependency of M2, not phase 0 inside M2.

### 2. [BLOCKING] The 20-28 task estimate is materially low and hides contract-wide migration work

M2 is not six local patches. The current lifecycle writes `running` and emits `AttemptStarted` in the creation transaction before spawn (`crates/db/src/models/execution_process/queries.rs:435-509`), then creates executor-session state and spawns later (`crates/services/src/services/container.rs:1514-1569`). The real spawn path treats PID-persistence failure as a warning and continues (`crates/local-deployment/src/container.rs:1925-1966`). Retry stops, drops history, and clears the draft before replacement startup (`crates/server/src/routes/task_attempts/handlers/follow_up.rs:171-257`, `:341-361`). Those are separate state-machine, compensation, API, and regression-test changes.

The status vocabulary is also cross-cutting. Rust has many hard-coded `running`/`killed` predicates across DB queries, services, assignment handling, and routes; representative examples are `crates/db/src/models/task/queries.rs:49-64`, `crates/services/src/services/drafts.rs:154-163`, `crates/services/src/services/assignment_handler.rs:203-227`, and `crates/server/src/routes/execution_processes.rs:204-237`. The frontend independently derives running state and terminal behavior (`frontend/src/hooks/useExecutionProcesses.ts:147-153`, `frontend/src/hooks/useConversationHistory.ts:448-451`, `frontend/src/components/tasks/TaskDetails/ProcessesTab.tsx:53-80`). Hive mirrors status as an unrestricted string and has a running-only query (`crates/remote/src/db/node_execution_processes.rs:20-37`, `:177-190`). Treating these as incidental edits would create contradictory UI/API behavior.

An honest surgical TDD budget is approximately **41-53 tasks**, not 20-28:

| M2 phase | Task budget | Required behavior |
|---|---:|---|
| Contract, schema, migration, transition API, event semantics | 7-9 | One durable generation/ownership identity; legacy backfill; legal transitions; generated/wire types |
| Launch, Create & Start compensation, context, all callers | 7-9 | `starting`; durable owner/PID before `running`; kill on persistence failure; structured `start_failed`; initial/retry/follow-up context |
| Cancellation and retry | 6-8 | `stopping`/`stop_failed`; verified quiescence; no swallowed failure; preserve draft/history until replacement is running |
| Output durability and migration | 8-10 | Atomic history/live handoff; lag behavior; acknowledged raw flush; awaited normalization; ordered completion; resumable migration |
| Startup barrier and pre-M6 reconciliation | 6-8 | Mutation gate; fence every stale writer; no auto-resume; interrupted state; explicit retry; all launch surfaces |
| UI, Hive projection, integration/fault tests, live evidence | 7-9 | Truthful states/actions; cross-surface compatibility; AC1-13; operator evidence and review |

Keep these phases in **one M2 WAI workstream** and one canonical spec. Small PRs are allowed, but the roadmap explicitly says a milestone does not complete until all outcome criteria, tests, live evidence, documentation, and remediation pass (`docs/superpowers/specs/2026-08-21-local-node-first-use-roadmap-design.md:202-204`). The schema/transition phase must land first; every later phase consumes it. This preserves one identity model while avoiding a 50-task undifferentiated plan.

### 3. [BLOCKING] The shipped event-bus contract is a missing M2 design fork

The proposal calls the pre-M6 auto-resume conflict the first design fork, but misses an equally important P4 contract conflict. Today `ExecutionProcess::create` inserts status `running` and appends `AttemptStarted` in the same transaction (`crates/db/src/models/execution_process/queries.rs:443-507`). M2 requires a durable `starting -> running -> terminal` handshake and forbids `running` before ownership/process identity is durable (`docs/superpowers/specs/2026-08-21-local-node-first-use-roadmap-design.md:277-288`). Therefore the existing event can no longer be emitted at creation while retaining its current "started" meaning.

This is a shipped external surface, not dead internal code. `NodeEvent` exposes `AttemptStarted`, `AttemptFinished`, and `AttemptFailed` (`crates/db/src/models/event.rs:20-51`), the type is TS-exported as a public discriminated union (`shared/types.ts:33`), and P4's accepted spec fixes those event names (`dev-docs/workstreams/vk-swarm-event-bus/spec/2026-08-07-vk-swarm-event-bus.md:83`). Terminal transition emission is also tied to the old `running` predicate (`crates/db/src/models/execution_process/lifecycle.rs:104-123`, `:151-194`).

Required revision: add an explicit M2 decision for event semantics and compatibility. Either introduce a generation-created/starting event and move `AttemptStarted` to the durable `running` transition, or formally redefine/version the existing event. Include generation identity and terminal classification where consumers need them, regenerate shared types, and update P4 emission/E2E/conformance tests. P5/P6 need not start, but M2 cannot treat the shipped P4 contract as out of scope.

### 4. [BLOCKING] The output plan identifies one race but not the mechanisms needed to satisfy AC6-AC9

The proposal correctly folds the confirmed fast-execution defect into M2, but its mechanism and task estimate are incomplete:

| Required property | Current contradiction |
|---|---|
| Zero loss at history/live handoff | `push` broadcasts before history insertion, while `history_plus_stream` snapshots before subscribing (`crates/utils/src/msg_store.rs:55-70`, `:124-135`). |
| Zero loss under receiver lag | Both streams discard every `BroadcastStream` error with `res.ok()`, including `Lagged` (`crates/utils/src/msg_store.rs:131-148`). Atomic subscription alone does not repair this. |
| Acknowledged batcher finish | `finish` returns `()` and only logs channel-send failure (`crates/services/src/services/log_batcher.rs:84-99`); flush failure is logged rather than acknowledged (`:203-228`). |
| Completion after durable raw and normalized output | The exit monitor records terminal completion and runs migration before batcher finish and normalization completion (`crates/local-deployment/src/container.rs:620-673`, `:835-864`). |
| Resumable partial migration | Incremental migration skips the entire execution when **any** normalized entry exists (`crates/services/src/services/log_migration.rs:192-212`), the opposite of partial resume. |
| Deterministic migration completion | Migration discards the normalizer handle and guesses completion after three stable 50 ms polls (`crates/services/src/services/log_migration.rs:307-384`). |

The confirmed tracker proves the sub-second zero-patch case is a completed lost-log race, not merely a discarded timeout (`dev-docs/workstreams/normalize-fast-execution-lost-logs-flake/README.md:85-109`, `:111-141`). Required revision: enumerate each row above as implementation plus failure-injection/stress coverage. Do not describe `MsgStore` atomicity alone as the fix for M2 output durability.

### 5. [BLOCKING] The restart barrier must cover Hive, drafts, setup/review, and breakdown entry points

Startup recovery currently runs in a detached task after deployment construction (`crates/server/src/main.rs:124-146`). A router-only guard would still leave non-router launch paths. Hive assignments invoke `start_attempt` directly (`crates/services/src/services/assignment_handler.rs:115-154`), queued drafts invoke `start_execution` (`crates/services/src/services/drafts.rs:211-279`), and setup/review/follow-up routes also invoke the shared service. The current code has numerous `start_attempt`, `start_execution`, and `stop_execution` call sites across `crates/server`, `crates/services`, and `crates/local-deployment`; the barrier belongs at the shared mutation service boundary.

Breakdown cannot be deferred wholesale to M7. `start_breakdown_attempt` flows through the same `start_execution` function (`crates/services/src/services/container.rs:1378-1495`), and a breakdown proposal can already remain Draft across restart when no process is linked or completion handling is skipped (`dev-docs/workstreams/task-breakdown-followups/README.md:48-60`). M2 need not implement breakdown-product recovery UX, but it must define how every existing run reason participates in generation ownership, terminal/interrupted classification, event emission, and the startup mutation barrier. A regression must prove breakdown cannot bypass the barrier or create a second writer; any remaining Draft-proposal repair can stay in the follow-up tracker behind a named reconciliation hook.

Required revision: inventory every mutation caller during precheck and make the gate central. Add breakdown and Hive-assignment compatibility tests to M2 rather than saying breakdown can consume the hook later.

### 6. [SHOULD-FIX] M2 needs explicit M3/M6 compatibility fixtures, not implementation dependencies

The proposal correctly keeps M3 separate, but "M3 consumes M2" is not enough to prove that M2's schema is permanent. Existing recovery identity is spread over `execution_processes.server_instance_id` (`crates/db/migrations/20260118184126_add_server_instance_id_to_execution_processes.sql:1-11`), `resume_state` (`crates/db/migrations/20260201000100_add_resume_state_to_execution_processes.sql:1-13`), PID plus worktree/session projection (`crates/db/migrations/20260201000200_add_workstream_state_view.sql:1-21`), and `executor_sessions.session_id` (`crates/db/migrations/20250623120000_executor_sessions.sql:3-17`). M2 must evolve this data rather than add a parallel generation/recovery model.

The roadmap also requires each M2 launch identity to include adapter process/session identities and says M6 must claim the same schema (`docs/superpowers/specs/2026-08-21-local-node-first-use-roadmap-design.md:281-288`, `:345-346`, `:665-695`). M3 later requires a durable remote OpenCode session before prompt submission (`:378-401`).

Required revision: define an adapter-neutral durable identity/ownership envelope in M2 and add schema/API fixtures representing attached OpenCode, direct CLI, Claude, and an M6 claim. This does not pull M3 behavior into M2; it proves M3-M6 can extend the schema without replacement. M3 should still remain a separate workstream.

### 7. [BLOCKING] The recommended sequence omits the actual first-release certification step

The proposal ends after creating M3. The roadmap requires more after M3 implementation: the full release build and all Rust/frontend/remote-frontend gates (`docs/superpowers/specs/2026-08-21-local-node-first-use-roadmap-design.md:450-528`), a 16-step real-LAN run (`:530-554`), archived evidence, and an **integrated adversarial review over M1-M3 followed by complete gate reruns** (`:556-565`). The local frontend does not currently expose `test:e2e` in its scripts (`frontend/package.json:6-18`), consistent with the roadmap assigning that setup to M3 (`docs/superpowers/specs/2026-08-21-local-node-first-use-roadmap-design.md:521`).

Required revision: add a final umbrella release-certification step after M3. It must have an owner and cannot be implied by M3's normal close. It should run the exact section 9 automated gate, the final-build LAN journey, evidence archival, the M1-M3 integrated review, remediation, and final reruns.

### 8. [SHOULD-FIX] Two proposal facts and two anchors need correction

1. "M2 is the only unstarted item on the critical path" is false as written: M3 is also unstarted; the proposal's own table says it does not exist. Correct this to "M2 is the earliest unmet dependency and therefore the next product milestone." The graph includes M3 before release (`docs/superpowers/specs/2026-08-21-local-node-first-use-roadmap-design.md:176-200`).
2. The auto-resume claim is true, but `container.rs:322-469` stops at the branch/comment. The actual call to `resume_execution` and success classification is at `crates/services/src/services/container.rs:468-530`.
3. `container.rs:320` locates the sweep body, not constructor reachability. Cite `crates/local-deployment/src/container.rs:143-165` for constructor activation and `:514-520` for the spawned sweep.
4. The handoff is not "fully consumed (all five WS-A..E shipped)." WS-E explicitly includes F-2026-08-04-02 (`dev-docs/2026-08-05-next-session-handoff.md:80-87`), and that same defect remains `ready`/confirmed (`dev-docs/workstreams/normalize-fast-execution-lost-logs-flake/README.md:1-17`, `:85-109`). Mark the handoff superseded only with a note that WS-E moved to M2; do not say it shipped.

## Anchor audit

| Proposal claim/anchor | Result |
|---|---|
| Main is `492e8e94`; GitHub has zero open issues/PRs | **Verified.** `git rev-parse HEAD` returned `492e8e94746327e1b39f2531d9f84afb6d8a3d9f`; both `gh ... list --state open` commands returned no rows. |
| P3/P4/M1 shipped | **Verified.** `dev-docs/workstreams/vk-swarm-task-breakdown/README.md:1-8`, `dev-docs/workstreams/vk-swarm-event-bus/README.md:1-9`, `dev-docs/workstreams/local-node-browser-oauth/README.md:1-16`. |
| Backlog 48 total, 8 high, 15 open | **Verified.** Totals are generated at `dev-docs/MASTER.md:59-65`; the 15 open rows are `:68-84` excluding the two triaged rows. |
| Stale `/wai:next` task 301 | **Verified.** The task remains `status: ready` at `dev-docs/workstreams/hive-oauth-sw-bypass/plans/hive-oauth-sw-bypass/phase-3/301-full-repo-gates-exit-0-live-deploy-verification-evidence-sw-registered-sign-ins-cache-inspection-stalled-flow-timeout.md:1-15`, despite shipped README status and round-2 evidence at `dev-docs/workstreams/hive-oauth-sw-bypass/plans/hive-oauth-sw-bypass/decisions-ledger.md:287-302`. |
| Create & Start reports success on start failure (`core.rs:486-511`) | **Verified exactly.** `is_ok()` becomes a boolean and the handler returns success at `crates/server/src/routes/tasks/handlers/core.rs:486-511`. |
| Stop failures suppressed (`container.rs:714-748`) | **Verified exactly.** Errors are logged at debug and timeout proceeds at `crates/services/src/services/container.rs:714-748`. |
| `MsgStore` handoff race (`msg_store.rs:55-71`, `:125-135`) | **Verified**, plus unmentioned lag loss at `crates/utils/src/msg_store.rs:131-148`. |
| Migration precedes flush/normalizer completion (`local container :657`, `:836-858`) | **Verified exactly.** `crates/local-deployment/src/container.rs:655-673`, `:835-864`. |
| Only four statuses (`execution_process/mod.rs:56-61`) | **Verified exactly.** `crates/db/src/models/execution_process/mod.rs:52-61`. |
| No generation column | **Verified.** Existing ownership/recovery pieces are cited in finding 6; repository search found no `generation` occurrence in `crates/db/migrations/*.sql`. |
| Auto-resume at `services/container.rs:322-469` | **Claim verified; anchor incomplete.** Use `:322-326` plus `:450-530`. |
| Lost-log tracker is a real product race | **Verified.** `dev-docs/workstreams/normalize-fast-execution-lost-logs-flake/README.md:85-109`, `:111-141`. |
| Constructor safety anchors | **Claim verified; activation anchor incomplete.** Use `crates/local-deployment/src/container.rs:143-165`, `:514-520`, and `crates/local-deployment/src/lib.rs:256-279`. |
| Serve `Err` skips cleanup (`main.rs:273-275`) | **Verified exactly.** `?` precedes cleanup at `crates/server/src/main.rs:273-280`. Include this in M2 only if M2 owns all acknowledged-shutdown exits and tests this error path; file overlap alone is not a scope rule. |
| Host OpenCode is 1.18.26 while roadmap pins 1.18.19 | **Verified.** `opencode --version` returned `1.18.26`; certification text is `docs/superpowers/specs/2026-08-21-local-node-first-use-roadmap-design.md:366-376`. |
| Claude hooks and MCP dev-loop anchors | **Verified.** Hook merge is defined/called at `crates/executors/src/executors/claude.rs:172-192`, `:391`; root dev watch only watches/runs the node server at `package.json:28-35`. |
| Handoff fully consumed/all WS-A-E shipped | **Incorrect.** WS-E's log defect is still open and proposed for M2; see finding 8. |

## Panel questions

| Question | Answer |
|---|---|
| Q1 | M2 is the right next product milestone. First complete M0 tracker hygiene and the constructor-side-effect predecessor; then create/decompose M2. No smaller product milestone displaces it. |
| Q2 | Do not fold F-2026-08-19-01/-02 into the M2 lifecycle spec. Prefer one separate predecessor only if both are expressed and tested as the single invariant "test/scratch construction has no operator-owned external side effects." |
| Q3 | Keep `claude-executor-hooks-hang` out of M2. The first release uses attached OpenCode, while Claude is M5 (`docs/superpowers/specs/2026-08-21-local-node-first-use-roadmap-design.md:16-29`, `:619-650`). Its high severity justifies a parallel bugfix worktree, not scope corruption. |
| Q4 | One M2 workstream is appropriate but 20-28 tasks is overloaded. Use the six dependent phases and roughly 41-53 tasks above, all consuming one schema/transition contract. Do not close or fork the lifecycle identity between phases. |
| Q5 | Core anchors are mostly correct. Fix the incomplete recovery and constructor anchors, the false handoff-consumed claim, and the "only unstarted item" wording. Add the missing event, lag, partial-migration, caller, and release-gate anchors. |

## Required proposal revision

1. Keep `local-execution-lifecycle-correctness` as the next product milestone.
2. Complete the M0 tracker-hygiene PR first.
3. Move F-2026-08-19-01/-02 to a coherent constructor-side-effect-isolation predecessor with its own reachability gate.
4. Re-scope M2 to the six dependent phases above and budget roughly 41-53 surgical tasks.
5. Add explicit decisions for P4 event semantics and for evolving the existing `server_instance_id`/`resume_state`/session projection into the permanent generation identity.
6. Enumerate every output-loss mechanism, mutation caller, run reason, UI query, Hive projection, and fault-injection gate.
7. Keep M3, Claude hooks, MCP tooling, and unrelated backlog items separate; treat F-2026-09-02-01 according to shutdown behavior, not file locality.
8. Add the post-M3 first-release certification/review/rerun step.
9. Correct the four factual/anchor defects in finding 8.

VERDICT: REVISE
