---
id: "302"
phase: 3
title: "Repoint the board onto ProjectWithStats and delete LocationBadges"
status: ready
depends_on: ["301"]
parallel: false
conflicts_with: []
files:
  - frontend/src/hooks/useProjectsWithStats.ts
  - frontend/src/hooks/useMergedProjects.ts
  - frontend/src/lib/api/projects.ts
  - frontend/src/components/projects/ProjectList.tsx
  - frontend/src/components/projects/UnifiedProjectCard.tsx
  - frontend/src/components/projects/LocationBadges.tsx
  - frontend/src/components/layout/ProjectSwitcher.tsx
  - frontend/src/components/projects/ProjectList.test.tsx
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

Adapt the import of `ProjectList` and the count assertion to how the component actually renders
counts (read the component first). If `ProjectList` requires props or additional providers,
supply them — but do NOT weaken the assertion to "renders without crashing": a test that does not
assert a task count does not guard the regression this task exists to prevent.

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
and 5). If `frontend/src/hooks/index.ts` exports it, that file must be added to `files:` — if it
does, STOP and report rather than editing an unlisted file.

### 3. `ProjectList.tsx` — retype

- Replace the `MergedProject` type import with `ProjectWithStats`.
- Replace `useMergedProjects` with `useProjectsWithStats`.
- Replace every `MergedProject` annotation (lines ~23, 25, 57, 59, 126) with `ProjectWithStats`.
- Remove any filtering/branching on `has_local` or `nodes`: with the merge fields gone every
  entry is a local project. Where the component branched on `p.has_local`, keep the local branch
  and delete the other.

### 4. `UnifiedProjectCard.tsx` — retype and drop the badges

- Replace the `MergedProject` type import and the `project: MergedProject` / `onEdit` prop types
  with `ProjectWithStats`.
- **Anchor:** line 36 — delete `import { LocationBadges } from './LocationBadges';`
- **Anchor:** line 310 — delete `<LocationBadges project={project} />`

### 5. `ProjectSwitcher.tsx` — retype

- Replace `useMergedProjects` with `useProjectsWithStats` and rename the destructured
  `mergedData` to `projectsData`.
- **Anchor:** the `allProjects` `useMemo` (~line 50-70). It branches on `p.has_local` to decide
  local-vs-remote presentation. Every entry is now local: keep the `has_local === true` branch
  body, delete the `else` branch, and delete the `if (p.has_local)` condition itself.

### 6. Delete `frontend/src/components/projects/LocationBadges.tsx`

Its only consumer was `UnifiedProjectCard` (removed in step 4), and it renders `project.nodes`,
which the handler has hardcoded to `[]` since node-foundations — it has been rendering nothing.

## Allowed moves

- Only the files in `files:`. `/api/merged-projects`, `MergedProject`, and `getMerged` survive
  this task and are deleted by 303.

## STOP triggers

- If `ProjectList` or `ProjectSwitcher` uses `has_local` or `nodes` for anything other than the
  local/remote branch described above (e.g. a count, a sort key, a badge) — STOP and report; that
  is behaviour the plan did not anticipate.
- If `frontend/src/hooks/index.ts` re-exports `useMergedProjects` — STOP (that file is not in
  `files:`).
- If any file outside `files:` imports `LocationBadges` — STOP.
- If the new test passes without step 1-5 applied, it is hollow — STOP and strengthen it.

## Manual verification (record in decisions-ledger)

```bash
cd frontend && npx vitest run src/components/projects
# Expected: the new ProjectList test passes

cd frontend && npx tsc --noEmit
# Expected: no output

cd frontend && npm run lint
# Expected: clean

grep -rn 'useMergedProjects\|LocationBadges' frontend/src
# Expected: NO output
```

## Done when

- The board and the project switcher render from `/api/projects/with-stats`.
- `useMergedProjects.ts` and `LocationBadges.tsx` are deleted, with no surviving references.
- The new test asserts a task count renders (the `a85f7d63` regression guard).
- vitest, `tsc --noEmit`, and lint are clean.
