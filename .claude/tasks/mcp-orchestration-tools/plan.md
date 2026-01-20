# Plan: MCP Orchestration Tools

## Overview

Enable orchestrators (AI agents and human MCP users) to manage long-running coding sessions by:
1. Providing a unified templates API that merges system, swarm, and local templates
2. Adding MCP orchestration tools for sending follow-ups, using templates, and checking execution status
3. Enhancing the MCP task attempt tools with node targeting and status fields
4. Adding a `stop_script` field to projects for graceful dev server shutdown

**Scope:**
- Backend: New API endpoints, MCP tool additions, database migration
- Frontend: Update TemplatePicker to use unified API
- TypeScript types: Auto-generated via `npm run generate-types`

**Out of Scope:**
- Labels API changes (already have Hive sync via `shared_label_id`)
- Documentation updates (separate task)

## User Stories

- **US1**: As an AI orchestrator, I want to send follow-up prompts to a task so I can continue long-running sessions
- **US2**: As an MCP user, I want to see all templates (system + swarm + local) in one list so I can choose the right one
- **US3**: As an orchestrator, I want to check if a task execution has failed so I can decide whether to retry
- **US4**: As an orchestrator, I want to target a specific node when starting a task attempt for swarm coordination
- **US5**: As a user, I want to configure a stop_script for graceful dev server shutdown

## Architecture

### Components Affected
1. **Backend Routes** (`crates/server/src/routes/templates.rs`) - Add unified templates endpoints
2. **DB Models** (`crates/db/src/models/template.rs`) - Add `UnifiedTemplate` type
3. **MCP Server** (`crates/server/src/mcp/task_server.rs`) - Add 5 new tools, enhance 2 existing
4. **DB Migration** - Add `stop_script` column to projects
5. **Frontend** (`frontend/src/components/tasks/TemplatePicker.tsx`, `frontend/src/lib/api/templates.ts`) - Use unified API

### Data Flow
```text
MCP Tool Call → TaskServer → HTTP to Backend → Database/Hive
                                  ↓
Frontend TemplatePicker → GET /api/templates/all → Merge system+swarm+local
```

---

## Tasks

### Task 001: Add UnifiedTemplate type to db models (simple)

**Goal**: Define the `UnifiedTemplate` response struct with source field for template unification.

**Depends on**: none

**Files**:
- `crates/db/src/models/template.rs` - Add new struct

**Steps**:
1. Open `crates/db/src/models/template.rs`
2. Add imports for `ts_rs::TS` if not present (already there)
3. Add `UnifiedTemplate` struct after the existing `UpdateTemplate` struct (around line 26)
4. Include `#[derive(Debug, Clone, Serialize, Deserialize, TS)]` and `#[ts(export)]`

**Code**:
```rust
/// Unified template combining system, swarm, and local sources
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UnifiedTemplate {
    pub id: String,
    pub name: String,
    pub content: String,
    pub description: Option<String>,
    /// Source of the template: "system", "swarm", or "local"
    pub source: String,
    #[ts(type = "Date | null")]
    pub created_at: Option<DateTime<Utc>>,
    #[ts(type = "Date | null")]
    pub updated_at: Option<DateTime<Utc>>,
}
```

**Verification**:
- [ ] `cargo check -p db` succeeds
- [ ] No duplicate struct definitions

**Commit**: `feat(db): add UnifiedTemplate type for template unification`

---

### Task 002: Add system templates constant to routes (simple)

**Goal**: Define hardcoded system templates in the templates route module.

**Depends on**: 001

**Files**:
- `crates/server/src/routes/templates.rs` - Add constant at top of file

**Steps**:
1. Open `crates/server/src/routes/templates.rs`
2. After the imports (around line 14), add the `SystemTemplate` struct and `SYSTEM_TEMPLATES` static array
3. These are built-in templates that are always available

**Code**:
```rust
/// A built-in system template
struct SystemTemplate {
    id: &'static str,
    name: &'static str,
    content: &'static str,
    description: &'static str,
}

/// Built-in templates that are always available
static SYSTEM_TEMPLATES: &[SystemTemplate] = &[
    SystemTemplate {
        id: "system-bug-report",
        name: "Bug Report",
        content: "## Bug Description\nDescribe the bug clearly and concisely.\n\n## Steps to Reproduce\n1. Go to '...'\n2. Click on '...'\n3. See error\n\n## Expected Behavior\nDescribe what you expected to happen.\n\n## Actual Behavior\nDescribe what actually happened.",
        description: "Structured bug report template",
    },
    SystemTemplate {
        id: "system-feature-request",
        name: "Feature Request",
        content: "## Feature Description\nDescribe the feature you'd like to see.\n\n## Problem Statement\nWhat problem does this feature solve?\n\n## Proposed Solution\nDescribe how you think this should work.\n\n## Alternatives Considered\nAre there other ways to solve this?",
        description: "Feature request template",
    },
    SystemTemplate {
        id: "system-code-review",
        name: "Code Review Checklist",
        content: "## Code Review Checklist\n\n### Functionality\n- [ ] Code works as expected\n- [ ] Edge cases are handled\n- [ ] Error handling is appropriate\n\n### Code Quality\n- [ ] Code is readable and well-organized\n- [ ] No unnecessary complexity\n- [ ] DRY principle followed\n\n### Testing\n- [ ] Tests are included\n- [ ] Tests cover key scenarios",
        description: "Code review checklist template",
    },
    SystemTemplate {
        id: "system-quick-task",
        name: "Quick Task",
        content: "## Goal\nWhat needs to be accomplished?\n\n## Acceptance Criteria\n- [ ] \n\n## Notes\n",
        description: "Simple task template",
    },
];
```

**Verification**:
- [ ] `cargo check -p server` succeeds
- [ ] 4 system templates defined

**Commit**: `feat(server): add system templates constant`

---

### Task 003: Add get_all_templates endpoint (moderate)

**Goal**: Create endpoint that returns all templates (system + swarm + local) with source indicators.

**Depends on**: 001, 002

**Files**:
- `crates/server/src/routes/templates.rs` - Add handler and route

**Steps**:
1. Add import for `UnifiedTemplate` from db models: `use db::models::template::{..., UnifiedTemplate};`
2. Add `get_all_templates` handler function after `get_templates` (around line 40)
3. Update the router function to add the new route: `.route("/all", get(get_all_templates))`

**Code**:
```rust
/// GET /api/templates/all - Returns all templates from all sources
pub async fn get_all_templates(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<UnifiedTemplate>>>, ApiError> {
    let mut all_templates: Vec<UnifiedTemplate> = Vec::new();

    // 1. Add system templates
    for t in SYSTEM_TEMPLATES {
        all_templates.push(UnifiedTemplate {
            id: t.id.to_string(),
            name: t.name.to_string(),
            content: t.content.to_string(),
            description: Some(t.description.to_string()),
            source: "system".to_string(),
            created_at: None,
            updated_at: None,
        });
    }

    // 2. Add swarm templates if connected to Hive
    if let Ok(remote_client) = deployment.remote_client() {
        if let Ok(swarm_templates) = remote_client.list_swarm_templates().await {
            for t in swarm_templates {
                all_templates.push(UnifiedTemplate {
                    id: t.id.to_string(),
                    name: t.template_name,
                    content: t.content,
                    description: None,
                    source: "swarm".to_string(),
                    created_at: Some(t.created_at),
                    updated_at: Some(t.updated_at),
                });
            }
        }
    }

    // 3. Add local templates
    let local_templates = Template::find_all(&deployment.db().pool).await?;
    for t in local_templates {
        all_templates.push(UnifiedTemplate {
            id: t.id.to_string(),
            name: t.template_name,
            content: t.content,
            description: None,
            source: "local".to_string(),
            created_at: Some(t.created_at),
            updated_at: Some(t.updated_at),
        });
    }

    // Sort alphabetically by name
    all_templates.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(ResponseJson(ApiResponse::success(all_templates)))
}
```

**Router update** (in `router` function):
```rust
let inner = Router::new()
    .route("/", get(get_templates).post(create_template))
    .route("/all", get(get_all_templates))  // Add this line
    .nest("/{template_id}", template_router);
```

**Verification**:
- [ ] `cargo check -p server` succeeds
- [ ] `cargo test -p server` passes

**Commit**: `feat(server): add GET /api/templates/all endpoint`

---

### Task 004: Add get_template_by_name endpoint (moderate)

**Goal**: Create endpoint to fetch a single template by name, resolving from system → swarm → local priority.

**Depends on**: 001, 002

**Files**:
- `crates/server/src/routes/templates.rs` - Add handler, query params, and route

**Steps**:
1. Add `Path` to axum imports
2. Add query params struct for optional task_id context
3. Add `get_template_by_name` handler
4. Add route to router: `.route("/by-name/{name}", get(get_template_by_name))`

**Code**:
```rust
use axum::extract::Path;

#[derive(Deserialize)]
pub struct TemplateByNameParams {
    /// Task ID for swarm context (optional)
    pub task_id: Option<Uuid>,
}

/// GET /api/templates/by-name/{name} - Get template by name
pub async fn get_template_by_name(
    Path(name): Path<String>,
    Query(params): Query<TemplateByNameParams>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<UnifiedTemplate>>, ApiError> {
    let name_lower = name.to_lowercase();

    // 1. Check system templates first
    for t in SYSTEM_TEMPLATES {
        if t.name.to_lowercase() == name_lower || t.id == name {
            return Ok(ResponseJson(ApiResponse::success(UnifiedTemplate {
                id: t.id.to_string(),
                name: t.name.to_string(),
                content: t.content.to_string(),
                description: Some(t.description.to_string()),
                source: "system".to_string(),
                created_at: None,
                updated_at: None,
            })));
        }
    }

    // 2. Check swarm templates if connected
    if let Ok(remote_client) = deployment.remote_client() {
        if let Ok(swarm_templates) = remote_client.list_swarm_templates().await {
            for t in swarm_templates {
                if t.template_name.to_lowercase() == name_lower {
                    return Ok(ResponseJson(ApiResponse::success(UnifiedTemplate {
                        id: t.id.to_string(),
                        name: t.template_name,
                        content: t.content,
                        description: None,
                        source: "swarm".to_string(),
                        created_at: Some(t.created_at),
                        updated_at: Some(t.updated_at),
                    })));
                }
            }
        }
    }

    // 3. Check local templates
    let local_templates = Template::find_all(&deployment.db().pool).await?;
    for t in local_templates {
        if t.template_name.to_lowercase() == name_lower {
            return Ok(ResponseJson(ApiResponse::success(UnifiedTemplate {
                id: t.id.to_string(),
                name: t.template_name,
                content: t.content,
                description: None,
                source: "local".to_string(),
                created_at: Some(t.created_at),
                updated_at: Some(t.updated_at),
            })));
        }
    }

    Err(ApiError::NotFound(format!("Template '{}' not found", name)))
}
```

**Router update**:
```rust
let inner = Router::new()
    .route("/", get(get_templates).post(create_template))
    .route("/all", get(get_all_templates))
    .route("/by-name/{name}", get(get_template_by_name))  // Add this line
    .nest("/{template_id}", template_router);
```

**Verification**:
- [ ] `cargo check -p server` succeeds
- [ ] System template lookup works by name
- [ ] Returns 404 for unknown template

**Commit**: `feat(server): add GET /api/templates/by-name/{name} endpoint`

---

### Task 005: Add frontend templates API methods (simple)

**Goal**: Add `listAll()` and `getByName()` methods to the frontend templates API.

**Depends on**: 003, 004

**Files**:
- `frontend/src/lib/api/templates.ts` - Add two new methods

**Steps**:
1. Open `frontend/src/lib/api/templates.ts`
2. Add import for `UnifiedTemplate` type (will be generated)
3. Add `listAll` method after `list` method
4. Add `getByName` method after `listAll`

**Code**:
```typescript
// Add to imports at top
import {
  Template,
  TemplateSearchParams,
  CreateTemplate,
  UpdateTemplate,
  UnifiedTemplate,  // Add this
} from 'shared/types';

// Add these methods to templatesApi object:

  /**
   * List all templates from all sources (system + swarm + local)
   */
  listAll: async (): Promise<UnifiedTemplate[]> => {
    const response = await makeRequest('/api/templates/all');
    return handleApiResponse<UnifiedTemplate[]>(response);
  },

  /**
   * Get a template by name (searches system → swarm → local)
   */
  getByName: async (name: string, taskId?: string): Promise<UnifiedTemplate> => {
    const queryParam = taskId ? `?task_id=${encodeURIComponent(taskId)}` : '';
    const response = await makeRequest(
      `/api/templates/by-name/${encodeURIComponent(name)}${queryParam}`
    );
    return handleApiResponse<UnifiedTemplate>(response);
  },
```

**Verification**:
- [ ] `cd frontend && npx tsc --noEmit` succeeds
- [ ] `npm run generate-types` run first to generate `UnifiedTemplate` type

**Commit**: `feat(frontend): add listAll and getByName to templates API`

---

### Task 006: Update TemplatePicker to use unified API (moderate)

**Goal**: Replace hardcoded DEFAULT_TEMPLATES with API call to `/api/templates/all`.

**Depends on**: 005

**Files**:
- `frontend/src/components/tasks/TemplatePicker.tsx` - Update to fetch from API

**Steps**:
1. Add `useQuery` import from `@tanstack/react-query`
2. Add `templatesApi` import
3. Remove the `DEFAULT_TEMPLATES` constant (lines 34-129)
4. Add query to fetch templates inside the component
5. Update the `allTemplates` memo to use fetched data
6. Pass loading/error state to existing props

**Code changes**:

Add imports:
```typescript
import { useQuery } from '@tanstack/react-query';
import { templatesApi } from '@/lib/api/templates';
import type { UnifiedTemplate } from 'shared/types';
```

Remove or comment out `DEFAULT_TEMPLATES` constant (lines 34-129).

Update Template interface to match UnifiedTemplate:
```typescript
export interface Template {
  id: string;
  name: string;
  description: string;
  content: string;
  icon?: React.ReactNode;
  source?: string;  // Add source field
}
```

Inside the component, add the query:
```typescript
// Fetch all templates from API
const {
  data: fetchedTemplates,
  isLoading: templatesLoading,
  error: templatesError,
  refetch: refetchTemplates,
} = useQuery({
  queryKey: ['templates', 'all'],
  queryFn: () => templatesApi.listAll(),
  staleTime: 5 * 60 * 1000, // 5 minutes
});
```

Update allTemplates memo:
```typescript
const allTemplates = useMemo(() => {
  const templates: Template[] = [];

  // Map fetched templates to Template interface
  if (fetchedTemplates) {
    for (const t of fetchedTemplates) {
      templates.push({
        id: t.id,
        name: t.name,
        description: t.description || `${t.source} template`,
        content: t.content,
        source: t.source,
      });
    }
  }

  // Add custom templates passed as props
  templates.push(...customTemplates);

  return templates;
}, [fetchedTemplates, customTemplates]);
```

Update loading/error handling:
```typescript
// In the component, combine loading states
const isLoading = loading || templatesLoading;
const errorMessage = error || (templatesError ? 'Failed to load templates' : null);
const handleRetry = onRetry || refetchTemplates;
```

**Verification**:
- [ ] `cd frontend && npm run lint` passes
- [ ] `cd frontend && npx tsc --noEmit` succeeds
- [ ] Template picker shows templates from API

**Commit**: `feat(frontend): update TemplatePicker to use unified templates API`

---

### Task 007: Add MCP list_templates tool (simple)

**Goal**: Add MCP tool to list all templates (system + swarm + local).

**Depends on**: 003

**Files**:
- `crates/server/src/mcp/task_server.rs` - Add tool

**Steps**:
1. Add `ListTemplatesResponse` struct after the Nodes MCP Types section (around line 467)
2. Add `list_templates` tool in the `#[tool_router] impl TaskServer` block

**Code**:

Add response type (after line 467):
```rust
// ===== Template MCP Types =====

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TemplateSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub source: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListTemplatesResponse {
    pub templates: Vec<TemplateSummary>,
    pub count: usize,
}
```

Add tool (in the `#[tool_router] impl TaskServer` block):
```rust
#[tool(description = "List all available templates (system + swarm + local).")]
async fn list_templates(&self) -> Result<CallToolResult, ErrorData> {
    let url = self.url("/api/templates/all");

    #[derive(Deserialize)]
    struct UnifiedTemplate {
        id: String,
        name: String,
        description: Option<String>,
        source: String,
    }

    let templates: Vec<UnifiedTemplate> = match self.send_json(self.client.get(&url)).await {
        Ok(t) => t,
        Err(e) => return Ok(e),
    };

    let summaries: Vec<TemplateSummary> = templates
        .into_iter()
        .map(|t| TemplateSummary {
            id: t.id,
            name: t.name,
            description: t.description,
            source: t.source,
        })
        .collect();

    let response = ListTemplatesResponse {
        count: summaries.len(),
        templates: summaries,
    };

    TaskServer::success(&response)
}
```

**Verification**:
- [ ] `cargo check -p server` succeeds
- [ ] Tool shows in MCP tool list

**Commit**: `feat(mcp): add list_templates tool`

---

### Task 008: Add MCP get_template tool (simple)

**Goal**: Add MCP tool to get a specific template by name.

**Depends on**: 004, 007

**Files**:
- `crates/server/src/mcp/task_server.rs` - Add tool

**Steps**:
1. Add request type for the tool
2. Add `get_template` tool in the `#[tool_router] impl TaskServer` block

**Code**:

Add request type (after `ListTemplatesResponse`):
```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTemplateRequest {
    #[schemars(description = "Template name to look up")]
    pub name: String,
    #[schemars(description = "Task ID for swarm context (optional)")]
    pub task_id: Option<Uuid>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct GetTemplateResponse {
    pub id: String,
    pub name: String,
    pub content: String,
    pub description: Option<String>,
    pub source: String,
}
```

Add tool:
```rust
#[tool(description = "Get a specific template by name.")]
async fn get_template(
    &self,
    Parameters(GetTemplateRequest { name, task_id }): Parameters<GetTemplateRequest>,
) -> Result<CallToolResult, ErrorData> {
    let mut url = self.url(&format!("/api/templates/by-name/{}", urlencoding::encode(&name)));
    if let Some(tid) = task_id {
        url = format!("{}?task_id={}", url, tid);
    }

    #[derive(Deserialize)]
    struct UnifiedTemplate {
        id: String,
        name: String,
        content: String,
        description: Option<String>,
        source: String,
    }

    let template: UnifiedTemplate = match self.send_json(self.client.get(&url)).await {
        Ok(t) => t,
        Err(e) => return Ok(e),
    };

    let response = GetTemplateResponse {
        id: template.id,
        name: template.name,
        content: template.content,
        description: template.description,
        source: template.source,
    };

    TaskServer::success(&response)
}
```

**Verification**:
- [ ] `cargo check -p server` succeeds
- [ ] Tool returns template content

**Commit**: `feat(mcp): add get_template tool`

---

### Task 009: Add MCP send_follow_up tool (moderate)

**Goal**: Add MCP tool to send a follow-up prompt to a task.

**Depends on**: none

**Files**:
- `crates/server/src/mcp/task_server.rs` - Add tool

**Steps**:
1. Add request/response types for send_follow_up
2. Add the tool implementation that calls the follow-up endpoint

**Code**:

Add types:
```rust
// ===== Orchestration MCP Types =====

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SendFollowUpRequest {
    #[schemars(description = "Task ID to send follow-up to")]
    pub task_id: Uuid,
    #[schemars(description = "The follow-up prompt text")]
    pub prompt: String,
    #[schemars(description = "Variant: DEFAULT, NO_CONTEXT, PLAN, APPROVALS")]
    pub variant: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SendFollowUpResponse {
    pub task_id: String,
    pub attempt_id: String,
    pub execution_process_id: String,
}
```

Add tool:
```rust
#[tool(description = "Send a follow-up prompt to continue a task. Use variant='NO_CONTEXT' for fresh context.")]
async fn send_follow_up(
    &self,
    Parameters(SendFollowUpRequest { task_id, prompt, variant }): Parameters<SendFollowUpRequest>,
) -> Result<CallToolResult, ErrorData> {
    let url = self.url(&format!("/api/task-attempts/by-task-id/{}/follow-up", task_id));

    let payload = serde_json::json!({
        "prompt": prompt,
        "variant": variant,
    });

    #[derive(Deserialize)]
    struct FollowUpResult {
        id: Uuid,
        task_attempt_id: Uuid,
    }

    let result: FollowUpResult = match self
        .send_json(self.client.post(&url).json(&payload))
        .await
    {
        Ok(r) => r,
        Err(e) => return Ok(e),
    };

    let response = SendFollowUpResponse {
        task_id: task_id.to_string(),
        attempt_id: result.task_attempt_id.to_string(),
        execution_process_id: result.id.to_string(),
    };

    TaskServer::success(&response)
}
```

**Verification**:
- [ ] `cargo check -p server` succeeds
- [ ] Tool description clear for orchestrators

**Commit**: `feat(mcp): add send_follow_up tool`

---

### Task 010: Add MCP send_template tool (simple)

**Goal**: Add MCP tool to send a template as a follow-up prompt.

**Depends on**: 008, 009

**Files**:
- `crates/server/src/mcp/task_server.rs` - Add tool

**Steps**:
1. Add request type
2. Add tool that fetches template then sends as follow-up

**Code**:

Add request type:
```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SendTemplateRequest {
    #[schemars(description = "Task ID to send template to")]
    pub task_id: Uuid,
    #[schemars(description = "Template name to send")]
    pub template_name: String,
    #[schemars(description = "Variant: DEFAULT, NO_CONTEXT, PLAN, APPROVALS")]
    pub variant: Option<String>,
}
```

Add tool:
```rust
#[tool(description = "Send a template as follow-up prompt. Resolves template by name from system/swarm/local sources.")]
async fn send_template(
    &self,
    Parameters(SendTemplateRequest { task_id, template_name, variant }): Parameters<SendTemplateRequest>,
) -> Result<CallToolResult, ErrorData> {
    // 1. Fetch template content
    let template_url = self.url(&format!(
        "/api/templates/by-name/{}?task_id={}",
        urlencoding::encode(&template_name),
        task_id
    ));

    #[derive(Deserialize)]
    struct TemplateContent {
        content: String,
    }

    let template: TemplateContent = match self.send_json(self.client.get(&template_url)).await {
        Ok(t) => t,
        Err(e) => return Ok(e),
    };

    // 2. Send as follow-up
    let follow_up_url = self.url(&format!("/api/task-attempts/by-task-id/{}/follow-up", task_id));
    let payload = serde_json::json!({
        "prompt": template.content,
        "variant": variant,
    });

    #[derive(Deserialize)]
    struct FollowUpResult {
        id: Uuid,
        task_attempt_id: Uuid,
    }

    let result: FollowUpResult = match self
        .send_json(self.client.post(&follow_up_url).json(&payload))
        .await
    {
        Ok(r) => r,
        Err(e) => return Ok(e),
    };

    let response = SendFollowUpResponse {
        task_id: task_id.to_string(),
        attempt_id: result.task_attempt_id.to_string(),
        execution_process_id: result.id.to_string(),
    };

    TaskServer::success(&response)
}
```

**Verification**:
- [ ] `cargo check -p server` succeeds
- [ ] Tool resolves template then sends

**Commit**: `feat(mcp): add send_template tool`

---

### Task 011: Add MCP get_last_message tool (simple)

**Goal**: Add MCP tool to get recent log entries from an execution process.

**Depends on**: none

**Files**:
- `crates/server/src/mcp/task_server.rs` - Add tool

**Steps**:
1. Add request/response types
2. Add tool that calls logs API

**Code**:

Add types:
```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetLastMessageRequest {
    #[schemars(description = "Execution process ID")]
    pub execution_process_id: Uuid,
    #[schemars(description = "Number of entries to return (default 1, max 10)")]
    pub count: Option<i32>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct LogEntrySummary {
    pub id: String,
    pub content: String,
    pub entry_type: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct GetLastMessageResponse {
    pub execution_process_id: String,
    pub entries: Vec<LogEntrySummary>,
    pub count: usize,
}
```

Add tool:
```rust
#[tool(description = "Get the most recent log entries from an execution process.")]
async fn get_last_message(
    &self,
    Parameters(GetLastMessageRequest { execution_process_id, count }): Parameters<GetLastMessageRequest>,
) -> Result<CallToolResult, ErrorData> {
    let limit = count.unwrap_or(1).min(10).max(1);
    let url = self.url(&format!(
        "/api/logs/{}?direction=backward&limit={}",
        execution_process_id, limit
    ));

    #[derive(Deserialize)]
    struct LogEntry {
        id: Uuid,
        content: String,
        entry_type: String,
        created_at: String,
    }

    #[derive(Deserialize)]
    struct LogsResponse {
        entries: Vec<LogEntry>,
    }

    let logs: LogsResponse = match self.send_json(self.client.get(&url)).await {
        Ok(l) => l,
        Err(e) => return Ok(e),
    };

    let entries: Vec<LogEntrySummary> = logs
        .entries
        .into_iter()
        .map(|e| LogEntrySummary {
            id: e.id.to_string(),
            content: e.content,
            entry_type: e.entry_type,
            created_at: e.created_at,
        })
        .collect();

    let response = GetLastMessageResponse {
        execution_process_id: execution_process_id.to_string(),
        count: entries.len(),
        entries,
    };

    TaskServer::success(&response)
}
```

**Verification**:
- [ ] `cargo check -p server` succeeds
- [ ] Returns recent log entries

**Commit**: `feat(mcp): add get_last_message tool`

---

### Task 012: Add target_node_id to start_task_attempt MCP tool (simple)

**Goal**: Add `target_node_id` parameter to the existing `start_task_attempt` MCP tool.

**Depends on**: none

**Files**:
- `crates/server/src/mcp/task_server.rs` - Update existing tool

**Steps**:
1. Find `StartTaskAttemptRequest` struct (around line 191)
2. Add `target_node_id` field
3. Update `start_task_attempt` tool to use the new field (around line 845-851)

**Code changes**:

Update `StartTaskAttemptRequest`:
```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StartTaskAttemptRequest {
    #[schemars(description = "Task ID")]
    pub task_id: Uuid,
    #[schemars(description = "Executor: 'CLAUDE_CODE', 'CODEX', 'GEMINI', 'CURSOR_AGENT', 'OPENCODE'")]
    pub executor: String,
    #[schemars(description = "Executor variant (optional)")]
    pub variant: Option<String>,
    #[schemars(description = "Base branch")]
    pub base_branch: String,
    #[schemars(description = "Target node ID for swarm execution. Use list_nodes to find available nodes.")]
    pub target_node_id: Option<Uuid>,
}
```

Update destructuring in `start_task_attempt` tool (around line 803):
```rust
Parameters(StartTaskAttemptRequest {
    task_id,
    executor,
    variant,
    base_branch,
    target_node_id,  // Add this
}): Parameters<StartTaskAttemptRequest>,
```

Update payload creation (around line 845):
```rust
let payload = CreateTaskAttemptBody {
    task_id,
    executor_profile_id,
    base_branch,
    target_node_id,  // Change from: target_node_id: None,
    use_parent_worktree: None,
};
```

**Verification**:
- [ ] `cargo check -p server` succeeds
- [ ] target_node_id passed to backend

**Commit**: `feat(mcp): add target_node_id to start_task_attempt tool`

---

### Task 013: Enhance get_task_attempt_status response (moderate)

**Goal**: Add `task_title`, `is_running`, and `has_failed` computed fields to status response.

**Depends on**: none

**Files**:
- `crates/server/src/mcp/task_server.rs` - Update response type and tool

**Steps**:
1. Find `TaskAttemptStatusResponse` struct (around line 339)
2. Add new fields
3. Update `get_task_attempt_status` tool to compute and populate new fields

**Code changes**:

Update `TaskAttemptStatusResponse`:
```rust
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TaskAttemptStatusResponse {
    pub attempt_id: String,
    pub task_id: String,
    pub task_title: String,  // NEW
    pub branch: String,
    pub executor: String,
    pub worktree_deleted: bool,
    pub created_at: String,
    pub processes: Vec<ExecutionProcessSummary>,
    pub is_running: bool,   // NEW - any process with status=Running
    pub has_failed: bool,   // NEW - latest CodingAgent process failed
}
```

Update `get_task_attempt_status` tool to compute new fields:

Find where the response is constructed and update:
```rust
// Compute is_running: any process has status "running"
let is_running = process_summaries
    .iter()
    .any(|p| p.status.to_lowercase() == "running");

// Compute has_failed: latest CodingAgent process has status "failed"
let has_failed = process_summaries
    .iter()
    .filter(|p| p.run_reason.to_lowercase() == "codingagent")
    .last()
    .map(|p| p.status.to_lowercase() == "failed")
    .unwrap_or(false);

// Need to fetch task title - add API call
let task_url = self.url(&format!("/api/tasks/{}", attempt.task_id));
let task_title = match self.send_json::<Task>(self.client.get(&task_url)).await {
    Ok(t) => t.title,
    Err(_) => "Unknown".to_string(),
};

let response = TaskAttemptStatusResponse {
    attempt_id: attempt.id.to_string(),
    task_id: attempt.task_id.to_string(),
    task_title,
    branch: attempt.branch,
    executor: attempt.executor,
    worktree_deleted: attempt.worktree_deleted,
    created_at: attempt.created_at.to_rfc3339(),
    processes: process_summaries,
    is_running,
    has_failed,
};
```

**Verification**:
- [ ] `cargo check -p server` succeeds
- [ ] `is_running` correctly computed
- [ ] `has_failed` correctly computed

**Commit**: `feat(mcp): enhance get_task_attempt_status with computed fields`

---

### Task 014: Create stop_script migration (simple)

**Goal**: Add `stop_script` column to projects table.

**Depends on**: none

**Files**:
- `crates/db/migrations/` - New migration file

**Steps**:
1. Create new migration file with timestamp
2. Add ALTER TABLE statement

**Code**:
```sql
-- Migration: add_stop_script_to_projects
-- Add stop_script field for graceful dev server shutdown

ALTER TABLE projects ADD COLUMN stop_script TEXT;
```

Filename: `20260120000000_add_stop_script_to_projects.sql`

**Verification**:
- [ ] Migration file exists with correct format
- [ ] `sqlx migrate run` succeeds (or check with `cargo sqlx prepare --check`)

**Commit**: `feat(db): add stop_script column to projects`

---

### Task 015: Add stop_script to Project model (simple)

**Goal**: Add `stop_script` field to Project struct and related types.

**Depends on**: 014

**Files**:
- `crates/db/src/models/project/mod.rs` - Add field to structs

**Steps**:
1. Add `stop_script: Option<String>` to `Project` struct (after `cleanup_script`)
2. Add to `CreateProject` struct
3. Add to `UpdateProject` struct

**Code changes**:

In `Project` struct (after line 42 `cleanup_script`):
```rust
pub stop_script: Option<String>,
```

In `CreateProject` struct (after line 84 `cleanup_script`):
```rust
pub stop_script: Option<String>,
```

In `UpdateProject` struct (after line 94 `cleanup_script`):
```rust
pub stop_script: Option<String>,
```

**Verification**:
- [ ] `cargo check -p db` succeeds
- [ ] Field appears in all three structs

**Commit**: `feat(db): add stop_script field to Project model`

---

### Task 016: Update Project queries for stop_script (moderate)

**Goal**: Update SQL queries to include stop_script in select, insert, and update.

**Depends on**: 015

**Files**:
- `crates/db/src/models/project/queries.rs` - Update queries

**Steps**:
1. Find all `SELECT` queries and add `stop_script` to column list
2. Find `INSERT` query and add `stop_script`
3. Find `UPDATE` query and add `stop_script`

**Code changes**:

The exact changes depend on the current query structure. General pattern:

For SELECT queries, add `stop_script` to the column list:
```sql
SELECT id, name, git_repo_path, setup_script, dev_script, cleanup_script, stop_script, ...
```

For INSERT, add the column and value:
```sql
INSERT INTO projects (id, name, ..., stop_script) VALUES ($1, $2, ..., $N)
```

For UPDATE, add to SET clause:
```sql
SET name = $2, ..., stop_script = $N, updated_at = ...
```

**Verification**:
- [ ] `cargo check -p db` succeeds
- [ ] `cargo test -p db` passes
- [ ] `cargo sqlx prepare --check` passes

**Commit**: `feat(db): update Project queries for stop_script`

---

### Task 017: Update frontend Project types (simple)

**Goal**: Add stop_script to frontend project types and form.

**Depends on**: 016

**Files**:
- `frontend/src/components/projects/ProjectForm.tsx` - Add input field (if exists)
- Types will be auto-generated via `npm run generate-types`

**Steps**:
1. Run `npm run generate-types` to update TypeScript types
2. If ProjectForm exists, add stop_script input field similar to cleanup_script
3. Add label and placeholder text

**Code** (for ProjectForm if it has script fields):
```typescript
// Add after cleanup_script field
<div className="space-y-2">
  <Label htmlFor="stop_script">{t('projects.stopScript', 'Stop Script')}</Label>
  <Textarea
    id="stop_script"
    value={formData.stop_script || ''}
    onChange={(e) => setFormData({ ...formData, stop_script: e.target.value })}
    placeholder={t('projects.stopScriptPlaceholder', 'Script to stop dev server gracefully')}
    rows={3}
  />
</div>
```

**Verification**:
- [ ] `npm run generate-types` succeeds
- [ ] `cd frontend && npx tsc --noEmit` succeeds
- [ ] Form shows stop_script field if applicable

**Commit**: `feat(frontend): add stop_script to project form`

---

### Task 018: Run full verification (simple)

**Goal**: Verify all changes work together.

**Depends on**: 001-017

**Files**: None (verification only)

**Steps**:
1. Run `cargo build --workspace`
2. Run `cargo test --workspace`
3. Run `npm run generate-types`
4. Run `cd frontend && npm run lint`
5. Run `cd frontend && npx tsc --noEmit`
6. Run `npm run check` (all checks)

**Verification**:
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` passes
- [ ] `npm run check` passes
- [ ] MCP tools appear in server instructions

**Commit**: None (verification only)

---

## Verification Checklist

- [ ] All Rust code compiles: `cargo build --workspace`
- [ ] All Rust tests pass: `cargo test --workspace`
- [ ] TypeScript types generated: `npm run generate-types`
- [ ] Frontend lints pass: `cd frontend && npm run lint`
- [ ] Frontend types check: `cd frontend && npx tsc --noEmit`
- [ ] New MCP tools visible in `vks-mcp-server` instructions
- [ ] GET /api/templates/all returns system + local templates
- [ ] TemplatePicker shows templates from API

## Out of Scope

- Documentation updates (separate task)
- run_project_script MCP tool (can be added later if needed)
- Swarm template sync (already works via Hive connection)
- Labels API changes (already have Hive sync)
