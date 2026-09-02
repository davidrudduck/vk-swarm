# Code Review — Round 3

**Target:** branch clever-pangolin (round-2 remediation delta, working tree)   **Range:** round-2 fixes within `git diff HEAD`   **Effort:** high

Final verification round on the round-2 remediation delta only (6 fixes: `None`-arm spawned
pool close, `last_guard_attempt` throttle, `guard_mode` field + `WalGuard::mode()` getter,
help-text `quick`→`baseline`, exit-code-2 documentation, `node_died` gating in Leg B).
Two parallel finder subagents (Rust delta, script delta). Round-1/2 non-actionable lists in force.

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|------------|-------------|
| — | — | — | — | No actionable findings (both finders) | — | — |

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|------------|---------------------|
| N1 | wal_monitor.rs:424-426 | — | correctness | `if !retry_due { return; }` early-return nesting | high | Verified: inside `guard_pending` inside `guard.is_none()`, whose block ends in an unconditional return; the existing-guard liveness path is unreachable there — nothing skipped |
| N2 | wal_monitor.rs:421-428 | — | correctness | Throttle correctness (first attempt immediate; failed attempt throttled one interval; success exits the branch forever) | high | Verified on all three paths; `last_guard_attempt` written exactly once, unconditionally followed by the connect — no starvation path |
| N3 | wal_monitor.rs test constructors | — | quality | New fields inert in tests | high | All five set `guard_pending: false`; no test references the lazy-retry behavior |
| N4 | wal_guard.rs:39-41 | — | quality | `mode()` getter | high | `Mode` is Copy; `pub` required by the wal_monitor consumer; sole use is the spawn inference |
| N5 | wal_monitor.rs:773-785 | — | correctness | None-arm spawned close | high | Byte-identical pattern to the Err arm; `last_salvage` recorded before the match; `tripped` already gates post-close ticks |
| N6 | wal_monitor.rs:465 | low | correctness | `check_interval_secs == 0` would make the throttle always-due | high | Pre-existing configuration concern (`tokio::time::interval` already panics at 0); not introduced by this delta |
| N7 | repro script:507-542 | — | correctness | Fall-through with `node_died=1` | high | Every subsequent check bounded (30s detector budget, curl `--connect-timeout 2 --max-time 10`, stop_node early-dead branch); no hang; no spurious attempt-2 retry (FAIL_COUNT gate falsified); `LEG_RESULTS[B]=FAIL` |
| N8 | repro script:542 | low | quality | "marker-B-post was not persisted" passes vacuously on a dead node | high | Transcript-accurate (the marker genuinely was not persisted); leg already fails via FAIL_COUNT; cosmetic |
| N9 | repro script:472 | — | correctness | `node_died` initialization | high | Function-local, re-initialized per call; attempt 2 gets a fresh value |
| N10 | repro script Leg A:437-439 | — | correctness | Leg A node-death parity | high | Already handled in round 1; Leg A needs no `node_died` (no trip-state-keyed branch chain follows) |
| N11 | repro script:499 | — | correctness | `elif [ $? -eq 2 ]` idiom | high | `$?` reflects the detector's exit status; nothing intervenes |
| N12 | repro script:40,45 | — | quality | Help text consistency | high | `MODE full\|baseline` matches preflight; exit line matches main (2 usage/preflight, 1 check failure, 0 all-pass) |

## Verdict: Approve

Actionable: []
