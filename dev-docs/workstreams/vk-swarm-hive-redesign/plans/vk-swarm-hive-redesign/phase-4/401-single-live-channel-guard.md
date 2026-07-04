---
id: "401"
phase: 4
title: TS5 acceptance — single inbound channel, one delete, one conflict (SC7)
status: done
depends_on: ["402", "403", "404"]
parallel: false
conflicts_with: ["402", "403"]
files:
  - crates/db/src/models/task/sync.rs
  - crates/services/src/services/node_runner.rs
irreversible: false
scope_test: "crates/db/src/models/task/sync.rs"
allowed_change: edit
covers_criteria: [SC7]
covers_tests: [TS5]
---
## Failing test (write first)
This is the consolidated **TS5** acceptance task for SC7. It asserts the three invariants tasks 402–404
establish, end-to-end at the seams users actually hit, and adds the "single live channel" topology guard.
The three TS5 assertions (spec `## Test strategy` TS5):

1. **One delete semantic regardless of leg.** A hive soft-delete yields ONE node outcome — soft-unlink +
   tombstone, local `task_attempt` RETAINED — whether applied via the WS leg (`&mut Transaction`) or the
   reconcile leg (`&SqlitePool`). Both now call the SAME `unlink_by_shared_task_id` (402); this test
   drives it through BOTH executor types and asserts identical state.
2. **Concurrent local edit not clobbered (dirty-guard).** With an unacked outbox op for a task, an
   inbound `upsert_remote_task` is SKIPPED (403).
3. **`task.reassigned` applied on the single channel** — routed through `process_task_upsert_event`
   (404), not dropped.

Add to the `#[cfg(test)] mod tests` block at the bottom of `crates/db/src/models/task/sync.rs` (it
already imports `CreateProject`, `Project`, `CreateTask`, `setup_test_pool`; `TaskAttempt` +
`CreateTaskAttempt` come from `crate::models::task_attempt`):

```rust
    #[tokio::test]
    async fn ts5_one_delete_outcome_both_legs_attempt_retained() {
        use crate::models::task_attempt::{CreateTaskAttempt, TaskAttempt};
        use executors::executors::BaseCodingAgent;

        // Helper: build a project + a hive-linked task that has a local task_attempt.
        async fn linked_task_with_attempt(pool: &sqlx::SqlitePool) -> (uuid::Uuid, uuid::Uuid) {
            let project_id = Uuid::new_v4();
            let project_data = CreateProject {
                name: "Test Project".to_string(),
                git_repo_path: format!("/tmp/test-repo-{}", project_id),
                use_existing_repo: true,
                clone_url: None,
                setup_script: None,
                dev_script: None,
                cleanup_script: None,
                copy_files: None,
            };
            Project::create(pool, &project_data, project_id).await.unwrap();
            let local_id = Uuid::new_v4();
            let shared_id = Uuid::new_v4();
            Task::create(pool, &CreateTask::from_title_description(project_id, "t".into(), None), local_id)
                .await.unwrap();
            Task::set_shared_task_id(pool, local_id, Some(shared_id)).await.unwrap();
            // TaskAttempt::create(pool, &data, attempt_id, task_id) — 4 args (see task_attempt.rs:870).
            TaskAttempt::create(
                pool,
                &CreateTaskAttempt {
                    executor: BaseCodingAgent::ClaudeCode,
                    base_branch: "main".into(),
                    branch: "vk/x".into(),
                    origin_node_id: None,
                },
                Uuid::new_v4(),
                local_id,
            ).await.unwrap();
            (local_id, shared_id)
        }

        // LEG A — reconcile path (&SqlitePool).
        let (pool, _t1) = setup_test_pool().await;
        let (local_a, shared_a) = linked_task_with_attempt(&pool).await;
        let n = Task::unlink_by_shared_task_id(&pool, shared_a).await.unwrap();
        assert_eq!(n, 1);

        // LEG B — WS path (&mut Transaction), SAME helper.
        let (local_b, shared_b) = linked_task_with_attempt(&pool).await;
        let mut tx = pool.begin().await.unwrap();
        let n = Task::unlink_by_shared_task_id(tx.as_mut(), shared_b).await.unwrap();
        assert_eq!(n, 1);
        tx.commit().await.unwrap();

        // IDENTICAL outcome on both legs: row retained, shared_task_id cleared, attempt retained.
        for local_id in [local_a, local_b] {
            let task = Task::find_by_id(&pool, local_id).await.unwrap();
            assert!(task.is_some(), "local task RETAINED (soft-unlink, not hard-delete)");
            assert!(task.unwrap().shared_task_id.is_none(), "shared_task_id cleared (tombstone)");
            let attempts = TaskAttempt::fetch_all(&pool, Some(local_id)).await.unwrap();
            assert_eq!(attempts.len(), 1, "local task_attempt RETAINED — node never loses work it ran");
        }
    }
```
> The dirty-guard (assertion 2) is proven by 403's
> `upsert_remote_task_skips_when_local_op_unacked`. `task.reassigned` (assertion 3) is verified by
> 404's manual-verification fallback (grep + compile — no runtime test harness exists for
> `ActivityProcessor`; 404 task L57-60 explicitly authorizes the fallback). TS5 is covered by THIS
> task's test (assertion 1, both-legs delete-equivalence) plus those assertions (one task claims
> `covers_tests: [TS5]`; the assertions live where their seams are). Do NOT duplicate them here.
>
> If `executors::executors::BaseCodingAgent` is not the import path used by the existing
> `task_attempt` tests, mirror whatever those tests use to construct a `CreateTaskAttempt`.

## Change

### 1. `crates/db/src/models/task/sync.rs` — the TS5 both-legs delete-equivalence test
- **File:** `crates/db/src/models/task/sync.rs`
- **Anchor:** the `#[cfg(test)] mod tests` block (~L790).
- **Before/After:** add ONLY the `ts5_one_delete_outcome_both_legs_attempt_retained` test above (and its
  `use` lines inside the test fn). No production code changes in this file.

### 2. `crates/services/src/services/node_runner.rs` — single-channel topology guard (comment fence)
- **File:** `crates/services/src/services/node_runner.rs`
- **Anchor:** the `HiveEvent::Connected` arm (~L671-680) where `sync_remote_projects` is invoked. This is
  the ONLY caller of the bulk-snapshot reconcile (verified: no periodic timer drives it — the only
  `tokio::time::sleep` is the reconnect backoff @1874; `spawn_hive_sync_service` @658 syncs
  attempts/executions/logs, NOT tasks). The guard is a comment fence asserting the invariant so a future
  edit that adds a periodic task re-sync is caught in review (ADR-0007: WS activity is the SINGLE live
  inbound channel; the snapshot is cold-start/gap-fill ONLY).
- **Before:**
```rust
                    // Sync remote projects into unified schema on connect
                    if let Some(ref client) = remote_client
                        && let Err(e) =
                            sync_remote_projects(&db.pool, client, organization_id, node_id).await
                    {
                        tracing::warn!(error = ?e, "Failed to sync remote projects on connect");
                    }
```
- **After:**
```rust
                    // ADR-0007 SINGLE LIVE INBOUND CHANNEL: the bulk-snapshot reconcile runs ONLY here,
                    // on (re)connect — it is cold-start / gap-fill, NOT a second continuous channel.
                    // The WS activity stream (project_watcher_task → ActivityProcessor::process_event)
                    // is the single LIVE inbound path. Do NOT call sync_remote_projects on a timer / in a
                    // periodic loop — that re-introduces the double-delivery class SC7 eliminates.
                    if let Some(ref client) = remote_client
                        && let Err(e) =
                            sync_remote_projects(&db.pool, client, organization_id, node_id).await
                    {
                        tracing::warn!(error = ?e, "Failed to sync remote projects on connect");
                    }
```

## Allowed moves
ONLY: add the one TS5 both-legs test to `sync.rs`; add the comment-fence block above the
`sync_remote_projects` call in `node_runner.rs`. Do NOT add a new behavior, a new periodic loop, or any
production logic change — 402/403/404 own the behavior; this task asserts + fences it. Do NOT duplicate
403's dirty-guard test or 404's reassigned test.

## STOP triggers
- `sync_remote_projects` is invoked from MORE than the `HiveEvent::Connected` arm (a second caller / a
  `tokio::spawn` loop / an interval) → STOP: the single-channel invariant is NOT met in code; this is a
  real SC7 violation to escalate, not a comment to paper over.
- `unlink_by_shared_task_id` is NOT executor-generic (402 not `passed`, or wrote it `pool`-only) → the
  LEG B tx call won't compile; depends_on: 402. STOP.
- `TaskAttempt::create` / `CreateTaskAttempt` fields differ from the cited shape → mirror the existing
  `task_attempt` tests' constructor; if `TaskAttempt` is not reachable from the `db` test module, STOP
  and report (the attempt-retention assertion is load-bearing for TS5).
- `TaskAttempt::fetch_all(pool, Some(task_id))` is not the by-task query → use whatever the
  `task_attempt` model exposes to count attempts for a task.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p db && cargo check -p services" WAI_TEST_CMD="cargo test -p db ts5_one_delete_outcome_both_legs" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 401` exits 0
(export `DATABASE_URL=sqlite://<repo>/dev_assets/db.sqlite` migrated through tasks 101 + 402's schema before running — Trap 2. `cargo check -p services` covers the `node_runner.rs` comment-fence edit.)
