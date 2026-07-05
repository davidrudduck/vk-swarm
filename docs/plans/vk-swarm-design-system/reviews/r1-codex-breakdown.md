### FINDING 1: Phase-3 UI Path Violates Frozen Spec
**Lens:** both  
**Severity:** major  
**Task(s):** 301, 302, 303, 304, 307, 308, 309, 310  
**Claim:** The frozen spec requires the app UI kit under `remote-frontend/src/ui/`, but the plan and task files create/import it under `remote-frontend/src/app/`. A literal implementer will satisfy the task files while missing SC7’s stated location.  
**Evidence:** `docs/superpowers/specs/2026-07-04-vk-swarm-design-system.md:97-101` and `:170-172` require `remote-frontend/src/ui/`; `docs/plans/vk-swarm-design-system/plan.md:47` says `remote-frontend/src/app/`; task 301 lists `remote-frontend/src/app/board/...` at `docs/plans/vk-swarm-design-system/phase-3/301-port-boardview-columnheader.md:9-15`.  
**Remediation:** Edit plan phase 3 and tasks 301-304, 307-310 to use `remote-frontend/src/ui/{board,chrome,panels}/...` and imports like `@/ui/board`, `@/ui/chrome`, `@/ui/panels`.

### FINDING 2: Task 101 Declares Create-Only Scope But Edits `.prettierignore`
**Lens:** mechanics  
**Severity:** blocker  
**Task(s):** 101  
**Claim:** Task 101 instructs an edit to `remote-frontend/.prettierignore`, but the file is absent from `files:`, `scope_test` is a directory not listed in `files:`, and `allowed_change` is `create`. This will fail a strict scope gate or produce an unauthorized edit.  
**Evidence:** task 101 frontmatter lists only token CSS/tests at `docs/plans/vk-swarm-design-system/phase-1/101-port-color-typography-tokens.md:9-16`; the task then says to edit `.prettierignore` at `:107-115` and allowed moves include appending to it at `:117-123`; existing file is real at `remote-frontend/.prettierignore:1`.  
**Remediation:** Add `remote-frontend/.prettierignore` to `files:`, change `allowed_change: edit`, and make `scope_test` one of the listed test files, not the directory. Alternatively move the `.prettierignore` edit to a separate task.

### FINDING 3: Task 306 Depends On Nonexistent Task And Misnames Its Test File
**Lens:** mechanics  
**Severity:** blocker  
**Task(s):** 306  
**Claim:** Task 306 declares `depends_on: ["100","201"]`, but this plan has tasks 101-106, 201-208, 301-310. It also says to create `eric.test.ts` while `files:` and Done-when use `electric.test.ts`.  
**Evidence:** plan IDs start at 101 and phase 3 table shows task 306 depends on `100 201` at `docs/plans/vk-swarm-design-system/plan.md:51-60`; task 306 frontmatter repeats `depends_on: ["100","201"]` at `docs/plans/vk-swarm-design-system/phase-3/306-electric-collections-task-tables.md:1-17`; task body says `eric.test.ts` at `:89-93`, while Done-when runs `electric.test.ts` at `:107-108`.  
**Remediation:** Replace dependency `100` with the real prerequisite task that installs/copies the remote frontend Electric dependencies, or document it as an external precondition outside `depends_on`. Change `eric.test.ts` to `electric.test.ts`.

### FINDING 4: REST Client Uses `ApiError` Constructor Backwards
**Lens:** fidelity  
**Severity:** blocker  
**Task(s):** 305  
**Claim:** The sample client code calls `new ApiError(r.status, await r.text())`, but the existing constructor expects `(message: string, statusCode?: number, response?: Response, ...)`. This is a TypeScript compile failure.  
**Evidence:** constructor signature is `message: string` then `statusCode?: number` in `remote-frontend/src/lib/api/utils.ts:20-25`; task 305’s generated `nodesApi` uses `new ApiError(r.status, await r.text())` at `docs/plans/vk-swarm-design-system/phase-3/305-hive-rest-clients.md:84-88`.  
**Remediation:** Edit task 305 to use `const body = await r.text(); throw new ApiError(body || 'Request failed', r.status, r);` in all generated clients.

### FINDING 5: Task Bulk API Shape Is Wrong
**Lens:** fidelity  
**Severity:** blocker  
**Task(s):** 305, 308, 310  
**Claim:** The plan treats `GET /v1/tasks/bulk` as parameterless and returning `Task[]`. The real route requires `project_id` and returns `{ tasks, deleted_task_ids, latest_seq }`. Board wiring and integration tests will pass mocks but fail against the real hive.  
**Evidence:** backend query requires `project_id` at `crates/remote/src/routes/tasks.rs:50-64`; response is `Json(BulkSharedTasksResponse { tasks, deleted_task_ids, latest_seq })` at `:74-84`, with struct fields at `:654-659`. Task 308 mocks a bare array at `docs/plans/vk-swarm-design-system/phase-3/308-wire-boardview-taskdrawer-data.md:33-40` and maps `tasks` directly at `:74-90`; task 310 reachability trace says `Json(bulk_tasks)` at `docs/plans/vk-swarm-design-system/phase-3/310-app-integration-reachability-gate.md:95-98`.  
**Remediation:** Edit task 305 so `tasksApi.bulk(projectId: string): Promise<BulkSharedTasksResponse>` calls `/v1/tasks/bulk?project_id=${encodeURIComponent(projectId)}`. Edit tasks 308/310 to obtain a project id or explicitly stop/escalate if no project selection source exists, then group `response.tasks`, not the whole response.

### FINDING 6: Nodes And Swarm Labels Clients Omit Required `organization_id`
**Lens:** fidelity  
**Severity:** blocker  
**Task(s):** 305, 309, 310  
**Claim:** The planned clients call `/v1/nodes` and `/v1/swarm/labels` without `organization_id`, but both real handlers require it as a query parameter. NodesPage will not render real data.  
**Evidence:** nodes route deserializes `ListNodesQuery { organization_id }` at `crates/remote/src/routes/nodes.rs:361-385`; swarm labels route uses `Query(params): Query<ListSwarmLabelsQuery>` and `params.organization_id` at `crates/remote/src/routes/swarm_labels.rs:123-149`. Task 305 tests only `/v1/nodes` and `/v1/swarm/labels` with no query at `docs/plans/vk-swarm-design-system/phase-3/305-hive-rest-clients.md:39-70`; task 309 calls `nodesApi.list` with no parameters at `docs/plans/vk-swarm-design-system/phase-3/309-wire-nodesview-processesview-data.md:64-66`.  
**Remediation:** Change `nodesApi.list(organizationId)` and `swarmLabelsApi.list(organizationId)` to append `?organization_id=...`. Add a task before 308/309 to source selected organization/project context, or make 308/309 STOP until that context exists.

### FINDING 7: Electric Collection Options Use The Wrong API Shape
**Lens:** fidelity  
**Severity:** blocker  
**Task(s):** 306  
**Claim:** Task 306 says to add new collections with `electricCollectionOptions({ shape: ..., primaryKey: ... })`, but the repo’s existing Electric code uses `shapeOptions: { url }` and `getKey`. Literal implementation will not match the installed API/pattern and likely fails typecheck.  
**Evidence:** existing copy source uses `electricCollectionOptions({ shapeOptions: { url: createShapeUrl(...) }, getKey: ... })` at `frontend/src/lib/electric/collections.ts:78-86` and `:112-120`; task 306 instructs `shape`/`primaryKey` at `docs/plans/vk-swarm-design-system/phase-3/306-electric-collections-task-tables.md:83-87`.  
**Remediation:** Replace those three bullets with the existing API shape: `electricCollectionOptions<ElectricTaskAssignment>({ shapeOptions: { url: createShapeUrl('node_task_assignments') }, getKey: item => item.assignment_id })`, similarly for logs/progress using `item.id`.

### FINDING 8: Task 310 Requires Ledger Edit But Forbids It By Scope
**Lens:** mechanics  
**Severity:** blocker  
**Task(s):** 310  
**Claim:** Task 310’s Done-when requires a non-empty decisions-ledger reachability section, but the ledger is not in `files:` and `allowed_change` is `create`. It also allows editing `index.css` despite `allowed_change: create`.  
**Evidence:** task 310 frontmatter lists only `app-integration.test.tsx` and `index.css` with `allowed_change: create` at `docs/plans/vk-swarm-design-system/phase-3/310-app-integration-reachability-gate.md:9-15`; it then says edit `index.css` if missing imports at `:85-91`; it requires ledger evidence at `:93-99` and Done-when checks the ledger at `:111-112`.  
**Remediation:** Add `docs/plans/vk-swarm-design-system/decisions-ledger.md` to `files:`, change `allowed_change: edit`, and explicitly list the ledger section text the implementer must append.

TALLY: 8 findings (6 blockers, 2 major, 0 minor)
