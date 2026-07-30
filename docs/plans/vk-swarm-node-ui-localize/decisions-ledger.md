# Decisions Ledger — vk-swarm-node-ui-localize

> Implementer appends here for ANY choice the task didn't dictate. Empty section = perfect.

## Pre-existing decisions (from spec + ADRs)

| Decision | Source | Reversible? |
|----------|--------|-------------|
| D1: Restore node-surface routes as thin RemoteClient proxies, verbatim paths | spec / ADR-0013 | Irreversible |
| D2: Proxy through the node server, not browser→hive direct | spec / ADR-0013 | Irreversible |
| D3: Do not restore `/nodes/api-keys*`; delete the node's API-key UI | spec / ADR-0013 | Irreversible |
| D4: `ProjectWithStats` at `/api/projects/with-stats` replaces `MergedProject` | spec / ADR-0014 | Irreversible |
| D5: Delete `LocationBadges` and the `nodes`/`has_local`/`local_project_id` fields | spec / ADR-0014 | Irreversible |
| D6: Keep the four remote stream hooks; harden rather than remove | spec | Reversible |
| D7: Node and hive `components/swarm/` trees stay separate | spec | Reversible |

## Decomposition-time decisions (dictated, not left to the implementer)

- **Rust tasks verify via `## Manual verification`, not `scope_test`.** The gate's `scope_test`
  runner is toolchain-detected for Python/Node and would run vitest against a Rust path. TS tasks
  use real `scope_test` paths (`frontend/` has vitest + 34 test files).
- **`forbid_after` is omitted on tasks 202 and 303**, with the reason recorded in each task file.
  It greps every tracked file, and the obvious terms have legitimate survivors: `/merge-to/` in
  `crates/remote/` and `remote-frontend/` (the hive's, SC7), and `merged-projects` /
  `/api/nodes/api-keys` in ADRs, specs, and `docs/architecture/`. Scoped greps are used instead.
- **`ProjectWithStats` field types are copied from `MergedProject` verbatim** —
  `github_open_issues: i32`, `github_open_prs: i32`. The spec's illustrative sketch shows
  `Option<i64>`; its governing sentence ("identical to today's `MergedProject` minus `nodes`,
  `has_local`, `local_project_id`") is authoritative, so the real field types win. Not a spec
  contradiction — the spec's own prose resolves it, so no re-precheck was triggered.
- **Task 105 offers no handler-level unit test on purpose.** A test calling a restored handler
  directly would pass on `main` today, before any task runs, because the handler was never broken
  — registration was. The realest available seam is an HTTP request to a running server.

## Implementer decisions

_(empty — the implementer appends here)_

## Advisory sibling warnings (plan-lint W: lines) — adjudicated at decompose

Each `W:` line from `wai-plan-lint.sh` is acknowledged below. None was a real pattern sibling;
the real sibling in every case is already listed in the task's `siblings:` field.

| W: on | Suggested sibling | Verdict |
|---|---|---|
| 101–104 | `crates/server/src/routes/all_tasks.rs` | **Not a sibling.** It is a local-DB query router, not a `RemoteClient` proxy. The genuine pattern sibling is `crates/server/src/routes/organizations.rs` — the only live hive-proxy router in the crate — and it IS listed in `siblings:` on all four tasks, with a required read step. |
| 301 | `crates/server/src/routes/projects/handlers/core.rs` | **Not the sibling.** `with_stats.rs` is a near-copy of `merged.rs` (same query, same mapping, same sort), which is listed in `siblings:` with a required read step. `core.rs` holds unrelated CRUD handlers. |
| 302 | `frontend/src/hooks/index.ts` | **Not a sibling** (a barrel file, not a pattern). It IS a hazard, and the task handles it explicitly: a STOP trigger fires if `index.ts` re-exports `useMergedProjects`, because that file is outside `files:`. |
| 302 | `frontend/src/components/projects/CloneProgress.tsx` | **Not a sibling.** Unrelated component; the new file is a test for `ProjectList`. |
| 402 | `frontend/src/components/swarm/MergeLabelsDialog.tsx` | **Not a sibling.** A dialog, not a status/empty state. The real sibling is `frontend/src/components/ui/alert.tsx` plus the existing error branch in `SwarmProjectsSection.tsx`, both named in the task with a required read step. |
| 403 | `frontend/src/hooks/index.ts` | **Not a sibling** (barrel file). The pattern sibling is `useRemoteConnectionStatus.ts`, listed in `siblings:` with a required read step. |
