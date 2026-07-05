# Round 1 — Claude Code CLI Adversarial Breakdown Review

**Target:** `docs/plans/vk-swarm-hive-ui/` decomposition (phases 1–3, tasks 101–308)
**Panelist:** Claude Code CLI (`claude -p … --dangerously-skip-permissions`, v2.1.201)
**Lenses:** mechanics + fidelity
**Date:** 2026-07-04

---

### FINDING 1: Phase 3 electric module is unreachable from `remote-frontend/`
**Lens:** mechanics + fidelity
**Severity:** blocker
**Task(s):** 301, 302, 303, 304, 305
**Claim:** Tasks 301–304 add the three new Electric collection factories to `frontend/src/lib/electric/collections.ts` and export them from `frontend/src/lib/electric/index.ts`. Task 305 then imports them via `@/lib/electric` from inside `remote-frontend/src/pages/Tasks.tsx`. The `@/*` alias set up by task 201 maps to `remote-frontend/src/*`, so `@/lib/electric` resolves to `remote-frontend/src/lib/electric/` — a path that does not exist. The electric module lives in the node frontend, not the hive frontend. There is no bridging task (neither a copy task, nor a Vite/tsconfig alias redirect).

**Evidence:**
- `remote-frontend/src/` directory listing (confirmed above): `App.tsx AppRouter.tsx api.ts index.css main.tsx pages pkce.ts vite-env.d.ts` — no `lib/` directory.
- `task-201` sets `"@/*": ["src/*"]` in `remote-frontend/tsconfig.json`, so `@/lib/electric` → `remote-frontend/src/lib/electric/` (nonexistent).
- Tasks 301–304 only edit `frontend/src/lib/electric/collections.ts` and `frontend/src/lib/electric/index.ts`. Allowed moves sections say "No other file" — no copy to `remote-frontend/`.
- Task 305 STOP trigger reads: "If `@/lib/electric` does not resolve from `remote-frontend/` — HALT; task 201's `@/*` alias is missing or 304's exports are absent." This misdiagnoses the root cause: the `@/*` alias IS set up but resolves to the wrong directory. The alias working correctly is exactly the problem.

**Remediation:** Add a task between 304 and 305 that either (a) copies `frontend/src/lib/electric/` into `remote-frontend/src/lib/electric/` (same copy-not-move pattern as the swarm components in 202), or (b) adds a Vite alias in `remote-frontend/vite.config.ts` redirecting `@/lib/electric` to `path.resolve(__dirname, '../frontend/src/lib/electric/')` (with a matching tsconfig path). Update task 305's STOP trigger to check for the bridging file/alias, not for the `@/*` alias.

---

### FINDING 2: Task 102 oauthApi request bodies don't match hive routes (PKCE fields missing / wrong)
**Lens:** fidelity
**Severity:** blocker
**Task(s):** 102
**Claim:** Task 102 defines `oauthApi.init(provider, returnTo)` posting `{ provider, return_to: returnTo }`. The actual hive `web_init` handler takes `HandoffInitRequest { provider, return_to, app_challenge }` — the PKCE challenge is required. Task 102 also defines `oauthApi.redeem(code, state)` posting `{ code, state }`. The actual `web_redeem` handler takes `HandoffRedeemRequest { handoff_id, app_code, app_verifier }` — every field name is different. An implementer who trusts the task's failing-test spec will write the wrong request payloads.

**Evidence:**
- `crates/utils/src/api/oauth.rs:7-11`: `pub struct HandoffInitRequest { pub provider: String, pub return_to: String, pub app_challenge: String, }` — three fields, not two.
- `crates/utils/src/api/oauth.rs:22-26`: `pub struct HandoffRedeemRequest { pub handoff_id: Uuid, pub app_code: String, pub app_verifier: String, }` — none of the field names match `{ code, state }`.
- `remote-frontend/src/api.ts:37-55` (existing code, already correct): `initOAuth(provider, returnTo, appChallenge)` sends `{ provider, return_to: returnTo, app_challenge: appChallenge }` — the task should be modelling this, not rewriting it incorrectly.

**Remediation:** Update task 102's `oauthApi.init` signature to `init(provider: string, returnTo: string, appChallenge: string)`, body `{ provider, return_to: returnTo, app_challenge: appChallenge }`. Update `oauthApi.redeem` signature to `redeem(handoffId: string, appCode: string, appVerifier: string)`, body `{ handoff_id: handoffId, app_code: appCode, app_verifier: appVerifier }`. Update the failing test accordingly.

---

### FINDING 3: Task 102 test asserts wrong HandoffInitResponse and HandoffRedeemResponse shapes
**Lens:** fidelity
**Severity:** blocker
**Task(s):** 102
**Claim:** Task 102's failing test states `oauthApi.init` returns `HandoffInitResponse` shaped as `{ redirect_url: string }`. The actual hive returns `{ handoff_id: Uuid, authorize_url: String }`. Task 102 also states `oauthApi.redeem` returns `{ profile: ProfileResponse }`. The actual hive `web_redeem` returns `HandoffRedeemResponse { access_token: String, refresh_token: String }` — no profile field at all.

**Evidence:**
- `crates/utils/src/api/oauth.rs:15-18`: `pub struct HandoffInitResponse { pub handoff_id: Uuid, pub authorize_url: String, }` — no `redirect_url`.
- `crates/utils/src/api/oauth.rs:30-33`: `pub struct HandoffRedeemResponse { pub access_token: String, pub refresh_token: String, }` — no `profile`.
- `remote-frontend/src/api.ts:13-21` (existing): `HandoffInitResponse = { handoff_id: string; authorize_url: string; }` and `HandoffRedeemResponse = { access_token: string; refresh_token: string; }` — the existing code is correct.

**Remediation:** Update task 102's `oauthApi.init` test to assert return shape `{ handoff_id: string; authorize_url: string }`. Update `oauthApi.redeem` test to assert return shape `{ access_token: string; refresh_token: string }`. The task's `HandoffInitResponse` type definition in `oauth.ts` must match.

---

### FINDING 4: Tasks 101 and 102 use route paths without the `/v1/` prefix
**Lens:** fidelity
**Severity:** blocker
**Task(s):** 101, 102
**Claim:** Task 101 directs the implementer to `fetch('/profile', ...)` and task 102's `oauthApi` posts to `/oauth/web/init`, `/oauth/web/redeem`, `/oauth/logout`. All hive routes are nested under `/v1/` in the production router, making the correct paths `/v1/profile`, `/v1/oauth/web/init`, `/v1/oauth/web/redeem`, `/v1/oauth/logout`. An implementation following the tasks' paths will get 404s at runtime.

**Evidence:**
- `crates/remote/src/routes/mod.rs:112-113`: `.nest("/v1", v1_public)` + `.nest("/v1", v1_protected)` — every route gains the `/v1` prefix at mount time.
- `oauth::public_router()` declares `/oauth/web/init` (line 27) → mounted as `/v1/oauth/web/init`.
- `oauth::protected_router()` declares `/profile` (line 35) → mounted as `/v1/profile`.
- `remote-frontend/src/api.ts:42`: `fetch(\`${API_BASE}/v1/oauth/web/init\`, ...)` — the existing code already uses `/v1/`.

**Remediation:** In task 101: change `fetch('/profile')` to `fetch('/v1/profile')`. In task 102: change all route strings to include the `/v1/` prefix: `/v1/oauth/web/init`, `/v1/oauth/web/redeem`, `/v1/oauth/logout`, `/v1/profile`.

---

### FINDING 5: Task 307 `tasksApi.assign` targets a user-assignment route with a node-assignment body
**Lens:** fidelity
**Severity:** blocker
**Task(s):** 307
**Claim:** Task 307 states `POST /tasks/{task_id}/assign` body `{ node_id }` "assigns task to a node." The actual `assign_task` handler takes `AssignSharedTaskRequest { new_assignee_user_id: Option<Uuid>, version: Option<i64> }` — this is a GitHub-style user-assignment endpoint, not a node-execution-assignment endpoint. Posting `{ node_id }` will fail deserialization or be silently ignored, and the action described ("assign to a node") does not match the route's actual semantics.

**Evidence:**
- `crates/remote/src/routes/tasks.rs:701-704`: `pub struct AssignSharedTaskRequest { pub new_assignee_user_id: Option<Uuid>, pub version: Option<i64> }`.
- `crates/remote/src/routes/tasks.rs:480-484`: `pub async fn assign_task(... Json(payload): Json<AssignSharedTaskRequest>)` — handler expects `new_assignee_user_id`.
- The `PATCH /tasks/{task_id}/executing-node` handler at line 631 uses `SetExecutingNodeRequest { node_id: Option<Uuid> }` — that IS the node-assignment route, not `/assign`.

**Remediation:** Task 307's "assign to a node" action has no matching REST endpoint in the current hive. Either (a) escalate to spec: the cross-node board's assignment action is not implementable via existing REST routes (needs a new route or a different mechanism), or (b) reframe task 307 to use `PATCH /tasks/{task_id}/executing-node` with body `{ node_id }` for the "reassign to node" action, and drop the distinct "assign" action (which would need a separate user-assignment flow). Record the discrepancy in the decisions ledger.

---

### FINDING 6: Task 101 `ProfileResponse` type has wrong nullability for all optional fields
**Lens:** fidelity
**Severity:** major
**Task(s):** 101
**Claim:** Task 101 defines `ProfileResponse.username: string` (non-nullable) and all `ProviderProfile` fields (`username`, `display_name`, `email`, `avatar_url`) as `string` (non-nullable). The authoritative Rust struct has `username: Option<String>` on `ProfileResponse` and all of those `ProviderProfile` fields as `Option<String>`. TypeScript code consuming these types will assert non-null where the server may return `null`, causing runtime errors.

**Evidence:**
- `crates/utils/src/api/oauth.rs:49-55`: `pub struct ProviderProfile { pub username: Option<String>, pub display_name: Option<String>, pub email: Option<String>, pub avatar_url: Option<String>, }` — all four are optional.
- `crates/utils/src/api/oauth.rs:58-63`: `pub struct ProfileResponse { pub user_id: Uuid, pub username: Option<String>, pub email: String, pub providers: Vec<ProviderProfile>, }` — `username` is optional.
- Task 101 Change section quotes: `username: string` and provider fields as non-nullable `string`.

**Remediation:** Update task 101's `ProfileResponse` type definition: `username: string | null`. Update `ProviderProfile` fields: `username: string | null; display_name: string | null; email: string | null; avatar_url: string | null`. Update the `ProfileState` / `isSignedIn` logic to not assume `username` is non-null.

---

### FINDING 7: Plan.md Phase 2 task table describes task 201 incorrectly
**Lens:** mechanics
**Severity:** major
**Task(s):** 201 (plan.md vs task file)
**Claim:** The plan.md Phase 2 task table shows task 201 as "Port swarm API clients (nodesApi, swarmProjectsApi, swarmLabelsApi, swarmTemplatesApi) into remote-frontend" with `dep: 106`. The actual task file 201 is "Rehost setup: add @/* and shared/* path aliases + copy shared types into remote-frontend" — a completely different task (alias/copy setup) not mentioned anywhere in the plan.md. Porting the swarm API clients was folded into task 202's Change section. An executor reading the plan to pick the next task gets a wrong summary of what 201 does.

**Evidence:**
- `docs/plans/vk-swarm-hive-ui/plan.md:60`: "201 | Port swarm API clients (nodesApi, swarmProjectsApi, swarmLabelsApi, swarmTemplatesApi) into remote-frontend".
- `docs/plans/vk-swarm-hive-ui/phase-2/201-setup-path-aliases.md` title field: `"Rehost setup: add @/* and shared/* path aliases + copy shared types into remote-frontend"`.
- The SC coverage in plan.md says "SC1: 201, 202, 203, 204" — task 201's `covers_criteria: [SC1]` matches but the coverage entry in plan.md now describes the wrong work.

**Remediation:** Update plan.md Phase 2 task table: change task 201's description to "Add @/* and shared/* path aliases + copy shared types into remote-frontend" and note that the swarm API clients are copied as part of task 202.

---

### FINDING 8: Spec's Electric type field names don't match actual DB schema; tasks silently diverge without flagging it
**Lens:** fidelity
**Severity:** major
**Task(s):** 301, 302, 303
**Claim:** The spec's Design section lists the Electric type fields for the three new tables. All three types diverge from the actual DB schema — and therefore from what tasks 301/302/303 implement — without the task Change sections flagging these divergences relative to the spec. An implementer cross-checking the spec against the task will encounter unexplained discrepancies and may trust the spec's field names over the correct DB-derived ones.

**Evidence (per table):**

`ElectricTaskAssignment` — spec: `(id, task_id, node_id, lease_owner, fencing_token, status, assigned_at, leased_until)`. DB (migrations): `execution_status` not `status` (20251202000000:80), `lease_expires_at` not `leased_until` (20260128000001:6), no `lease_owner` column exists, six additional columns not listed in spec. Task 301 correctly uses DB field names but the Change section does not note the divergence from the spec.

`ElectricTaskOutputLog` — spec: `(id, task_id, node_id, entry_index, content, timestamp, stream)`. DB: `assignment_id` (not `task_id`/`node_id`), `output_type` (not `stream`), no `entry_index`. Task 302 correctly uses `assignment_id` + `output_type` but doesn't flag the spec mismatch.

`ElectricTaskProgressEvent` — spec: `(id, task_id, node_id, event_type, payload, timestamp)`. DB: `assignment_id` (not `task_id`/`node_id`), `message` + `metadata` (not `payload`). Task 303 correctly uses DB fields but doesn't flag the spec mismatch.

**Evidence files:**
- `docs/superpowers/specs/2026-07-04-vk-swarm-hive-ui.md:229-231` (spec type field lists).
- `crates/remote/migrations/20251202000000_nodes_swarm.sql:73-85` (assignments schema).
- `crates/remote/migrations/20251202000001_task_output_logs.sql:1-9` (output logs schema).
- `crates/remote/migrations/20251202000002_task_progress_events.sql:1-10` (progress events schema).

**Remediation:** In each of tasks 301, 302, 303, add a note in the Change section (or decisions-ledger record) explicitly listing the spec's type field names alongside the correct DB field names and why the task uses the DB-accurate version. This prevents the next reviewer or an adversarial second-pass from flagging these as fidelity violations.

---

### FINDING 9: ESLint absent from `remote-frontend/package.json` but plan.md gate requires `npm run lint`
**Lens:** mechanics
**Severity:** major
**Task(s):** plan.md gate; 101 through 308 (all remote-frontend tasks)
**Claim:** The plan.md gate command is `cd remote-frontend && npm run lint`. The current `remote-frontend/package.json` has no `lint` script and no `eslint` in devDependencies. No task in the decomposition adds eslint. Tasks 101–106 and 201–308 all include "Manual verification: `npm run lint` exits 0" but none have a STOP trigger for the missing lint toolchain. The final gate will fail unconditionally.

**Evidence:**
- `remote-frontend/package.json` (full content shown above): `"scripts": { "dev": ..., "build": ..., "preview": ... }` — no `lint` entry.
- `devDependencies` section has no eslint or @typescript-eslint packages.
- `plan.md:111`: `cd remote-frontend && npm run lint   # eslint --max-warnings 0` — explicit gate requirement.

**Remediation:** Add to task 102 (or a new pre-phase task) a STOP trigger: "If `remote-frontend/package.json` does not include `eslint` or lacks a `lint` script — add `eslint`, `@typescript-eslint/eslint-plugin`, `@typescript-eslint/parser` as devDependencies and add `"lint": "eslint src --max-warnings 0"` to scripts (record in ledger and copy config from `frontend/.eslintrc*` or equivalent)."

---

### FINDING 10: Task 304 `covers_criteria` is empty but plan.md claims SC5 coverage includes it
**Lens:** mechanics
**Severity:** minor
**Task(s):** 304
**Claim:** Task 304's frontmatter has `covers_criteria: []` (empty). The plan.md SC→task coverage section lists "SC5 (rewired to Electric shapes, no new push): 301, 302, 303, 304, 308." The inconsistency means WAI tooling that reads `covers_criteria` to verify SC coverage will undercount SC5.

**Evidence:**
- `docs/plans/vk-swarm-hive-ui/phase-3/304-export-electric-index.md:14`: `covers_criteria: []`.
- `docs/plans/vk-swarm-hive-ui/plan.md:96`: `SC5 (rewired to Electric shapes, no new push): 301, 302, 303, 304, 308`.

**Remediation:** Change task 304's frontmatter to `covers_criteria: [SC5]`.

---

### FINDING 11: `node_task_output_logs` and `node_task_progress_events` have BIGSERIAL PKs typed as `string`
**Lens:** mechanics
**Severity:** minor
**Task(s):** 302, 303
**Claim:** Both tables use `id BIGSERIAL PRIMARY KEY` (64-bit integer auto-increment). Tasks 302 and 303 type `id: string` in the Electric row types. All existing Electric types (`ElectricNode`, `ElectricProject`, `ElectricNodeProject`) have UUID PKs that are correctly typed as `string`. BIGSERIAL PKs have no established pattern in this codebase. If the Electric SQL client serializes BIGSERIAL as JavaScript `number` (not `string`), the type annotation is wrong and `getKey: (item) => item.id` would return `number` where the type promises `string`.

**Evidence:**
- `crates/remote/migrations/20251202000001_task_output_logs.sql:3`: `id BIGSERIAL PRIMARY KEY`.
- `crates/remote/migrations/20251202000002_task_progress_events.sql:3`: `id BIGSERIAL PRIMARY KEY`.
- `frontend/src/lib/electric/collections.ts:66-70`: `ElectricCollectionConfig<T>` has `getKey: (item: T) => string | number` — so a `number` return from `getKey` is valid, but the `id: string` field type would be a TypeScript lie if Electric sends numbers.
- No existing BIGSERIAL Electric type in the codebase to establish the pattern.

**Remediation:** Add a STOP trigger to tasks 302 and 303: "Before writing the type, verify how `@tanstack/electric-db-collection` serializes BIGSERIAL/BIGINT columns by checking `node_modules/@tanstack/electric-db-collection` or the Electric SQL docs. If it serializes as `number`, change `id: string` to `id: number`; if as `string`, keep `string`. Record in the ledger." As a safe default, use `id: string | number` until verified.

---

### FINDING 12: SC4 enforcement absent from `Done when` gate for tasks 202 and 204
**Lens:** mechanics
**Severity:** minor
**Task(s):** 202, 204
**Claim:** SC4 requires the node frontend to remain unmodified. Tasks 202 and 204 include "Manual verification: `cd frontend && npx tsc --noEmit` exits 0" — correctly requiring the node frontend typecheck. But neither task's `Done when` gate command runs this check. An automated executor running `task-gate.sh` will not verify SC4 and could silently regress it.

**Evidence:**
- Task 202 `Done when`: `WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/swarm/index.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 202` — only checks `remote-frontend`.
- Task 204 `Done when`: `WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/pages/Nodes.parity.test.tsx" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-ui 204` — only checks `remote-frontend`.
- Plan.md gate: `cd frontend && npx tsc --noEmit   # node frontend typecheck (HA fallback untouched)` — the plan requires this check.

**Remediation:** Update the `Done when` commands for tasks 202 and 204 to include the node frontend typecheck, e.g. by running it as a second command in the manual verification and adding a post-gate assertion: `WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit && cd frontend && npx tsc --noEmit" ...` (if `task-gate.sh` supports compound commands), or add a dedicated `WAI_NODE_TYPECHECK_CMD` env var if the gate script supports it.