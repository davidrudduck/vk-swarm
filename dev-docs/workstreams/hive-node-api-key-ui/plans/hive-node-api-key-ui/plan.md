---
workstream: hive-node-api-key-ui
spec: docs/superpowers/specs/2026-07-07-hive-node-api-key-ui.md
phase_count: 3
task_count: 8
---

# hive-node-api-key-ui — Plan

## Approach

A UI-only workstream that surfaces Node API key management on the existing
`Nodes.tsx` page. The backend, API client, and types are already in place
(`/v1/nodes/api-keys` is live; `nodesApi.{listApiKeys,createApiKey,revokeApiKey,unblockApiKey}`
and the `NodeApiKey` family of types are exported). All changes land in
`remote-frontend/` and four shared locale JSON files in `frontend/src/i18n/locales/`.

The work decomposes as a single component (`NodeApiKeySection`) plus the integration steps
that wire it onto the page and into the i18n surface. Each task bundles a failing test
with the implementation step that makes it pass, so progress is verifiable task-by-task
and the gate can run after every change. A Sibling-read step on task 001 grounds the
divergence from `frontend/src/components/org/NodeApiKeySection.tsx` (the reference impl)
and the in-tree pattern siblings in `remote-frontend/src/components/swarm/`.

Sequential tracer-bullet order — no parallel authoring: each phase depends on the
previous one's output, and the Phase 1 component build depends on the test file being
in place before each behavior is added. Phase 2 integrates and localizes; Phase 3 is
a single verification task that exercises the full gate.

## Phases

### Phase 1 — Component build (foundation + behaviors)

Build `remote-frontend/src/components/swarm/NodeApiKeySection.tsx` incrementally,
in lockstep with its test file. The component file is single; each behavior is one
task. The test file is created in task 001 with the rendering baseline; tasks 002–004
extend it alongside the component. The pre-flight Sibling-read in task 001 prevents
the "reimplemented-without-reading" failure class (extension-sdk-boundary-fix
post-mortem).

| id  | task                                                                | dep:      | conflicts: |
|-----|---------------------------------------------------------------------|-----------|------------|
| 001 | Create NodeApiKeySection skeleton + list/loading/empty tests (TS1-TS3) | dep: -    | conflicts: - |
| 002 | Add create-Dialog flow + TS4 test                                    | dep: 001  | conflicts: - |
| 003 | Add revoke + unblock mutations + TS5/TS6 tests                       | dep: 002  | conflicts: - |
| 004 | Add error-state Alert + TS7 test                                     | dep: 002  | conflicts: - |

### Phase 2 — Integration & i18n

Mount the section into the existing Nodes page (above the node grid) and add the
localization surface. The barrel export is its own task so the integration is a
single import from `@/components/swarm`. The i18n task adds the `settings.swarm.apiKeys.*`
key family to all four locale files and adds the TS9 "every t() has a matching key"
test in the same diff so the task is self-verifying.

| id  | task                                                              | dep:      | conflicts: |
|-----|-------------------------------------------------------------------|-----------|------------|
| 005 | Export NodeApiKeySection from the swarm barrel + smoke test       | dep: 001  | conflicts: - |
| 006 | Compose into Nodes.tsx + extend Nodes.test.tsx (TS8)              | dep: 002, 005 | conflicts: - |
| 007 | Add settings.swarm.apiKeys.* to en/es/ja/ko + TS9 test            | dep: 004  | conflicts: - |

### Phase 3 — Verification

Final gate. Manual verification: run the typecheck and the full remote-frontend
test suite; record results in the decisions-ledger.

| id  | task                                  | dep:      | conflicts: |
|-----|---------------------------------------|-----------|------------|
| 008 | Run typecheck + vitest (verification) | dep: 006, 007 | conflicts: - |

## SC / TS coverage map

| Spec ID | Claims                                       |
|---------|----------------------------------------------|
| SC1     | 001, 005, 006                                |
| SC2     | 002                                          |
| SC3     | 001                                          |
| SC4     | 003                                          |
| SC5     | 003                                          |
| SC6     | 007                                          |
| SC7     | 001, 002, 003, 004, 006, 007                 |
| TS1     | 001                                          |
| TS2     | 001                                          |
| TS3     | 001                                          |
| TS4     | 002                                          |
| TS5     | 003                                          |
| TS6     | 003                                          |
| TS7     | 004                                          |
| TS8     | 006                                          |
| TS9     | 007                                          |

## Avoid these traps (pre-empted from prior ledgers + spec)

- **Sibling-read is mandatory on the new component file.** The reference impl
  `frontend/src/components/org/NodeApiKeySection.tsx` and the in-tree pattern siblings
  (`SwarmHealthSection.tsx`, `NodeProjectsSection.tsx`) carry exclusions, guards, and
  structural choices (e.g. `useTranslation(['settings', 'common'])`, `t(key, fallback)`,
  `Card > CardHeader > CardContent` skeleton, `if (!organizationId) return <Card><Alert>…</Card>`)
  that must be read before authoring. The implementer records divergences in the
  task 001 decisions-ledger entry.
- **i18n fallbacks are mandatory.** Every `t(key, fallback)` call passes an English
  fallback string, so the UI works even if the locale JSON is missing the key. The
  TS9 test in task 007 fails until every fallback is promoted into the en locale.
- **Vitest is green.** A prior commit fixed the vite/vitest version conflict in
  `remote-frontend/package.json` (see `docs/plans/vk-swarm-design-system/decisions-ledger.md`,
  "task 101 second pass"). Tasks must not touch the package version.
- **The component is the only consumer of the API hooks.** Per spec Decision 7,
  `useQuery` / `useMutation` live in the component; no `useNodeApiKeys` hook is
  extracted this pass.
- **`useOrganizations` is the org-loading seam.** `Nodes.tsx` is the only place that
  calls `useOrganizations`; the section receives `organizationId` as a prop and
  gates its render on a non-empty value. No new role model.
- **`confirm()` for destructive actions, not AlertDialog.** Per spec Decision 6, the
  section uses `window.confirm` for revoke/unblock. The AlertDialog primitive stays
  in `@/components/ui/` and is not adopted this pass.
- **No new Settings page.** Per Out of scope, the section is composed into
  `Nodes.tsx` only; no route, no new page file.
- **No `isAdmin` prop.** The reference impl's `isAdmin` gating is dropped per
  spec Decision 5; all org members see all controls.
