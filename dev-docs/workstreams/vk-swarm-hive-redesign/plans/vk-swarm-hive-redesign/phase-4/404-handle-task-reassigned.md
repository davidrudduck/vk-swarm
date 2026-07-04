---
id: "404"
phase: 4
title: Handle task.reassigned in the activity processor — no dropped event types
status: done
depends_on: []
parallel: false
conflicts_with: ["402"]
files:
  - crates/services/src/services/share/processor.rs
irreversible: false
scope_test: "crates/services/src/services/share/processor.rs"
allowed_change: edit
covers_criteria: [SC7]
covers_tests: []
---
## Failing test (write first)
The bug (ADR-0007 §No dropped event types): `process_event` (`processor.rs:57`) drops
`task.reassigned` into the `_ =>` "Ignoring unknown event type" arm (`processor.rs:77`), so a
reassignment lands ONLY via the bulk-snapshot reconcile — a hard channel dependency the inbound-collapse
removes. The hive emits `task.reassigned` with the IDENTICAL `SharedTaskActivityPayload { task, user }`
payload as `task.updated` (`crates/remote/src/db/tasks.rs:982` → `insert_activity` builds the same
payload), so the fix is to route `task.reassigned` through the SAME `process_task_upsert_event` handler
— the new assignee is just a field on the task.

Add a `#[cfg(test)] mod tests` test in `crates/services/src/services/share/processor.rs` (if no test
module exists, add one). It drives a `task.reassigned` `ActivityEvent` through `process_event` and
asserts the local task's `remote_assignee_user_id` is updated (not ignored):

```rust
    #[tokio::test]
    async fn process_event_applies_task_reassigned() {
        // Arrange: a processor over a hermetic pool, a local project linked to a remote project,
        // and a local task linked to a shared task. (Reuse this module's existing processor/test
        // harness — see the task.updated / task.deleted tests already present; mirror their setup.)
        // ...build `processor`, `local_project` (remote_project_id = R), `local_task` (shared_task_id = S)...

        let new_assignee = uuid::Uuid::new_v4();
        let event = make_task_activity_event(
            "task.reassigned",
            /* shared task id */ S,
            /* remote project id */ R,
            /* version */ 2,
            /* assignee_user_id */ Some(new_assignee),
        );

        processor.process_event(event).await.unwrap();

        let after = Task::find_by_shared_task_id(&processor.db.pool, S).await.unwrap().unwrap();
        assert_eq!(
            after.remote_assignee_user_id, Some(new_assignee),
            "task.reassigned is APPLIED (routed through process_task_upsert_event), not dropped",
        );
        assert_eq!(after.remote_version, 2);
    }
```
> Mirror the EXISTING `task.updated`/`task.deleted` tests in this module for the harness (`processor`
> construction, `SharedTaskActivityPayload` JSON builder, project/task linking). If no such test exists,
> the `## Manual verification` fallback below applies and `scope_test` is the compile gate — but PREFER
> the unit test using the module's own helpers. Do NOT invent a new test-DB pattern.

## Change

### `crates/services/src/services/share/processor.rs` — route task.reassigned to the upsert handler
- **File:** `crates/services/src/services/share/processor.rs`
- **Anchor:** `process_event` (L57), the match on `event.event_type.as_str()`, the
  `"task.created" | "task.updated"` arm (~L67-69) — directly above the `"task.deleted"` arm.
- **Before:**
```rust
            // Task events - sync version and metadata to keep local cache fresh
            "task.created" | "task.updated" => {
                self.process_task_upsert_event(&mut tx, &event).await?
            }
```
- **After:**
```rust
            // Task events - sync version and metadata to keep local cache fresh.
            // `task.reassigned` carries the same SharedTaskActivityPayload { task, user } as
            // `task.updated` (the new assignee is a field on `task`), so it routes through the same
            // upsert handler — no dropped event type (ADR-0007). It must NOT fall through to the `_` arm.
            "task.created" | "task.updated" | "task.reassigned" => {
                self.process_task_upsert_event(&mut tx, &event).await?
            }
```

## Allowed moves
ONLY: add `"task.reassigned"` to the existing `task.created | task.updated` match arm (and its comment);
add the `task.reassigned` unit test. Do NOT add a separate handler fn (the payload is identical — a new
fn would duplicate `process_task_upsert_event`). Do NOT touch the `task.deleted` arm, the `_ =>` arm, or
the cursor-upsert tail.

## STOP triggers
- The `task.created" | "task.updated"` arm is not at the cited anchor / its body is not
  `process_task_upsert_event` → re-locate via
  `grep -n "task.created\|task.updated\|process_task_upsert_event" crates/services/src/services/share/processor.rs`;
  if the dispatch shape changed, STOP.
- The hive payload for `task.reassigned` is NOT `SharedTaskActivityPayload` (verify
  `crates/remote/src/db/tasks.rs` `insert_activity` still builds `{ task, user }` for the reassign path,
  ~L982) → if reassigned grew a distinct payload type, STOP (a separate handler is then needed; escalate).
- `process_task_upsert_event` early-returns on a missing project link (`processor.rs:339`) — that is
  CORRECT (transient: the project isn't linked yet); the test must link the project first, as the
  existing `task.updated` test does.

## Manual verification (record in decisions-ledger)
If this module has no unit-test harness to reuse and the unit test cannot be authored without inventing a
new pattern, the gate's compile + the following hand-check suffice (record output in the ledger):
`grep -n 'task.reassigned' crates/services/src/services/share/processor.rs` shows it in the
`process_task_upsert_event` arm and NOT in any `_ =>`/"Ignoring unknown event type" arm; and
`cargo check -p services` is clean. PREFER the unit test — this fallback is only if no harness exists.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p services" WAI_TEST_CMD="cargo test -p services task_reassigned" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 404` exits 0
(export `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` migrated dev DB before running — Trap 2.)
