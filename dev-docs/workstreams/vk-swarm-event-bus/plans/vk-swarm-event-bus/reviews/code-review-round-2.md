# Code Review — Round 2

**Target:** wai/vk-swarm-event-bus (remediation verification)   **Range:** `4495fa4c..119533e0`   **Effort:** high

One adversarial reviewer over the round-1 remediation diff, primed with the ledger's
Post-review known issues. All 17 round-1 fixes verified substantively correct area-by-area
(cursor.max guard incl. empty-journal arm; Initializing reset duplicate/gap-free; cursor-before-
send safety; select! recv cancel-safety and None-exit; 410 ordering incl. empty-journal case;
From impl semantics identical to the six deleted fns; min_cursor fully unreferenced; shutdown
ordering hang-free; both new tests fail on old code).

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| 1 | `crates/server/src/routes/events.rs:180,192` | low | correctness | Both 410 messages still carry literal ~20-space runs — the round-1 reflow's backslash continuations were swallowed by the edit tooling's escaping chain, re-introducing the exact defect being fixed | high | yes |
| 2 | `crates/server/src/main.rs:358-363`, `local-deployment/src/lib.rs` | low | correctness | `shutdown_event_services` comments overclaim: shutdown is signal-only (command queued / async abort), so an in-flight compaction pass can still commit after the WAL truncate; consequence benign (SQLite replays residual WAL; pool.close awaits), but the comment asserts prevention | high | yes |

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| 3 | `crates/services/src/services/trigger_hooks.rs:783` | info | quality | The no-redelivery assertion is bounded by a 300ms sleep (probabilistic pin under heavy load) | medium | Matches the established style of every sibling test in the module |

## Verdict: With fixes

Actionable: [1,2]
