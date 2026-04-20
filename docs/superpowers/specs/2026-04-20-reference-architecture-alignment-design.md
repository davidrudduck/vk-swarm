# Reference Architecture Alignment Design

**Date:** 2026-04-20

**Goal**

Align `vk-swarm` with the execution, logging, playback, and executor-configuration architecture used in `/data/Code/reference/vibe-kanban`, while preserving the local extensions that still provide product value.

**Non-goals**

- Rebuild the entire settings system into the reference modal/host UI in one pass.
- Replace `log_entries` or Electric-oriented storage immediately.
- Remove live message injection as a capability if the backend can support it safely.

## Current Divergences

### Executor Configuration And Versioning

The reference separates three concerns:

- `ExecutorProfileId`: executor identity and variant
- `ExecutorProfile` / `ExecutorConfigs`: persisted preset definitions in `profiles.json`
- `ExecutorConfig`: selected executor plus lightweight runtime overrides for a run

`vk-swarm` still overloads `ExecutorConfig` to mean stored preset maps. That makes runtime selection, queue persistence, and executor discovery harder than they should be. It also leaves the UI too dependent on the raw shape of `profiles.json`.

The reference also pins executor package versions in code. `vk-swarm` currently uses `@latest` for major executors, which increases drift risk and makes debugging harder.

### Execution Lifecycle, Logs, And Replay

The reference keeps one chronological stream model through the run:

- raw output is captured immediately
- normalization is driven from the same ordered stream
- websocket replay and live updates come from the same source of truth

`vk-swarm` currently splits this apart:

- batched raw JSONL is persisted to `execution_process_logs`
- normalization for durable replay is reconstructed later by `log_migration`
- frontend live viewing relies on append-only repair logic to stabilize ordering

That architecture is the root cause of the recent Codex playback bugs. It allows ordering drift, lossy replay, and lifecycle races between final batch flushes and migration.

### Queueing And Live Steering

The reference queue contract is simple: one queued follow-up for the next turn, consumed server-side after the current run completes successfully.

`vk-swarm` changed this to an attempt-scoped ordered message queue with optional live injection into the running process. The queue and the injection path are now partially conflated:

- some code/comments still describe next-turn execution semantics
- the actual queue is not auto-consumed in the normal completion path
- the frontend treats queueing and injection as one compound action

This makes behavior harder to reason about for both the user and the implementation.

## Target Architecture

### 1. Runtime Executor Model

Adopt the reference split as the primary contract:

- `ExecutorProfileId` remains the stable preset identity.
- `ExecutorProfile` / `ExecutorConfigs` remain the persisted preset store.
- `ExecutorConfig` becomes the runtime run-selection object:
  - `executor`
  - `variant`
  - optional runtime overrides such as model, reasoning, agent mode, permission policy

This runtime object is what flows through:

- task execution
- follow-up queueing
- next-turn continuation
- runtime discovery and preset selection UI

The settings UI may continue using the current route-based layout for now, but it must stop using raw preset JSON as the runtime selection contract.

### 2. Ordered Log Stream As Source Of Truth

The canonical truth for an execution is the ordered stream emitted while it runs.

Required behavior:

- live websocket playback must preserve backend chronology
- completed-run replay must preserve the same chronology
- frontend code must not invent ordering or append-only revisions

For this migration, the reference contract is the target:

- running normalized snapshots replace the current per-process snapshot in order
- rows are rendered in process creation order and snapshot entry order

Longer-term, `vk-swarm` can evolve toward a durable append-only event journal. That is not required for this migration.

### 3. Durable Storage Strategy

Short term:

- keep `execution_process_logs` as raw archival storage
- keep `log_entries` for pagination/Electric-style access
- stop treating post-run reconstruction as the primary replay model

Required lifecycle ordering:

1. flush remaining raw logs
2. finalize in-memory normalization/live stream state
3. write any derived durable replay rows
4. expose replay from a complete ordered state

If `log_entries` cannot yet store full chronological evolution, the frontend replay path should still prefer the normalized websocket/history contract for conversation playback, not a lossy reconstructed approximation.

### 4. Queueing And Steering Contract

Re-establish two distinct concepts:

- `Queue for next turn`
  - server-side, deterministic, uses `ExecutorConfig`
  - consumed after the current run completes successfully
- `Inject into running turn`
  - explicit live steering action
  - only available when executor capabilities declare support

The queue is not the injection mechanism.

UI and backend must both reflect that split.

### 5. Capability Contract

Executor capability exposure must be executor-agnostic and declarative.

The frontend should rely on stable capability flags such as:

- `supports_interrupt`
- `supports_review`
- `supports_live_follow_up_messages`

Executor-specific behavior such as Claude/Codex protocol differences stays behind backend abstractions.

### 6. Executor Versioning

Pin executor binary/package versions in code instead of `@latest`.

Profiles remain responsible for behavior and model defaults. They should not be the only line of defense against binary/runtime drift.

## Migration Strategy

The path of least resistance is a staged migration toward the reference architecture.

### Stage 1: Chronological Playback Parity

Replace frontend append-only reconciliation with reference-style snapshot replacement for running normalized logs.

Outcome:

- live output displays in chronological order
- replay and live behavior match
- tool calls, tool results, messages, thinking, and reasoning stay in backend order

### Stage 2: Queue Contract Parity

Separate queued next-turn follow-ups from live injection.

Outcome:

- queue semantics become predictable again
- backend consumes queued follow-ups after successful completion
- injection remains available only as an explicit capability-gated action

### Stage 3: Executor Config Alignment

Introduce the reference-style runtime `ExecutorConfig` contract and preset/discovery flow without requiring a full settings rewrite.

Outcome:

- runtime selection stops depending on raw profile JSON
- executor discovery becomes a stable API surface
- queueing and execution use one consistent runtime object

### Stage 4: Storage Lifecycle Alignment

Fix the completion-order race and reduce replay dependence on post-run lossy migration.

Outcome:

- durable replay is built from complete ordered data
- `log_entries` remain useful without becoming the primary live chronology source

## Testing Strategy

Every migration slice follows TDD:

1. add a focused failing test for the target behavior
2. run it and verify the failure is the correct one
3. implement the minimum code to make it pass
4. run focused verification
5. run broader regression verification

Required coverage areas:

- frontend running-log ordering and rendering
- websocket replay/live ordering invariants
- queue consumption semantics
- capability gating for live steering
- executor config serialization and runtime selection
- completion ordering between flush, normalization, and durable replay

## Risks

- Some local UI affordances currently depend on attempt-scoped queue state; moving toward reference semantics may require a compatibility layer during the transition.
- `log_entries` consumers outside the conversation UI may rely on the current post-run reconstruction behavior.
- Introducing the reference runtime `ExecutorConfig` meaning will touch shared types and API contracts.

## Decisions

- Align to the reference execution/log playback model now.
- Preserve live injection only as an explicit extension, not as the queue contract.
- Keep route-based settings for now; align the data model before reworking the entire settings shell.
- Pin executor versions in code as part of the executor-configuration migration.
