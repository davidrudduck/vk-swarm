---
doc_type: spec
status: shipped
workstream: vk-swarm-node-ui-localize
change_kind: bugfix
verify_cmd: "curl -fsS http://127.0.0.1:${BACKEND_PORT:-3001}/api/projects/with-stats | grep -q 'success.:true'"
---

# vk-swarm-node-ui-localize — Intent

Finishes the React-layer half of the node/hive split that `vk-swarm-node-foundations` started.
node-foundations delivered the local-only backend core and deleted the node-surface API route
layer; the node frontend was never updated to match, so parts of it now call HTTP routes that no
longer exist.

Supersedes the "Scope (the entangled remainder)" and "Keep (do NOT remove)" lists in
`dev-docs/workstreams/vk-swarm-node-ui-localize/README.md`, which were written during
node-foundations decompose and have since drifted from merged code (see Constraint C1).

## Intent (what / why)

Two things are wrong in the node frontend today, and they pull in opposite directions:

**1. The swarm/nodes management UI is live but broken.** Its pages render and then every data
call 404s:

| Route | Component | Calls | Status |
|---|---|---|---|
| `/nodes` (`App.tsx:153`) | `pages/Nodes.tsx` | `nodesApi.list` → `/api/nodes` | 404 |
| `/settings/swarm` (`App.tsx:189`) | `SwarmProjectsSection` | `/api/swarm/projects` | 404 |
| ″ | `NodeProjectsSection` (`SwarmSettings.tsx:163`) | `/api/nodes`, `/api/nodes/{id}/projects` | 404 |
| ″ | `SwarmLabelsSection` | `/api/swarm/labels` | 404 |
| ″ | `SwarmTemplatesSection` | `/api/swarm/templates` | 404 |
| ″ | `NodeTemplatesSection` | `/api/swarm/templates`, `/api/nodes` | 404 |

`crates/server/src/routes/mod.rs:44-71` registers no `nodes` or `swarm` module. The org picker at
the top of `/settings/swarm` works — `/api/organizations` survived as a hive proxy — which makes
the failure look like empty data rather than a broken page.

The decision (2026-07-30) is to **repoint, not delete**: the node keeps its swarm management UI
and it talks to the hive. The route layer is restored as thin proxy handlers in the node server,
mirroring the pattern already live in `crates/server/src/routes/organizations.rs`. This is
tractable because node-foundations deleted only the HTTP layer — `RemoteClient`
(`crates/services/src/services/remote_client.rs`) still carries every method needed:
`list_nodes`, `get_node`, `delete_node`, `list_node_projects`, `list_linked_node_projects`,
`get_node_statuses`, and the full `*_swarm_project` / `*_swarm_label` / `*_swarm_template` sets
including the merge and `promote_label_to_swarm` operations. (It also retains the node API-key
methods, which this workstream deliberately leaves unused — see D3.)

**2. The task board still renders through the remote-merge types.** `useMergedProjects` →
`projectsApi.getMerged()` → `/api/merged-projects`, and `ProjectList` / `ProjectSwitcher` are
typed on `MergedProject`. The board should show **local projects only**; the merged view is
retired. This is the typed refactor node-foundations task 403 deliberately deferred to stay
codegen-neutral.

The distinction to hold onto: **management of the swarm is a hive concern the node proxies to;
display of the node's own work is local-only.** Those are not in conflict — they are the two
halves of the split.

## Users / who is affected

- **Node operators** managing swarm projects, labels, templates, and node-project links from a
  node's own UI. Today every one of those screens is silently empty.
- **Node operators pairing a node to a hive** via `/nodes` — currently a dead page. Key minting
  moves to the hive UI (D3), so their onboarding path changes.
- **Anyone using the node task board**, who is served through a merge path the node no longer
  needs.
- **Future maintainers:** `frontend/src/components/swarm/` and
  `remote-frontend/src/components/swarm/` are near-identical 15-component trees. This workstream
  does not merge them, but the spec must say which is authoritative so the duplication is a
  known cost rather than an accident.

## User stories

- **US1:** As a node operator whose node is paired to a hive, when I open `/settings/swarm`, I
  expect swarm projects, labels, templates, and node-project links to load and be manageable from
  the node I am already looking at.
- **US2:** As a node operator, when I open `/nodes`, I expect to see my organization's nodes with
  their live status.
- **US3:** As the operator of a standalone node with no hive configured, when I open any swarm
  screen, I expect a clear "not connected to a hive" message instead of an empty list, an endless
  spinner, or a raw error body.
- **US4:** As a node operator, when I use the task board, I expect my projects — hive-bound and
  local-only alike — with their task counts and recent activity, and full CRUD over the tasks
  created and executed on this node.
- **US5:** As the operator of a standalone node, when I view attempt logs, diffs, connection
  status, or pick a node in the attempt dialog, I expect local state to render without errors from
  absent remote data.
- **US6:** As a hive user, when this workstream ships, I expect the hive's own UI to be entirely
  unchanged.
- **US7:** As an operator minting or revoking a node API key, I expect exactly one place to do it
  — the hive UI — rather than two admin-gated paths to the same privileged operation.

## Success criteria

Runtime-observable. No criterion is "test X passes". Each derives from a user story above.

- **SC1:** With a hive configured, `GET /api/nodes?organization_id=<org>` on a running node
  returns `200` with `success: true`, and the same for `/api/nodes/{id}`,
  `/api/nodes/{id}/projects`, `/api/swarm/projects`, `/api/swarm/labels`,
  `/api/swarm/templates`. Zero `404`s across the surface listed in the Intent table. → US1
- **SC2:** Loading `/settings/swarm` in a browser against a hive-connected node renders real
  swarm projects, labels, and templates from the hive; the browser network log shows no `404`. → US1
- **SC3:** Loading `/nodes` lists the organization's nodes with live status. No API-key
  management appears anywhere in the node UI: `/api/nodes/api-keys*` returns `404`, and
  `OrganizationSettings` renders no key section (per D3 — the hive owns key management).
  → US2, → US7
- **SC4:** With **no** hive configured, every one of those screens renders an explicit
  "not connected to a hive" state. No unhandled rejection, no infinite spinner, no raw error
  body. (`RemoteClient` construction fails with `RemoteClientNotConfigured`; the UI must handle
  it as a first-class state.) → US3
- **SC5:** The task board renders projects from `/api/projects`. `/api/merged-projects` receives
  zero requests from the frontend during a full board session (observable in the network log).
  → US4
- **SC6:** Attempt logs, diffs, connection status, and node selection in the attempt UI behave
  correctly on a node with no hive: `ProcessLogsViewer`, `DiffsPanel`, `AttemptHeaderActions`,
  and `CreateAttemptDialog` render local state without erroring on absent remote data. → US5
- **SC7:** `remote-frontend` behaviour is unchanged: the hive's own swarm UI still works
  end-to-end (its E2E suite is the regression signal). → US6

## Constraints

- **C1 — The README's entanglement map is stale; re-derive it, do not trust it.** Its "Keep (do
  NOT remove — live non-Nodes consumers)" list asserts the Nodes feature was deleted by
  node-foundations. It was not: `/nodes` is still routed at `App.tsx:153`, and the "live
  consumers" it protects (`nodesApi`, `NodeProjectsSection`) are exactly the ones calling dead
  routes. Every claim in that section must be re-verified against merged code during
  `/wai:spec`. Recorded as a spec defect in the README and in `dev-docs/BACKLOG.md`.

- **C2 — Restoring node-surface proxies partially reverses ADR-0002 and needs its own ADR.**
  node-foundations removed the node-surface API proxies as an architectural decision. Re-adding
  them — even as thin `RemoteClient` pass-throughs — is a reversal of that decision and is
  irreversible in the ADR sense. `/wai:spec` must author an ADR covering: why proxy-through-node
  beats browser→hive direct, the auth boundary, and what stays deleted. Do not decompose without
  it.

- **C3 — The browser cannot hold hive credentials.** `VK_NODE_API_KEY` is a server-side secret.
  Proxying through the node server (the `organizations.rs` pattern) is what keeps it that way;
  any design that has the node frontend call the hive directly must solve browser-side auth and
  CORS, and should be rejected on that basis unless it can.

- **C4 — Hive-absent is a normal state, not an error path.** A node may run standalone. Every
  proxied surface must degrade to SC4's explicit disconnected state.

- **C5 — Type-generation discipline.** Retiring `MergedProject` touches a `#[ts(export)]` struct.
  `npm run generate-types` must be run and `npm run generate-types:check` must pass; no
  hand-edited `shared/types.ts`.

- **C6 — Full quality gates (CLAUDE.md "Finish What We Start").** `cargo clippy --all
  --all-targets --all-features -- -D warnings`, `cargo test --workspace`, `cd frontend && npm run
  lint`, `npx tsc --noEmit`, `cd remote-frontend && npm run lint && npx tsc --noEmit && npx
  vitest run` all green before the PR.

- **C7 — The two `components/swarm/` trees stay separate.** Unifying node and hive swarm
  components is explicitly not in scope; the spec records which is authoritative and the
  duplication is accepted for now.

## Out of scope

- Merging / deduplicating `frontend/src/components/swarm/` with
  `remote-frontend/src/components/swarm/` (C7).
- Any change to hive-side (`crates/remote/`) endpoints or `remote-frontend/`, beyond not
  regressing them (SC7).
- The hive UI findings **F-2026-07-29-03** (drawer/navbar actions blocked on hive APIs) and
  **F-2026-07-22-01** (design-system tokens) — different workstreams.
- **F-2026-07-29-04** (local vitest/vite-build flake) — no product impact.
- Sync plumbing, WebSocket transport, and the node↔hive protocol itself.
- Any new swarm capability. This restores and relocates existing surfaces; it does not add
  features.

## Findings this closes

- **F-2026-07-29-01** (high) — node Nodes page and swarm sections call removed `/api/nodes` and
  `/api/swarm` routes.
- **F-2026-07-29-02** (medium) — node board consumes `MergedProject` via the bridge endpoint;
  repoint to `Project`.

## Approach

Three independent tracks. Track A is mostly a `git show` restore; Track B is the typed refactor;
Track C is hardening. A and B do not touch the same files and can run in parallel.

**Track A — restore the proxy route layer (backend).** Recover the four route modules deleted by
`35b378a5` verbatim:

```bash
git show 35b378a5^:crates/server/src/routes/nodes.rs
git show 35b378a5^:crates/server/src/routes/swarm_projects.rs
git show 35b378a5^:crates/server/src/routes/swarm_labels.rs
git show 35b378a5^:crates/server/src/routes/swarm_templates.rs
```

Re-register them in `crates/server/src/routes/mod.rs` alongside `organizations::router()`. Restoring the paths
verbatim means **zero frontend diff** for this track — `lib/api/{nodes,swarmProjects,swarmLabels,
swarmTemplates}.ts` already target exactly these URLs. The restored surface:

| Module | Routes |
|---|---|
| `nodes.rs` | `/nodes`, `/nodes/{node_id}`, `/nodes/{node_id}/projects` |
| `swarm_projects.rs` | `/swarm/projects`, `/{project_id}`, `/{project_id}/merge`, `/{project_id}/nodes`, `/{project_id}/nodes/{node_id}` |
| `swarm_labels.rs` | `/swarm/labels`, `/{label_id}`, `/{label_id}/merge`, `/swarm/labels/promote` |
| `swarm_templates.rs` | `/swarm/templates`, `/{template_id}`, `/{template_id}/merge` |

Deliberately **not** restored: `/nodes/api-keys*` — see Decision D3.

**Track B — retire `MergedProject` (full-stack typed refactor).** Add `ProjectWithStats` +
`/api/projects/with-stats`, delete `MergedProject` / `MergedProjectsResponse` /
`/api/merged-projects`, regenerate types, retype the four consumers. Per
[ADR-0014](../../../dev-docs/adr/0014-retire-mergedproject-for-projectwithstats.md).

**Track C — hive-absent hardening.** Make `RemoteClientNotConfigured` a first-class UI state
across every proxied surface, and settle the four remote stream hooks.

Ordering: A and B in parallel; C after A (it needs the restored routes to have a disconnected
state to render).

## Design / architecture

### Proxy handler shape

Every restored handler is an extract → call → wrap pass-through with no business logic, exactly
as `crates/server/src/routes/organizations.rs` (`list_organizations`) already does on `main`:

```rust
async fn list_nodes(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<ListNodesQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<Node>>>, ApiError> {
    let client = deployment.remote_client()?;
    let nodes = client.list_nodes(params.organization_id).await?;
    Ok(ResponseJson(ApiResponse::success(nodes)))
}
```

`deployment.remote_client()` returns `Result<RemoteClient, RemoteClientNotConfigured>`; the `?`
carries the hive-absent case into `ApiError` uniformly, so the disconnected state is one error
variant the frontend can branch on rather than a per-endpoint special case.

No new `RemoteClient` methods are needed — every call the restored routes make already exists in
`crates/services/src/services/remote_client.rs`. (The two methods that would have been needed,
`unblock_node_api_key` and node merge, belong to the API-key surface that D3 deletes.)

### `ProjectWithStats`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProjectWithStats {
    // Project row
    pub id: Uuid, pub name: String, pub git_repo_path: String,
    #[ts(type = "Date")] pub created_at: DateTime<Utc>,
    pub remote_project_id: Option<Uuid>,
    // enrichment
    pub task_counts: TaskCounts,
    #[ts(type = "Date | null")] pub last_attempt_at: Option<DateTime<Utc>>,
    pub github_enabled: bool, pub github_owner: Option<String>, pub github_repo: Option<String>,
    pub github_open_issues: Option<i64>, pub github_open_prs: Option<i64>,
    #[ts(type = "Date | null")] pub github_last_synced_at: Option<DateTime<Utc>>,
}
```

Identical to today's `MergedProject` minus `nodes`, `has_local`, and `local_project_id` — the
three fields the handler hardcodes. The query (`Project::find_local_projects_with_stats`) and the
name-sort are reused unchanged, so the payload the board renders is byte-equivalent apart from the
dropped constants. `remote_project_id` survives, which is what keeps hive-bound projects
identifiable and manageable from the node.

Frontend: `useMergedProjects` → `useProjectsWithStats`; `ProjectList`, `ProjectSwitcher`,
`UnifiedProjectCard` retyped; `LocationBadges` deleted (it renders `nodes`, which is always empty).

### Hive-absent state

`ApiError` gains (or reuses) a discriminable not-configured variant so the frontend can render
"not connected to a hive" rather than a raw error body. One shared presentational component is
used by all five swarm sections and `/nodes`, so the state is consistent and testable in one place.

### Remote stream hooks

`useNodeLogStream`, `useDiffStream`, `useRemoteConnectionStatus`, and `useAvailableNodes` stay —
they serve cross-node viewing, which survives under the repoint decision. They are hardened, not
removed: each must return a clean empty/disabled result when no hive is configured, so
`ProcessLogsViewer`, `DiffsPanel`, `AttemptHeaderActions`, and `CreateAttemptDialog` render local
state without erroring (SC6). `useRemoteConnectionStatus` already branches on
`connectionInfo.direct_url`; that branch is the model for the others.

## Decisions

| # | Decision | Irreversible? | ADR |
|---|---|---|---|
| D1 | Restore node-surface routes as thin `RemoteClient` proxies, verbatim paths, mirroring `organizations.rs` | **Yes** — reverses part of node-foundations | [ADR-0013](../../../dev-docs/adr/0013-restore-node-surface-hive-proxy-routes.md) |
| D2 | Proxy through the node server rather than browser→hive direct | **Yes** — sets the auth boundary | [ADR-0013](../../../dev-docs/adr/0013-restore-node-surface-hive-proxy-routes.md) |
| D3 | Do **not** restore `/nodes/api-keys*`; delete `components/org/NodeApiKeySection.tsx` and its `OrganizationSettings.tsx:380` mount. The hive owns key management | **Yes** — deletes a live user-facing surface | [ADR-0013](../../../dev-docs/adr/0013-restore-node-surface-hive-proxy-routes.md) |
| D4 | Replace `MergedProject` with `ProjectWithStats` at `/api/projects/with-stats`; delete `/api/merged-projects` | **Yes** — deletes a `#[ts(export)]` type, changes wire format | [ADR-0014](../../../dev-docs/adr/0014-retire-mergedproject-for-projectwithstats.md) |
| D5 | Delete `LocationBadges` and the `nodes` / `has_local` / `local_project_id` fields | **Yes** — deletes a component | [ADR-0014](../../../dev-docs/adr/0014-retire-mergedproject-for-projectwithstats.md) |
| D6 | Keep the four remote stream hooks; harden for hive-absent instead of removing | No | — |
| D7 | Keep `frontend/` and `remote-frontend/` swarm trees separate; hive tree is authoritative | No | — |

## Test strategy

Rust:

- Per-module route tests for each restored proxy: hive-configured returns `200` + `success: true`
  (against a mocked `RemoteClient`), and hive-absent returns the not-configured variant rather
  than a 500. This is the SC1/SC4 signal.
- `ProjectWithStats` handler test asserting the enrichment survives: a project with tasks in each
  status returns correct `task_counts`, a non-null `last_attempt_at`, and name-sorted ordering —
  i.e. the regression `a85f7d63` was fixing cannot come back.
- Assert `/api/merged-projects` is gone (404) once D4 lands.
- `npm run generate-types:check` in CI to catch a stale `shared/types.ts`.

Frontend (`frontend/`):

- `ProjectList` / `ProjectSwitcher` render from `ProjectWithStats` fixtures; task counts and
  last-attempt ordering asserted.
- Each swarm section renders the disconnected state when the API returns not-configured.
- `ProcessLogsViewer`, `DiffsPanel`, `AttemptHeaderActions`, `CreateAttemptDialog` render without
  error when their remote hook returns the hive-absent result (SC6).

Reachability (per `/wai:execute`'s close gate — `change_kind: bugfix`):

- The real-seam test must drive the **HTTP route**, not the handler function. A test calling
  `list_nodes()` directly proves the proxy works, never that `/api/nodes` is *registered* — and
  an unregistered route is the entire bug. At least one test must exercise the mounted router.
- Incident-symptom assertion: a request to each URL in the Intent table returns non-404.

Regression:

- `remote-frontend` suite unchanged and green (SC7) — the hive UI must not move.
- Full gates per C6.

## Resolved open questions

1. **Route shape** → verbatim restore from `35b378a5^`. Zero frontend diff for Track A.
2. **API keys on the node** → deleted (D3). The hive owns them; the node's copy is removed rather
   than reconnected.
3. **`MergedProject`** → deleted and replaced by `ProjectWithStats` (D4). The endpoint was not
   merging anything; the board's real dependency is the enrichment, which is preserved.
4. **Stream hooks** → kept and hardened (D6).
5. **`verify_cmd`** → now targets `/api/projects/with-stats`, which works on a standard dev
   deploy with no hive and no `VK_ORG_ID`. Hive-connected checks are covered by SC1's route
   sweep during deploy verification.
