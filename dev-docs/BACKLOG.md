# Findings Backlog

> Raw findings (bugs, paper-cuts, tech-debt, drift signals) not yet promoted to a workstream.
> Schema: ADR-0007 (`plugins/wai/schema/finding.frontmatter.md`). Do not edit rows by hand
> outside the marked region — use `/wai:finding-new` to append, `/wai:finding-promote` to
> promote, `/wai:backlog` to list/triage.

<!-- WAI:BACKLOG:BEGIN -->
| id | title | severity | status | location | source | discovered | workstream | links |
|---|---|---|---|---|---|---|---|---|
| F-2026-07-04-01 | hive-redesign 14 task files still ready despite PR 451 merged | medium | fixed | dev-docs/workstreams/vk-swarm-hive-redesign/ | sweep/2026-07-04 | 2026-07-04 | vk-swarm-hive-redesign | — |
| F-2026-07-04-02 | orphan spec reference-architecture-alignment-design unreferenced pre-fork | low | wontfix | docs/superpowers/specs/2026-04-20-reference-architecture-alignment-design.md | sweep/2026-07-04 | 2026-07-04 | — | — |
| F-2026-07-04-03 | stale repo-root PLAN.md planning doc | low | wontfix | PLAN.md | sweep/2026-07-04 | 2026-07-04 | — | — |
| F-2026-07-04-04 | crisp-river uncommitted Cargo.toml doctest edits on merged branch | medium | fixed | crates/remote/Cargo.toml | sweep/2026-07-04 | 2026-07-04 | — | — |
| F-2026-07-06-01 | Hive UI lacks Generate API key button — node onboarding blocked | high | fixed | remote-frontend/src/pages/Nodes.tsx:7-51 | session/2026-07-06 | 2026-07-06 | hive-node-api-key-ui | dev-docs/workstreams/hive-node-api-key-ui/spec/2026-07-07-hive-node-api-key-ui.md |
| F-2026-07-06-02 | Sign-in broken on non-loopback HTTP origins (crypto.subtle undefined) | high | fixed | remote-frontend/src/pkce.ts:10 | session/2026-07-06 | 2026-07-06 | fix-nonloopback-signin | dev-docs/workstreams/fix-nonloopback-signin/spec/2026-07-08-fix-nonloopback-signin.md |
| F-2026-07-11-01 | AppRouter test isolation: authenticated / → /nodes redirect fails | medium | fixed | remote-frontend/src/AppRouter.test.tsx | session/2026-07-11 | 2026-07-11 | — | — |
| F-2026-07-11-02 | no-push-invariant test fails on baseline | medium | fixed | remote-frontend/scripts/no-push-invariant.test.mjs | session/2026-07-11 | 2026-07-11 | — | — |
| F-2026-07-22-01 | NodeCard references undefined vks tokens (vks-pulse, --vks-text-dim) | low | fixed | remote-frontend/src/components/swarm/NodeCard.tsx:48-53 | sweep/2026-07-22 | 2026-07-22 | vk-swarm-design-system | fixed 2026-08-07 (tokens shipped in design-system Phase 1) |
| F-2026-07-29-01 | node Nodes page and swarm labels call removed /api/nodes and /api/swarm routes | high | fixed | frontend/src/pages/Nodes.tsx | session/2026-07-29 | 2026-07-29 | vk-swarm-node-ui-localize | shipped 2026-08-05 |
| F-2026-07-29-02 | node board still consumes MergedProject via bridge endpoint, repoint to Project | medium | fixed | frontend/src/hooks/useMergedProjects.ts | session/2026-07-29 | 2026-07-29 | vk-swarm-node-ui-localize | shipped 2026-08-05 |
| F-2026-07-29-03 | hive drawer and navbar actions disabled pending hive APIs, no assign or delete E2E | medium | open | remote-frontend/src/ui/panels/TaskDrawer.tsx | session/2026-07-29 | 2026-07-29 | — | — |
| F-2026-07-29-04 | remote-frontend vitest flaky when run concurrently with vite build in same dir | low | open | remote-frontend/ | session/2026-07-29 | 2026-07-29 | — | — |
| F-2026-07-30-01 | `cargo test -p db` fails to compile: integration test needs the test-utils feature | medium | fixed | crates/db/tests/task_visibility_discriminator.rs:9 | session/2026-07-30 | 2026-07-30 | services-normalize-flaky-test | dev-docs/workstreams/services-normalize-flaky-test/README.md |
| F-2026-07-30-02 | Empty VK_DATABASE_PATH silently relocates the database to CWD | low | fixed | crates/utils/src/assets.rs:61 | session/2026-07-30 | 2026-07-30 | worktree-orphan-sweep-guard | dev-docs/workstreams/worktree-orphan-sweep-guard/README.md |
| F-2026-07-30-03 | Orphan worktree cleanup deletes worktrees with uncommitted changes (no dirty guard) | high | fixed | crates/local-deployment/src/container.rs:319-383 | session/2026-07-30 | 2026-07-30 | worktree-orphan-sweep-guard | dev-docs/workstreams/worktree-orphan-sweep-guard/README.md |
| F-2026-07-30-04 | WAL diagnostics hardcode asset_dir()/db.sqlite, ignoring VK_DATABASE_PATH | low | fixed | crates/server/src/routes/diagnostics.rs:109 | session/2026-07-30 | 2026-07-30 | — | fixed 2026-08-07 |
| F-2026-07-30-05 | Instance registry keyed on project_root only; two instances collide | low | fixed | crates/utils/src/port_file.rs:123-141 | session/2026-07-30 | 2026-07-30 | — | fixed 2026-08-07 |
| F-2026-07-31-01 | SettingsMobile.test.tsx asserts 6 accordion sections but component renders 8 (stale test) | medium | fixed | frontend/src/pages/settings/__tests__/SettingsMobile.test.tsx:85,185,217,245 | session/2026-07-31 | 2026-07-31 | — | fixed 2026-08-07 |
| F-2026-07-31-02 | SystemSettings.test.tsx suite fails to load: vi.mock factory closes over hoisted import | medium | fixed | frontend/src/pages/settings/__tests__/SystemSettings.test.tsx:40 | session/2026-07-31 | 2026-07-31 | — | fixed 2026-08-07 |
| F-2026-07-31-03 | frontend vitest red at baseline: 8 files / 15 tests failing, unrelated to any active workstream | medium | fixed | frontend/ (BottomNav, MessageQueuePanel, ConversationFocusMode, taskSorting, DesignSystem, MobileIntegration) | session/2026-07-31 | 2026-07-31 | — | fixed 2026-08-07 |
| F-2026-07-31-04 | LinkToLocalFolderDialog orphaned by task 302; its API client, hook and server route are still live | medium | fixed | frontend/src/components/dialogs/projects/LinkToLocalFolderDialog.tsx | session/2026-07-31 | 2026-07-31 | node-ui-localize-followups | dev-docs/workstreams/node-ui-localize-followups/README.md |
| F-2026-07-31-05 | Stale query key ['mergedProjects'] invalidated in linkLocalFolder onSuccess is now a no-op | low | fixed | frontend/src/hooks/useProjectMutations.ts:79 | session/2026-07-31 | 2026-07-31 | node-ui-localize-followups | dev-docs/workstreams/node-ui-localize-followups/README.md |
| F-2026-07-31-06 | Stale doc comment references the removed merged projects view | low | fixed | crates/db/src/models/project/mod.rs:106 | session/2026-07-31 | 2026-07-31 | node-ui-localize-followups | dev-docs/workstreams/node-ui-localize-followups/README.md |
| F-2026-07-31-07 | remote-frontend/src/types/shared/types.ts is a hand-copied duplicate that has drifted from generated shared/types.ts | medium | fixed | remote-frontend/src/types/shared/types.ts | session/2026-07-31 | 2026-07-31 | — | fixed 2026-08-07 |
| F-2026-07-31-08 | i18n key settings.swarm.hiveNotConnected undefined in all locales; ja/ko/es fall back to English | low | fixed | frontend/src/i18n/locales/*/settings.json | session/2026-07-31 | 2026-07-31 | node-ui-localize-followups | dev-docs/workstreams/node-ui-localize-followups/README.md |
| F-2026-08-01-01 | useDiffStream and useRemoteConnectionStatus 503 discrimination is unpinned; an unconditional guard survives the suite | low | fixed | frontend/src/hooks/useDiffStream.ts:86, frontend/src/hooks/useRemoteConnectionStatus.ts:68 | session/2026-08-01 | 2026-08-01 | node-ui-localize-followups | dev-docs/workstreams/node-ui-localize-followups/README.md |
| F-2026-08-01-02 | useAvailableNodes retry suppression is unpinned; test wrapper sets retry:false so retry behaviour cannot be observed | low | fixed | frontend/src/hooks/useAvailableNodes.test.ts | session/2026-08-01 | 2026-08-01 | node-ui-localize-followups | dev-docs/workstreams/node-ui-localize-followups/README.md |
| F-2026-08-03-01 | Node OAuth handoff does not complete; user cannot sign in on a node and has no permissions | high | triaged | crates/server/src/routes/oauth.rs:80-119 | session/2026-08-03 | 2026-08-03 | hive-oauth-sw-bypass | dev-docs/workstreams/hive-oauth-sw-bypass/README.md |
| F-2026-08-03-02 | Hive SW intercepts /v1/oauth/* OAuth navigations; blocks node AND hive sign-in until unregistered | high | triaged | remote-frontend/vite.config.ts:19-20 | session/2026-08-03 | 2026-08-03 | hive-oauth-sw-bypass | dev-docs/workstreams/hive-oauth-sw-bypass/README.md |
| F-2026-08-04-01 | OAuthDialog polls /api/auth/status forever with no timeout or error branch, so auth failures present as an endless spinner | medium | open | frontend/src/components/dialogs/global/OAuthDialog.tsx:95-113 | session/2026-08-04 | 2026-08-04 | hive-oauth-sw-bypass | dev-docs/workstreams/hive-oauth-sw-bypass/README.md |
| F-2026-08-04-02 | test_fast_execution_no_lost_logs flakes in full-workspace runs; discarded tokio timeout races the assertion | medium | promoted | crates/services/tests/normalize_sync_test.rs:359-368 | session/2026-08-04 | 2026-08-04 | services-normalize-flaky-test | dev-docs/workstreams/services-normalize-flaky-test/README.md |
| F-2026-08-05-01 | Dangling shared_task_id makes a node task permanently undeletable; hive 404 aborts the delete instead of falling back to local | high | promoted | crates/server/src/routes/tasks/handlers/remote.rs:229-231 | session/2026-08-05 | 2026-08-05 | node-task-delete-dangling-shared-id | dev-docs/workstreams/node-task-delete-dangling-shared-id/README.md |
| F-2026-08-05-02 | frontend format:check fails on 35 files; lint is ESLint-only so Prettier was never gated | medium | fixed | frontend/package.json | session/2026-08-05 | 2026-08-05 | frontend-prettier-debt | dev-docs/workstreams/frontend-prettier-debt/README.md |
| F-2026-08-09-01 | Claude executor initialize hooks payload hangs claude-code 2.1.114 — all server-spawned agent runs stall pre-init when ~/.claude/settings.json has hooks | high | open | crates/executors/src/executors/claude.rs:175 | wai/vk-swarm-task-breakdown DV-3 | 2026-08-09 | claude-executor-hooks-hang | dev-docs/workstreams/claude-executor-hooks-hang/README.md |
| F-2026-08-09-02 | cargo-watch dev loop never rebuilds/restarts vks-mcp-server — node spawns a stale MCP binary after backend changes | medium | open | package.json dev scripts | wai/vk-swarm-task-breakdown DV-5 | 2026-08-09 | mcp-server-dev-loop-rebuild | dev-docs/workstreams/mcp-server-dev-loop-rebuild/README.md |
| F-2026-08-16-01 | Dead task-lifecycle sync fns: sync_from_shared_task (zero callers), delete_by_shared_task_id + delete_stale_shared_tasks (test-only; superseded by ADR-0007 soft-unlink) — remove with their tests | low | open | crates/db/src/models/task/sync.rs:20,375,396 | wai/vk-swarm-event-bus amendment 3 | 2026-08-16 | — | allowlisted in event-bus conformance guard (task 021) |
| F-2026-08-16-03 | Dead SQL arm: upsert_remote_task version guard "OR tasks.remote_version IS NULL" is unreachable (remote_version INTEGER NOT NULL DEFAULT 1 since 20260102051142) | low | open | crates/db/src/models/task/sync.rs:351 | wai/vk-swarm-event-bus panel 022A | 2026-08-16 | — | pre-existing; found during task 022 review |
| F-2026-08-16-02 | reconcile_completed entity_count is unreliable on sync failure: 0 is ambiguous ("sync failed" vs "found nothing") AND a failed org sync returns a stale non-zero count (find_remote_projects reads the local table after sync_organization warns-and-continues) — needs enum field or separate failure event | low | open | crates/services/src/services/node_runner.rs:879-890,1298-1331 | wai/vk-swarm-event-bus task 008 + panel B | 2026-08-16 | — | deliberate reading recorded in task 008; no live consumer yet |
| F-2026-08-19-01 | Container orphan-worktree sweep reachable from test/scratch deployments — any LocalContainerService construction spawns cleanup_orphaned_worktrees against the SHARED worktree base dir; a test DB with no task_attempts rows marks every real worktree orphaned (mitigated per-binary via env Once in for_test, but the reach is structural) | medium | open | crates/local-deployment/src/container.rs:320, crates/local-deployment/src/lib.rs:514-531 | wai/vk-swarm-event-bus close | 2026-08-19 | — | needs a constructor-level opt-out instead of ambient env |
| F-2026-08-19-02 | NodeRunnerConfig::from_env reads ambient VK_HIVE_URL/VK_NODE_API_KEY — a test or scratch deployment built via from_parts on a machine with node env configured will connect to the LIVE hive; no test-mode guard exists (run mitigated by explicitly unsetting hive vars in the harness env) | medium | open | crates/services/src/services/node_runner.rs:62 | wai/vk-swarm-event-bus close | 2026-08-19 | — | consider an explicit override/disable seam on from_parts |
| F-2026-08-23-01 | FileBackend::clear swallows delete errors — Hive disconnect returns success even when the credentials file survives | low | open | crates/services/src/services/oauth_credentials.rs:201-204 | session/2026-08-23 | 2026-08-23 | — | O8 invariant holds (sessions revoked first); observability gap found during task 012 round-2 (ledger ## Task 012 decisions) |
<!-- WAI:BACKLOG:END -->

## Triage notes

### 2026-08-07 — frontend cleanup bundle (branch `fix/frontend-cleanup-bundle`)

- **F-2026-07-31-04/-05 → fixed.** `LinkToLocalFolderDialog` deleted along with
  `projectsApi.linkLocalFolder`, the `linkLocalFolder` mutation (which also removes the stale
  `['mergedProjects']` invalidation), the Rust `POST /api/projects/link-local` route/handler,
  and `LinkToLocalFolderRequest` (types.rs + generate_types + regenerated `shared/types.ts`).
  Verified unreferenced by `remote-frontend` before removal.
- **F-2026-07-31-06 → fixed.** Doc comment repointed from "merged projects view" to
  "project stats".
- **F-2026-07-31-07 → fixed.** `remote-frontend/src/types/shared/` hand-copy deleted;
  tsconfig/vite `shared` alias repointed to the generated repo-root `shared/`; five
  `@/types/shared/types` imports repointed to `shared/types`. Copy-only types
  (`MergedProject`, `MergedProjectsResponse`, `NodeLocation`) were unreferenced.
- **F-2026-07-31-08 → fixed.** `settings.swarm.hiveNotConnected` added to en/ja/ko/es.
- **F-2026-08-01-01 → fixed.** New `useDiffStream.test.ts` and
  `useRemoteConnectionStatus.test.ts` pin the 503 discrimination: HiveNotConfigured (503 +
  discriminator) is quiet, a plain 503 (outage) and a 500 must surface — an unconditional
  guard fails both suites.
- **F-2026-08-01-02 → fixed.** `useAvailableNodes.test.ts` wrapper now enables retries
  (`retry: 2, retryDelay: 0`); asserts exactly 1 call for HiveNotConfigured (resolution
  suppresses retry) and 3 calls for a real 500 (control proving retries are live).
- **F-2026-07-22-01 → fixed (already resolved on main).** `--vks-text-dim`
  (`styles/tokens/colors.css:39`) and `@keyframes vks-pulse` (`styles/components.css:157`)
  shipped with design-system Phase 1 (PR #466) and are test-pinned
  (`colors.test.ts`, `components.test.ts`); NodeCard needs no change.
- **F-2026-08-05-02 → fixed.** `prettier --write` applied to the 36 red files in a dedicated
  formatting-only commit; `npm run format:check` now green.

### 2026-07-04 — backlog triage

- **F-2026-07-04-02 → wontfix (deleted).** The April 2026 pre-fork
  `reference-architecture-alignment-design` spec is unreferenced anywhere in the repo
  (grep over the whole tree returns only BACKLOG.md and MASTER.md). Its themes — executor
  config split, ordered log stream as source of truth, queue-vs-injection contract,
  capability flags, executor version pinning — have all been absorbed and shipped via the
  post-fork workstreams: `vk-swarm-node-foundations` (PR #447), `vk-swarm-hive-redesign`
  (PR #451), and the `560a3400 "Align live playback and queue semantics with reference"`
  commit. The post-fork umbrella spec
  `docs/superpowers/specs/2026-06-25-vk-swarm-refactor.md` plus
  `docs/specs/2026-06-25-vk-swarm-phase1-analysis.md` are the canonical replacements.
  File `git rm`'d in this same commit.
- **F-2026-07-04-03 → wontfix (deleted).** `PLAN.md` ("Fix Cross-Node Task and Attempt
  Viewing") is a stale repo-root planning doc, unreferenced anywhere except BACKLOG/MASTER.
  Its work is fully shipped: PR #403 (cross-node viewing via Hive fallback), #428 (cross-node
  task display), #442 (remote task variable fallbacks), plus the `RemoteTaskContext` /
  `RemoteAttemptNeeded` middleware types now live in
  `crates/server/src/middleware/model_loaders.rs:100-121`. File `git rm`'d in this same commit.
- **F-2026-07-04-04 → left open (owned by another session).** The finding's title says
  "uncommitted … on merged branch" but the actual state is the opposite on both counts: the
  `serial_test = { version = "3", features = ["file_locks"] }` edit in
  `crates/remote/Cargo.toml` IS committed (on branch `fix/preexisting-gate-failures`,
  commit `7fc7955e`) and the branch is NOT merged — it sits 2 commits ahead of `origin/main`
  and `git branch -r --contains` shows it only on `origin/fix/preexisting-gate-failures`.
  The edit is NOT superseded on main: `origin/main` still carries plain
  `serial_test = "3"` (no `file_locks` feature). The `file_locks` feature is not referenced
  by any code on main, so its relevance depends on the crisp-river session's unmerged test
  changes. Per the user's instruction the crisp-river worktree is an ACTIVE session and must
  not be touched from here; the finding stays `open` and is owned by that session.

### 2026-07-22 — reconciliation sweep

- **F-2026-07-04-04 → fixed.** Main now carries
  `serial_test = { version = "3", features = ["file_locks"] }` at `crates/remote/Cargo.toml:45`
  and the file is clean in git. The offending downgrade (removing `file_locks`) exists only on
  the unmerged `opencode/crisp-river` branch (1 commit ahead of main); main is unaffected, so
  the finding is closed here. If crisp-river merges it must rebase past the main state.
- **F-2026-07-22-01 → open.** `remote-frontend/src/components/swarm/NodeCard.tsx:48,53` uses
  `vks-pulse` / `--vks-text-dim` tokens that are defined nowhere in `remote-frontend`
  (silent styling no-op on the live Nodes page). Tokens are a Phase-1 deliverable of
  `vk-swarm-design-system`, so tagged to that workstream — verify during its Phase 1.
- **Link hygiene:** F-2026-07-06-01/-02 spec links repointed from deleted
  `docs/superpowers/specs/` staging paths to the graduated `dev-docs/workstreams/*/spec/` homes.

### 2026-07-07 — F-2026-07-06-01 promoted

- **F-2026-07-06-01 → promoted (workstream `hive-node-api-key-ui`).** The finding (Hive UI
  lacks a Generate API Key button, blocking node onboarding) was promoted via
  `/wai:finding-promote`. Intent spec at
  `docs/superpowers/specs/2026-07-07-hive-node-api-key-ui.md`; workstream tracker at
  `dev-docs/workstreams/hive-node-api-key-ui/README.md`. Confirmed the gap is UI-only: the
  `/v1/nodes/api-keys` backend, the `remote-frontend` API client
  (`nodesApi.{listApiKeys,createApiKey,revokeApiKey,unblockApiKey}`), and the
  `NodeApiKey`/`CreateNodeApiKey*` types already exist. Next: `/wai:spec hive-node-api-key-ui`
  to add the design, then `/wai:precheck`.

### 2026-07-10 — F-2026-07-06-02 shipped

- **F-2026-07-06-02 → shipped (workstream `fix-nonloopback-signin`).** The finding (sign-in
  broken on non-loopback HTTP origins due to `crypto.subtle` undefined) was resolved by
  PR #463 (merged 2026-07-10). Pure-TS SHA-256 fallback implemented in `pkce.ts` with
  capability detection. 137 tests, 100% line coverage on target files.

### 2026-07-29 — backlog triage

- **F-2026-07-11-01 → fixed.** No longer reproduces. `npx vitest run src/AppRouter.test.tsx`
  → `Test Files 1 passed (1) / Tests 23 passed (23)`, and the full suite is green
  (`Test Files 52 passed (52) / Tests 405 passed (405)`), so the isolation failure that only
  appeared in the full run is gone.
- **F-2026-07-11-02 → fixed.** Location corrected: the test is at
  `remote-frontend/scripts/no-push-invariant.test.mjs`, not repo-root `scripts/`.
  `node --test scripts/no-push-invariant.test.mjs` (from `remote-frontend/`) →
  `✔ no new push channels (WebSocket/EventSource/SSE) in the hive frontend source` · `pass 1 / fail 0`.

- **F-2026-07-29-01 → severity raised medium → high; workstream `vk-swarm-node-ui-localize`.**
  Both call sites are on LIVE routes, so this is user-visible 404s, not dead code:
  - `frontend/src/App.tsx:153` routes `/nodes` → `pages/Nodes.tsx` → `nodesApi` → `/api/nodes`
  - `frontend/src/App.tsx:189` routes `/settings/swarm` → `SwarmSettings.tsx:163` →
    `NodeProjectsSection` → `nodesApi.list(orgId)` / `nodesApi.listProjects(nodeId)`
    (also reachable via `MobileSettingsAccordion.tsx:60`)
  Neither `nodes` nor `swarm` is merged into the node server's router
  (`crates/server/src/routes/mod.rs:44-71` registers no such module), so every one of these
  requests 404s.
- **F-2026-07-29-02 → workstream `vk-swarm-node-ui-localize`.** Unchanged severity; this is the
  typed `useMergedProjects → useProjects` refactor the workstream already owns.

**Spec defect recorded against `vk-swarm-node-ui-localize`.** Its README's "Keep (do NOT remove —
live non-Nodes consumers)" list justifies keeping `nodesApi` and `NodeProjectsSection` on the basis
that the Nodes feature was deleted by node-foundations. It was not: `pages/Nodes.tsx` is still
routed at `App.tsx:153`. The entanglement map must be re-verified against merged code during
`/wai:prd-new` + `/wai:spec` rather than trusted as written.

Not actioned this session (correctly parked, no work needed now):
- **F-2026-07-22-01** — owned by `vk-swarm-design-system`; belongs to that workstream's token pass.
- **F-2026-07-29-03** — blocked on the hive APIs it names; nothing to build against yet.
- **F-2026-07-29-04** — low-severity local flake (concurrent vitest + vite build in one dir);
  no product impact.
