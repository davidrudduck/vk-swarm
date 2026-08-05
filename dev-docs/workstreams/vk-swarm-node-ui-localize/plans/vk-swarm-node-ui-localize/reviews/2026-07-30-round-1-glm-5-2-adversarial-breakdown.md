# Adversarial breakdown review — vk-swarm-node-ui-localize

Reviewer: GLM-5.2 (opencode). Date: 2026-07-30. Round 1.

Target: the DECOMPOSITION (task files under `docs/plans/vk-swarm-node-ui-localize/`), not code.
Every anchor below was verified against the merged tree at `HEAD` and against `35b378a5^` via
`git show`. Commands are quoted in evidence where they establish a fact.

---

### F1: Task 302 does not dictate how to handle `local_project_id` and `has_local` references in `ProjectList.tsx` and `UnifiedProjectCard.tsx` — a literal implementer hits compile errors or a STOP
- **Severity:** blocker
- **Location:** `phase-3/302-repoint-board-to-projectwithstats.md` steps 3 and 4
- **Evidence:**
  - `ProjectWithStats` (task 301) drops `has_local`, `local_project_id`, and `nodes` (ADR-0014 / D5). Confirmed in the planned struct: none of those fields appear.
  - `frontend/src/components/projects/ProjectList.tsx` references the dropped fields:
    ```
    62:      return projects.filter((p) => p.has_local);
    90:      local: projects.filter((p) => p.has_local).length,
    100:      p.nodes.forEach((n) => uniqueNodeIds.add(n.node_id));
    127:      if (project.has_local && project.local_project_id) {
    128:        navigate(`/settings/projects?projectId=${project.local_project_id}`);
    ```
  - `frontend/src/components/projects/UnifiedProjectCard.tsx` references the dropped fields in 11+ places:
    ```
    75:    if (!project.has_local || !project.local_project_id) return;
    85:      await projectsApi.delete(project.local_project_id);
    100:   if (!project.has_local || !project.local_project_id) return;
    103:   const response = await projectsApi.openEditor(project.local_project_id, ...);
    116:   id: project.local_project_id,
    132:   if (!project.has_local || !project.git_repo_path) return;
    145:   if (!project.has_local || !project.local_project_id) return;
    149:   id: project.local_project_id,
    216:   {project.has_local && (
    251:   {project.has_local && (
    279:   {!project.has_local && project.remote_project_id && (
    316:   {project.github_enabled && project.has_local && (
    ```
  - Task 302 step 3 (ProjectList) says only: "Remove any filtering/branching on `has_local` or `nodes`" — it does **not** mention `local_project_id` (lines 127-128), which is used for navigation, not filtering.
  - Task 302 step 4 (UnifiedProjectCard) says only: "Replace the `MergedProject` type import and the `project: MergedProject` / `onEdit` prop types with `ProjectWithStats`" and delete the two `LocationBadges` lines. It says **nothing** about the 11 `has_local` / `local_project_id` guards above.
  - The STOP trigger covers `ProjectList` and `ProjectSwitcher` ("If `ProjectList` or `ProjectSwitcher` uses `has_local` or `nodes` for anything other than the local/remote branch described above … STOP"). It does **not** mention `UnifiedProjectCard`, and it does not mention `local_project_id` at all.
- **Why it breaks a literal implementer:** Retyping `project: MergedProject` to `project: ProjectWithStats` makes every `project.has_local` and `project.local_project_id` access a compile error (`Property 'has_local' does not exist on type 'ProjectWithStats'`). The task gives no instruction for what to replace them with. A literal implementer either (a) stops the task on the STOP trigger (blocked), or (b) makes an undictated decision (e.g. "drop the guard", "use `project.id` for `local_project_id`") that may be wrong — `projectsApi.delete(project.local_project_id)` semantically relied on `local_project_id` being the local row id; with the merge fiction gone, `project.id` IS the local row id, but the task never says so.
- **Remediation:** Add to task 302 step 3 (ProjectList) and step 4 (UnifiedProjectCard) the explicit dictation:

  > Every project is now a local project (the merge fiction is gone), so:
  > - Replace every `project.has_local` (and `p.has_local`) with `true`-semantics: delete the `if (!project.has_local …)` guards entirely (keep the guarded body, which is now always reached), and delete the `{project.has_local && …}` condition (keep the JSX child).
  > - Replace every `project.local_project_id` with `project.id`. With the merge envelope removed, the project row id is the local id; `projectsApi.delete` / `openEditor` / navigation all take the local project id.
  > - Delete the `counts.local` / `counts.swarm` and `nodeCount` computations that branch on `p.has_local` / `p.nodes` (ProjectList lines 86-103) — with no `nodes` field they cannot compile; a single `total` count is the only meaningful one. If the filter tabs (`ProjectTypeFilterTabs`) require a `local`/`swarm` split, STOP — that is a plan gap.
  >
  > Add `UnifiedProjectCard.tsx` to the STOP trigger's covered files list.

---

### F2: Task 402 assumes `Nodes.tsx` has an `error` object in scope — it has only `isError` (boolean), so the instructed branch cannot be written and the STOP fires
- **Severity:** major
- **Location:** `phase-4/402-render-hive-not-connected-state.md` step 3 (and STOP triggers)
- **Evidence:**
  - `frontend/src/pages/Nodes.tsx` destructures only `isError`, not `error`:
    ```
    14:   const {
    15:     data: nodes = [],
    16:     isLoading: nodesLoading,
    17:     isError,
    18:   } = useQuery({
    ```
    and branches on `isError ?` at line 39. There is no `error` variable in scope.
  - Task 402 step 3 instructs, for each of the five sections **and `Nodes.tsx`**: "branch FIRST on `isHiveNotConfigured(error)` and render `<HiveNotConnected />`".
  - Task 402 STOP trigger: "If a section has no `error` value in scope from its query hook — STOP and report which one; do not add a new query."
  - The other four swarm sections (`SwarmProjectsSection`, `SwarmLabelsSection`, `SwarmTemplatesSection`, `NodeTemplatesSection`) DO destructure `error` (verified: `SwarmLabelsSection.tsx:61 error,`, `SwarmTemplatesSection.tsx:52 error,`, `NodeTemplatesSection.tsx:54 error,`, `SwarmProjectsSection.tsx` uses `error` at `:212`). `NodeProjectsSection.tsx` uses `nodesError` (`:104`, `:282`). Only `Nodes.tsx` lacks an `error` object.
- **Why it breaks a literal implementer:** The implementer is told to branch on `isHiveNotConfigured(error)` in `Nodes.tsx`, but `error` is undefined there. The STOP trigger fires ("no `error` value in scope"), halting the task with `Nodes.tsx` unwired — so SC4's "open `/nodes` → same [not-connected state]" is not delivered and the task cannot pass.
- **Remediation:** Add a `Nodes.tsx`-specific instruction to step 3:

  > For `frontend/src/pages/Nodes.tsx` specifically: the query currently destructures `isError`, not `error`. Change the destructure to also pull `error` (`const { data: nodes = [], isLoading: nodesLoading, error } = useQuery(...)`), then branch `isHiveNotConfigured(error) ? <HiveNotConnected /> : isError ? <p>Failed to load nodes.</p> : ...`. This is the only permitted change to the query destructure on this file.

  (Equivalently, the task could dictate using `isError && ...` plus a status check, but exposing `error` is the consistent shape with the other five sections.)

---

### F3: Task 301's draft handler code block omits the `Project` and `Deployment` imports needed to compile — the prose says "copy from merged.rs" but the code block is presented as "The file:"
- **Severity:** major
- **Location:** `phase-3/301-add-projectwithstats-endpoint.md` step 2 (the code block)
- **Evidence:**
  - The task 301 step 2 code block's `use` section is:
    ```
    use axum::{extract::State, response::Json as ResponseJson};
    use utils::response::ApiResponse;
    use crate::{
        DeploymentImpl,
        error::ApiError,
        routes::projects::types::{ProjectWithStats, ProjectsWithStatsResponse, TaskCounts},
    };
    ```
  - The handler body calls `Project::find_local_projects_with_stats(pool)` and `deployment.db().pool`. The authority (`crates/server/src/routes/projects/handlers/merged.rs`) imports both:
    ```
    9: use db::models::project::Project;
    10: use deployment::Deployment;
    ```
  - Without `use db::models::project::Project;` the name `Project` is unresolved. Without `use deployment::Deployment;` the method `deployment.db()` is unresolved (the `Deployment` trait must be in scope).
  - The prose immediately after the block says "Copy the `use` lines for `Project` (and anything else) from `merged.rs` verbatim — it is the authority on the exact import paths." This contradicts the code block, which is introduced as "The file:" (i.e. the file's content).
- **Why it breaks a literal implementer:** A constrained implementer treats the fenced code block as the file to write. It would write the block verbatim, run `cargo check`, and get `unresolved import Project` / `no method named db found`. The implementer then has to decide whether to trust the code block or the prose — an undictated choice. The STOP trigger ("If any import in the recovered file fails to resolve … STOP") would fire and halt the task.
- **Remediation:** Replace the code block's `use` section with the complete, compiling import list (copied from `merged.rs` plus the new types):

  ```rust
  use axum::{extract::State, response::Json as ResponseJson};
  use db::models::project::Project;
  use deployment::Deployment;
  use utils::response::ApiResponse;

  use crate::{
      DeploymentImpl,
      error::ApiError,
      routes::projects::types::{ProjectWithStats, ProjectsWithStatsResponse, TaskCounts},
  };
  ```

  and delete the "Copy the `use` lines …" prose so there is one authoritative source.

---

### F4: Task 403 dictates a new return shape for `useAvailableNodes` ("expose an empty list plus a boolean") without specifying the field names — the test asserts `result.current.nodes` against an interface the implementer must invent
- **Severity:** major
- **Location:** `phase-4/403-harden-remote-stream-hooks.md` step 1 + Failing test
- **Evidence:**
  - The hook today returns the raw `UseQueryResult<ListProjectNodesResponse>` (`frontend/src/hooks/useAvailableNodes.ts:12` — `return useQuery<ListProjectNodesResponse>({...})`), which has `.data`, `.isLoading`, `.isError`, etc. — no `.nodes` field.
  - Task 403 step 1 says: "treat a `isHiveNotConfigured(error)` failure as 'no nodes available' and expose an empty list plus a boolean the dialog can read, rather than an error the caller must handle."
  - The Failing test asserts `expect(result.current.nodes ?? []).toEqual([]);` — i.e. it is written against a post-change shape that has a `nodes` field.
  - The task does not dictate the field name(s) of the new return shape (is it `{ nodes, hasHive }`? `{ nodes, isLoading, disabled }`? does it keep `data`/`isError`?). It says only "adapt `result.current.nodes` to whatever shape the hook actually returns after your change."
  - `CreateAttemptDialog` is the consumer and is NOT in `files:` and must NOT be modified (STOP trigger). So the implementer cannot see what interface the dialog expects, and must invent one the dialog can consume without changing the dialog.
- **Why it breaks a literal implementer:** The implementer must (a) choose a return shape, (b) ensure `CreateAttemptDialog` (which they cannot read for the contract, per the STOP-trigger spirit, and cannot edit) still works with it. That is an undictated interface decision with a hidden constraint. The test passes against whatever shape exposes `.nodes`, but the consumer may break silently — and the manual verification is a browser check, not a type check against the dialog.
- **Remediation:** Dictate the exact return shape. Read `CreateAttemptDialog` first, then specify, e.g.:

  > `useAvailableNodes` returns `{ nodes: ListProjectNodesResponse['nodes'], isLoading: boolean, hasHive: boolean }` where `nodes` is `data?.nodes ?? []` on success and `[]` on `isHiveNotConfigured(error)`. Keep `isLoading` semantics unchanged for the hive-configured path. The consumer (`CreateAttemptDialog`) reads `nodes` and `isLoading` only (verify by reading the dialog before implementing); if it reads any other field, STOP.

  Add the same level of dictation for the return shapes of `useNodeLogStream`, `useDiffStream`, and `useRemoteConnectionStatus` (see F5).

---

### F5: Task 403 provides a unit test for only 1 of the 4 hardened hooks — the other 3 (`useNodeLogStream`, `useDiffStream`, `useRemoteConnectionStatus`) have no test, so "Done when" cannot be objectively verified
- **Severity:** major
- **Location:** `phase-4/403-harden-remote-stream-hooks.md` Failing test + Done when
- **Evidence:**
  - The Failing test creates only `frontend/src/hooks/useAvailableNodes.test.ts`.
  - `useNodeLogStream.ts` returns `{ logs, error, ... }` (lines 42-43); `useDiffStream.ts` returns `{ diffs, error, ... }` (lines 28-29); `useRemoteConnectionStatus.ts` returns `{ status, isLoading, error }` (lines 15-21). None has a test file under `frontend/src/hooks/`:
    ```
    $ ls frontend/src/hooks/useNodeLogStream.test.ts frontend/src/hooks/useDiffStream.test.ts frontend/src/hooks/useRemoteConnectionStatus.test.ts
    ls: ...: No such file or directory  (all three)
    ```
  - "Done when" requires: "All four hooks return a clean, settled, empty result when no hive is configured." For 3 of 4, the only verification is the Manual verification browser check ("ProcessLogsViewer renders local logs …"), which is a human observation, not a gate-able test.
  - SC6 is claimed by task 403 alone. Three of the four components in SC6 (`ProcessLogsViewer`, `DiffsPanel`, `AttemptHeaderActions`) are covered only by the browser check on the corresponding hook.
- **Why it breaks a literal implementer:** The implementer can mark the task done after writing one passing test and a browser check; a regression in any of the three untested hooks' hive-absent path would not be caught by the suite. The "No Deferred Remediation" rule wants gate-able proof; a browser observation is not that.
- **Remediation:** Add a Failing test for each of the four hooks (or at minimum one combined test file) asserting the hive-absent return shape: `useNodeLogStream` → `{ logs: [], error: null, ... }`; `useDiffStream` → `{ diffs: [], error: null, ... }`; `useRemoteConnectionStatus` → `status: 'disconnected'` (or the enum's disconnected variant) with `isLoading: false`, `error: null`. Each test mocks the underlying fetch to reject with `new ApiError('no hive', 503)` (after task 401/402). Dictate the expected field values in the task body so the implementer does not choose.

---

### F6: Task 203's line table omits a 5th "Used By" citation (`routes/nodes.rs - Hard delete option` at line 189) that still points at the deleted node route module
- **Severity:** minor
- **Location:** `phase-2/203-update-node-api-key-architecture-doc.md` Change table + Done when
- **Evidence:**
  - `docs/architecture/db/functions/postgresql-node-api-keys.mdx` has five `routes/nodes.rs` citations, not four:
    ```
    63:  - `routes/nodes.rs` - POST /api/nodes/api-keys
    108: - `routes/nodes.rs` - Key management
    141: - `routes/nodes.rs` - GET /api/nodes/api-keys
    171: - `routes/nodes.rs` - DELETE /api/nodes/api-keys/:id
    189: - `routes/nodes.rs` - Hard delete option
    329: - `routes/nodes.rs` - POST /api/nodes/api-keys/:id/unblock
    ```
  - The task table lists lines 63, 108, 141, 171 only, plus a conditional for the unblock citation (line 329). Line 189 (`Hard delete option`) is not mentioned at all.
  - "Done when" says "Every 'Used By' citation points at the hive's route module with a `/v1/` path." Line 189, unedited, still reads `routes/nodes.rs - Hard delete option` — pointing at the node module that task 101 deletes the API-key surface from. This violates the Done-when.
  - The Manual verification grep `grep -n '/api/nodes/api-keys' docs/...` does NOT catch line 189 (it contains no `/api/nodes/api-keys` substring), so the gate passes while the drift remains.
- **Why it breaks a literal implementer:** Following the table literally, the implementer edits 4 (or 5) lines and leaves line 189 pointing at a deleted module. The grep gate is green, but the Done-when is violated and the doc still describes a 404 as live — the exact drift class the workstream exists to remove.
- **Remediation:** Add line 189 to the table:

  | 189 | `- ``routes/nodes.rs`` - Hard delete option` | `- ``crates/remote/src/routes/nodes.rs`` - Hard delete option` |

  and update the Manual verification `grep -c 'crates/remote/src/routes/nodes.rs'` expected count to "5 (or 6 if the unblock citation was present)".

---

### F7: Task 401 leaves the `HiveNotConfigured` error *message* to the catch-all arm in the second `match` — the API body will read `"HiveNotConfigured: This node is not connected to a hive"` rather than a clean message
- **Severity:** minor
- **Location:** `phase-4/401-hive-not-configured-error-variant.md` step 3 (only adds an arm to the first match)
- **Evidence:**
  - `crates/server/src/error.rs` has two `match &self` blocks in `IntoResponse`: the first (line 111) maps to `(status_code, error_type)`, the second (line 236) maps to `error_message` and ends with a catch-all `_ => format!("{}: {}", error_type, self)` at line 341.
  - Task 401 step 3 adds a `HiveNotConfigured => (StatusCode::SERVICE_UNAVAILABLE, "HiveNotConfigured")` arm only to the first match. It does not add a message arm to the second match.
  - With no specific message arm, `HiveNotConfigured` falls through to the catch-all, producing `error_message = "HiveNotConfigured: This node is not connected to a hive"` (because `error_type = "HiveNotConfigured"` and `self` renders via `#[error("This node is not connected to a hive")]`).
  - Task 401's own Manual verification asserts the body `contains "HiveNotConfigured"` — the catch-all body does contain that string, so the gate passes, but the user-visible message is the ugly compound form.
- **Why it breaks a literal implementer:** The implementer follows step 3 literally (one arm, first match only) and ships a compound error message. Not a compile or gate failure, but a polish defect the task could have pre-empted. The `HiveNotConnected` UI component renders its own message, so the API message is secondary — hence minor.
- **Remediation:** Add to step 3 a second arm in the `error_message` match:

  ```rust
  ApiError::HiveNotConfigured => "This node is not connected to a hive.".to_string(),
  ```

  placed immediately before the catch-all `_ =>` arm. Update the Manual verification body assertion to `grep -q 'This node is not connected to a hive'` (and keep the `HiveNotConfigured` type assertion on the status-code arm).

---

### F8: SC5's literal text says "renders projects from `/api/projects`" but the plan (and the spec's own Approach + ADR-0014) deliver `/api/projects/with-stats` — an unrecorded spec-internal inconsistency
- **Severity:** minor
- **Location:** spec SC5 vs `plan.md` success-criterion coverage; `phase-3/301-...md`
- **Evidence:**
  - Spec SC5: "The task board renders projects from `/api/projects`. `/api/merged-projects` receives zero requests …"
  - Spec Approach (Track B): "Add `ProjectWithStats` + `/api/projects/with-stats`, delete `MergedProject` …"
  - Spec `verify_cmd` frontmatter: `curl -fsS http://127.0.0.1:${BACKEND_PORT:-3001}/api/projects/with-stats | grep -q 'success.:true'`
  - ADR-0014 Decision: "served from `/api/projects/with-stats`."
  - Plan task 301 registers `/api/projects/with-stats`; SC5 coverage is claimed by 301, 302, 303.
  - The decisions-ledger does not record the reconciliation between SC5's `/api/projects` wording and the `/api/projects/with-stats` delivery.
- **Why it breaks a literal implementer:** The plan is consistent with the spec's authoritative Approach, the `verify_cmd`, and ADR-0014, so an implementer following the plan will not diverge. The risk is the reverse: a future reader checking SC5 against the network log looks for `/api/projects` and sees `/api/projects/with-stats`, and wrongly concludes the criterion failed. The spec is FROZEN, so this is a spec defect to record, not a plan defect to fix.
- **Remediation:** Add one line to the decisions-ledger under "Decomposition-time decisions":

  > **SC5 wording vs delivery.** SC5's text says "renders projects from `/api/projects`"; the delivered endpoint is `/api/projects/with-stats` (per the spec's own Approach, `verify_cmd`, and ADR-0014). US4 requires enrichment (task counts, last-activity), which bare `/api/projects` does not provide; the spec's Approach is authoritative, so SC5 is read as "from the projects API surface (`/api/projects/with-stats`)". Recorded here as a spec-internal inconsistency, not a plan divergence.

---

### F9: Task 101's `base_routes` anchor cites line 59; the actual `.merge(organizations::router())` is on line 60
- **Severity:** minor
- **Location:** `phase-1/101-restore-nodes-routes.md` step 2 ("Anchor: the `base_routes` builder, line 59")
- **Evidence:**
  - `crates/server/src/routes/mod.rs`:
    ```
    60:        .merge(organizations::router())
    ```
  - Task 101 says "line 59". (Task 105, the spec, and ADR-0013 all say `mod.rs:44-71` / `:59` — the same stale number.)
- **Why it breaks a literal implementer:** The Before/After snippet is unambiguous and the implementer matches by content, not line number, so this does not halt the task. It is a stale anchor that erodes trust in the plan's line citations.
- **Remediation:** Change "line 59" to "line 60" in task 101 step 2. (Optional: refresh the `:44-71` / `:59` references in the spec and ADR-0013 in a follow-up doc pass — out of this plan's scope.)

---

### F10: Task 303's conditional "If a `impl From<...> for NodeLocation` block exists (around line 90-110)" points at a non-existent impl — the line range actually holds `impl From<Project> for RemoteNodeProject`
- **Severity:** minor
- **Location:** `phase-3/303-delete-mergedproject.md` step 3 (last bullet)
- **Evidence:**
  - `crates/server/src/routes/projects/types.rs` lines 88-110:
    ```
    88: impl From<Project> for RemoteNodeProject {
    ...
    109: }
    ```
  - There is no `impl From<...> for NodeLocation` anywhere in the file:
    ```
    $ grep -n "impl From.*NodeLocation\|impl.*NodeLocation" crates/server/src/routes/projects/types.rs
    (no output)
    ```
- **Why it breaks a literal implementer:** The conditional is guarded by "If … exists", so a literal implementer correctly finds no such impl and does nothing — no breakage. The defect is that the line range 90-110 is misleading: it names a real impl (`From<Project> for RemoteNodeProject`) that the implementer might wrongly delete if they misread "NodeLocation" as "the impl at those lines". A careful implementer will not, but a constrained one could.
- **Remediation:** Replace the bullet with: "There is no `impl From<...> for NodeLocation` in this file (verified at decomposition); only `NodeLocation`'s struct definition (~line 150-163) is deleted. Do not touch the `impl From<Project> for RemoteNodeProject` block at lines 88-110."

---

## Summary

Counts by severity:

- **Blocker:** 1 (F1)
- **Major:** 4 (F2, F3, F4, F5)
- **Minor:** 5 (F6, F7, F8, F9, F10)

Categories attacked and findings:

- **Wrong anchors:** F9 (line 59 vs 60 in `mod.rs`), F10 (non-existent `NodeLocation` impl cited at lines that hold a different impl). All other cited anchors verified correct: `mod.rs:60` `.merge(organizations::router())`, `projects/mod.rs:135` `/scan-config`, `projects/mod.rs:148` `/merged-projects`, `OrganizationSettings.tsx:37`/`:379-381`, `nodes.ts:5-12`/`:41-92`, `projects.ts:106-109` `getMerged`, `UnifiedProjectCard.tsx:36`/`:310`, `error.rs:66`/`:103-107`/`:197` region, `postgresql-node-api-keys.mdx:63/108/141/171/329`, `handlers/mod.rs:14`/`:29`.
- **Verbatim-restore claims:** Verified sound. All four route modules exist at `35b378a5^` and every imported type/method resolves on `main`: `remote::nodes::{Node, NodeApiKey, NodeLocalProjectInfo}` (in `crates/remote/src/nodes/domain.rs`), `remote::routes::swarm_{projects,labels,templates}::*` (present), `services::services::remote_client::*Request` structs (present at lines 1413-1614), `RemoteClient::{list_nodes,get_node,delete_node,list_node_projects,list_node_api_keys,create_node_api_key,revoke_node_api_key}` (present at lines 851-982). Task 101's post-deletion import block is correct (`Json`, `Serialize`, `NodeApiKey`, and `routing::delete` are all genuinely unused after the API-key deletion; `routing::get` survives; `Deserialize`/`Uuid`/`Node`/`NodeLocalProjectInfo` survive).
- **Ordering/dependency errors:** None found. 101-104 conflict correctly (all edit `mod.rs`). 301↔303 conflict correctly (both edit `projects/mod.rs`, `types.rs`, `handlers/mod.rs`, `generate_types.rs`). 402 `depends_on [401, 104]` is correct (needs restored routes + the 503 variant). 201 `depends_on [101]` is correct (needs nodes route restored before its UI is "the hive owns keys"). 501 `depends_on [105,202,203,303,402,403]` covers every producing task. 302↔303 share `frontend/src/lib/api/projects.ts` but are ordered (303 dep 302), so the omitted conflict is acceptable.
- **Hollow or missing tests:** F5 (3 of 4 hardened hooks untested). Task 105's no-unit-test argument is **correct**: the bug is registration, not handler logic — a handler-level test would pass on `main` today (verified: the recovered handlers are unchanged from `35b378a5^` and `RemoteClient` methods all exist, so `list_nodes()` already works in isolation; only the route registration was deleted). The plan's refusal of a hollow test and use of HTTP reachability is sound.
- **Blast radius:** No omitted file found. Every file a task necessarily touches is in its `files:` list. `handlers/mod.rs` and `generate_types.rs` are correctly listed on both 301 and 303. `core.rs` is listed on 303 only defensively (verified it has no `MergedProject` reference — it will be untouched). `frontend/src/hooks/index.ts` is NOT in 302's `files:` and verified to NOT re-export `useMergedProjects` (so the STOP trigger does not fire and the omission is correct).
- **Spec fidelity:** F8 (SC5 `/api/projects` vs `/with-stats` — spec-internal inconsistency; plan follows the authoritative Approach + `verify_cmd` + ADR-0014). All other SCs covered: SC1 (101-105), SC2 (105,402), SC3 (101,201,202,203), SC4 (401,402), SC5 (301,302,303), SC6 (403), SC7 (501). No task contradicts D1-D7 or ADR-0013/0014.
- **Undictated decisions:** F1 (`has_local`/`local_project_id` replacement in ProjectList + UnifiedProjectCard), F2 (`Nodes.tsx` error-vs-isError), F3 (handler imports), F4 (`useAvailableNodes` return shape).

The blocker (F1) and the four majors must be remediated in the task files before execution begins; a literal implementer would otherwise halt or ship drift on each.