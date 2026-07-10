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
| F-2026-07-04-04 | crisp-river uncommitted Cargo.toml doctest edits on merged branch | medium | open | crates/remote/Cargo.toml | sweep/2026-07-04 | 2026-07-04 | — | — |
| F-2026-07-06-01 | Hive UI lacks Generate API key button — node onboarding blocked | high | fixed | remote-frontend/src/pages/Nodes.tsx:7-51 | session/2026-07-06 | 2026-07-06 | hive-node-api-key-ui | docs/superpowers/specs/2026-07-07-hive-node-api-key-ui.md |
| F-2026-07-06-02 | Sign-in broken on non-loopback HTTP origins (crypto.subtle undefined) | high | fixed | remote-frontend/src/pkce.ts:10 | session/2026-07-06 | 2026-07-06 | fix-nonloopback-signin | docs/superpowers/specs/2026-07-08-fix-nonloopback-signin.md |
<!-- WAI:BACKLOG:END -->

## Triage notes

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