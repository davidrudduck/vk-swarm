---
id: "302"
phase: 3
title: "Repoint the board onto ProjectWithStats and delete LocationBadges"
status: passed
depends_on: ["301"]
parallel: false
conflicts_with: ["303"]
files:
  - frontend/src/hooks/useProjectsWithStats.ts
  - frontend/src/hooks/useMergedProjects.ts
  - frontend/src/lib/api/projects.ts
  - frontend/src/components/projects/ProjectList.tsx
  - frontend/src/components/projects/UnifiedProjectCard.tsx
  - frontend/src/components/projects/LocationBadges.tsx
  - frontend/src/components/layout/ProjectSwitcher.tsx
  - frontend/src/components/projects/ProjectList.test.tsx
  - frontend/src/components/projects/ProjectTypeFilter.tsx
siblings:
  - frontend/src/hooks/useMergedProjects.ts
irreversible: true
scope_test: "frontend/src/components/projects"
allowed_change: mixed
covers_criteria: [SC5]
---

## Failing test (write first)

This is the task that must prove the board regression `a85f7d63` fixed does not return — a blank
board with no task counts. Create
`frontend/src/components/projects/ProjectList.test.tsx` (if a test for this component already
exists at that path, APPEND to it rather than overwriting):

```typescript
import { describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ProjectWithStats } from 'shared/types';

vi.mock('@/lib/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/api')>();
  return {
    ...actual,
    projectsApi: {
      ...actual.projectsApi,
      getWithStats: vi.fn(),
    },
  };
});

const fixture: ProjectWithStats[] = [
  {
    id: '11111111-1111-1111-1111-111111111111',
    name: 'zeta',
    git_repo_path: '/tmp/zeta',
    created_at: new Date('2026-01-01T00:00:00Z'),
    remote_project_id: null,
    last_attempt_at: new Date('2026-02-01T00:00:00Z'),
    github_enabled: false,
    github_owner: null,
    github_repo: null,
    github_open_issues: 0,
    github_open_prs: 0,
    github_last_synced_at: null,
    task_counts: { todo: 3, in_progress: 1, in_review: 0, done: 2 },
  } as unknown as ProjectWithStats,
];

describe('ProjectList on ProjectWithStats', () => {
  it('renders a project and its task counts from /api/projects/with-stats', async () => {
    const { projectsApi } = await import('@/lib/api');
    vi.mocked(projectsApi.getWithStats).mockResolvedValue({ projects: fixture });

    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    render(
      <QueryClientProvider client={client}>
        <ProjectList />
      </QueryClientProvider>
    );

    await waitFor(() => expect(screen.getByText('zeta')).toBeInTheDocument());
    // The enrichment must survive the type change — this is the a85f7d63 regression guard
    expect(screen.getByText('3')).toBeInTheDocument();
  });
});
```

**Providers are NOT optional and NOT your decision.** Decomposition checked: `ProjectList.tsx:2`
imports `useNavigate` from `react-router-dom` (used at line 72) and `useTranslation` from
`react-i18next` (line 73). The render must therefore be wrapped in a `MemoryRouter` as well as the
`QueryClientProvider`:

```tsx
import { MemoryRouter } from 'react-router-dom';
...
render(
  <MemoryRouter>
    <QueryClientProvider client={client}>
      <ProjectList />
    </QueryClientProvider>
  </MemoryRouter>
);
```

If `react-i18next` is not already globally initialised by the vitest setup file, follow whatever
pattern the existing tests under `frontend/src/__tests__/` use — read one first; do not invent a
new i18n bootstrap.

**The count assertion:** counts are rendered by `<TaskCountPills counts={project.task_counts} … />`
(`UnifiedProjectCard.tsx:332`), and only when at least one count is non-zero
(`hasTaskCounts`, lines 172-175). Read `TaskCountPills` and assert against how it actually renders
the `todo: 3` value. Do NOT weaken the assertion to "renders without crashing" — a test that does
not assert a task count does not guard the regression this task exists to prevent. If
`TaskCountPills` renders counts in a way you cannot assert on text alone, add a `data-testid` to
`TaskCountPills` — and if that file is not in `files:`, STOP and report rather than editing it.

## Amendments (ORCHESTRATOR, pre-dispatch — DICTATED)

**F1 — the `Done when` / verification grep for `local_project_id` was IMPOSSIBLE; corrected.** The
original asserted `grep -rn 'has_local\|local_project_id' frontend/src` returns NO output. But
`local_project_id` is a legitimate field on FOUR unrelated types that must survive:

| File | What it is |
|---|---|
| `frontend/src/types/nodes.ts:36` | `NodeProject.local_project_id` |
| `frontend/src/types/swarm.ts:47,73` | swarm project types |
| `frontend/src/lib/api/tasks.ts:25` | task API type |
| `frontend/src/lib/electric/collections.ts:56` | electric collection schema |

plus live consumers in `components/swarm/NodeProjectsSection.tsx` and `SwarmProjectRow.tsx`. The
unscoped grep would report failure even on a PERFECT implementation. Same defect class as tasks
201/202's vacuous `scope_test`. Corrected below: `has_local` (which exists ONLY on `MergedProject`)
stays unscoped; `local_project_id` is scoped to the three consumer directories.

**F2 — blast radius VERIFIED before dispatch; the STOP trigger should not fire.**
`has_local` appears in EXACTLY the four files in `files:` — `ProjectSwitcher.tsx` (1),
`LocationBadges.tsx` (2), `ProjectList.tsx` (3), `UnifiedProjectCard.tsx` (8). `MergedProject`
itself is referenced by exactly six files, all in `files:` (the four above plus
`hooks/useMergedProjects.ts` and `lib/api/projects.ts`). **No dropped-field reference exists outside
your allowlist.** If you find one, the STOP trigger is real — report it.

**F3 — `.nodes` hits in `CreateAttemptDialog.tsx` and `SwarmProjectRow.tsx` are NOT yours.**
`CreateAttemptDialog.tsx:73` is `availableNodesData?.nodes` (that file does not reference
`MergedProject` at all) and `SwarmProjectRow.tsx` uses swarm node types. Do not touch either.

**F4 — the STOP triggers verified clear:** `frontend/src/hooks/index.ts` does NOT re-export
`useMergedProjects`; `LocationBadges` is imported only by `UnifiedProjectCard.tsx`;
`ProjectTypeFilter` is imported only by `ProjectList.tsx`.

**F5 — `scope_test: "frontend/src/components/projects"` currently has NO test files.** It will have
exactly one after you create `ProjectList.test.tsx`, so the gate becomes meaningful only because of
your test. That makes the "if the new test passes without steps 1-5 applied, it is hollow" STOP
trigger the real check — take it seriously.

**F6 — the rest of the frontend vitest suite is RED AT BASELINE** (8 files / 15 tests:
F-2026-07-31-01..03). Your new test must PASS, and the pre-existing failing set must remain
byte-identical. Do NOT fix the baseline failures. Note `ProjectList.test.tsx` is NEW, so the counts
will change by exactly your added test — report the new totals and confirm no PREVIOUSLY-passing
test broke.

## Change

### 1. Add the API client method — `frontend/src/lib/api/projects.ts`

- **Anchor:** line 106-109, the `getMerged` member
- **After:** add a sibling member immediately below it (leave `getMerged` in place; task 303
  removes it):

```typescript
  getWithStats: async (): Promise<ProjectsWithStatsResponse> => {
    const response = await makeRequest('/api/projects/with-stats');
    return handleApiResponse<ProjectsWithStatsResponse>(response);
  },
```

Add `ProjectsWithStatsResponse` to the existing `shared/types` type import in this file.

### 2. Create `frontend/src/hooks/useProjectsWithStats.ts`

Read `frontend/src/hooks/useMergedProjects.ts` first — the new hook must keep its `staleTime` and
overall shape (sibling alignment):

```typescript
import { useQuery } from '@tanstack/react-query';
import { projectsApi } from '@/lib/api';
import type { ProjectsWithStatsResponse } from 'shared/types';

/**
 * Hook to fetch this node's local projects with display enrichment
 * (task counts, last attempt, GitHub counts).
 */
export function useProjectsWithStats() {
  return useQuery<ProjectsWithStatsResponse>({
    queryKey: ['projects-with-stats'],
    queryFn: () => projectsApi.getWithStats(),
    staleTime: 30000,
  });
}
```

Delete `frontend/src/hooks/useMergedProjects.ts` once its two consumers are repointed (steps 3
and 5). Decomposition verified `frontend/src/hooks/index.ts` does NOT re-export it
(`grep -n 'useMergedProjects' frontend/src/hooks/index.ts` → no output), so no barrel edit is
needed. If that grep now returns a hit, STOP — `index.ts` is not in `files:`.

### The dropped-field rewrite rule (read before steps 3-5)

`ProjectWithStats` drops `has_local`, `local_project_id`, and `nodes`, and the three consumers
reference them **20 times**. Retyping alone will NOT compile. Apply this rule mechanically —
it is derived from what the handler has actually been returning since node-foundations
(`merged.rs`: `has_local: true`, `local_project_id: Some(project.id)`, `nodes: Vec::new()`):

| Expression | Always evaluated to | Rewrite |
|---|---|---|
| `project.has_local` | `true` | drop the condition, keep the guarded branch |
| `!project.has_local` | `false` | the branch is dead — delete it |
| `project.local_project_id` | `Some(project.id)` | `project.id` |
| `project.nodes` | `[]` | any filter/count over it is 0 — delete the branch |

### 3. `ProjectList.tsx` — retype and drop the dead filter

- Replace the `MergedProject` type import with `ProjectWithStats`, and `useMergedProjects` with
  `useProjectsWithStats` (rename `mergedData` → `projectsData`, `refetchMerged` → `refetchProjects`).
- Replace every `MergedProject` annotation (lines 23, 25, 57, 59, 126) with `ProjectWithStats`.
- **Line 127-128** — before:
```typescript
      if (project.has_local && project.local_project_id) {
        navigate(`/settings/projects?projectId=${project.local_project_id}`);
```
  after:
```typescript
      navigate(`/settings/projects?projectId=${project.id}`);
```
  (keep the surrounding function body; only the guard and the id change)
- **Delete `filterProjects` entirely** (lines 56-69). Its `local` case is now every project and
  its `swarm` case is always empty. At line 107 replace
  `const filtered = filterProjects(projects, typeFilter);` with `const filtered = projects;`
  and drop `typeFilter` from that `useMemo`'s dependency array.
- **Delete the `counts` `useMemo`** (lines 85-93) and the `nodeCount` `useMemo` (lines 96-104) —
  both exist only to count `has_local` / `nodes`.
- **Delete the type-filter UI:** the `typeFilter`/`setTypeFilter` state (line 76), the
  `ProjectTypeFilterTabs` import (lines 16-18), and the `<ProjectTypeFilterTabs … />` block
  (lines 172-176). Keep `<ProjectSortControls …>` and the `{hasProjects && (…)}` wrapper around it.
- **Delete the node-count subtitle** (lines 155-160, the `{nodeCount > 0 && (…)}` span). Keep
  `{t('subtitle')}`.

### 4. `UnifiedProjectCard.tsx` — retype and drop the badges

- Replace the `MergedProject` type import and the `project: MergedProject` / `onEdit` prop types
  with `ProjectWithStats`.
- **Anchor:** line 36 — delete `import { LocationBadges } from './LocationBadges';`
- **Anchor:** line 310 — delete `<LocationBadges project={project} />`
- Apply the rewrite rule to all eleven dropped-field references:
  - **lines 75, 100, 145** — `if (!project.has_local || !project.local_project_id) return;` →
    delete the guard line entirely
  - **lines 85, 103, 116, 149** — `project.local_project_id` → `project.id`
  - **line 132** — `if (!project.has_local || !project.git_repo_path) return;` →
    `if (!project.git_repo_path) return;`
  - **lines 216, 251, 316** — `project.has_local && ` → delete just that conjunct, keep the rest
    of the condition and the JSX
  - **line 279** — `{!project.has_local && project.remote_project_id && (…)}` → the whole block
    is dead (`!has_local` is always false); delete it

### 5. `ProjectSwitcher.tsx` — retype

- Replace `useMergedProjects` with `useProjectsWithStats`, and `mergedData` → `projectsData`.
- **Anchor:** the `allProjects` `useMemo`, lines 56-70. Before it branches
  `if (p.has_local) { …local… } else if (p.nodes.length > 0) { …remote… }`. After: keep the
  `has_local` branch body unconditionally and delete the `else if (p.nodes.length > 0)` branch
  and everything in it (including the `firstNode` lookup).

### 6. Delete `frontend/src/components/projects/LocationBadges.tsx`

Its only consumer was `UnifiedProjectCard` (removed in step 4), and it renders `project.nodes`,
which the handler has hardcoded to `[]` since node-foundations — it has been rendering nothing.

### 7. Delete `frontend/src/components/projects/ProjectTypeFilter.tsx`

Decomposition verified `ProjectList.tsx` is its only consumer
(`git grep -n 'ProjectTypeFilter' -- frontend/src` → hits only in `ProjectList.tsx` and its own
file). With the local/swarm distinction gone it has nothing to filter.

## Allowed moves

- Only the files in `files:`. `/api/merged-projects`, `MergedProject`, and `getMerged` survive
  this task and are deleted by 303.

## STOP triggers

- If you find a `has_local`, `local_project_id`, or `nodes` reference NOT listed in the tables
  above — STOP and report it. Do not apply the rewrite rule by analogy to an unlisted site.
- If deleting the type-filter UI leaves `hasProjects` or `ProjectSortControls` unused — STOP;
  they are meant to survive.
- If `frontend/src/hooks/index.ts` re-exports `useMergedProjects` — STOP (that file is not in
  `files:`; decomposition verified it does not, so a hit means something changed).
- If any file outside `files:` imports `LocationBadges` — STOP.
- If the new test passes without step 1-5 applied, it is hollow — STOP and strengthen it.

## Manual verification (emit verbatim; the ORCHESTRATOR records it)

```bash
cd frontend && npx vitest run src/components/projects
# Expected: the new ProjectList test passes

cd frontend && npx tsc --noEmit
# Expected: no output

cd frontend && npm run lint
# Expected: clean

grep -rn 'useMergedProjects\|LocationBadges\|ProjectTypeFilter' frontend/src
# Expected: NO output

# F1 (see Amendments): this grep MUST be scoped. `local_project_id` and `.nodes` are legitimate
# field names on UNRELATED types that must survive — NodeProject (frontend/src/types/nodes.ts:36),
# the swarm types (types/swarm.ts:47,73), the task API type (lib/api/tasks.ts:25) and the electric
# collection (lib/electric/collections.ts:56). An unscoped grep reports failure even on a perfect
# implementation.
grep -rn 'has_local' frontend/src
# Expected: NO output — has_local exists ONLY on MergedProject

grep -rn 'local_project_id' frontend/src/components/projects frontend/src/components/layout frontend/src/hooks
# Expected: NO output (the MergedProject-typed sites only)
```

## Done when

- The board and the project switcher render from `/api/projects/with-stats`.
- `useMergedProjects.ts`, `LocationBadges.tsx`, and `ProjectTypeFilter.tsx` are deleted, with no
  surviving references.
- No `has_local`, `local_project_id`, or `nodes` reference survives in `frontend/src`.
- The new test asserts a task count renders (the `a85f7d63` regression guard).
- vitest, `tsc --noEmit`, and lint are clean.
