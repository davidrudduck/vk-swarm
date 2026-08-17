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

## REQUIRED — added after panel 6 on task 013

### 1. `EventBus::Clone` shares one tailer handle — assert it HERE, where the call sites exist

Panel 6 proved that giving clones an independent `tailer_handle` survives the whole 268-test suite:

```text
MUTATION APPLIED: clones get an independent (empty) tailer handle
test result: ok. 268 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 10.73s
```

It was recorded as non-blocking for 013 because there are **zero call sites today** — the only
reference to the module outside itself is `crates/services/src/services/mod.rs:22`. This task creates
the first real ones, so this is where it becomes live.

`EventBus`'s own doc comment states the contract: *"All clones of this EventBus share the same tailer
handle. If one clone calls `shutdown()`, the tailer stops for ALL clones."* The hazard is specific and
this task walks straight into it: `DeploymentImpl` is cloned per request, so if `event_bus()` returns
a clone whose handle is independent, `shutdown()` becomes a silent no-op — the tailer survives process
shutdown holding a SQLite connection and polling every 75ms, and this task's own
`shutdown_stops_the_background_tasks` passes anyway, because it only asserts silence.

REQUIRED: a test that clones the deployment (or the bus), calls `shutdown()` on ONE clone, and asserts
the tailer is stopped as observed through **another** clone. Asserting silence is not enough — silence
is also what a no-op produces when nothing is being written. Assert a row committed AFTER the shutdown
is never delivered.

Mutation proof: give clones an independent handle → this test must FAIL.

### 2. Reachability gate (b) — the real HTTP seam

The run-level reachability gate requires at least one test driving the real entry point rather than a
mock past it, and it blocks closing this run. Task 017 covers the bus seam
(`commit → tailer → broadcast → subscribe_from`); task 015 covers write-site → journal. **Nothing
covers the HTTP entry point**, which is where the feature actually lives.

REQUIRED: one test that drives a real `GET /api/events` request against the wired deployment and
observes an event produced by a real state change — not a fabricated `SequencedEvent`, not a
hand-driven `sender`. If that cannot be expressed at this layer, say so explicitly and record where it
CAN be, because the run cannot be declared done without it.

## REQUIRED — added after panel-009b (2026-08-17): runner supervision + registration row

### 3. Supervised respawn for the trigger-hook runner

`run_hook` terminates permanently on ANY fallible operation — cursor load (`trigger_hooks.rs:112`),
MIN(seq) read (`:125`), flag write (`:135`), `subscribe_from` (`:139`), any mid-stream
`EventBusError` (`:143`), or a cursor write hitting e.g. SQLITE_BUSY (`:150`/`:153`) — and a
`let _ = run_hook(...)` spawn makes the death unobservable. The tailer performs the SAME journal
reads and retries forever (`event_bus/tailer.rs:99-107`); a dead runner pins the compaction soft
floor at its stale cursor. When spawning the runner here:

- Wrap it in a supervised loop: on `Err`, `error!` with hook name + error, back off (1s doubling to
  a 60s cap), respawn. Spawn the inner `run_hook` future as its own task and match on `JoinError`
  too, so a `fire()` panic is also caught and respawned rather than silently killing the loop.
- Each respawn re-reads the cursor AND `needs_rebootstrap` — this is what makes a flag raised by
  LIVE compaction observable without a process restart (task 009's runner reads the flag only at
  start; `clear_rebootstrap` is only called from its rebootstrap branch).
- Test: poison `trigger_cursors` with a `RAISE(ABORT)` BEFORE INSERT/UPDATE trigger (the task 009
  test-4 technique), let the runner die, drop the triggers, and assert the supervised loop resumes
  processing events WITHOUT a process restart.

### 4. Cursor row at registration

A hook with no `trigger_cursors` row contributes nothing to the compaction floor
(`trigger_cursor.rs:76-85`) and matches no row in compaction's flag UPDATE
(`event_journal/queries.rs:174`) — a brand-new hook mid-replay can have the journal deleted
underneath it and never be flagged. At registration, BEFORE spawning the runner, call
`trigger_cursor::ensure_row(&pool, hook.name())` (added by task 009's remediation). Do not use
`set()` here — it would overwrite an existing cursor.

## REQUIRED — added after panel-009c (2026-08-17): two 009-inherited obligations

1. **Cover `ensure_row`'s fresh-row insert path.** Task 009 added
   `trigger_cursor::ensure_row` but only its no-op-on-existing branch is tested, and nothing
   calls it yet. When this task wires `ensure_row` into registration, add a test asserting that
   registering a hook with NO existing cursor row creates the row at `last_processed_seq = 0`,
   `needs_rebootstrap = 0` — the untested half panel-009c flagged.
2. **Expect a FULL journal replay after a live-raised flag.** Because `set()` no longer clears
   `needs_rebootstrap` (F2 fix), a flag raised by compaction while a runner is live survives all
   of that runner's cursor writes; the NEXT start rewinds to `MIN(seq) - 1` and re-delivers every
   surviving event (panel-009c probe: journal `[1,2,3]`, cursor 3, flag 1 → refired `[1,2,3]`).
   This is the D11 at-least-once contract working as dictated, NOT a bug: `clear_rebootstrap`
   runs on that first start, so a supervised crash-loop does not replay repeatedly, and the
   momentary `MIN(seq)-1` floor deletes nothing. Hooks registered here must therefore be
   idempotent under full-journal redelivery; do not "fix" the rewind.

## REQUIRED — added with the 010 STOP resolution (2026-08-17): accessor contract

Task 010's route handler will call `deployment.event_bus()` on the concrete `LocalDeployment`
(`DeploymentImpl` alias, `crates/server/src/lib.rs:12`). Expose the bus as an INHERENT method:
`pub fn event_bus(&self) -> Arc<EventBus>` (clone of the startup-created handle). Do NOT add an
`event_bus()` method to the `Deployment` trait — `crates/deployment/src/lib.rs` is outside this
task's file-set, and test 1 (`deployment_exposes_an_event_bus`) must assert through the inherent
accessor.

## REQUIRED — added after panel-014 attempt 1 (2026-08-17): constructor seam

`LocalDeployment::new()` (lib.rs:115) takes NO arguments and builds its own `DBService` from
`database_path()` while writing config files, so the dictated "construct a deployment against a
test pool" was unsatisfiable from this task's file-set — the attempt-1 implementer substituted
standalone-component tests, leaving the wiring block itself with zero coverage. RESOLUTION
(orchestrator, STOP-class): split the constructor INSIDE this file. Extract everything after
DBService creation (bus construction, hook registration, runner spawn, compaction spawn, field
assembly) into an internal constructor taking the `DBService` (visibility `pub(crate)` or
`#[cfg(test)]` — implementer records the exact choice and signature in the ledger); `new()`
delegates to it. Tests construct the real deployment through this seam against
`db::test_utils::create_test_pool_with_migrations()`. Public API is otherwise unchanged; the
`Deployment` trait stays untouched.
