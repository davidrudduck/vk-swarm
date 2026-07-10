# AGENTS.md - Development Guide & Agent Instructions

This is the canonical development guide for the Vibe Kanban codebase. All AI agents
(Claude Code, Codex, Gemini, etc.) must read and enforce these rules.

## 1. Core Principles

- **Type Safety First**: All data structures must be typed. Use Rust's type system and TypeScript strict mode.
- **Single Source of Truth**: Rust types are authoritative. Run `npm run generate-types` after modifying Rust structs.
- **Error Transparency**: Use `thiserror` with `#[from]` for error propagation. Never swallow errors silently.
- **Stateless Services**: Service structs should be stateless and `Clone`. Pass dependencies via function parameters.
- **UUID Identifiers**: All entities use UUID v4 for primary keys.
- **UTC Timestamps**: All timestamps stored as `DateTime<Utc>` (Rust) and ISO 8601 strings (JSON/SQLite).
- **GitHub Targeting**: Open pull requests only against `davidrudduck/vk-swarm`. Do NOT open PRs against `BloopAI/vibe-kanban` from this workspace.
- **Finish What We Start**: A PR must leave all validation checks green (see §9 Validation). No CI-breaking or half-implemented debt is deferred to the next session. Legitimate scope splits (entangled features, approved `vk-swarm-node-ui-localize`-style workstreams) are the only exception — they must be explicitly named, tracked in a follow-up workstream, and documented in the decisions-ledger before the PR is submitted.
- **No Deferred Remediation**: Code review findings (from adversarial panels, Gemini review, `/dr:pr`, the WAI reachability gate, or any other review step) must be fixed in the **same session** before the PR is pushed or merged. "Fix in the next session" is not an option. If a finding is determined to be a false positive, document why in the decisions-ledger (with the specific grep/file:line evidence that disproves it). If it is real, fix it now. **Pre-existing debt discovered during a session** (tests/lint/typecheck red on the baseline before the session's changes) is likewise NOT carried forward silently: fix it now, split it as a legitimate named scope split with a tracked follow-up workstream created in THIS session, or escalate to the user. A remediation prompt written for the next session does not satisfy this rule — "we finish what we start" means the next session inherits a clean ledger, not a backlog. Globally disabling a quality gate, linter, or entire test category via configuration (e.g. `doctest = false` in `Cargo.toml`) to bypass compilation or execution errors is itself a **silent deferral** and is prohibited unless paired with a tracked follow-up workstream created in THIS session or explicit user approval. Broken tests or documentation examples must be resolved at the source level — fixed, or selectively marked with the standard per-item attributes (e.g. `#[ignore]`, `rust,ignore`, `no_run`) so the remaining tests in the category continue to run and catch regressions. Per-item `#[ignore]`/`rust,ignore` markers are legitimate PROVIDED (a) at least one test in the category remains live, AND (b) a tracked follow-up workstream (`dev-docs/workstreams/<name>/README.md`) documents which remain ignored and what's needed to bring them live. When a test requires a populated SQLite DB, use `db::test_utils::create_test_pool()` (fast) or `create_test_pool_with_migrations()` (full schema); never hand-write `CREATE TABLE` in test helpers (schema drift → false greens).

### Pre-existing debt discovered during a session (no carry-forward)

When a session discovers pre-existing failures (tests, lint, typecheck, or any gate red on the baseline before the session's changes) — whether surfaced by a review panel, the mandatory gate, or ad-hoc investigation — they are **not** silently handed to the next session. The session that finds them MUST do one of the following before it ends:

1. **Fix now** — remediate the pre-existing failure in this session, even if it falls outside the session's primary workstream; OR
2. **Split as a legitimate scope split** — explicitly named, with a tracked follow-up workstream (`dev-docs/workstreams/<name>/README.md`) created in THIS session, and documented in the decisions-ledger before the PR is submitted; OR
3. **Escalate to the user** — if the fix is architecturally entangled or requires a decision the agent cannot make.

A remediation prompt written for "the next session" does NOT satisfy this rule. "We finish what we start" means the debt is resolved (fixed, split, or escalated) before the session closes — never carried forward silently. The next session must inherit a clean ledger, not a backlog of "fix this later" notes.

Globally disabling a quality gate, linter, or entire test category via configuration (e.g. `doctest = false` in `Cargo.toml`, `#[cfg_attr(..., skip)]` on a whole module, or removing a test from the workspace) to bypass compilation or execution errors is itself a **silent deferral** and is prohibited unless paired with a tracked follow-up workstream created in THIS session or explicit user approval. Broken tests or documentation examples must be resolved at the source level — fixed, or selectively marked with the standard per-item attributes (e.g. `#[ignore]`, `rust,ignore`, `no_run`) so the remaining tests in the category continue to run and catch regressions.

Per-item `#[ignore]` or `rust,ignore` markers are the sanctioned source-level path for tests that cannot currently run (e.g., requiring a live database, network endpoint, or PTY device unavailable in CI). Their use is legitimate PROVIDED the session either: (a) makes at least one test in the category live so the suite is not entirely dead; AND (b) creates a tracked follow-up workstream (`dev-docs/workstreams/<name>/README.md`) documenting which tests remain ignored and what is required to bring them live. Marking tests ignored without (b) is a deferred deferral — it satisfies the letter of "source-level per-item attribute" while violating the spirit of "clean ledger."

## 2. Tech Stack

### Backend
- **Language**: Rust (Edition 2024)
- **Framework**: Axum 0.8 with Tokio async runtime
- **Database**: SQLite with SQLx (compile-time checked queries)
- **Error Handling**: `thiserror` for custom errors, `anyhow` for ad-hoc errors
- **Type Generation**: `ts-rs` for Rust → TypeScript
- **Logging**: `tracing` crate with structured logging
- **Linting**: Clippy with `-D warnings`
- **Formatting**: `cargo fmt`

### Frontend
- **Framework**: React 18 with TypeScript (strict mode)
- **Build Tool**: Vite 5
- **Styling**: Tailwind CSS + shadcn/ui components
- **State**: React Context API + TanStack React Query
- **i18n**: i18next (en, ja, ko, es)
- **Linting**: ESLint 8.55
- **Formatting**: Prettier

### Package Management
- **Node**: pnpm 10.x (required)
- **Rust**: Cargo workspace

## 3. Architecture

### Backend Structure (`crates/`)
```text
server/           # Axum HTTP server, routes, MCP server
├── routes/       # HTTP handlers (directory modules for complex domains)
│   ├── task_attempts/  # Directory module with handlers/, types.rs
│   ├── projects/       # Directory module with handlers/, types.rs
│   └── *.rs            # Simple routes (single file per domain)
├── mcp/          # Model Protocol server
├── middleware/   # Auth, request handling
└── error.rs      # Global API error types

db/               # Database layer
├── models/       # SQLx models (directory modules for complex models)
│   ├── task/           # Directory module: queries.rs, archive.rs, sync.rs, hierarchy.rs
│   ├── project/        # Directory module: queries.rs, github.rs, stats.rs, sync.rs
│   ├── execution_process/  # Directory module: queries.rs, lifecycle.rs, sync.rs
│   ├── log_entry/      # Directory module: queries.rs, pagination.rs, sync.rs
│   └── *.rs            # Simple models (single file)
└── migrations/   # SQL migration files

services/         # Business logic (stateless)
├── git.rs        # Git operations
├── github.rs     # GitHub API integration
└── worktree.rs   # Worktree management

executors/        # AI agent integrations
├── logs/         # Log processing
│   ├── normalizer.rs  # LogNormalizer trait and driver function
│   ├── tool_states.rs # Shared tool state structures
│   └── *.rs           # Executor-specific implementations
└── *.rs          # Executor implementations (ACP, Droid, Codex)

utils/            # Shared utilities, ApiResponse
```

### Frontend Structure (`frontend/src/`)
```text
components/       # Organized by domain
├── projects/     # ProjectCard, ProjectForm, etc.
├── tasks/        # TaskCard, TaskList, etc.
└── ui/           # shadcn/ui base components

pages/            # Route pages (one file per route)
hooks/            # Custom hooks (60+ hooks by domain)
contexts/         # React context providers
lib/              # API client, utilities
├── api.ts        # API namespace objects
└── utils.ts      # Helper functions
```

### Data Flow
```text
Frontend Component
    ↓ (API call)
lib/api.ts (fetch wrapper)
    ↓ (HTTP)
routes/*.rs (Axum handlers)
    ↓ (business logic)
services/*.rs (stateless services)
    ↓ (data access)
db/models/*.rs (SQLx queries)
    ↓
SQLite Database
```

## 4. Code Style

### Rust Naming
```rust
// Enums: PascalCase, serialize as snake_case
#[derive(Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    InReview,
    Done,
    Cancelled,
}

// Structs: PascalCase
pub struct Project { ... }
pub struct CreateProject { ... }  // Request types
pub struct ProjectError { ... }   // Error types

// Functions: snake_case
pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Project, sqlx::Error>
pub async fn get_projects(State(deployment): State<DeploymentImpl>) -> Result<...>
```

### TypeScript Naming
```typescript
// Components: PascalCase files and exports
// File: ProjectCard.tsx
function ProjectCard({ project, isFocused }: Props) { ... }
export default ProjectCard;

// Hooks: camelCase with 'use' prefix
// File: useProjectMutations.ts
export function useProjectMutations(options?: Options) { ... }

// Types: PascalCase
type Props = { project: Project; isFocused: boolean; };
interface UserSystemContextType { ... }

// Variables/functions: camelCase
const [loading, setLoading] = useState(true);
const fetchProjects = async () => { ... };
```

### Callbacks Convention
```typescript
// Options interfaces use on{Action}, on{Action}Success, on{Action}Error
interface UseProjectMutationsOptions {
  onCreateSuccess?: (project: Project) => void;
  onCreateError?: (err: unknown) => void;
  onUpdateSuccess?: (project: Project) => void;
  onUpdateError?: (err: unknown) => void;
}
```

## 5. Logging

### Backend (Rust)
```rust
use tracing::{info, warn, error, debug};

// Structured logging with context
info!(project_id = %id, "Project created successfully");
warn!(attempt = %attempt_id, "Retrying failed operation");
error!(error = ?e, "Failed to connect to database");
debug!(payload = ?request, "Received API request");

// Environment: RUST_LOG=info (default), RUST_LOG=debug for verbose
```

### Frontend (TypeScript)
```typescript
// Use console methods with context
console.error('Failed to create project:', error);
console.warn('Retrying operation', { attempt, maxRetries });

// In catch blocks, always log before user feedback
try {
  await projectsApi.create(data);
} catch (error) {
  console.error('Failed to create project:', error);
  setError('Failed to create project');
}
```

## 6. Testing

### Backend Tests
```bash
cargo test --workspace               # All tests
cargo test -p services              # Specific crate
cargo test test_git_operation       # Specific test
```

```rust
// Integration tests in crates/*/tests/
use tempfile::TempDir;

#[test]
fn test_git_operation() {
    let root = TempDir::new().unwrap();
    // Setup test environment
    // Assertions
}

// Unit tests colocated in src/
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_helper_function() { ... }
}
```

### Database Test Utilities

For database tests, use the shared test pool utilities in `crates/db/src/test_utils.rs`:

```rust
use db::test_utils::create_test_pool;

#[tokio::test]
async fn test_db_operation() {
    let (pool, _temp_dir) = create_test_pool().await;
    // The pool has migrations already applied via template database
    // Much faster than running migrations per test
}
```

**Benefits:**
- Template database approach: migrations run once, then copied for each test
- ~90% faster than running migrations per test
- Full test isolation (each test gets its own database copy)

### Testing Standards

When a test requires a populated SQLite database, use one of:
- `db::test_utils::create_test_pool()` — fast template-copy approach; prefer for the majority of tests.
- `db::test_utils::create_test_pool_with_migrations()` — fresh migrations per test; use when the test exercises migration behavior itself or needs the full production schema.

Never manually duplicate `CREATE TABLE` SQL in test helpers (e.g., `setup_db()`-style functions). Schema defined outside `crates/db/migrations/` will drift from production and produce false-green tests that mask real regressions.

## 7. API Contracts

### Response Wrapper
All endpoints return `ApiResponse<T>`:
```rust
// Backend
pub struct ApiResponse<T, E = T> {
    success: bool,
    data: Option<T>,
    error_data: Option<E>,
    message: Option<String>,
}

// Usage in handlers
Ok(ResponseJson(ApiResponse::success(project)))
```

```typescript
// Frontend unwrapping
const result: ApiResponse<Project> = await response.json();
if (!response.ok || !result.success) {
  throw new Error(result.message || 'Failed to fetch');
}
return result.data!;
```

### Type Synchronization
```rust
// Rust model (source of truth)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}
```

```typescript
// Auto-generated in shared/types.ts (DO NOT EDIT)
export type Project = {
  id: string;
  name: string;
  created_at: Date;
};
```

After modifying Rust types: `npm run generate-types`

## 8. Common Patterns

### Backend: Route Handler
```rust
pub async fn create_project(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateProject>,
) -> Result<ResponseJson<ApiResponse<Project>>, ApiError> {
    // Validate input
    if payload.name.is_empty() {
        return Err(ApiError::BadRequest("Name is required".into()));
    }

    // Business logic via services
    let project = ProjectService::create(&deployment.db().pool, payload).await?;

    // Return wrapped response
    Ok(ResponseJson(ApiResponse::success(project)))
}
```

### Backend: Database Model
```rust
impl Project {
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"SELECT id as "id: Uuid", name, created_at as "created_at: DateTime<Utc>"
               FROM projects WHERE id = ?"#,
            id
        )
        .fetch_optional(pool)
        .await
    }
}
```

### Frontend: API Client
```typescript
// lib/api.ts - Namespace pattern
export const projectsApi = {
  async list(): Promise<Project[]> {
    const response = await fetch('/api/projects');
    const result: ApiResponse<Project[]> = await response.json();
    if (!response.ok || !result.success) {
      throw new Error(result.message || 'Failed to fetch projects');
    }
    return result.data || [];
  },

  async create(data: CreateProject): Promise<Project> {
    const response = await fetch('/api/projects', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    const result: ApiResponse<Project> = await response.json();
    if (!response.ok || !result.success) {
      throw new Error(result.message || 'Failed to create project');
    }
    return result.data!;
  },
};
```

### Frontend: Custom Hook with Mutations
```typescript
export function useProjectMutations(options?: UseProjectMutationsOptions) {
  const queryClient = useQueryClient();

  const createProject = useMutation({
    mutationKey: ['createProject'],
    mutationFn: (data: CreateProject) => projectsApi.create(data),
    onSuccess: (project: Project) => {
      queryClient.invalidateQueries({ queryKey: ['projects'] });
      options?.onCreateSuccess?.(project);
    },
    onError: (err) => {
      console.error('Failed to create project:', err);
      options?.onCreateError?.(err);
    },
  });

  return { createProject };
}
```

### Frontend: Component Structure
```typescript
type Props = {
  project: Project;
  onEdit: (project: Project) => void;
};

function ProjectCard({ project, onEdit }: Props) {
  const navigate = useNavigateWithSearch();
  const { deleteProject } = useProjectMutations({
    onDeleteSuccess: () => navigate('/'),
  });

  const handleDelete = async () => {
    if (!confirm('Are you sure?')) return;
    deleteProject.mutate(project.id);
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>{project.name}</CardTitle>
      </CardHeader>
      <CardContent>
        <Button onClick={() => onEdit(project)}>Edit</Button>
        <Button variant="destructive" onClick={handleDelete}>Delete</Button>
      </CardContent>
    </Card>
  );
}

export default ProjectCard;
```

### Backend: Route Handler Directory Module
For complex route domains with many handlers, use a directory module structure:

```text
routes/task_attempts/
├── mod.rs           # Router and submodule declarations
├── types.rs         # Request/response types for handlers
└── handlers/
    ├── mod.rs       # Re-exports all handlers
    ├── core.rs      # CRUD and basic operations
    ├── git_ops.rs   # Git-related operations
    ├── github.rs    # GitHub integration
    └── worktree.rs  # Worktree file operations
```

```rust
// handlers/mod.rs - Organize by concern and re-export
pub mod core;
pub mod git_ops;
pub mod github;
pub mod worktree;

// Re-export all handlers for convenient access from the router
pub use core::{create_task_attempt, get_task_attempt, stop_task_attempt};
pub use git_ops::{merge_task_attempt, rebase_task_attempt, push_task_attempt_branch};
pub use github::{create_github_pr, attach_existing_pr};
pub use worktree::{list_worktree_files, read_worktree_file};
```

### Backend: Model Directory Module
For complex models with many queries, use a directory module structure:

```text
models/task/
├── mod.rs           # Struct definition, types, tests, submodule declarations
├── queries.rs       # CRUD operations (find_by_id, create, update, delete)
├── archive.rs       # Archiving operations
├── hierarchy.rs     # Parent/child relationships
└── sync.rs          # Hive synchronization queries
```

```rust
// mod.rs - Struct definition with submodule declarations
mod archive;
mod hierarchy;
mod queries;
mod sync;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    // ...
}

// Re-export commonly used items
pub use archive::{archive, unarchive};
pub use queries::{find_by_id, create, update, delete};
```

### Backend: LogNormalizer Trait
For processing executor logs, implement the `LogNormalizer` trait:

```rust
use crate::logs::{LogNormalizer, normalize_logs_with};

pub struct MyExecutorNormalizer {
    // Executor-specific state
}

impl LogNormalizer for MyExecutorNormalizer {
    type Event = MyEventType;

    fn parse_line(&self, line: &str) -> Option<Self::Event> {
        // Parse executor-specific log format
    }

    fn extract_session_id(&self, event: &Self::Event) -> Option<String> {
        // Extract session ID from event
    }

    fn process_event(
        &mut self,
        event: Self::Event,
        msg_store: &Arc<MsgStore>,
        entry_index: &EntryIndexProvider,
    ) -> Vec<Patch> {
        // Process event and return conversation patches
    }
}

// Usage in executor
let normalizer = MyExecutorNormalizer::new();
let handle = normalize_logs_with(normalizer, msg_store.clone(), &worktree_path);
```

## 9. Development Commands

### Setup
```bash
pnpm install                          # Install all dependencies
./init.sh                             # Full setup: prerequisites, .env, database, dependencies
./init.sh start                       # Start servers after setup
```

**Note on init.sh (January 2026)**: The init.sh script has been significantly enhanced to support isolated development in worktrees. Key improvements include:
- **Respect existing .env values**: Loads FRONTEND_PORT, BACKEND_PORT, and MCP_PORT from .env if present before auto-assigning ports
- **Database isolation**: Copies from ~/.vkswarm/db/test.sqlite or production database to local dev_assets/db.sqlite for worktree-local development
- **Port conflict detection**: Validates port availability before starting servers and provides clear error messages
- **Integrated stop command**: Uses pnpm run stop for graceful shutdown (see "Stopping the Server" section)
- **Status checking**: ./init.sh status shows running instances and port usage

This approach prevents worktree conflicts when multiple developers or AI agents work on different tasks simultaneously, each with isolated databases and ports.

### Development
```bash
pnpm run dev                          # Start frontend + backend (hot reload)
npm run frontend:dev                  # Frontend only (port 3000)
npm run backend:dev                   # Backend only (auto-assigned port)
HOST=0.0.0.0 pnpm run dev            # Network-accessible dev server
```

### Stopping the Server
```bash
pnpm run stop                         # Stop instance for current directory
pnpm run stop /path/to/project        # Stop a specific project's instance
pnpm run stop --all                   # Stop all running instances
pnpm run stop --list                  # List all running instances
```

**Graceful Shutdown Sequence**: The stop command ensures safe termination in dev mode:
1. **SIGTERM → backend** - Triggers graceful shutdown of the Rust process
2. **Wait for backend exit** (max 10s) - Backend performs critical cleanup:
   - Flushes all log buffers (executor logs)
   - Runs `PRAGMA wal_checkpoint(TRUNCATE)` (SQLite durability)
   - Closes database connection pool
3. **SIGTERM → dev_root_pid** - Kills concurrently, terminating cargo-watch and Vite
4. **Port-based fallback** - Uses `lsof` to clean up any orphaned processes

**Critical**: The backend's cleanup sequence MUST complete before dev processes are killed. This ensures database integrity and prevents log loss.

**Multi-instance support**: Multiple vibe-kanban instances can run simultaneously. Each instance registers in `/tmp/vibe-kanban/instances/` with project root, PID, dev_root_pid (dev mode), and all ports.

**Process hierarchy in dev mode**:
```text
start-dev.js
  └─ concurrently (dev_root_pid)
      ├─ cargo watch
      │   └─ vks-node-server (backend PID)
      └─ npm/vite (frontend)
```

**Troubleshooting**: If you encounter "port already in use" errors after stopping:
- Run `pnpm run stop --all` to clean up all instances
- Check for orphaned processes: `lsof -i :PORT`
- Manually kill if needed: `kill -9 <PID>`

### Production
```bash
pnpm run prod                         # Full build + run production
pnpm run prod:build                   # Build frontend + release binary
pnpm run prod:run                     # Run production binary (after build)
pnpm run prod:backend                 # Backend only (release mode, no frontend build)
```

### Validation

Run **all** applicable checks before declaring a PR ready for merge. Do not skip steps.

```bash
# Backend
cargo fmt --all -- --check                                     # Rust formatting
cargo clippy --all --all-targets --all-features -- -D warnings # Clippy
cargo test --workspace                                          # All backend tests

# Frontend
cd frontend && npm run lint                                     # ESLint
cd frontend && npm run format:check                             # Prettier
cd frontend && npx tsc --noEmit                                 # TypeScript
cd frontend && npm run test:run                                 # Component tests

# Remote frontend (when remote-frontend/ changes)
cd remote-frontend && npx tsc --noEmit                          # TypeScript
cd remote-frontend && npm run lint                              # ESLint
cd remote-frontend && npm run test:run                          # Component tests
docker build -f crates/remote/Dockerfile .                      # Docker build smoke test
```

**Why the Docker check matters:** The `remote-frontend` build runs `tsc && vite build` inside a container that only has `remote-frontend/` — not `frontend/`. Cross-package imports (e.g. a test reaching into `frontend/src/`) resolve locally but break in Docker. Always verify the Docker build when touching `remote-frontend/`.

#### Pre-merge checklist (local)

Before pushing, confirm:
1. All checks above pass locally
2. The dev server starts (`pnpm run dev`) and the affected pages load without console errors
3. If the change touches API endpoints, test the happy path and one error path manually or via the component test suite
4. If the change touches the Docker build context, run `docker build -f crates/remote/Dockerfile .`

#### Post-merge deployed testing

Tests that require a deployed host (E2E against a live server, smoke tests against staging, integration with external services) are **not** blocking for merge. Run them in a **separate workstream** after the PR is merged and deployed:

1. Merge the PR
2. Deploy to the target environment
3. Create a follow-up workstream (`dev-docs/workstreams/<name>/README.md`) for deployed validation
4. Run E2E tests (`cd remote-frontend && pnpm test:e2e`) against the deployed host
5. Record results in the workstream README

This keeps the merge gate fast and local while ensuring deployed validation is tracked and not forgotten.

### Type Generation
```bash
npm run generate-types               # Regenerate TypeScript from Rust
npm run generate-types:check        # Verify types are current
```

### Database

**Location:**
- **Development**: `<project_root>/dev_assets/db.sqlite`
- **Production**: Platform-specific data directory (e.g., `~/.local/share/vibe-kanban/db.sqlite`)

**Automatic Backups:**
- Pre-migration backups are created automatically on server startup
- Backups stored in `dev_assets/backups/` (dev) or alongside the database (prod)
- Last 5 backups retained; older ones are automatically deleted
- Backup format: `db_backup_YYYYMMDD_HHMMSS.sqlite`

**Commands:**
```bash
sqlx migrate run                     # Apply migrations
sqlx database create                 # Create database
# Note: Dev database auto-copied from dev_assets_seed/ on startup
```

**Recovery from Backup:**
```bash
# Stop the server first, then restore
cp dev_assets/backups/db_backup_YYYYMMDD_HHMMSS.sqlite dev_assets/db.sqlite
```

### Build
```bash
./build-npm-package.sh              # Production build
```

### Environment Variables

See `.env.example` for complete documentation. Key variables:

Network:
- `HOST`: Backend host (default: `127.0.0.1`, use `0.0.0.0` for network access)
- `BACKEND_PORT`: Backend server port (default: auto-assign)
- `FRONTEND_PORT`: Frontend dev port (default: `3000`)
- `MCP_PORT`: HTTP MCP server port. If set, spawns MCP at `http://{HOST}:{MCP_PORT}/mcp`

Storage (see `docs/configuration-customisation/storage-configuration.mdx`):
- `VK_DATABASE_PATH`: Override database file location (supports tilde expansion)
- `VK_BACKUP_DIR`: Override backup directory (default: `{data_dir}/backups`)
- `VK_WORKTREE_DIR`: Override worktree directory (default: `/var/tmp/vibe-kanban/worktrees`)

Database Performance:
- `VK_SQLITE_MAX_CONNECTIONS`: Connection pool size (default: `10`)
- `VK_SLOW_QUERY_MS`: Slow query threshold in ms (default: `100`)

Scheduled Backups:
- `VK_SCHEDULED_BACKUPS`: Enable auto backups (default: `true`)
- `VK_BACKUP_INTERVAL_HOURS`: Backup interval (default: `4`)
- `VK_BACKUP_RETENTION`: Backups to keep - single value or `pre-migration,scheduled` (default: `5,10`)

WAL Monitoring:
- `VK_WAL_CHECK_INTERVAL_SECS`: Check interval (default: `60`)
- `VK_WAL_WARNING_THRESHOLD_MB`: Warning threshold (default: `50`)
- `VK_WAL_CHECKPOINT_THRESHOLD_MB`: Auto checkpoint threshold (default: `100`)
- `VK_WAL_AUTO_CHECKPOINT`: Enable auto checkpoints (default: `true`)
- `VK_WAL_TRUNCATE_INTERVAL_SECS`: TRUNCATE checkpoint interval for durability (default: `300`)

Logging:
- `RUST_LOG`: Log level - trace, debug, info, warn, error (default: `info`)
- `VK_FILE_LOGGING`: Enable file-based logging (default: `false`)
- `VK_LOG_DIR`: Override log directory (default: `{data_dir}/logs`)
- `VK_LOG_MAX_FILES`: Max daily log files to retain (default: `7`)

Worktree Cleanup:
- `DISABLE_WORKTREE_ORPHAN_CLEANUP`: Disable orphan cleanup (default: `0`)
- `DISABLE_WORKTREE_EXPIRED_CLEANUP`: Disable expired (72h+) cleanup (default: `0`)

Swarm/Hive Node (see `docs/swarm-hive-setup.mdx`):
- `VK_HIVE_URL`: WebSocket URL of hive server (e.g., `wss://hive.example.com`)
- `VK_NODE_API_KEY`: API key for authenticating with the hive
- `VK_NODE_NAME`: Human-readable node name (defaults to hostname)
- `VK_NODE_PUBLIC_URL`: Public URL for direct log streaming
- `VK_CONNECTION_TOKEN_SECRET`: JWT secret for direct connection tokens

## 10. AI Coding Assistant Instructions

1. **Read before writing**: Always read existing files before modifying. Understand the patterns in place.

2. **Run checks before committing**: Execute the full validation checklist in §9 — not just `npm run check`. Include remote-frontend and Docker checks when applicable.

3. **Generate types after Rust changes**: Run `npm run generate-types` after modifying any Rust struct with `#[derive(TS)]`.

4. **Follow the error pattern**: Use `thiserror` with `#[from]` for Rust errors. Map to HTTP status codes in `error.rs`.

5. **Use existing hooks**: Check `frontend/src/hooks/` before creating new data-fetching logic. Likely a hook exists.

6. **Match API patterns**: Backend routes return `Result<ResponseJson<ApiResponse<T>>, ApiError>`. Frontend unwraps via `api.ts`.

7. **Respect naming conventions**: Rust enums serialize as `snake_case`. TypeScript components are PascalCase. Hooks use `use` prefix.

8. **Keep services stateless**: Service structs should be `Clone` with no fields. Pass `&SqlitePool` to methods.

9. **Test database operations**: Use `db::test_utils::create_test_pool()` for fast, isolated test databases. Falls back to `tempfile::TempDir` for non-DB tests.

10. **Check existing components**: Look at similar components in `frontend/src/components/` before creating new ones. Follow established patterns for Props, hooks, and error handling.

11. **CRITICAL - Safe Process Management**: When running in a worktree spawned by vibe-kanban, NEVER use `pkill`, `killall`, or pattern-based process killing. These commands can accidentally kill the parent vibe-kanban server, causing database corruption. Instead:
    - To stop the vibe-kanban dev server: `pnpm run stop` (stops instance for current directory)
    - To list all running instances: `pnpm run stop --list`
    - To stop all instances: `pnpm run stop --all`
    - To kill a specific process: Use `kill <PID>` with the exact PID
    - The server binary is named `vks-node-server`, not "server"
    - Instance registry is at `/tmp/vibe-kanban/instances/` (JSON files keyed by project path hash)
    - Each instance file contains: project_root, PID, ports (backend, frontend, mcp, hive)

## 11. Post-Phase Integrated Adversarial Review (mandatory)

Per-task adversarial panels verify each task in isolation. They **cannot** catch cross-task
interaction bugs — e.g. a fencing guard (task 205) + a reclaim path (task 209) + a completion path
combining to produce a query that returns `None` at the wrong time. After completing each WAI
phase, run an **integrated adversarial review** (Gemini or cross-model) over the full phase diff
before moving to the next phase. Findings are subject to the No Deferred Remediation rule above:
fix in-session or dismiss with ledger evidence. No exceptions for "I'll catch it in the next phase."

Report path: `.agents/reports/YYYY-MM-DD-round-N-<panelist>-<2-word-description>.md`.
