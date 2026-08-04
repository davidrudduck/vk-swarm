# Code Review — Round 1

**Target:** `feat/vk-swarm-node-ui-localize`   **Range:** `feff74be..aec6ab83`   **Effort:** high

Pre-graduation gate for `/wai:close`. 104 files, +8706/−1191.

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| 1 | `crates/server/tests/harness_smoke.rs:30` | medium | correctness | `assert_ne!(res.status, 500, "absent hive is not a server error")` for `/api/organizations` with no hive. `list_organizations` goes through `deployment.remote_client()?` (`organizations.rs:72`), so the correct status is the same `HiveNotConfigured` 503 the four swarm route tests pin. The assertion passed for 200/400/401/503 alike — the **fifth** instance of the hollow-predicate class in this run. | high | yes — **FIXED** |

**Finding 1 remediation (applied in this round, commit `aec6ab83`):** tightened to
`assert_eq!(res.status, 503)`. Mutation-verified — flipping `HiveNotConfigured` to `BAD_GATEWAY`
(`error.rs:201`) fails the test (`left: 502 / right: 503`); reverted clean and green.

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| 2 | `crates/services/tests/normalize_sync_test.rs:359-368` | medium | correctness | `test_fast_execution_no_lost_logs` flakes in full-workspace runs. The `tokio::time::timeout` result is discarded (`let _ =`), so under CPU contention the assertion runs before the normalizer writes and reads 0 patches — misreporting a harness race as "lost logs". | high | **Pre-existing, out of scope.** This branch never touches `crates/services`/`crates/executors` (empty `git diff`), and it fails with this run's own edit stashed. Promoted to a tracked workstream created THIS session — `dev-docs/workstreams/services-normalize-flaky-test/README.md`, `F-2026-08-04-02`. NOT suppressed: no `#[ignore]`, test stays live. |
| 3 | `~/.config/pnpm/rc` | high | quality | Global pnpm `virtual-store-dir=/tmp/dag-w-q1PrCI/node_modules/.pnpm` pins the virtual store to one temp DAG worktree. After `/tmp` cleanup stripped `xhr-sync-worker.js`, the entire frontend suite failed to start any worker ("no tests, 37 errors"). | high | **Not repo code** — user machine config, outside the diff. Surfaced to the user; will break every pnpm project on the machine, not just this repo. Worked around locally with `--virtual-store-dir=node_modules/.pnpm`; suite restored to 37 files / 433 tests. |

## Coverage and its limits — read before trusting this record

**Three `high`-effort finder subagents were dispatched** (`cr-rust` — Rust backend correctness;
`cr-tests` — hollow test predicates; `cr-frontend` — frontend correctness). **None returned
findings**, despite explicit follow-up requests. This matches the earlier failure of the Stage-2
panel `panel-501-a` on tasks 501/502, so it appears systemic to subagent dispatch in this session
rather than a property of any one prompt.

**Everything in this record was therefore found by the orchestrator's own inline review**, which is
weaker than independent review — it shares the author's blind spots. Recorded plainly rather than
presented as a completed high-effort fan-out.

**Verified inline (with method, so a re-reviewer need not redo it):**

- **SC7 — hive untouched.** `git diff --stat feff74be..HEAD -- remote-frontend/ crates/remote/` is
  EMPTY. `remote-frontend` at baseline: lint 0, tsc 0, 405 tests.
- **`with_stats.rs` vs deleted `merged.rs`.** Direct `diff` of the two files: the only deltas are
  the three intentionally-dropped fields (`has_local`, `local_project_id`, `nodes`) plus renames.
  Query, `sort_by`, and enrichment logic byte-identical — no silent behaviour change.
- **`MergedProject`/`NodeLocation`/`has_local` fully removed.** Zero references across
  `frontend/src`, `shared`, `crates`. The single grep hit is a deliberate NEGATIVE assertion
  (`projects_with_stats.rs:46`) proving the field was dropped.
- **`local_project_id` survivors are a different domain.** They belong to `NodeProject`
  (`types/nodes.ts:33-42`), unrelated to the deleted `MergedProject` field. Not a defect.
- **`ProjectTypeFilter.tsx` deletion is clean.** Zero surviving references (deleted by task 302).
- **`shared/types.ts` generated, not hand-edited.** `npm run generate-types:check` exits 0.
- **Stream-hook 503 guards are narrow, not over-broad.** `useAvailableNodes`, `useDiffStream`,
  `useRemoteConnectionStatus` each branch on `isHiveNotConfigured(e)` → quiet, else throw/report.
  Genuine errors are not swallowed.
- **No route shadowing introduced.** New patterns checked against the full registered set; the
  `/organizations` "duplicates" are distinct HTTP methods on one path in a file this branch does not
  touch. All four new routes were additionally proven reachable against the live deployment.
- **`crates/utils/src/assets.rs` is in scope** — task 099 (`phase-1/099-vk-asset-dir-override.md`,
  passed, covers SC1/SC4), not an accidental inclusion.

**NOT independently covered** — the residual risk a re-reviewer should target first:
semantic (non-signature) drift in the four byte-verbatim restored route modules against current
surrounding code; totality of `From<RemoteClientNotConfigured>`; N+1/unbounded-result risk in
`with_stats.rs`; the frontend test files as a body; `HiveNotConnected` render conditions across all
consumers; orphaned i18n keys.

## Gate results at this commit

```text
cargo fmt --all -- --check                                    0
cargo clippy --all --all-targets --all-features -- -D warnings 0
cargo test --workspace                                        0   (57 "test result: ok" blocks)
npm run generate-types:check                                  0
frontend:        lint 0, tsc 0, vitest 37 files / 433 tests passed
remote-frontend: lint 0, tsc 0, vitest 405 tests passed (baseline)
```

## Verdict: With fixes

Finding 1 was actionable and is remediated in this round. Findings 2 and 3 are logged, tracked, and
out of scope for this diff. No actionable findings remain.

Actionable: []
