# Round 1 — Codex Breakdown Review

Panelist: Codex CLI (gpt-5.2 requested; ran on default model due to ChatGPT-account restriction)
Target: `docs/plans/vk-swarm-hive-ui/` (plan + 18 task files + spec at `docs/superpowers/specs/2026-07-04-vk-swarm-hive-ui.md`)
Date: 2026-07-04

Note: The requested model `gpt-5.2` is not supported on this Codex ChatGPT account ("The 'gpt-5.2' model is not supported when using Codex with a ChatGPT account"). The review ran on the account default model so the round could complete.

---

### FINDING 1: Mandatory gate is rewritten incorrectly
**Lens:** mechanics
**Severity:** blocker
**Task(s):** plan-wide
**Claim:** The plan claims to define the AGENTS.md mandatory gate, but replaces the required frontend lint/typecheck with `remote-frontend` lint/typecheck and omits `cd frontend && npm run lint`.
**Evidence:**
`AGENTS.md:8-15`: PR is not done until:
```bash
cargo clippy --all --all-targets --all-features -- -D warnings
cargo test --workspace
cd frontend && npm run lint
cd frontend && npx tsc --noEmit
```
`docs/plans/vk-swarm-hive-ui/plan.md:102-113` says “Gate (per AGENTS.md)” but lists:
```bash
cd remote-frontend && npm run lint
cd remote-frontend && npx tsc --noEmit
cd frontend && npx tsc --noEmit
```
**Remediation:** Replace the plan gate with the four exact AGENTS.md commands, then add any extra `remote-frontend` checks as supplemental, not as substitutes.

### FINDING 2: `remote-frontend` lint/test gates are not real
**Lens:** mechanics
**Severity:** blocker
**Task(s):** 101-106, 201-205, 305-308
**Claim:** Many tasks require `cd remote-frontend && npm run lint` and `npx vitest`, but `remote-frontend/package.json` has no `lint`, `test`, or `vitest` setup. The “Done when” gates are not executable from the current repo.
**Evidence:**
`remote-frontend/package.json:6-9` scripts are only:
```json
"dev": "vite",
"build": "tsc && vite build",
"preview": "vite preview"
```
`remote-frontend/package.json:16-24` devDependencies omit `vitest`, `@testing-library/react`, and ESLint packages.
`docs/plans/vk-swarm-hive-ui/phase-1/101-port-config-provider.md:49-54` requires `npx vitest` and `npm run lint`.
**Remediation:** Add an explicit early task before 101 to install/configure Vitest, Testing Library, jsdom, ESLint, and `lint`/`test` scripts for `remote-frontend`, or change every task gate to commands that actually exist.

### FINDING 3: OAuth client contract is wrong
**Lens:** fidelity
**Severity:** blocker
**Task(s):** 102, 105
**Claim:** Task 102’s OAuth API tests and implementation use the wrong request and response shapes. It omits `app_challenge`, expects `redirect_url`, and redeems with `{ code, state }`, but the real server requires handoff PKCE fields and returns access/refresh tokens.
**Evidence:**
`docs/plans/vk-swarm-hive-ui/phase-1/102-port-oauth-api.md:25-28` says init posts `{ provider, return_to }` and returns `{ redirect_url }`; redeem posts `{ code, state }` and returns `{ profile }`.
`crates/utils/src/api/oauth.rs:7-18` defines init request/response as `provider`, `return_to`, `app_challenge` and `handoff_id`, `authorize_url`.
`crates/utils/src/api/oauth.rs:22-32` defines redeem request/response as `handoff_id`, `app_code`, `app_verifier` and `access_token`, `refresh_token`.
`crates/remote/src/routes/oauth.rs:65-80` passes those exact fields through `web_redeem`.
**Remediation:** Rewrite task 102 to preserve the existing `remote-frontend/src/api.ts` PKCE handoff contract: `init(provider, returnTo, appChallenge)` and `redeem(handoffId, appCode, appVerifier)`, returning `handoff_id/authorize_url` and `access_token/refresh_token`.

### FINDING 4: Top-level API paths omit the `/v1` mount
**Lens:** fidelity
**Severity:** blocker
**Task(s):** 101, 102, 307
**Claim:** The plan says the hive frontend should call `/profile`, `/oauth/web/*`, `/oauth/logout`, and `/tasks/*` directly, but the actual router nests those routes under `/v1`. The generated clients will call non-existent paths.
**Evidence:**
`crates/remote/src/routes/mod.rs:111-114` nests public/protected routers under `/v1`.
`crates/remote/src/routes/oauth.rs:25-36` defines relative routes `/oauth/web/init`, `/profile`, `/oauth/logout`.
`crates/remote/src/routes/tasks.rs:34-43` defines relative routes `/tasks/...`.
`docs/plans/vk-swarm-hive-ui/phase-1/101-port-config-provider.md:32` says `fetch('/profile'...)`.
`docs/plans/vk-swarm-hive-ui/phase-3/307-management-actions-rest.md:72-87` builds URLs like `/tasks/${taskId}/assign`.
**Remediation:** Update all hive API client tasks to use `API_BASE` defaulting to `/v1`, or explicit `/v1/...` paths. Keep invitation/OAuth behavior consistent with existing `remote-frontend/src/api.ts`.

### FINDING 5: Task assignment REST action targets the wrong domain
**Lens:** fidelity
**Severity:** blocker
**Task(s):** 307
**Claim:** Task 307 says `POST /tasks/{id}/assign` assigns a task to a node with `{ node_id }`. The real route assigns/reassigns the human task assignee with `{ new_assignee_user_id, version }`; node execution location is a different endpoint.
**Evidence:**
`docs/plans/vk-swarm-hive-ui/phase-3/307-management-actions-rest.md:59-62` claims `/assign` body is `{ node_id }`.
`crates/remote/src/routes/tasks.rs:480-485` handles `assign_task` with `AssignSharedTaskRequest`.
`crates/remote/src/routes/tasks.rs:700-704` defines:
```rust
pub struct AssignSharedTaskRequest {
    pub new_assignee_user_id: Option<Uuid>,
    pub version: Option<i64>,
}
```
`crates/remote/src/routes/tasks.rs:617-647` shows `SetExecutingNodeRequest { node_id: Option<Uuid> }` belongs to `/tasks/{task_id}/executing-node`.
**Remediation:** Change task 307: either remove node assign/reassign from this workstream, or wire node execution changes only to `/v1/tasks/{id}/executing-node` and use `/assign` only for user assignee changes.

### FINDING 6: Hive board imports Electric code that is only edited in node frontend
**Lens:** mechanics
**Severity:** blocker
**Task(s):** 301-305
**Claim:** Tasks 301-304 add/export Electric collections only in `frontend/src/lib/electric`, but task 305 creates `remote-frontend/src/pages/Tasks.tsx` importing `@/lib/electric`. In `remote-frontend`, `@/*` resolves to `remote-frontend/src/*`, where no `lib/electric` is created.
**Evidence:**
`docs/plans/vk-swarm-hive-ui/phase-3/304-export-electric-index.md:22-61` edits only `frontend/src/lib/electric/index.ts`.
`docs/plans/vk-swarm-hive-ui/phase-3/305-tasks-board-page.md:73-80` imports from `@/lib/electric`.
`docs/plans/vk-swarm-hive-ui/phase-2/201-setup-path-aliases.md:30-34` maps `@/*` to `src/*` inside `remote-frontend`.
`remote-frontend/tsconfig.json:19` includes only `src`.
**Remediation:** Add a task to copy or create `remote-frontend/src/lib/electric/{config,collections,index}.ts` plus dependencies, or change task 201 to explicitly alias `@/lib/electric` to the repo-root frontend module and make Vite/TS include it safely.

### FINDING 7: Missing attempt/execution collections for SC2
**Lens:** fidelity
**Severity:** major
**Task(s):** 301-306
**Claim:** SC2 requires cross-node views for tasks, attempts, and executions. The plan only adds assignment/output/progress collections and treats logs as attempts. The repo has first-class `node_task_attempts` and `node_execution_processes` tables, but no task adds collections or UI over them.
**Evidence:**
`docs/superpowers/specs/2026-07-04-vk-swarm-hive-ui.md:82-86` requires “tasks, attempts, and executions across all connected nodes”.
`crates/remote/migrations/20251229100000_sync_task_attempts.sql:3-17` creates `node_task_attempts`.
`crates/remote/migrations/20251229100000_sync_task_attempts.sql:25-40` creates `node_execution_processes`.
`docs/plans/vk-swarm-hive-ui/plan.md:67-69` only names `node_task_assignments`, `node_task_output_logs`, and `node_task_progress_events`.
`docs/plans/vk-swarm-hive-ui/phase-3/306-task-detail-panel.md:4` says attempts are “output logs”.
**Remediation:** Add tasks to publish/configure collections for `node_task_attempts` and `node_execution_processes`, then update board/detail UI to render actual attempts and executions, using output logs only as logs.

### FINDING 8: `remote-frontend` lacks required TanStack Electric dependencies
**Lens:** mechanics
**Severity:** major
**Task(s):** 305, 306
**Claim:** Task 305 imports `@tanstack/react-db` and relies on Electric collections, but `remote-frontend/package.json` does not include `@tanstack/react-db`, `@tanstack/electric-db-collection`, or `@electric-sql/client`.
**Evidence:**
`docs/plans/vk-swarm-hive-ui/phase-3/305-tasks-board-page.md:73` imports `useCollection` from `@tanstack/react-db`.
`frontend/package.json:29,52-53` has Electric/TanStack DB dependencies in the node frontend.
`remote-frontend/package.json:11-15` has only `react`, `react-dom`, and `react-router-dom` dependencies.
**Remediation:** Add an explicit dependency setup task before 305 to install the Electric/TanStack DB packages into `remote-frontend`, and make 305 depend on it.

### FINDING 9: Task 201 file list does not match its change section
**Lens:** mechanics
**Severity:** major
**Task(s):** 201
**Claim:** The task metadata says it creates one shared file and does not list Vite config, but the change section creates a different path and edits Vite config. This violates the file-list/Change consistency rule.
**Evidence:**
`docs/plans/vk-swarm-hive-ui/phase-2/201-setup-path-aliases.md:9-16` lists `remote-frontend/src/types/shared.ts` and `allowed_change: create`.
`docs/plans/vk-swarm-hive-ui/phase-2/201-setup-path-aliases.md:25-34` edits `remote-frontend/tsconfig.json`.
`docs/plans/vk-swarm-hive-ui/phase-2/201-setup-path-aliases.md:39-47` creates `remote-frontend/src/types/shared/types.ts` and edits `remote-frontend/src/vite.config.ts`.
`remote-frontend/vite.config.ts:1-9` exists at `remote-frontend/vite.config.ts`, not under `src`.
**Remediation:** Update frontmatter files to `remote-frontend/tsconfig.json`, `remote-frontend/vite.config.ts`, `shared/types.ts`, `remote-frontend/src/types/shared/types.ts`, and change `allowed_change` to `edit/create`.

### FINDING 10: No-push “failing test” is explicitly green-on-arrival
**Lens:** mechanics
**Severity:** major
**Task(s):** 308
**Claim:** Task 308’s “Failing test” section is not a real failing test. It admits the guard will pass initially and suggests adding a temporary comment to force red. That violates the decomposition requirement that each failing test be real, not a TODO/manual trick.
**Evidence:**
`docs/plans/vk-swarm-hive-ui/phase-3/308-no-push-invariant.md:57` says the test “won’t” fail and is “green-on-arrival”.
`docs/plans/vk-swarm-hive-ui/phase-3/308-no-push-invariant.md:57-58` suggests adding a temporary `// WebSocket` line and removing it.
**Remediation:** Either reclassify 308 as a verification-only guard with no “Failing test” claim, or make it a real failing test by first introducing the guarded behavior in a prior task and asserting the guard catches a real regression fixture committed under `scripts/fixtures`.

### FINDING 11: Post-phase integrated review gate is missing
**Lens:** mechanics
**Severity:** blocker
**Task(s):** plan-wide
**Claim:** AGENTS.md requires an integrated adversarial review after each WAI phase before moving to the next phase. The plan contains no phase-review tasks or report outputs.
**Evidence:**
`AGENTS.md:44-53` requires “After completing each WAI phase, run an integrated adversarial review” and gives report path `.agents/reports/YYYY-MM-DD-round-N-...md`.
`rg -n "integrated adversarial|adversarial review|\\.agents/reports|round-|review" docs/plans/vk-swarm-hive-ui/plan.md docs/plans/vk-swarm-hive-ui/phase-*/*.md` returns only unrelated NormalLayout text, no phase review gate.
**Remediation:** Add review tasks after phases 1, 2, and 3, each producing the required `.agents/reports/...` report and making the next phase depend on the prior phase’s integrated review.