# Round 1 — Gemini Breakdown Review (vk-swarm-hive-ui)

**Reviewer:** Gemini CLI (gemini-3.x pipeline, headless `-p` mode)
**Date:** 2026-07-04
**Target:** `docs/plans/vk-swarm-hive-ui/` decomposition (plan.md + 18 task files) vs. frozen spec `docs/superpowers/specs/2026-07-04-vk-swarm-hive-ui.md`
**Lenses:** mechanics + fidelity
**Invocation:** `gemini -p "$(cat /tmp/opencode/hive-ui-breakdown-review-prompt.md)" --skip-trust`

---

### FINDING 1: Missing Critical Dependencies for Rehosted Swarm Components
**Lens:** mechanics
**Severity:** blocker
**Task(s):** 202
**Claim:** Task 202 instructs the implementer to copy the existing swarm components verbatim into `remote-frontend/src/components/swarm/`. However, it fails to copy or ensure the existence of several critical hooks, label components, utility functions, and Shadcn UI components that these swarm components import. Since `remote-frontend` contains only a minimal skeleton with no UI library components or custom hooks, the copied components will fail to compile and render.
**Evidence:**
* In `frontend/src/components/swarm/NodeCard.tsx:3`:
  `import { cn } from '@/lib/utils';`
* In `frontend/src/components/swarm/SwarmHealthSection.tsx:20`:
  `import { useSwarmHealth } from '@/hooks/useSwarmHealth';`
* In `frontend/src/components/swarm/SwarmLabelsSection.tsx:24-27`:
  ```tsx
  import { useSwarmLabels, useSwarmLabelMutations } from '@/hooks/useSwarmLabels';
  ...
  import { LabelBadge } from '@/components/labels/LabelBadge';
  ```
* In `frontend/src/components/swarm/SwarmLabelDialog.tsx:16-17`:
  ```tsx
  import { ColorPicker } from '@/components/labels/ColorPicker';
  import { IconPicker } from '@/components/labels/IconPicker';
  ```
* UI component imports across multiple files under `frontend/src/components/swarm/` (e.g., `NodeProjectsSection.tsx`, `SwarmProjectDialog.tsx`):
  ```tsx
  import { Button } from '@/components/ui/button';
  import { Badge } from '@/components/ui/badge';
  import { Alert, AlertDescription } from '@/components/ui/alert';
  import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
  import { Input } from '@/components/ui/input';
  import { Label } from '@/components/ui/label';
  import { Textarea } from '@/components/ui/textarea';
  import { TooltipProvider } from '@/components/ui/tooltip';
  ```
* Listing the contents of the `remote-frontend/src/` directory confirms these dependencies do not exist:
  ```
  remote-frontend/src: [DIR] pages, api.ts, App.tsx, AppRouter.tsx, index.css, main.tsx, pkce.ts, vite-env.d.ts
  ```
**Remediation:**
Extend the scope of task 202 (or add a pre-requisite task in Phase 2) to copy all required dependencies from `frontend/` to `remote-frontend/`:
1. Add `remote-frontend/src/lib/utils.ts` to `files:` and copy `frontend/src/lib/utils.ts` verbatim.
2. Copy the required UI components from `frontend/src/components/ui/` into `remote-frontend/src/components/ui/` (specifically `button.tsx`, `badge.tsx`, `alert.tsx`, `tabs.tsx`, `input.tsx`, `label.tsx`, `textarea.tsx`, `tooltip.tsx`, `dialog.tsx`, `popover.tsx`, `select.tsx`, `switch.tsx`).
3. Copy the swarm hooks `useSwarmHealth.ts`, `useSwarmLabels.ts`, `useSwarmProjects.ts`, `useSwarmTemplates.ts`, `useSwarmHealthActions.ts` from `frontend/src/hooks/` to `remote-frontend/src/hooks/`.
4. Copy the entire labels component folder `frontend/src/components/labels/` to `remote-frontend/src/components/labels/` (including `LabelBadge.tsx`, `ColorPicker.tsx`, `IconPicker.tsx`, `index.ts`).

---

### FINDING 2: Hallucinated Handoff Redemption Endpoint Contract
**Lens:** fidelity | mechanics
**Severity:** blocker
**Task(s):** 102
**Claim:** Task 102 defines the signature and contract for `oauthApi.redeem` as accepting `(code, state)`, performing a POST request to `/oauth/web/redeem` with the payload `{ code, state }`, and returning `{ profile: ProfileResponse }`. This is a hallucinated API contract. The actual Hive server's `web_redeem` route accepts `HandoffRedeemRequest` (which takes `handoff_id`, `app_code`, and `app_verifier`) and returns `HandoffRedeemResponse` (which contains `{ access_token, refresh_token }`).
**Evidence:**
* In `docs/plans/vk-swarm-hive-ui/phase-1/102-port-oauth-api.md`:
  `2. oauthApi.redeem(code, state) POSTs /oauth/web/redeem with { code, state } and returns { profile: ProfileResponse } (or whatever the hive returns — verify against crates/remote/src/routes/oauth.rs web_redeem).`
* In `crates/remote/src/routes/oauth.rs:65-72`:
  ```rust
  pub async fn web_redeem(
      State(state): State<AppState>,
      Json(payload): Json<HandoffRedeemRequest>,
  ) -> Response {
      let handoff = state.handoff();

      match handoff
          .redeem(payload.handoff_id, &payload.app_code, &payload.app_verifier)
  ```
* In `crates/utils/src/api/oauth.rs:22-33`:
  ```rust
  pub struct HandoffRedeemRequest {
      pub handoff_id: Uuid,
      pub app_code: String,
      pub app_verifier: String,
  }

  pub struct HandoffRedeemResponse {
      pub access_token: String,
      pub refresh_token: String,
  }
  ```
* In `remote-frontend/src/api.ts:57-78`:
  ```typescript
  export async function redeemOAuth(
    handoffId: string,
    appCode: string,
    appVerifier: string,
  ): Promise<HandoffRedeemResponse> {
  ```
**Remediation:**
Modify task 102's description, implementation details, and tests to correctly match the real schema:
* `oauthApi.redeem` must take `(handoffId: string, appCode: string, appVerifier: string)` as arguments.
* It must send a POST payload of `{ handoff_id: handoffId, app_code: appCode, app_verifier: appVerifier }` to `/v1/oauth/web/redeem`.
* It must return a promise resolving to `HandoffRedeemResponse` (`{ access_token: string, refresh_token: string }`).

---

### FINDING 3: Missing `/v1` Path Prefix for Hive REST API Requests
**Lens:** fidelity
**Severity:** blocker
**Task(s):** 101, 102, 307
**Claim:** The task files specify fetching endpoints like `/profile`, `/oauth/logout`, `/oauth/web/init`, and `/tasks/*` directly. However, the Hive server mounts all user/protected routes under a `/v1` router namespace. Fetching without the `/v1` prefix will result in immediate 404 errors.
**Evidence:**
* In `crates/remote/src/routes/mod.rs:111-113`:
  ```rust
  Router::<AppState>::new()
      .nest("/v1", v1_public)
      .nest("/v1", v1_protected)
  ```
* Existing correct fetches in `remote-frontend/src/api.ts:28-40` use the prefix:
  `const res = await fetch(`${API_BASE}/v1/invitations/${token}`);`
  `const res = await fetch(`${API_BASE}/v1/oauth/web/init`, ...`
* In `101-port-config-provider.md`:
  `fetch('/profile', { credentials: 'include' })`
* In `102-port-oauth-api.md`:
  `1. oauthApi.init(provider, returnTo) POSTs /oauth/web/init`
  `3. oauthApi.logout() POSTs /oauth/logout`
  `4. profileApi.get() GETs /profile`
* In `307-management-actions-rest.md`:
  `makeRequest(`${API_BASE}/tasks/${taskId}/assign`, ...)`
  `makeRequest(`${API_BASE}/tasks/${taskId}/executing-node`, ...)`
  `makeRequest(`${API_BASE}/tasks/${taskId}`, ...)`
**Remediation:**
Explicitly adjust the request URLs in all relevant task files (101, 102, 307) to use the `/v1` path prefix:
* Replace `/profile` with `/v1/profile`
* Replace `/oauth/logout` with `/v1/oauth/logout`
* Replace `/oauth/web/init` with `/v1/oauth/web/init`
* Replace `/oauth/web/redeem` with `/v1/oauth/web/redeem`
* Update `tasksApi` base paths in 307 to:
  * `${API_BASE}/v1/tasks/${taskId}/assign`
  * `${API_BASE}/v1/tasks/${taskId}/executing-node`
  * `${API_BASE}/v1/tasks/${taskId}`

---

### FINDING 4: Out-of-sync Task 201 Title and Purpose in `plan.md`
**Lens:** mechanics
**Severity:** minor
**Task(s):** 201, 202
**Claim:** There is an inconsistency between the table of tasks in `plan.md` and the actual task 201 file. `plan.md` describes task 201 as "Port swarm API clients ...", but the actual task file `201-setup-path-aliases.md` defines it as setting up path aliases and copying shared types. Meanwhile, task 202 in `plan.md` is "Copy swarm components tree...", but the actual task file `202-copy-swarm-components.md` copies both swarm components AND API clients.
**Evidence:**
* In `docs/plans/vk-swarm-hive-ui/plan.md`:
  `201 | Port swarm API clients (nodesApi, swarmProjectsApi, swarmLabelsApi, swarmTemplatesApi) into remote-frontend | dep: 106 | conflicts: none`
* In `docs/plans/vk-swarm-hive-ui/phase-2/201-setup-path-aliases.md` YAML frontmatter and title:
  ```yaml
  id: "201"
  title: "Rehost setup: add @/* and shared/* path aliases + copy shared types into remote-frontend"
  ```
* In `docs/plans/vk-swarm-hive-ui/phase-2/202-copy-swarm-components.md` YAML frontmatter and title:
  ```yaml
  id: "202"
  title: "Rehost: copy swarm components + API clients + types into remote-frontend (node frontend kept as HA fallback)"
  ```
**Remediation:**
Update the table of tasks for Phase 2 in `docs/plans/vk-swarm-hive-ui/plan.md` to match the exact titles and responsibilities defined in the actual task files:
* Change Task 201's title in the Phase 2 table to: "Rehost setup: add @/* and shared/* path aliases + copy shared types into remote-frontend"
* Change Task 202's title in the Phase 2 table to: "Rehost: copy swarm components + API clients + types into remote-frontend"

---

### FINDING 5: Mismatched Test File Extension (ts vs tsx) in Task 103
**Lens:** mechanics
**Severity:** minor
**Task(s):** 103
**Claim:** The metadata header `scope_test` specifies a `.ts` file, but the `## Failing test` section refers to a `.tsx` file. Because this test renders a React component/context to verify a hook, it must use `.tsx`.
**Evidence:**
* In `docs/plans/vk-swarm-hive-ui/phase-1/103-port-use-auth.md`:
  * Frontmatter: `scope_test: "remote-frontend/src/hooks/auth/useAuth.test.ts"`
  * Section title: `## Failing test (write first) File: remote-frontend/src/hooks/auth/useAuth.test.tsx`
**Remediation:**
Change the `scope_test:` value in the YAML frontmatter of `103-port-use-auth.md` to `"remote-frontend/src/hooks/auth/useAuth.test.tsx"`.

---

### FINDING 6: Incomplete `files` List in Task 104
**Lens:** mechanics
**Severity:** minor
**Task(s):** 104
**Claim:** The `files:` metadata list in task 104's frontmatter does not include the child components `Navbar.tsx` and `BottomNav.tsx`, which are imported by `NormalLayout.tsx` and must be ported or customized alongside it.
**Evidence:**
* In `docs/plans/vk-swarm-hive-ui/phase-1/104-port-normal-layout.md`:
  `files:` only lists `frontend/src/components/layout/NormalLayout.tsx` and `remote-frontend/src/components/layout/NormalLayout.tsx`.
* The `## Change` section says:
  `Create remote-frontend/src/components/layout/NormalLayout.tsx + any sub-components it needs (Navbar, BottomNav) + the test file.`
* In `frontend/src/components/layout/NormalLayout.tsx:3-4`:
  ```tsx
  import { Navbar } from '@/components/layout/Navbar';
  import { BottomNav } from '@/components/layout/BottomNav';
  ```
**Remediation:**
Add the navbar and bottomnav file paths to the `files:` array in task 104's YAML metadata header:
* `- frontend/src/components/layout/Navbar.tsx`
* `- remote-frontend/src/components/layout/Navbar.tsx`
* `- frontend/src/components/layout/BottomNav.tsx`
* `- remote-frontend/src/components/layout/BottomNav.tsx`

---

### FINDING 7: Ambiguous and Conditional URL Adjustment in Task 202
**Lens:** mechanics
**Severity:** minor
**Task(s):** 202
**Claim:** Task 202 leaves the verification and editing of copied API clients' URL constants as a conditional instruction for the implementer instead of specifying the concrete URL change. This introduces ambiguity into the implementation phase.
**Evidence:**
* In `docs/plans/vk-swarm-hive-ui/phase-2/202-copy-swarm-components.md`:
  `If the hive route prefix differs from /api/, the copied API clients need their base URLs adjusted. ... If they differ, edit the copied files' URL constants and record the delta in the ledger.`
* The hive server indeed serves these routes with the `/v1` prefix (e.g. `/v1/nodes`), while the node frontend API clients use `/api` (e.g. `/api/nodes`).
**Remediation:**
Make the instruction in `202-copy-swarm-components.md` explicit and deterministic: state clearly that the base URLs in the copied API clients must be changed from `/api/` to `/v1/` because the hive server routes nodes and other swarm-management resources under `/v1` (e.g. `/v1/nodes`).

---

### FINDING 8: Environment Configuration Planning Gap
**Lens:** mechanics & fidelity
**Severity:** major
**Task(s):** 101, 102, 103, 104, 105, 106, 201, 202
**Claim:** The task files and `plan.md` mandate linting (`npm run lint`), typechecking (`tsc --noEmit`), and running unit/integration tests with vitest inside `remote-frontend`. However, `remote-frontend/package.json` does not contain vitest, testing-library, eslint, or any lint script. The task files handle this by having a "STOP trigger" inside individual tasks to "add vitest/testing library" if missing, which is a structural planning anti-pattern that leads to ad-hoc, untracked changes across multiple tasks.
**Evidence:**
* In `remote-frontend/package.json`, there is no `eslint`, `vitest`, or `@testing-library/react`, and no `lint` script in `scripts`.
* Every Phase 1 and Phase 2 task lists running `vitest` or `npm run lint` as part of its manual verification and `Done when` check.
* Tasks like 101 and 102 have STOP triggers to add missing dependencies as devDependencies.
**Remediation:**
Introduce a dedicated setup task at the start of Phase 1 (e.g., "100-setup-remote-frontend-environment") to configure `eslint`, `vitest`, `@testing-library/react`, and add the corresponding scripts (`lint`, `test`, `test:run`) to `remote-frontend/package.json` and configure `vitest.config.ts`. This ensures the development environment is properly initialized before any components or hooks are implemented and verified.