An adversarial audit has been performed on the `vk-swarm-design-system` plan (`docs/plans/vk-swarm-design-system/plan.md`) and its 24 task files. 16 findings have been identified, including **5 critical blockers** that would prevent the compilation, testing, or runtime execution of the design system.

---

### FINDING 1: Broken child process `cwd: '..'` path in Integration Smoke Test
**Lens:** fidelity
**Severity:** blocker
**Task(s):** 106
**Claim:** The integration smoke test in task 106 executes `npm run build`, `npx tsc --noEmit`, and `npx eslint` with `cwd: '..'`. In the monorepo structure, `vitest` runs from `remote-frontend/`. Specifying `cwd: '..'` runs these commands in the workspace root (`hidden-sailor/`). However, the workspace root contains neither a `tsconfig.json` nor an `npm run build` script. Executing this test will trigger an immediate failure across all three test cases.
**Evidence:** `docs/plans/vk-swarm-design-system/phase-1/106-token-texture-tests.md:21-35`
**Remediation:** Remove `{ cwd: '..' }` or change it to `{ cwd: '.' }` in `remote-frontend/src/styles/tokens/smoke.test.ts` so that commands run inside the `remote-frontend/` subdirectory where the scripts, `tsconfig.json`, and ESLint configurations reside.

---

### FINDING 2: Critical file name mismatch (typo) under Change section
**Lens:** mechanics
**Severity:** blocker
**Task(s):** 306
**Claim:** The frontmatter `files:` list specifies `remote-frontend/src/lib/electric/electric.test.ts`. However, under the Change section, the markdown header instructs the implementer to create `remote-frontend/src/lib/electric/eric.test.ts` (CREATE). A literal Haiku implementer will write `eric.test.ts`. The Done-when gate command executes `npx vitest run src/lib/electric/electric.test.ts`, which will fail because the test file is named incorrectly.
**Evidence:** `docs/plans/vk-swarm-design-system/phase-3/306-electric-collections-task-tables.md:13` versus `line 69` (`File: remote-frontend/src/lib/electric/eric.test.ts (CREATE)`).
**Remediation:** Edit task 306 line 69 to say: `### File: remote-frontend/src/lib/electric/electric.test.ts (CREATE)`

---

### FINDING 3: Incompatible parameters for `electricCollectionOptions`
**Lens:** fidelity
**Severity:** blocker
**Task(s):** 306
**Claim:** The task instructs the implementer to define the three new task collections using `shape: { url: ... }` and `primaryKey: [...]`. However, the established and compiler-proven pattern in the reference file `frontend/src/lib/electric/collections.ts` requires `shapeOptions: { url: ... }` and `getKey: (item) => ...`. Writing the properties as instructed in the task file will cause immediate TypeScript compilation errors against `@tanstack/electric-db-collection@^0.3.12`.
**Evidence:** `frontend/src/lib/electric/collections.ts:77` vs `docs/plans/vk-swarm-design-system/phase-3/306-electric-collections-task-tables.md:73-82`
**Remediation:** Update the collection descriptions in task 306 to use the correct `@tanstack` parameters:
- `shapeOptions: { url: createShapeUrl('node_task_assignments') }, getKey: (item) => item.assignment_id`
- `shapeOptions: { url: createShapeUrl('node_task_output_logs') }, getKey: (item) => item.id`
- `shapeOptions: { url: createShapeUrl('node_task_progress_events') }, getKey: (item) => item.id`

---

### FINDING 4: Missing required query parameters in Axum REST API clients
**Lens:** fidelity
**Severity:** blocker
**Task(s):** 305, 308, 309
**Claim:** The Axum routing layer strictly enforces Serde deserialization of query parameters. Specifically, `GET /v1/nodes` requires an `organization_id` query parameter (`ListNodesQuery`), `GET /v1/tasks/bulk` requires a `project_id` query parameter (`BulkTasksQuery`), and `GET /v1/swarm/labels` requires an `organization_id` query parameter (`ListSwarmLabelsQuery`). However, the API clients created in task 305 and their usages in tasks 308/309 do not define or pass these required query parameters. Every REST call to list nodes, tasks, or swarm labels in the hive will fail at runtime with `400 Bad Request`.
**Evidence:** `crates/remote/src/routes/nodes.rs:361-364`, `crates/remote/src/routes/tasks.rs:51-53`, and `crates/remote/src/routes/swarm_labels.rs:30-33`
**Remediation:** 
1. Edit task 305 to update `nodesApi.list(orgId)`, `tasksApi.bulk(projectId)`, and `swarmLabelsApi.list(orgId)` to accept and append query parameters to the request URL: e.g., `${API_BASE}/v1/nodes?organization_id=${orgId}`.
2. Edit tasks 308 and 309 to first query `/v1/organizations` to obtain the available organization ID, query `/v1/swarm/projects?organization_id=...` to find the project ID, and then pass those IDs down to the respective API calls.

---

### FINDING 5: Mismatched REST payload structure for `tasksApi.bulk`
**Lens:** fidelity
**Severity:** blocker
**Task(s):** 305, 308
**Claim:** The task 308 `BoardPage.tsx` assumes that `tasksApi.bulk()` returns a flat array of tasks (`Task[]`), and the Vitest mock returns a bare array (`mockTasks`). However, the Rust Axum backend's `/v1/tasks/bulk` handler returns a `BulkSharedTasksResponse` object wrapper: `{ tasks: Vec<SharedTask>, deleted_task_ids: Vec<Uuid>, latest_seq: i64 }`. At runtime, `BoardPage.tsx` will receive an object instead of an array. The attempt to loop over it (`for (const t of tasks)`) will throw a runtime type error ("tasks is not iterable").
**Evidence:** `crates/remote/src/routes/tasks.rs:78-82` vs `docs/plans/vk-swarm-design-system/phase-3/308-wire-boardview-taskdrawer-data.md:21-35`
**Remediation:** 
1. Edit task 305 to type `tasksApi.bulk`'s return signature as `Promise<{ tasks: Task[], deleted_task_ids: string[], latest_seq: number }>`.
2. Edit task 308 to unpack the returned object: `const tasks = tasksQuery.data?.tasks ?? []` in `BoardPage.tsx`.
3. Update the Vitest mock in `BoardPage.test.tsx` to return `{ tasks: mockTasks, deleted_task_ids: [], latest_seq: 1 }`.

---

### FINDING 6: `allowed_change` mismatch in task 101
**Lens:** mechanics
**Severity:** major
**Task(s):** 101
**Claim:** Task 101 edits `remote-frontend/.prettierignore` (an existing file), but the frontmatter specifies `allowed_change: create`. Additionally, `.prettierignore` is omitted from the frontmatter `files:` array, violating the task validation schema.
**Evidence:** `docs/plans/vk-swarm-design-system/phase-1/101-port-color-typography-tokens.md:16` and `line 110`
**Remediation:** Append `- remote-frontend/.prettierignore` to the `files:` list, and change `allowed_change` to `mixed`.

---

### FINDING 7: `allowed_change` mismatch in task 105
**Lens:** mechanics
**Severity:** major
**Task(s):** 105
**Claim:** Task 105 creates a brand new unit test file (`remote-frontend/src/styles/tokens/index.test.ts`), but its frontmatter restricts operations to `allowed_change: edit`.
**Evidence:** `docs/plans/vk-swarm-design-system/phase-1/105-wire-tokens-into-index.md:14`
**Remediation:** Change `allowed_change: edit` to `allowed_change: mixed`.

---

### FINDING 8: `allowed_change` mismatch in task 203
**Lens:** mechanics
**Severity:** major
**Task(s):** 203
**Claim:** Task 203 creates three brand new component files (`Input.tsx`, `Switch.tsx`, `Checkbox.tsx`) and a test file (`input-switch-checkbox.test.tsx`), but its frontmatter specifies `allowed_change: edit`.
**Evidence:** `docs/plans/vk-swarm-design-system/phase-2/203-port-input-switch-checkbox.md:15`
**Remediation:** Change `allowed_change: edit` to `allowed_change: mixed`.

---

### FINDING 9: `allowed_change` mismatch in task 204
**Lens:** mechanics
**Severity:** major
**Task(s):** 204
**Claim:** Task 204 creates three brand new core components (`Tabs.tsx`, `Select.tsx`, `Loader.tsx`) and a test file, but its frontmatter specifies `allowed_change: edit`.
**Evidence:** `docs/plans/vk-swarm-design-system/phase-2/204-port-tabs-select-loader.md:15`
**Remediation:** Change `allowed_change: edit` to `allowed_change: mixed`.

---

### FINDING 10: `allowed_change` mismatch in task 206
**Lens:** mechanics
**Severity:** major
**Task(s):** 206
**Claim:** Task 206 creates the new component `NodeCard.tsx` and its test `nodecard.test.tsx`, but its frontmatter specifies `allowed_change: edit`.
**Evidence:** `docs/plans/vk-swarm-design-system/phase-2/206-port-nodecard.md:14`
**Remediation:** Change `allowed_change: edit` to `allowed_change: mixed`.

---

### FINDING 11: `allowed_change` mismatch in task 208
**Lens:** mechanics
**Severity:** major
**Task(s):** 208
**Claim:** Task 208 creates a new test file `render-parity.test.tsx` beside editing `index.css`, but its frontmatter specifies `allowed_change: edit`.
**Evidence:** `docs/plans/vk-swarm-design-system/phase-2/208-component-render-parity-tests.md:14`
**Remediation:** Change `allowed_change: edit` to `allowed_change: mixed`.

---

### FINDING 12: `allowed_change` mismatch in task 304
**Lens:** mechanics
**Severity:** major
**Task(s):** 304
**Claim:** Task 304 creates four brand new files (`TaskDrawer.tsx`, `DiffPanel.tsx`, `LogsPanel.tsx`, `AttemptsPanel.tsx`, and `drawer.test.tsx`), but its frontmatter specifies `allowed_change: edit`.
**Evidence:** `docs/plans/vk-swarm-design-system/phase-3/304-port-taskdrawer-diff-logs-attempts.md:16`
**Remediation:** Change `allowed_change: edit` to `allowed_change: mixed`.

---

### FINDING 13: `allowed_change` mismatch in task 308
**Lens:** mechanics
**Severity:** major
**Task(s):** 308
**Claim:** Task 308 creates brand new files (`BoardPage.tsx`, `BoardPage.test.tsx`), but its frontmatter specifies `allowed_change: edit`.
**Evidence:** `docs/plans/vk-swarm-design-system/phase-3/308-wire-boardview-taskdrawer-data.md:14`
**Remediation:** Change `allowed_change: edit` to `allowed_change: mixed`.

---

### FINDING 14: `allowed_change` mismatch in task 309
**Lens:** mechanics
**Severity:** major
**Task(s):** 309
**Claim:** Task 309 creates brand new files (`NodesPage.tsx`, `ProcessesPage.tsx`, `NodesPage.test.tsx`), but its frontmatter specifies `allowed_change: edit`.
**Evidence:** `docs/plans/vk-swarm-design-system/phase-3/309-wire-nodesview-processesview-data.md:15`
**Remediation:** Change `allowed_change: edit` to `allowed_change: mixed`.

---

### FINDING 15: `allowed_change` mismatch in task 310
**Lens:** mechanics
**Severity:** major
**Task(s):** 310
**Claim:** Task 310 edits `index.css` (existing file) while creating `app-integration.test.tsx` (new file), but its frontmatter specifies `allowed_change: create`.
**Evidence:** `docs/plans/vk-swarm-design-system/phase-3/310-app-integration-reachability-gate.md:14`
**Remediation:** Change `allowed_change: create` to `allowed_change: mixed`.

---

### FINDING 16: Broken test environment setup in `AppRouter` integration test
**Lens:** fidelity
**Severity:** major
**Task(s):** 307
**Claim:** The appended test in task 307 renders `RouterProvider` directly without wrapping it in a `QueryClientProvider` or mocking `useProfile`'s return value. The underlying route pages and layouts expect these providers/mocks (the existing test file uses `renderWithRouter` helper which handles this configuration). This will trigger immediate React hook and rendering context errors during test execution.
**Evidence:** `remote-frontend/src/AppRouter.test.tsx` references `QueryClientProvider` and `useProfile` mocking for all authed/unauthed tests.
**Remediation:** Update `307-integrate-chrome-into-approuter.md`'s appended test to use the existing, robust `renderWithRouter('/nodes')` helper instead of raw `render(<RouterProvider ... />)`.

---

TALLY: 16 findings (5 blockers, 11 major, 0 minor)
