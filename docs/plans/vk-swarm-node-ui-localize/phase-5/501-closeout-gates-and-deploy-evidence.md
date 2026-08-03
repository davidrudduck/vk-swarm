---
id: "501"
phase: 5
title: "Full gates, hive-unchanged regression, and live deploy evidence"
status: passed
depends_on: ["105", "202", "203", "204", "303", "402", "403"]
parallel: false
conflicts_with: []
files:
  - docs/plans/vk-swarm-node-ui-localize/decisions-ledger.md
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC7]
---

## Failing test (write first)

N/A — this task runs the existing suites; it writes no product code.

## Change

Run every gate, then record the results in the decisions-ledger under the two sections
`wai-evidence.sh` requires for a `bugfix` spec: `## Reachability gate` and
`## Deploy verification`. Evidence must be **verbatim command output in fenced blocks** — a prose
summary is rejected by the evidence gate and, more to the point, is not evidence.

## Allowed moves

- Append to `docs/plans/vk-swarm-node-ui-localize/decisions-ledger.md` only.
- If a gate fails, do NOT fix it from this task — report which task owns the file and stop.

## STOP triggers

- Any gate red. Report the failing output; the fix belongs in the owning task.
- If `remote-frontend` tests changed count or status versus the session baseline (52 files /
  405 tests at decomposition time), the hive was touched — that is an SC7 violation. STOP.

## Manual verification (record in decisions-ledger)

```bash
# Rust
cargo fmt --all -- --check
cargo clippy --all --all-targets --all-features -- -D warnings
cargo test --workspace

# Node frontend
cd frontend && npm run lint && npx tsc --noEmit && npx vitest run

# Hive frontend — MUST be unchanged (SC7)
cd remote-frontend && npm run lint && npx tsc --noEmit && npx vitest run
# Expected: 52 test files / 405 tests passing, same as the pre-workstream baseline

# Types
npm run generate-types:check
```

### Reachability gate evidence to record

- **(a) Call-path trace** — for the restored routes: browser → `frontend/src/lib/api/nodes.ts`
  → `GET /api/nodes` → `crates/server/src/routes/mod.rs` (`.merge(nodes::router())`) →
  `routes/nodes.rs::list_nodes` → `deployment.remote_client()` → hive. Cite real file:line from
  the merged tree, not from this plan.
- **(b) Real-seam test** — task 105's HTTP evidence file, quoted. Note explicitly that no
  handler-level unit test is offered in its place and why (see 105's Failing test section).
- **(c) Incident-symptom assertion** — the symptom was that the node's Nodes/swarm surfaces got
  the SPA's `index.html` instead of JSON, because their routes were unregistered. (The spec and
  the findings describe this as "404"; at HTTP level it is **200 + text/html**, because the outer
  catch-all `.route("/{*path}", ...)` at `crates/server/src/routes/mod.rs:76` serves index.html
  with `StatusCode::OK` — see `frontend.rs:40-43`. Record this correction.) Quote the six
  `application/json` content-type lines from task 105 and the `/api/merged-projects` →
  `text/html` line.

### Deploy verification evidence to record

Deploy the feature-branch build to the live/staging node, then capture verbatim:

```bash
curl -fsS "http://<deployed-host>/api/projects/with-stats" | head -c 300
curl -s -o /dev/null -w '%{http_code}\n' "http://<deployed-host>/api/nodes?organization_id=<real-org>"
```

## Done when

- Every gate above is green and its real output is in the ledger.
- `remote-frontend` is at its baseline count (SC7).
- `## Reachability gate` and `## Deploy verification` both exist in the ledger with fenced,
  verbatim output; `bash "$WAI_ROOT/scripts/wai-evidence.sh" vk-swarm-node-ui-localize` passes.
