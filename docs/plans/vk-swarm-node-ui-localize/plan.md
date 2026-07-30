# Plan — vk-swarm-node-ui-localize

Spec: `docs/superpowers/specs/2026-07-30-vk-swarm-node-ui-localize.md` (frozen, `spec_sha=6946be63`)
ADRs: [ADR-0013](../../../dev-docs/adr/0013-restore-node-surface-hive-proxy-routes.md),
[ADR-0014](../../../dev-docs/adr/0014-retire-mergedproject-for-projectwithstats.md)

## Approach

The node frontend still routes `/nodes` and `/settings/swarm`, but node-foundations (`35b378a5`)
deleted the HTTP route layer those screens call, so every request 404s. The fix is in three
independent tracks, and the first is cheaper than it looks: the four deleted route modules are
recoverable **verbatim** from `35b378a5^`, every type and `RemoteClient` method they import still
exists on `main` (verified), and their URL paths already match what `frontend/src/lib/api/*.ts`
calls — so Track A lands with **zero frontend diff**.

Track A (phase 1) restores those modules as thin `RemoteClient` pass-throughs and registers them,
one module per task so each task ships a working surface. Phase 2 executes decision D3: the node's
API-key UI is deleted rather than reconnected, because `hive-node-api-key-ui` shipped that feature
on the hive and two admin-gated paths to the same privileged operation is the duplication this
split exists to remove. Track B (phase 3) replaces `MergedProject` with `ProjectWithStats` — the
`/api/merged-projects` handler has not merged anything since node-foundations (`nodes: Vec::new()`,
`has_local: true` are hardcoded), so what the board actually needs is the enrichment, under an
honest name. Track C (phase 4) makes hive-absent a first-class state instead of an error path.
Phase 5 closes out the gates.

Phases 1, 3, and 4's backend task are file-disjoint and could run in any order; the executor runs
serially, so they are ordered 1→5 with explicit `depends_on` recording the real constraints (only
phase 4's frontend task genuinely needs phase 1, and only phase 2 needs the nodes route restored).

## Conventions baked in (so no implementer has to decide)

Prior decisions-ledgers show no recurring undictated choices to pre-empt (the
`error-handling-and-dialog-a11y` ledger's implementer section records only advisory sibling
warnings). These conventions are dictated here instead of left to judgement:

- **Rust tasks verify by `## Manual verification`, not `scope_test`.** The gate's `scope_test`
  runner is toolchain-detected for Python/Node and would run vitest against a Rust path. Every
  Rust task therefore carries exact `cargo` commands. TypeScript tasks use real `scope_test`
  paths (`frontend/` has vitest and 34 test files).
- **Never `forbid_after: ["NodeApiKeySection"]`.** `forbid_after` greps the whole repo, and
  `remote-frontend/src/components/swarm/NodeApiKeySection.tsx` is the hive's copy, which must
  survive (SC7). Scope deletions by reading the listed files instead.
- **Restored route modules are byte-verbatim from `35b378a5^`** except where a task says
  otherwise. Do not "improve" them — divergence is what the ledger is for.
- **Route registration goes in `crates/server/src/routes/mod.rs` inside the existing
  `base_routes` builder**, appended after `.merge(organizations::router())`, matching the style of
  the surrounding `.merge(...)` chain.

## Phases

| Phase | File | Tasks | Ships |
|---|---|---|---|
| 1 | `phase-1-restore-proxy-routes.md` | 101–105 | `/api/nodes` + `/api/swarm/*` answer instead of 404 |
| 2 | `phase-2-remove-node-api-key-surface.md` | 201–203 | node UI has no API-key management (D3) |
| 3 | `phase-3-projectwithstats.md` | 301–303 | board runs on `ProjectWithStats`; `MergedProject` gone |
| 4 | `phase-4-hive-absent-state.md` | 401–403 | "not connected to a hive" is a real state |
| 5 | `phase-5-closeout.md` | 501 | full gates + hive regression + deploy evidence |

## Task dependency graph

```text
101 ──▶ 102 ──▶ 103 ──▶ 104 ──▶ 105 ──┐
 └──▶ 201 ──▶ 202 ──▶ 203 ──────────────┤
301 ──▶ 302 ──▶ 303 ───────────────────┼──▶ 501
401 ──▶ 402 (also needs 104) ──────────┤
 └──▶ 403 ─────────────────────────────┘
```

| Task | Depends | Conflicts |
|---|---|---|
| 101 | dep:  | conflicts: 102 103 104 |
| 102 | dep: 101 | conflicts: 101 103 104 |
| 103 | dep: 102 | conflicts: 101 102 104 |
| 104 | dep: 103 | conflicts: 101 102 103 |
| 105 | dep: 104 | conflicts:  |
| 201 | dep: 101 | conflicts:  |
| 202 | dep: 201 | conflicts:  |
| 203 | dep: 202 | conflicts:  |
| 301 | dep:  | conflicts: 303 |
| 302 | dep: 301 | conflicts:  |
| 303 | dep: 302 | conflicts: 301 |
| 401 | dep:  | conflicts:  |
| 402 | dep: 104 401 | conflicts:  |
| 403 | dep: 401 | conflicts:  |
| 501 | dep: 105 202 203 303 402 403 | conflicts:  |

101–104 conflict because each edits `crates/server/src/routes/mod.rs`; they are ordered, never
parallel. 301 and 303 conflict on `crates/server/src/routes/projects/mod.rs` and `types.rs`.

## Success-criterion coverage

| SC | Claimed by |
|---|---|
| SC1 | 101, 102, 103, 104, 105 |
| SC2 | 105, 402 |
| SC3 | 101, 201, 202, 203 |
| SC4 | 401, 402 |
| SC5 | 301, 302, 303 |
| SC6 | 403 |
| SC7 | 501 |

## Known limitation — the real-seam test (read before task 105)

The reachability gate wants a test that drives the real entry point. No test in this repo
constructs the full `router(deployment)` — it needs a live `DeploymentImpl` (DB, config, remote
client), and no test-deployment helper exists. A unit test that calls `list_nodes()` directly
would prove the handler works and **prove nothing about registration**, which is the entire bug —
that is a hollow test and this plan refuses it.

Task 105 therefore uses the realest seam actually available: **HTTP requests against a running
server**, asserting each restored path answers non-404. That is a stronger reachability signal
than any in-process unit test would have been, and it doubles as the deploy evidence
`wai-evidence.sh` demands at close.
