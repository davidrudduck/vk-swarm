---
id: "014"
phase: 5
title: "Wire the EventBus, tailer, trigger-hook runner and compaction loop into deployment startup"
status: ready
depends_on: ["009","011","013"]
parallel: false
conflicts_with: []
files:
  - "crates/local-deployment/src/lib.rs"
irreversible: false
scope_test: "crates/local-deployment"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
**File:** `crates/local-deployment/src/lib.rs` colocated tests.

1. `deployment_exposes_an_event_bus` — construct a deployment against a test pool; assert an
   `EventBus` handle is reachable and `subscribe_from(0)` yields a working stream.
2. `startup_spawns_the_tailer` — construct a deployment, journal an event through a model function,
   and assert a subscriber receives it live within a bounded wait. This is the end-to-end proof that
   append → tail → broadcast is actually connected on a real deployment, not just in unit tests.
3. `startup_registers_the_real_trigger_hook` — assert the hook registry is non-empty and contains the
   status hook by name.
4. `startup_spawns_compaction` — assert the compaction handle exists.
5. `shutdown_stops_the_background_tasks` — drop/shut down the deployment and assert the spawned tasks
   terminate rather than leaking.
   **Two constraints, both learned the hard way in task 013 (added 2026-08-12):**
   (a) **Assert BEHAVIOURALLY, not on a handle.** `EventBus::shutdown()` `take()`s and drops its
       `JoinHandle`, so nothing outside that module can call `is_finished()`. Call
       `deployment.event_bus().shutdown().await`, then commit a journal row and assert nothing is
       published. `LocalDeployment::new` is already `async` (`crates/local-deployment/src/lib.rs:156`,
       `:165`) and already spawns background work at `:171`, so the `.await` is reachable from here —
       you do NOT need to edit `event_bus/mod.rs`, which is outside this task's file set.
   (b) **SUBSCRIBE BEFORE the commit-and-wait window.** Task 013 shipped this exact test twice with
       the subscriber created AFTER the post-shutdown commit, and BOTH challengers proved it vacuous by
       replacing `shutdown()` with a literal no-op and watching it still pass: a tokio broadcast
       receiver never sees history, so a still-running tailer's publish is gone before the subscriber
       exists. Take the subscriber FIRST, then shut down, then commit, then assert silence. Prove it
       with a mutation: a no-op shutdown must make this test FAIL.

## Change
**Why this task exists at all.** Tasks 009, 011 and 013 each create a component and each
correctly STOP rather than editing deployment startup, because startup was in none of their file
sets. Without this task every one of them ships as dead code: the tailer never runs so nothing is
ever broadcast, the hook is never registered so SC6 cannot be demonstrated live, and the compaction
loop never spawns so the journal is unbounded despite a correct predicate. The adversarial review
found this as a plan-level hole — the STOP guards prevented silent shipping but nothing completed the
wiring.

**File:** `crates/local-deployment/src/lib.rs`
**Anchor:** the `LocalDeployment` constructor, where the DB is built and background services are
assembled. Note `bootstrap()` at L159 is used ONLY to construct the `EventService` after-connect
hook; the live pool comes from `DBService::new_with_after_connect(hook)` at L165. **The `EventBus`
must be built over the LIVE `DBService`, not the bootstrap one** — building it over `bootstrap` is
the same class of mistake as the rejected process-global sender, and would leave the bus reading a
pool nothing writes to.

**After:**
1. Construct the `EventBus` over the live `DBService` pool.
2. Spawn the journal tailer (task 013) and retain its handle.
3. Build the trigger-hook registry, register the one real status hook (task 009), and spawn its
   runner with the same `EventBus`.
4. Spawn the compaction loop (task 011) with the same live pool.
5. Expose the `EventBus` on the deployment so task 010's route can reach it.
6. Retain every handle so shutdown can stop them; follow the existing supervised-loop pattern in this
   file rather than inventing a new one.

Read how the file already spawns and supervises its background work before adding to it, and follow
that shape exactly — record any divergence in the ledger.

## Allowed moves
ONLY the wiring in `crates/local-deployment/src/lib.rs`. Do NOT change the components
themselves — if a component's constructor is awkward to call from here, that is a defect in its
owning task (009/011/013); STOP and fix it there. Do NOT alter the existing `EventService`
after-connect hook wiring.

## STOP triggers
- The live `DBService` is NOT the one from `new_with_after_connect` — re-read L155-166 before
  wiring anything; attaching the bus to the wrong service is the single most likely failure here.
- Spawning requires a runtime handle that is not available at this point in construction — record
  where the other background loops obtain theirs and follow it; do NOT block on a runtime inside the
  constructor.
- The deployment has no shutdown path to hook into — record that, and still retain the handles rather
  than detaching the tasks.
- Any component's constructor signature does not match what this file can supply — STOP; fix the
  owning task rather than adapting here.

## Manual verification (record in decisions-ledger)
Gate invocation (the Done-when placeholders): this is a Rust crate, so the runner MUST be overridden — the auto-detected runner would try vitest. Use WAI_TYPECHECK_CMD="cargo check --workspace" with the WAI_TEST_CMD given below.
WAI_TEST_CMD="cargo test -p local-deployment"

On a running node built from this branch:
1. Create a task, then immediately `curl -N http://<node>/api/events` in another shell and create a
   second task — the second event arrives live. This proves the tailer is spawned and connected.
2. `sqlite3 $VK_DATABASE_PATH "select hook_name, last_processed_seq from trigger_cursors"` returns a
   row for the real hook, and its cursor advances as events flow. This proves the runner is spawned.
3. Confirm the compaction loop logs its first run at startup. Paste all three.

## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 014` exits 0
