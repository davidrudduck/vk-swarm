# 2026-08-05 — Next-session handoff

Written at the close of the `vk-swarm-node-ui-localize` ship. Start a fresh session from this file.

## State as of this handoff

- **`vk-swarm-node-ui-localize` is SHIPPED** (spec + README `status: shipped`, docs graduated to
  `dev-docs/workstreams/vk-swarm-node-ui-localize/{spec,plans}/`, 19/19 tasks passed, evidence gate
  PASS, `/dr:code-review` round 1 `Actionable: []`).
- **The branch `feat/vk-swarm-node-ui-localize` is pushed but NOT merged, and no PR exists yet.**
  This is the one loose end. See "Finish the merge" below.
- The live node `http://NODE_HOST` is still running the **feature branch** build
  (`374598a7`). It needs redeploying off `main` once the merge lands.

## Can the three open items be done together? — YES, with one caveat

They touch **completely disjoint files**, so there is no merge-conflict risk and they can run
concurrently in separate git worktrees:

| # | Item | Primary files |
|---|---|---|
| A | Hive SW blocks OAuth | `remote-frontend/vite.config.ts`, `remote-frontend/src/lib/pwa.ts` |
| B | Task delete dangling id | `crates/server/src/routes/tasks/handlers/remote.rs` |
| C | Orphan worktree delete | `crates/local-deployment/src/container.rs` |

**The caveat: do NOT merge them into one workstream.** One WAI workstream = one spec = one
reachability gate, and that gate is per-incident — it asks "does the merged change execute on the
path this specific bug lives on". Three unrelated bugs under one spec makes that gate meaningless
and the spec incoherent. The speedup comes from **parallel worktrees, not a combined spec**
(ADR-0004: one workflow = one worktree = one branch).

Also: never run two sessions in the same worktree. The decisions-ledger is orchestrator-owned and
single-writer; concurrent runs clobber each other and no gate catches it.

## Recommended grouping — backlog items folded in by file locality

Existing backlog items are grouped with whichever workstream already touches their code, so they
ride along at near-zero marginal cost.

### WS-A — `hive-oauth-sw-bypass` (HIGHEST PRIORITY: you cannot log in at all)
- `F-2026-08-03-02` — hive SW intercepts `/v1/oauth/*`; **root cause, confirmed by controlled
  experiment** (SW registered → node sign-in spins; unregistered → works)
- `F-2026-08-03-01` — node OAuth handoff doesn't complete (triaged as *caused by* `-02`)
- `F-2026-08-04-01` — `OAuthDialog` polls forever with no timeout/error branch, which is *why* this
  presented as a silent spinner. Fix alongside so the next auth regression is diagnosable.

Full diagnosis, ruled-out hypotheses, and the mechanism:
`dev-docs/2026-08-03-node-signin-blocked-findings.md`. **Read it before investigating** — it already
eliminates `crypto.subtle`, the `return_to` allowlist, and `returnTo` host derivation with evidence.

The likely fix is small and has a precedent in the same rule: `/v1/shape` is already excluded from
the `NetworkFirst` cache; `/v1/oauth` needs the same treatment. Confirm the mechanism (SW-handled
redirected navigation vs stale cache hit) before settling the fix.

### WS-B — `node-task-delete-dangling-shared-id` (no workaround; tasks stuck forever)
- `F-2026-08-05-01` — full root cause in
  `dev-docs/workstreams/node-task-delete-dangling-shared-id/README.md`
- **Explicit trap recorded there:** do NOT fix with a blanket `is_err()` catch. Discriminate on
  not-found only. This run produced FIVE over-broad-predicate defects; a catch-all here is the sixth.

### WS-C — `node-ui-localize-followups` (cheap cleanup, all one area, low risk)
The leftovers from the shipped workstream — small, mechanical, good for a single batched run:
- `F-2026-07-31-04` — `LinkToLocalFolderDialog` orphaned by task 302 (client + hook + server route
  still live). You previously chose "leave it, backlog only" — revisit now it can ride along.
- `F-2026-07-31-05` — stale `['mergedProjects']` query key invalidated in `useProjectMutations.ts:79`
  is now a no-op (**verified still present 2026-08-05**)
- `F-2026-07-31-06` — stale doc comment "merged projects view" at
  `crates/db/src/models/project/mod.rs:106` (**verified still present**)
- `F-2026-07-31-08` — i18n key `settings.swarm.hiveNotConnected` undefined in all locales
- `F-2026-08-01-01`, `F-2026-08-01-02` — 503-discrimination and retry behaviour are unpinned; an
  unconditional guard would survive the suite

### WS-D — `worktree-orphan-sweep-guard` (destructive; can eat uncommitted work)
- `F-2026-07-30-03` — orphan sweep at `crates/local-deployment/src/container.rs:319-383` calls
  `remove_dir_all` with **no dirty-file guard**, unlike `cleanup_expired_attempt` in the same file
- `F-2026-07-30-02` — empty `VK_DATABASE_PATH` silently relocates the DB to CWD
  (`crates/utils/src/assets.rs:61`). Same storage-safety theme, and `assets.rs` was just worked in
  task 099, so the context is fresh.

### WS-E — test infrastructure (unblocks confidence in everything above)
- `F-2026-08-04-02` — `test_fast_execution_no_lost_logs` flakes; discarded `tokio::time::timeout`
  races the assertion. Workstream README already written with the evidence.
- `F-2026-07-30-01` — `cargo test -p db` fails to compile (integration test needs the test-utils
  feature)

**Suggested order:** A and B first (both are hard blockers with no workaround), in parallel
worktrees. Then E (makes the suite trustworthy), then C and D.

## Finish the merge (do this first, it is one command)

The branch is pushed; there is no PR yet.

```bash
gh pr create --base main --head feat/vk-swarm-node-ui-localize   # target davidrudduck/vk-swarm ONLY
gh pr merge <PR> --squash
```

Then the **mandatory post-merge live verify** (`bugfix` spec, so this is not optional):

```bash
# Resolve WAI_ROOT dynamically — do NOT pin a version. This session pinned
# .../wai/0.27.2, which went stale the moment the launcher resolved 0.28.2.
WAI_ROOT="$HOME/.agents/wai"                       # canonical install (wai-install)
[ -d "$WAI_ROOT/scripts" ] || WAI_ROOT="$(ls -d "$HOME"/.claude/plugins/cache/agent-plugins/wai/* | sort -V | tail -1)"
bash "$WAI_ROOT/scripts/wai-verify.sh" vk-swarm-node-ui-localize
```

Non-zero = NOT shipped regardless of what the repo says. Deploy first — merging deploys nothing.

## Known environment traps

- **Subagent dispatch failed for every agent in this session** — the Stage-2 panel on tasks 501/502
  and all three `/dr:code-review` finders returned nothing across repeated requests. If that
  persists, do the review work inline and *say so* rather than reporting phantom coverage.
- **pnpm global config** — `~/.config/pnpm/rc` previously pinned `virtual-store-dir` to a temp DAG
  worktree, and `/tmp` cleanup then broke the entire frontend suite ("no tests, 37 errors").
  **Fixed by the user on 2026-08-05**; if the suite ever fails to start workers again, check that
  file first.
- `/home/david/Tools/vk-swarm` is the node's deploy target — **never edit it directly**; all work
  goes through `/data/Code/vk-swarm`.
- Never use `pkill`/`killall` (CLAUDE.md) — a foreign vibe-kanban instance runs on port 9002.

## Baselines to compare against

```text
cargo fmt / clippy --all --all-targets --all-features -D warnings / cargo test --workspace   all 0
cargo test --workspace          57 "test result: ok" blocks
frontend                        lint 0, tsc 0, vitest 37 files / 433 tests
remote-frontend                 lint 0, tsc 0, vitest 405 tests
```
