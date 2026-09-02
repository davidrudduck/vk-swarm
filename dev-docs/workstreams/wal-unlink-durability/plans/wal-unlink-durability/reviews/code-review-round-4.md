---
round: 4
date: 2026-09-02
target: gate-run flaky test `refusal_latch_fail_fast_under_production_busy_timeout` (test-only diff on top of 05ed1a5f)
effort: focused (single finder, subagent-haiku)
---

# Code review — round 4 (post-gate flake remediation)

Round 3 converged on the round-2 delta, but the full mandatory gate run on
05ed1a5f then exposed a pre-existing flaky test:
`wal_monitor::tests::refusal_latch_fail_fast_under_production_busy_timeout`
failed 1 of 2 workspace runs (`pooled write after release waited past 3s`),
passing in isolation (0.11s). ~1/8 full-suite failure rate; never reproduced
in 6 + 5 targeted repro runs.

## Root cause (confirmed against sqlx-core 0.8.6 source)

`PoolConnection::drop` SPAWNS the connection-return task
(`pool/connection.rs:199-210`). The test's `drop(pooled)` followed by
`execute(&pool)` with `.max_connections(5)` let the acquire win the race
against the spawned return, find the idle queue empty, and open a FRESH
connection with the options' 30s busy_timeout (this pool deliberately omits
`after_connect(apply_performance_pragmas)` to isolate the old-domain path).
A write on that fresh conn blocked on the latch's `BEGIN IMMEDIATE` past the
3s `tokio::time::timeout`. Load-dependent scheduling explains the rate.

## Fix

`.max_connections(5)` → `.max_connections(1)` in the test pool
(wal_monitor.rs:1572), with a load-bearing comment. With max 1 the acquire
must wait for the returned busy_timeout=0 connection — the exact
`after_release` behaviour under test. Validation: db --lib suite 8/8 green
post-fix; both refusal_latch tests pass.

## Findings (round-4 finder on the fix diff)

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|------------|-------------|

(empty)

## Non-actionable

| Item | Why non-actionable |
|------|--------------------|
| Earlier test steps under max_connections(1) | Verified identical: INSERT, acquire (pre-warmed min_connections(1) conn), `zero_pooled_busy_timeout` try_acquire drain (empty while `pooled` held, non-blocking), manual PRAGMA zero, first 3s write, disarm, close |
| Race elimination completeness | With max 1 the pool cannot open a fresh conn while the return pends; after_release returns `Ok(true)` unconditionally (lib.rs:87), so the conn is always returned, never detached |
| acquire_timeout(5s) vs spawned-return latency | Return completes in ms; 5s is ample; worst case is a deterministic acquire-timeout error, not a silent wrong-connection flake |
| Comment accuracy | sqlx 0.8.6 confirmed in crates/db/Cargo.toml; described spawn behaviour matches sqlx-core source |
| Flake classified as pre-existing test bug | The race existed since the test was written (task 040); rounds 1-3 remediation did not introduce it — the gate's second workspace run simply hit the ~1/8 window |

Actionable: []
