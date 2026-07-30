---
id: "301"
phase: 3
title: "Add ProjectWithStats and GET /api/projects/with-stats (additive — MergedProject untouched)"
status: ready
depends_on: []
parallel: false
conflicts_with: ["303"]
files:
  - crates/server/src/routes/projects/handlers/with_stats.rs
  - crates/server/src/routes/projects/types.rs
  - crates/server/src/routes/projects/handlers/mod.rs
  - crates/server/src/routes/projects/mod.rs
  - crates/server/src/bin/generate_types.rs
  - shared/types.ts
siblings:
  - crates/server/src/routes/projects/handlers/merged.rs
irreversible: false
scope_test: "N/A"
allowed_change: mixed
covers_criteria: [SC5]
---

## Failing test (write first)

N/A at this step — additive Rust endpoint, no unit-test seam in this crate. The behavioural
assertion (enrichment survives, ordering preserved) is made in task 302 against the rendered
board, and over HTTP in Manual verification below.

## Sibling alignment (required reading before you write)

Read `crates/server/src/routes/projects/handlers/merged.rs` in full. The new handler is a copy of
its structure minus three hardcoded fields. Note every choice it makes — that it calls
`Project::find_local_projects_with_stats(pool)`, that it maps into the response struct
field-by-field, and that it sorts by `name.to_lowercase()` before responding. Reproduce all of
them. **Any divergence must be recorded in the decisions-ledger.**

## Change

### 1. Add `ProjectWithStats` to `crates/server/src/routes/projects/types.rs`

- **Anchor:** immediately after the `MergedProjectsResponse` struct (around line 179)
- **After:** append:

```rust
/// A local project plus the display enrichment the board needs.
/// Replaces `MergedProject`, whose merge fields are dead (see ADR-0014).
#[derive(Debug, Clone, Serialize, TS)]
pub struct ProjectWithStats {
    pub id: Uuid,
    pub name: String,
    pub git_repo_path: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,

    /// Linking status - Hive project ID (if linked)
    pub remote_project_id: Option<Uuid>,

    /// For sorting - timestamp of last task attempt
    #[ts(type = "Date | null")]
    pub last_attempt_at: Option<DateTime<Utc>>,

    /// GitHub integration fields
    pub github_enabled: bool,
    pub github_owner: Option<String>,
    pub github_repo: Option<String>,
    pub github_open_issues: i32,
    pub github_open_prs: i32,
    #[ts(type = "Date | null")]
    pub github_last_synced_at: Option<DateTime<Utc>>,

    /// Task status counts for quick display
    pub task_counts: TaskCounts,
}

/// Response for the projects-with-stats endpoint
#[derive(Debug, Clone, Serialize, TS)]
pub struct ProjectsWithStatsResponse {
    pub projects: Vec<ProjectWithStats>,
}
```

> **Field types are copied from `MergedProject` exactly** (note `github_open_issues: i32` and
> `github_open_prs: i32` — the spec's illustrative sketch shows `Option<i64>`; the spec's
> governing sentence is "identical to today's `MergedProject` minus `nodes`, `has_local`, and
> `local_project_id`", so the real field types win). Record this in the decisions-ledger.

### 2. Create `crates/server/src/routes/projects/handlers/with_stats.rs`

Copy `merged.rs` and drop the three dead fields. The file:

```rust
use axum::{extract::State, response::Json as ResponseJson};
use utils::response::ApiResponse;

use crate::{
    DeploymentImpl,
    error::ApiError,
    routes::projects::types::{ProjectWithStats, ProjectsWithStatsResponse, TaskCounts},
};

/// List local projects with their display enrichment (task counts, last attempt, GitHub counts).
pub async fn get_projects_with_stats(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<ProjectsWithStatsResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    let local_projects_with_stats = Project::find_local_projects_with_stats(pool).await?;

    let mut projects: Vec<ProjectWithStats> = local_projects_with_stats
        .into_iter()
        .map(|stats| {
            let project = stats.project;
            ProjectWithStats {
                id: project.id,
                name: project.name,
                git_repo_path: project.git_repo_path.to_string_lossy().to_string(),
                created_at: project.created_at,
                remote_project_id: project.remote_project_id,
                last_attempt_at: stats.last_attempt_at,
                github_enabled: project.github_enabled,
                github_owner: project.github_owner,
                github_repo: project.github_repo,
                github_open_issues: project.github_open_issues,
                github_open_prs: project.github_open_prs,
                github_last_synced_at: project.github_last_synced_at,
                task_counts: TaskCounts {
                    todo: stats.task_counts.todo,
                    in_progress: stats.task_counts.in_progress,
                    in_review: stats.task_counts.in_review,
                    done: stats.task_counts.done,
                },
            }
        })
        .collect();

    projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(ResponseJson(ApiResponse::success(ProjectsWithStatsResponse {
        projects,
    })))
}
```

Copy the `use` lines for `Project` (and anything else) from `merged.rs` verbatim — it is the
authority on the exact import paths.

### 3. Export the handler — `crates/server/src/routes/projects/handlers/mod.rs`

- **Anchor:** the module declarations and the `pub use merged::get_merged_projects;` line
- **Change:** add `pub mod with_stats;` alongside the other module declarations, and
  `pub use with_stats::get_projects_with_stats;` immediately after the `merged` re-export.

### 4. Register the route — `crates/server/src/routes/projects/mod.rs`

- **Anchor:** the `projects_router` builder, the `.route("/scan-config", ...)` line (~line 134)
- **Before:**
```rust
        .route("/", get(get_projects).post(create_project))
        .route("/scan-config", post(scan_project_config))
```
- **After:**
```rust
        .route("/", get(get_projects).post(create_project))
        .route("/with-stats", get(get_projects_with_stats))
        .route("/scan-config", post(scan_project_config))
```

Also add `get_projects_with_stats` to the handler import list at the top of the file, and
`ProjectWithStats, ProjectsWithStatsResponse` to the `pub use types::{...}` re-export block.

### 5. Register the ts-rs exports — `crates/server/src/bin/generate_types.rs`

- **Anchor:** line 30-33, next to the `MergedProject` / `MergedProjectsResponse` decls
- **Change:** add `server::routes::projects::ProjectWithStats::decl(),` and
  `server::routes::projects::ProjectsWithStatsResponse::decl(),` in the same list.

### 6. Regenerate `shared/types.ts`

```bash
npm run generate-types
```

Never hand-edit `shared/types.ts` (constraint C5).

## Allowed moves

- Create the handler file; add the two structs, the two exports, the one route line, the two
  `decl()` lines; run `npm run generate-types`.
- Do **not** modify or delete `merged.rs`, `MergedProject`, or `/merged-projects` — task 303
  owns that. This task is purely additive so the board is never broken between tasks.

## STOP triggers

- If `Project::find_local_projects_with_stats` does not exist at
  `crates/db/src/models/project/stats.rs` — STOP.
- If `/with-stats` shadows or is shadowed by the `.nest("/{id}", project_id_router)` route (a
  request to `/api/projects/with-stats` reaching the per-project handler). The existing
  `/scan-config`, `/link-local`, and `/orphaned` static siblings prove static-before-dynamic
  matching works here — if it does NOT, STOP rather than renaming the route.
- If `npm run generate-types` produces a diff in `shared/types.ts` touching any type other than
  the two new ones — STOP and report the unexpected drift.

## Manual verification (record in decisions-ledger)

```bash
cargo clippy -p server --all-targets --all-features -- -D warnings
# Expected: clean

npm run generate-types:check
# Expected: passes (no pending regeneration)

git diff --stat shared/types.ts
# Expected: additions only, for ProjectWithStats and ProjectsWithStatsResponse

# With the dev server running (PORT = reported BACKEND_PORT):
curl -s "http://127.0.0.1:${PORT}/api/projects/with-stats" | head -c 400
# Expected: {"success":true,...} with a projects array; each entry carries task_counts
#           and last_attempt_at, and NO nodes/has_local/local_project_id fields

curl -s "http://127.0.0.1:${PORT}/api/merged-projects" | head -c 200
# Expected: still {"success":true,...} — this task must not have broken the old endpoint
```

## Done when

- `GET /api/projects/with-stats` returns the enriched, name-sorted local project list.
- `ProjectWithStats` and `ProjectsWithStatsResponse` appear in `shared/types.ts` via codegen.
- `/api/merged-projects` still works (deletion is task 303's job).
- Clippy and `generate-types:check` are clean.
