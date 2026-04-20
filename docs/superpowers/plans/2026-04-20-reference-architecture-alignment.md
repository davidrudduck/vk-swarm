# Reference Architecture Alignment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align `vk-swarm` with the reference execution/playback architecture so live and replayed executor output stays chronological, queueing semantics are deterministic, and executor runtime selection stops depending on raw preset JSON.

**Architecture:** First restore the reference data-flow for running and replayed logs, then separate next-turn queueing from live steering, then align executor runtime configuration and version pinning with the reference split between persisted presets and runtime overrides. Preserve local extensions only where they remain explicit and capability-gated.

**Tech Stack:** Rust, Axum, sqlx/SQLite, React, TanStack Query, Vitest, Cargo tests

---

## File Map

- `frontend/src/hooks/useConversationHistory.ts`
  - Running execution-process streaming, history loading, and timeline emission.
- `frontend/src/utils/logs/appendOnlyTimeline.ts`
  - Current append-only reconciliation logic that should be removed or reduced to non-ordering utilities.
- `frontend/src/components/logs/VirtualizedList.tsx`
  - Auto-follow behavior after chronology changes.
- `frontend/src/components/logs/VirtualizedList.test.ts`
  - Frontend playback ordering regression coverage.
- `frontend/src/components/tasks/TaskFollowUpSection.tsx`
  - Follow-up UI wiring for send, queue, and live injection actions.
- `frontend/src/hooks/message-queue/useMessageQueueInjection.ts`
  - Current queue+inject compound behavior to split.
- `frontend/src/hooks/message-queue/useMessageQueue*.ts`
  - Queue-only semantics and frontend query behavior.
- `crates/server/src/routes/execution_processes.rs`
  - Log WS endpoints and inject-message endpoint behavior.
- `crates/server/src/routes/message_queue.rs`
  - Attempt queue routes if retained, or compatibility layer during migration.
- `crates/local-deployment/src/container.rs`
  - Completion lifecycle, queued follow-up consumption, and ordering of flush/normalization.
- `crates/local-deployment/src/message_queue.rs`
  - Attempt-scoped queue model to simplify or constrain.
- `crates/server/src/routes/config.rs`
  - Capability and executor runtime discovery APIs.
- `crates/executors/src/profile.rs`
  - Runtime `ExecutorConfig` vs persisted preset structures.
- `crates/executors/src/executors/codex.rs`
  - Codex version pinning and runtime capability implementation.
- `crates/executors/src/executors/claude.rs`
  - Claude version pinning and capability parity.
- `crates/services/src/services/log_batcher.rs`
  - Raw log flush lifecycle.
- `crates/services/src/services/log_migration.rs`
  - Derived `log_entries` behavior to make chronology-safe.

## Task 1: Restore Reference-Style Running Playback

**Goal:** Make running normalized logs render in strict backend order by replacing the current snapshot per process instead of applying append-only reconciliation.

**Files:**
- Modify: `frontend/src/hooks/useConversationHistory.ts`
- Modify: `frontend/src/utils/logs/appendOnlyTimeline.ts`
- Modify: `frontend/src/components/logs/VirtualizedList.test.ts`
- Modify: `frontend/src/components/logs/VirtualizedList.tsx`

**Failing tests to add first:**
- A test proving that a later snapshot which inserts tool-use rows before a message renders in snapshot order instead of appending those rows to the end.
- A test proving that growing content for the same logical row replaces the current row instead of producing prefix-history duplicates.
- A test proving the auto-follow target stays on the latest real chronological row instead of the transient footer.

**Implementation steps:**
- [ ] Add the new ordering regression tests in `VirtualizedList.test.ts`.
- [ ] Run the focused Vitest file and confirm the new tests fail for the expected ordering reasons.
- [ ] Remove `getRunningAppendOnlyResult(...)` from the running normalized-log path in `useConversationHistory.ts` and replace it with the reference `patchWithKey + replace process entries` behavior.
- [ ] Reduce `appendOnlyTimeline.ts` to helper functionality that does not invent ordering, or remove it from the running timeline entirely.
- [ ] Keep or adjust the auto-follow target logic in `VirtualizedList.tsx` so chronological playback still scrolls to the latest non-transient row.
- [ ] Re-run the focused Vitest file until green.

**Verification steps:**
- `pnpm vitest run src/components/logs/VirtualizedList.test.ts`
- `pnpm run check`

**Risks / dependencies:**
- Depends on preserving current renderer expectations for stable `patchKey`s.
- May expose backend ordering defects that the frontend was previously masking.

## Task 2: Align Replay And Completion Ordering

**Goal:** Ensure completed-run replay is built from complete ordered data by fixing the exit lifecycle ordering around raw-log flush, normalization completion, and derived replay writes.

**Files:**
- Modify: `crates/local-deployment/src/container.rs`
- Modify: `crates/services/src/services/log_batcher.rs`
- Modify: `crates/services/src/services/log_migration.rs`
- Add/Modify tests in the relevant Rust modules

**Failing tests to add first:**
- A container/service test proving completion does not start durable replay derivation until the final raw batch flush has completed.
- A migration test proving chronological entry evolution is not truncated when the last patch batch arrives at process shutdown.

**Implementation steps:**
- [ ] Add a Rust test around exit handling or a narrow helper extracted from `container.rs` to assert flush/normalize/migrate ordering.
- [ ] Add a `log_migration` test fixture that includes incremental `replace`/growth behavior and verify the existing implementation loses chronology.
- [ ] Refactor the completion path so final raw-log flush and live normalizer completion happen before migration or other derived replay work.
- [ ] Narrow `log_migration` so it no longer becomes the primary source of conversation chronology; keep it only as a derived durable representation.
- [ ] Run the focused Rust tests until green.

**Verification steps:**
- `cargo test -p services log_migration -- --nocapture`
- `cargo test -p local-deployment completion -- --nocapture`

**Risks / dependencies:**
- Touches the process completion path and may affect cleanup/finalization timing.
- May require a small helper extraction from `container.rs` to make TDD practical.

## Task 3: Re-Establish Next-Turn Queue Semantics

**Goal:** Separate queued follow-ups from live injection so queued messages are consumed server-side after successful completion, matching reference behavior.

**Files:**
- Modify: `crates/local-deployment/src/container.rs`
- Modify: `crates/local-deployment/src/message_queue.rs`
- Modify: `crates/server/src/routes/message_queue.rs`
- Modify: `frontend/src/hooks/message-queue/useMessageQueueInjection.ts`
- Modify: `frontend/src/components/tasks/TaskFollowUpSection.tsx`
- Add/Modify associated frontend and Rust tests

**Failing tests to add first:**
- A backend test proving a queued follow-up is auto-consumed after a successful coding-agent completion.
- A backend test proving failed/killed runs do not auto-start queued follow-ups.
- A frontend hook test proving queueing does not implicitly mean “remove from queue on successful injection”.

**Implementation steps:**
- [ ] Add backend tests for queued-follow-up consumption semantics.
- [ ] Add frontend tests for the split between queue-only and inject-only behavior.
- [ ] Re-enable or re-implement queued follow-up consumption after successful completion in `container.rs`.
- [ ] Keep live injection as a distinct path that does not redefine queue semantics.
- [ ] Update `useMessageQueueInjection.ts` so queueing and injection become separate actions or clearly separate return paths.
- [ ] Update the follow-up UI copy and behavior in `TaskFollowUpSection.tsx` to reflect the split.
- [ ] Run focused backend and frontend tests until green.

**Verification steps:**
- `cargo test -p local-deployment queued -- --nocapture`
- `pnpm vitest run src/hooks/message-queue/__tests__/useMessageQueueInjection.test.ts`

**Risks / dependencies:**
- Existing users may rely on the current “queue then inject” behavior.
- May require a temporary compatibility path for attempt-scoped queued messages.

## Task 4: Unify Steering Capabilities

**Goal:** Make live steering, interrupt, and review behavior depend on explicit executor capabilities rather than executor-specific assumptions.

**Files:**
- Modify: `crates/server/src/routes/config.rs`
- Modify: `crates/executors/src/executors/codex.rs`
- Modify: `crates/executors/src/executors/claude.rs`
- Modify: `frontend/src/hooks/useAgentRuntimeCapabilities.ts`
- Modify: `frontend/src/components/tasks/TaskFollowUpSection.tsx`

**Failing tests to add first:**
- A backend serialization test proving both Claude and Codex can surface `supports_live_follow_up_messages` consistently.
- A frontend test proving the follow-up UI hides or disables injection controls when the capability flag is false.

**Implementation steps:**
- [ ] Add backend tests for runtime capability mapping.
- [ ] Add frontend tests for capability-gated action visibility.
- [ ] Expand runtime/static capability responses so supported executors expose the same contract.
- [ ] Remove stale Claude-only/Codex-only assumptions from UI comments and gating logic.
- [ ] Re-run focused tests until green.

**Verification steps:**
- `cargo test -p server runtime_capabilities -- --nocapture`
- `pnpm vitest run src/components/tasks`

**Risks / dependencies:**
- Depends on how much runtime discovery each executor can actually provide.
- Might need a hybrid static + discovered capability model.

## Task 5: Introduce Runtime ExecutorConfig Compatibility Layer

**Goal:** Reintroduce the reference-style runtime `ExecutorConfig` contract without breaking persisted preset definitions.

**Files:**
- Modify: `crates/executors/src/profile.rs`
- Modify: `crates/server/src/routes/config.rs`
- Modify: shared TS/Rust type consumers that carry executor selection
- Modify: frontend API/types and selection hooks

**Failing tests to add first:**
- A Rust serialization test proving persisted `profiles.json` still round-trips while runtime `ExecutorConfig` serializes as executor + variant + overrides.
- A frontend test proving runtime executor selection no longer depends on the full raw preset JSON object shape.

**Implementation steps:**
- [ ] Add Rust tests that define the compatibility boundary between persisted presets and runtime selection.
- [ ] Add any needed frontend type/selection tests.
- [ ] Introduce the new runtime `ExecutorConfig` struct and compatibility helpers while keeping `ExecutorProfile` / `ExecutorConfigs` as the persisted preset model.
- [ ] Update queue/follow-up request types to carry the new runtime object.
- [ ] Add reference-style preset/discovery API shapes where needed for the frontend runtime selector.
- [ ] Run focused tests until green.

**Verification steps:**
- `cargo test -p executors profile -- --nocapture`
- `cargo test -p server config -- --nocapture`

**Risks / dependencies:**
- Broad type-surface change across Rust and frontend code.
- Must preserve backward compatibility for stored executor actions where practical.

## Task 6: Pin Executor Versions In Code

**Goal:** Stop using `@latest` for executor package/runtime installation.

**Files:**
- Modify: `crates/executors/src/executors/codex.rs`
- Modify: `crates/executors/src/executors/claude.rs`
- Modify: any shared installer/version helpers

**Failing tests to add first:**
- Executor-specific tests asserting the base command or install command includes a pinned version.

**Implementation steps:**
- [ ] Add focused tests for Codex and Claude version pinning.
- [ ] Replace `@latest` with explicit pinned versions in code.
- [ ] Keep profile defaults focused on runtime behavior and model choice, not package drift.
- [ ] Run focused tests until green.

**Verification steps:**
- `cargo test -p executors codex -- --nocapture`
- `cargo test -p executors claude -- --nocapture`

**Risks / dependencies:**
- Requires choosing versions compatible with the rest of the current stack.
- May need follow-up work when upstream CLIs change APIs.

## Task 7: Broader Settings Alignment Foundation

**Goal:** Keep the current route-based settings shell, but align the underlying data flow with the reference model so runtime selection does not depend on raw `profiles.json`.

**Files:**
- Modify: `frontend/src/pages/settings/AgentSettings.tsx`
- Modify: `frontend/src/components/ExecutorConfigForm.tsx`
- Modify: runtime preset/discovery hooks or add new ones

**Failing tests to add first:**
- A frontend test proving settings edit persisted presets while task/runtime selection uses the runtime contract instead of the raw JSON form shape.

**Implementation steps:**
- [ ] Add the failing settings/runtime contract test.
- [ ] Extract or introduce hooks that mirror the reference preset/discovery split.
- [ ] Keep raw JSON editing available only as an advanced preset-authoring path.
- [ ] Re-run focused tests until green.

**Verification steps:**
- `pnpm vitest run src/pages/settings src/components/ExecutorConfigForm*`
- `pnpm run check`

**Risks / dependencies:**
- Depends on Task 5.
- This is a foundation task, not a full settings-shell rewrite.

## Task 8: Final Integration And Regression Verification

**Goal:** Verify the aligned architecture works end-to-end and does not regress existing execution flows.

**Files:**
- No primary code target; runs across the modified surface.

**Implementation steps:**
- [ ] Run all focused frontend tests for logs, message queue, follow-up UI, and settings/runtime selection.
- [ ] Run targeted Rust tests for executors, server routes, local deployment, and services.
- [ ] Run repo-wide static checks used by this project.
- [ ] Smoke test a live execution locally, verifying chronological playback, next-turn queueing, and explicit injection behavior.
- [ ] Commit the migration in logical chunks or as one final integration commit, depending on change boundaries.

**Verification steps:**
- `pnpm vitest run src/components/logs/VirtualizedList.test.ts src/hooks/message-queue/__tests__/useMessageQueueInjection.test.ts`
- `pnpm run check`
- `cargo test -p executors -- --nocapture`
- `cargo test -p server -- --nocapture`
- `cargo test -p local-deployment -- --nocapture`
- `cargo test -p services -- --nocapture`

**Risks / dependencies:**
- Depends on all earlier tasks.
- The smoke test may need a real executor installed and reachable in the local environment.
