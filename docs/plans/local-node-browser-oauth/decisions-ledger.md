# Decisions Ledger

## Submission
Plan accepted from submit envelope.

## Decomposition tournament closure

Tournament 1 closed on 2026-08-21. Durable report: `reviews/tournament-1.md`.

- Codex: 5 validated issues + 5 validated fixes = 10.
- Claude: 9 validated issues + 4 validated fixes = 13.
- OpenCode/GLM: 5 validated issues + 4 validated fixes = 9.
- All peer-validated findings were repaired through the strict plan envelope and resubmitted; no generated task was hand-edited.
- Two independently verified defects from an invalid partial seat (task-007 base64 token fixtures and nested optional node context) were also repaired rather than deferred.
- The restart-port supplemental finding was disproved against the final plan: task 006 already awaits old-server completion and explicitly permits port reuse.

## Review-time decisions

1. Keep task 013 as one cross-layer vertical slice and keep `execution_process_id` required. An optional legacy unscoped connection-token fallback would contradict frozen D7 exact resource scoping.
2. Keep the WAI framework convention that append-only plan-ledger writes do not need every task to list the ledger in `files:`. `task-gate.sh` explicitly exempts `docs/plans/$TOPIC/*` and separately validates append-only ledger writes.
3. Keep task 003/004 persisted-state concurrency proofs. sqlx-sqlite 0.8.6 applies its default 5-second busy timeout to every established connection, disproving the alleged arbitrary unconfigured pooled-connection failure mode.
4. Use stable unversioned `$HOME/.agents/wai/scripts` commands everywhere. The active WAI submitter was corrected before strict resubmission so all 21 generated Done-when footers use that path; no plugin cache version is embedded in the deliverable.
5. General Hive outage continuity belongs to task 015; task 018 scans concrete browser-visible token surfaces and uses a separate valid session so Hive disconnect actually executes.

## Sibling-alignment advisory acknowledgements

`wai-plan-lint.sh` reports one rotating alphabetical neighbour for new files. Adding each arbitrary neighbour only advances the advisory to another file, so the plan lists actual pattern siblings and records why the reported neighbours are not patterns:

- Task 001 — `crates/db/migrations/20250621120000_relate_activities_to_execution_processes.sql` destructively drops/recreates a table and uses `DATETIME DEFAULT datetime('now')`; task 001 is strictly additive and uses caller-bound integer epoch milliseconds.
- Task 008 — `crates/server/src/routes/breakdown.rs` is a domain router, not top-level public/protected composition or API fallback. `crates/server/tests/projects_with_stats.rs` tests ordering/counts/response fields, not authorization or fallback.
- Tasks 009, 011, 013, 014, 015 and 018 — `crates/server/tests/nodes_routes.rs` checks the ordinary `/api/nodes` response contract. It contains no browser OAuth, cookies, protocol upgrade, proxy audience, restart continuity, or sentinel-disclosure pattern.
- Task 013 — `frontend/src/hooks/useActivityDismiss.ts` is a REST mutation/cache-invalidation hook, not a stream or token-lifecycle sibling.
- Task 016 — `frontend/src/lib/api/breakdown.test.ts` is a downstream `makeRequest` consumer, not an auth-boundary or centralized unauthorized-event pattern.
- Task 019 — `scripts/dev-swarm-setup.sh` is interactive and state-mutating; the verifier and its fixture test are read-only deterministic checks.
- Task 020 — `docs/configuration-customisation/database-performance.mdx` shares only generic MDX structure. `network-access.mdx`, already named by the task, is the topical trusted-LAN sibling.

## SQL-anchor advisory

No live SQLite schema checker is configured for WAI, so plan lint cannot dynamically validate new SQL literals. This is acknowledged rather than silently treated as green: task 001 owns the only new schema, and all persistence tests use the repository's migrated `create_test_pool()` rather than duplicated test DDL.

## Execution boundary

No implementation has started. Task 001 is the sole irreversible task. Execution must halt with `human_gate_required` until `docs/plans/local-node-browser-oauth/reviews/001.approved` exists following explicit human approval.
