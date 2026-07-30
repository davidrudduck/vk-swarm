# ADR-0014 — Retire `MergedProject` in favour of `ProjectWithStats`

- **Status:** accepted
- **Date:** 2026-07-30
- **Workstream:** vk-swarm-node-ui-localize
- **Supersedes behaviour of:** `/api/merged-projects` + the `MergedProject` `#[ts(export)]` struct

## Context

The open question was whether a local node can operate without `/api/merged-projects` — it must
still show projects bound to it (hive-linked or local-only) and offer full CRUD over the tasks
created and executed on it.

Reading the handler answers it: **the endpoint no longer merges anything.**
`crates/server/src/routes/projects/handlers/merged.rs` calls
`Project::find_local_projects_with_stats(pool)` and then hardcodes

```rust
has_local: true,
local_project_id: Some(project.id),
nodes: Vec::new(),
```

on every row. There is no remote fetch. `nodes` is unconditionally empty, so
`LocationBadges` — the component whose entire purpose is rendering that array — renders nothing.
`has_local` is a constant. The "merge" in the name and type is a fiction left over from the
pre-fork architecture; `a85f7d63` restored the endpoint as local-only precisely because the board
went blank without it, not because the merge semantics were wanted.

What the board genuinely needs is the **enrichment**, which plain `/api/projects` does not
provide: `get_projects` returns `Vec<Project>` from `Project::find_all` — bare rows, no
`task_counts`, no `last_attempt_at`, no GitHub counts, no ordering.

So the dependency is on a projection, not on remoteness.

## Decision

Keep the enriched endpoint; delete the merge fiction.

- Introduce **`ProjectWithStats`** — the local project row plus `task_counts`,
  `last_attempt_at`, and the `github_*` count fields, name-sorted — served from
  **`/api/projects/with-stats`**.
- Delete the `MergedProject` and `MergedProjectsResponse` `#[ts(export)]` structs, the
  `/merged-projects` route (`routes/projects/mod.rs:148`), and its handler.
- Delete the now-provably-dead fields: `nodes`, `has_local`, `local_project_id`, and the
  `LocationBadges` component that exists only to render `nodes`.
- Retype `ProjectList`, `ProjectSwitcher`, and `UnifiedProjectCard` onto `ProjectWithStats`;
  `useMergedProjects` becomes `useProjectsWithStats`.

**How the objective is met.** `remote_project_id` is a column on `Project` and survives untouched,
so a hive-bound project is still identifiable and fully manageable from the node — binding is a
property of the project row, never of the merge envelope. Task CRUD is unaffected: tasks are
served by `/api/tasks` and the task-attempt routes, which this ADR does not touch. A node with no
hive shows its local projects with full stats, exactly as now. A hive-bound node shows the same,
plus the swarm-management surfaces restored under [ADR-0013](0013-restore-node-surface-hive-proxy-routes.md).

## Consequences

- **Wire-format change** on a `#[ts(export)]` type — irreversible in the ADR sense, hence this
  record. `npm run generate-types` must run and `generate-types:check` must pass; `shared/types.ts`
  is never hand-edited.
- `/api/merged-projects` disappears. It has no consumer outside
  `frontend/src/hooks/useMergedProjects.ts` (verified by grep over `frontend/src`), so no external
  contract breaks.
- This reverses the *shape* of `a85f7d63`, not its intent. That commit restored the endpoint so
  the node board would render at all; the enriched projection it provides is preserved verbatim
  under an honest name.
- `LocationBadges` and the remote card badges named in the workstream README are deleted rather
  than repointed — they had nothing to render.
- Closes **F-2026-07-29-02**.

## Alternatives rejected

- **Point the board at plain `/api/projects`.** Loses task counts, last-attempt ordering, and
  GitHub counts — a visible regression in the board.
- **Keep `/api/merged-projects` and only stop the frontend calling it.** Smaller diff, but leaves
  a dead route and a lying type on `main`, which is the exact class of drift this workstream
  exists to remove.
- **Keep the name, drop the fields.** `MergedProject` with no merge would mislead the next reader
  the same way it misled this workstream's own README.
