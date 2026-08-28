---
id: "001"
phase: 1
title: "Extend HiveHarness with delete(), seed_shared_task(), task_row_exists()"
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - "crates/server/tests/common/mod.rs"
siblings: []
irreversible: false
scope_test: "crates/server/tests/common"
allowed_change: edit
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
N/A — covered by existing tests: crates/server/tests/harness_smoke.rs (harness still compiles and passes) plus task 002's new tests which exercise all three helpers.


## Change
**File:** crates/server/tests/common/mod.rs
**Anchor:** inside the `#[allow(dead_code)] impl HiveHarness` block, immediately AFTER the closing brace of `pub async fn post(&self, path: &str, body: serde_json::Value) -> Resp { ... }` and BEFORE the doc comment of `pub async fn seed_project`.
**Before:** (insertion point — the blank line between `post`'s closing `}` and the `/// Seed a local project plus one task per entry...` doc comment)
**After:** insert exactly:

```rust
    /// DELETE against the REAL served router over HTTP
    pub async fn delete(&self, path: &str) -> Resp {
        let client = reqwest::Client::new();
        let res = client
            .delete(format!("http://{}{}", self.addr, path))
            .send()
            .await
            .unwrap();
        let status = res.status().as_u16();
        let content_type = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = res.text().await.unwrap();
        Resp {
            status,
            body,
            content_type,
        }
    }

    /// Seed one task carrying a `shared_task_id` under an existing project, inserted through
    /// the deployment's own pool (migrations already applied — never hand-written DDL).
    pub async fn seed_shared_task(&self, project_id: Uuid, shared_task_id: Uuid) -> Uuid {
        let pool = &self.deployment.db().pool;
        let task_id = Uuid::new_v4();
        let create_task = CreateTask {
            project_id,
            title: format!("shared-task-{task_id}"),
            description: None,
            status: Some(TaskStatus::Todo),
            parent_task_id: None,
            image_ids: None,
            shared_task_id: Some(shared_task_id),
        };
        Task::create(pool, &create_task, task_id)
            .await
            .expect("failed to seed shared task");
        task_id
    }

    /// True when the task row still exists in the node DB.
    pub async fn task_row_exists(&self, task_id: Uuid) -> bool {
        Task::find_by_id(&self.deployment.db().pool, task_id)
            .await
            .expect("task lookup failed")
            .is_some()
    }
```

All referenced items (`CreateTask`, `Task`, `TaskStatus`, `Uuid`, `reqwest`, `Resp`) are already imported at the top of the file — do NOT add imports.


## Allowed moves
ONLY insert the three methods shown, verbatim, inside the existing `impl HiveHarness` block at the stated anchor. Do not modify `configured()`, `hive_absent()`, `mock_json`, `get`, `post`, `seed_project`, `test_access_token`, `Resp`, or any import line.


## STOP triggers
- `pub async fn post` or `pub async fn seed_project` not found where stated in crates/server/tests/common/mod.rs
- `CreateTask` lacks a `shared_task_id` field or `Task::find_by_id` does not exist (compile error)
- any change needed to a file other than crates/server/tests/common/mod.rs


## Manual verification (record in decisions-ledger)



## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash ~/.claude/wai/scripts/task-gate.sh node-task-delete-dangling-shared-id 001` exits 0
