# Breakdown tournament — round 1

**Topic:** vk-swarm-node-ui-localize
**Date:** 2026-07-30
**Method:** external CLI competitors (find + remediate), orchestrator-verified before applying

## Competitors

| Seat | CLI / model | Status |
|---|---|---|
| 1 | `codex exec` (Codex CLI, read-only sandbox) | dispatched |
| 2 | `opencode run --model ollama-cloud/glm-5.2` | completed |
| 3 | `agy --model gemini-3.5-flash-high` | **FAILED — quota exhausted** |

Seat 3 returned `Error: Individual quota reached. Please upgrade your subscription to increase
your limits. Resets in 131h47m26s.` and produced no report. Only ONE of three competitors failed,
so the sub-agent fallback (triggered at ≥2 failures) was not used. The round therefore ran with
two external competitors plus orchestrator verification, not three — recorded here rather than
papered over.

Because seat 3 died, peer cross-judging was not possible for its (absent) submission. Every
finding below was instead **independently verified by the orchestrator against the repo** before
being applied; the evidence column is the orchestrator's own command output, not the competitor's
claim.

## Findings applied

### T1 (blocker) — task 303 pointed the implementer at the wrong `impl` block

- **Found by:** orchestrator (independently confirmed by opencode)
- **Location:** `phase-3/303-delete-mergedproject.md` § Change step 3
- **Defect:** the task said "If a `impl From<...> for NodeLocation` block exists (around line
  90-110), delete it too". No such impl exists. Lines 88-110 are
  `impl From<Project> for RemoteNodeProject` — a DIFFERENT type that must survive, because
  `RemoteNodeProject` backs `UnifiedProject::Remote`.
- **Evidence:**
  ```bash
  $ sed -n '88p' crates/server/src/routes/projects/types.rs
  impl From<Project> for RemoteNodeProject {
  ```

- **Failure it would cause:** a literal implementer deletes the wrong impl and breaks
  `UnifiedProject::Remote`.
- **Remediation applied:** replaced the conditional with an explicit "There is NO
  `impl From<...> for NodeLocation`" warning naming lines 88-110 as must-survive, plus a new STOP
  trigger. Exact struct line ranges corrected (`MergedProject` 113-146, `NodeLocation` 150-164,
  `MergedProjectsResponse` 176-179).

### T2 (blocker) — task 302 could not compile: 20 references to dropped fields, no rule

- **Found by:** opencode
- **Location:** `phase-3/302-repoint-board-to-projectwithstats.md` § Change steps 3-5
- **Defect:** `ProjectWithStats` drops `has_local`, `local_project_id`, and `nodes`, but
  `UnifiedProjectCard` (11 sites), `ProjectList` (7 sites), and `ProjectSwitcher` (3 sites)
  reference them. The task said "retype" and "remove any filtering/branching on `has_local` or
  `nodes`" — it never said what `local_project_id` becomes, and said nothing at all about
  `UnifiedProjectCard`'s references.
- **Evidence:**
  ```bash
  $ grep -n 'has_local\|local_project_id\|\.nodes' frontend/src/components/projects/UnifiedProjectCard.tsx
  75:    if (!project.has_local || !project.local_project_id) return;
  85:      await projectsApi.delete(project.local_project_id);
  … 11 hits total
  ```

- **Failure it would cause:** the retyped component does not compile; the implementer must invent
  a replacement for `local_project_id` (STOP trigger fires and blocks the task either way).
- **Remediation applied:** added an explicit rewrite rule derived from what `merged.rs` has
  actually been returning (`has_local: true` → drop the condition; `local_project_id` →
  `project.id`; `nodes` → `[]`, branch is dead), then enumerated **every one of the 20 sites** by
  line number across steps 3-5.

### T3 (major) — the local/swarm filter UI becomes dead and was unaddressed

- **Found by:** orchestrator, following T2
- **Location:** `phase-3/302-…` § Change step 3
- **Defect:** `filterProjects` (lines 56-69), the `counts` and `nodeCount` memos, and
  `<ProjectTypeFilterTabs>` all exist solely to distinguish local from swarm projects. On a
  local-only board the "swarm" tab is permanently empty and "local" equals "all".
- **Evidence:**
  ```text
  $ git grep -n 'ProjectTypeFilter' -- frontend/src
  frontend/src/components/projects/ProjectList.tsx:16,17,18,58,76,172
  ```

  (`ProjectList.tsx` is its only consumer)
- **Remediation applied:** step 3 now deletes `filterProjects`, both memos, the filter state, the
  tabs block, and the node-count subtitle; new step 7 deletes
  `frontend/src/components/projects/ProjectTypeFilter.tsx`, which is added to `files:`.

### T4 (major) — task 402 assumed `error` was in scope on all six targets

- **Found by:** orchestrator (independently confirmed by opencode)
- **Location:** `phase-4/402-render-hive-not-connected-state.md` § Change step 3
- **Defect:** the task told the implementer to branch on `isHiveNotConfigured(error)` in all six
  files. Two do not expose `error`: `NodeProjectsSection` aliases it to `nodesError`, and
  `pages/Nodes.tsx` destructures only the `isError` boolean. The task's own STOP trigger ("If a
  section has no `error` value in scope — STOP") would have fired and halted the run.
- **Evidence:**
  ```text
  $ grep -n 'error' frontend/src/components/swarm/NodeProjectsSection.tsx
  104:    error: nodesError,
  282:          ) : nodesError ? (
  $ grep -n 'isError' frontend/src/pages/Nodes.tsx
  17:    isError,
  38:      ) : isError ? (
  ```

- **Remediation applied:** replaced the blanket instruction with a per-file table naming the exact
  variable in each of the six, plus an explicit before/after authorising the `error` addition to
  the `Nodes.tsx` destructure. STOP trigger narrowed accordingly.

### T5 (major) — task 302's test would not render

- **Found by:** opencode
- **Location:** `phase-3/302-…` § Failing test
- **Defect:** the test rendered `<ProjectList />` inside only a `QueryClientProvider`, but
  `ProjectList` calls `useNavigate` (line 72) and `useTranslation` (line 73). The task left
  provider choice to the implementer ("If `ProjectList` requires props or additional providers,
  supply them") — an undictated decision on the one test that guards the `a85f7d63` regression.
  The `getByText('3')` assertion was also unverified against how counts actually render.
- **Evidence:**
  ```text
  $ grep -n 'useNavigate\|useTranslation' frontend/src/components/projects/ProjectList.tsx
  2:import { useNavigate } from 'react-router-dom';
  73:  const { t } = useTranslation('projects');
  $ grep -n 'task_counts' frontend/src/components/projects/UnifiedProjectCard.tsx
  332:          <TaskCountPills counts={project.task_counts} projectId={project.id} />
  ```

- **Remediation applied:** `MemoryRouter` is now dictated with an exact snippet; the i18n setup is
  pointed at the existing pattern under `frontend/src/__tests__/`; the assertion now names
  `TaskCountPills` (`UnifiedProjectCard.tsx:332`) and its `hasTaskCounts` guard, with a STOP rather
  than a silent weakening if the value cannot be asserted on text alone.

### T6 (minor) — task 101 cited the wrong line for the merge anchor

- **Found by:** orchestrator
- **Location:** `phase-1/101-restore-nodes-routes.md` § Change step 2
- **Defect:** cited line 59 for `.merge(organizations::router())`; line 59 is
  `.merge(oauth::router())`.
- **Evidence:**
  ```text
  $ grep -n 'organizations::router()' crates/server/src/routes/mod.rs
  60:        .merge(organizations::router())
  ```

- **Remediation applied:** corrected to line 60. (The before/after text anchor was already correct,
  so this was cosmetic — but a wrong line number is exactly what erodes trust in the other anchors.)

### T7 (minor) — `hooks/index.ts` STOP trigger was unverified

- **Found by:** orchestrator
- **Location:** `phase-3/302-…` § Change step 2
- **Remediation applied:** verified `frontend/src/hooks/index.ts` does NOT re-export
  `useMergedProjects` (grep → no output) and recorded that in the task, so the STOP fires only on a
  genuine change rather than on an unknown.

## Findings NOT applied

### N1 — SC5's wording vs the `/api/projects/with-stats` endpoint

- **Raised by:** opencode
- SC5 reads "The task board renders projects from `/api/projects`", while the plan, ADR-0014, the
  spec's own Approach and Design sections, and the spec's `verify_cmd` frontmatter all specify
  `/api/projects/with-stats`.
- **Not applied because the spec is FROZEN (ADR-0001)** — a run must never edit the spec to make
  itself pass. This is an internal inconsistency in the spec's prose, not a plan defect: SC5's own
  second clause ("`/api/merged-projects` receives zero requests") makes the intent unambiguous, and
  `/api/projects` returns bare `Vec<Project>` with no `task_counts`, which cannot satisfy US4
  ("with their task counts and recent activity").
- **Escalated to the user** rather than silently reinterpreted. If they want the wording tightened,
  that is a deliberate spec edit followed by a re-run of `/wai:precheck` to re-freeze.

### N2 — task 403 tests only one of four hooks

- **Raised by:** opencode
- Task 403 hardens four hooks but only unit-tests `useAvailableNodes`; the other three rely on the
  browser check.
- **Not applied.** Accepted as a deliberate cost: the other three are stream hooks whose value is
  in live connection behaviour, and a mocked unit test of "does not open a stream" is close to
  hollow. The browser observation in Manual verification is the real signal, and it is recorded
  verbatim in the ledger. Revisit if the browser check proves unreliable.

### N3 — `ApiError::HiveNotConfigured` has no dedicated `error_message` arm

- **Raised by:** opencode
- The second `match` in `IntoResponse` ends in a catch-all, so the message renders as
  `"HiveNotConfigured: This node is not connected to a hive"`.
- **Not applied.** The catch-all keeps the match exhaustive (no compile break), and the user-facing
  copy comes from the `HiveNotConnected` component, not the API message.

## Scoreboard

| Seat | Findings raised | Verified & applied | Rejected |
|---|---|---|---|
| opencode (glm-5.2) | 5 | 3 (T2, T5, and confirmation of T4) | 3 (N1, N2, N3 — each adjudicated above) |
| codex | see round-1 addendum | — | — |
| agy | 0 — quota failure | — | — |
| orchestrator | 4 | 4 (T1, T3, T6, T7) | — |

## Termination

Round 1 is NOT closed on "a round found zero" — it is closed on all validated findings being
remediated plus a focused re-check. Re-check after remediation:

```text
$ bash "$WAI_ROOT/scripts/wai-plan-lint.sh" vk-swarm-node-ui-localize
PLAN-LINT PASS: vk-swarm-node-ui-localize — plan/frontmatter consistent, verification + SC-coverage complete
```
