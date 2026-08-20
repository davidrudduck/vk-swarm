# Code Review — Round 1

**Target:** wai/vk-swarm-event-bus (pre-graduation close gate)   **Range:** `c5cc16d0..4495fa4c`   **Effort:** high

Four parallel finders (db layer, services layer, server/wiring, quality axis); every finding
verified by the orchestrator against the real file before classification. Adjudicated items from
the run (runtime sqlx queries, infallible `fire`, compaction actor shape, best-effort handle
sends, rebootstrap `< new_min - 1` boundary, retention cutoff format) were excluded from scope.

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| 1 | `crates/services/src/services/trigger_hooks.rs:132` | medium | correctness | Rebootstrap resets a persisted cursor to `MIN(seq)-1` unconditionally; a stale flag (set() preserves it, a live runner never re-reads it) regresses an already-advanced cursor on respawn, mass-redelivering processed events | high | yes |
| 2 | `crates/server/src/main.rs:339-379` | medium | correctness | Shutdown never calls the existing `event_bus().shutdown()` / compaction `shutdown()`; journal writers stay live across the final `wal_checkpoint(TRUNCATE)` and spin on `PoolClosed` after `pool.close()` | high | yes |
| 3 | `crates/server/src/routes/events.rs:181` | low | correctness | Client-visible 410 message carries a literal run of ~22 spaces from a wrapped format string | high | yes |
| 4 | `crates/server/src/routes/events.rs:161-185` | low | correctness | A cursor above the high-water mark (possible after restore-from-backup, a documented workflow) is accepted and yields a permanently silent stream — no 410, no frame | medium | yes |
| 5 | `crates/services/src/services/event_compaction.rs:127-131` | low | correctness | `retention_hours` is the one config knob never sanitised: extreme values panic `chrono::Duration::hours` inside the worker (supervised exit → compaction dead for process lifetime); negatives silently future-date the cutoff | medium | yes |
| 6 | `crates/services/src/services/event_compaction.rs:284-305` | low | correctness | Compaction actor has no closed-channel exit arm: if every handle drops, the loop ticks forever | high | yes |
| 7 | `crates/services/src/services/event_bus/mod.rs:261-283` | low | correctness | Lagged-refill failure leaves the stream `Live`; a consumer that polls past the `Err` silently skips the lagged-out range (latent — both in-repo consumers terminate on first `Err`) | high | yes |
| 8 | `crates/db/src/models/task/queries.rs:18` (+5 siblings) | medium | quality | `journal_err_to_sqlx` copy-pasted six times across db modules; per-task file-set constraint no longer binds at branch level | high | yes |
| 9 | `crates/services/src/services/trigger_hooks.rs:123-125` | medium | quality | `run_hook` re-inlines `SELECT MIN(seq)` instead of calling the exported `event_journal::low_water_mark` | high | yes |
| 10 | `crates/db/src/models/trigger_cursor.rs:111-120` | medium | quality | `min_cursor` is dead code (zero non-test callers; compact re-inlines the SQL for its tx executor) | high | yes |
| 11 | `crates/services/src/services/trigger_hooks.rs:108` | medium | quality | `Box<dyn Error>` (non-Send) return forces workarounds in the supervisor (`local-deployment/src/lib.rs:368`) and tests; all sources are Send+Sync | high | yes |
| 12 | `crates/services/src/services/trigger_hooks.rs:155-164` | low | quality | Match/non-match arms both end in the identical cursor persist; one persist site suffices | high | yes |
| 13 | `crates/services/src/services/event_bus/tailer.rs:88-89` | low | quality | Publishes `seq_ev.clone()` per event only because `seq` is read after the send; reorder removes a heap clone on the hot path | high | yes |
| 14 | `crates/services/src/services/event_bus/mod.rs:45-55` | low | quality | `StreamState::Closed` is never constructed; dead arm, unused `Clone` derive, blanket `#[allow(dead_code)]` | high | yes |
| 15 | `crates/db/src/models/execution_process/{lifecycle.rs:32,queries.rs:31}` | low | quality | `UNKNOWN_EXECUTOR` wire-visible sentinel duplicated in sibling modules; copies can drift | high | yes |
| 16 | `crates/services/src/services/event_bus/mod.rs:116-118` | low | quality | `_tx` underscore name on a binding that is used (cloned into tailer, stored as sender) | high | yes |
| 17 | `crates/server/src/routes/events.rs:46-47` | low | quality | Redundant `#[serde(default)]` on `Option<i64>` | low | yes |

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| 18 | `crates/db/src/models/execution_process/queries.rs:513`, `lifecycle.rs:174-213` | high | correctness | Attempt lifecycle events emit for every `run_reason` (setup/cleanup/dev-server/breakdown, not just coding agent) with no discriminator in the payload — a dev-server kill journals `attempt_failed{killed}`; an attempt "finishes" once per process | medium | Adjudicated in-run: SC2 + ledger undictated-choice-1 explicitly require executor identity "on every attempt event regardless of run reason"; changing emission scope is a spec change. Consumers can disambiguate via `execution_process_id` → `run_reason` join. Revisit when the first attempt-event consumer lands |
| 19 | `crates/server/src/routes/events.rs:170-184` | medium | correctness | The 410 staleness gate is advisory: a compaction pass landing between the route check and the stream's first `read_range` can still silently lose events for a cursor that passed | high | The residual window is one compaction tick between adjacent awaits; closing it requires the check to share a read transaction with the replay inside `subscribe_from` — a core-state-machine change disproportionate to the window. Gate catches the real case (long-disconnected clients) |
| 20 | `crates/services/src/services/trigger_hooks.rs:141-145` | low | correctness | A hard-cap pass landing inside the rebootstrap window has its fresh loss-flag erased by `clear_rebootstrap` (loss signal only; events already gone) | high | Narrow window; transactional flag-clear is follow-up work |
| 21 | `crates/services/src/services/event_bus/mod.rs:225,272` | low | efficiency | Replay/refill materialize the whole `(cursor, mark]` window per subscriber (bounded by hard cap ≈ 100k rows) | high | Bounded; batched replay is follow-up work |
| 22 | `crates/services/src/services/node_runner.rs` (Connected arm) | low | correctness | `ReconcileCompleted{entity_count:0}` journaled identically for success-with-zero, failure, and no-hive-client | medium | Possibly intended ("an occurrence, not a level"); needs a design decision, not a close-gate patch |
| 23 | `crates/server/src/error.rs:347` via `routes/events.rs:150-155` | low | security | Pre-stream 500 body carries transparent sqlx Display text (mid-stream frames were genericized; this path was not) | high | Matches codebase-wide `ApiError` Display behavior; changing it is a repo-wide policy decision |
| 24 | `crates/db/src/models/event_journal/queries.rs:55` | low | correctness | `read_range` fails the whole batch on one undecodable payload (poison pill under binary-rollback version skew) | medium | Requires version skew; per-row quarantine is follow-up work |
| 25 | `crates/db/src/models/event_journal/queries.rs:184-198` | low | correctness | `compact` under-flags if stage 2 empties the journal (`new_min_seq` falls back to 0) | high | Unreachable via the sole production caller (row clamps ≥ 1); defensive-API gap only |
| 26 | `crates/db/src/models/task_breakdown/queries.rs:373` | low | correctness | `accept_proposal` keeps the read-then-upgrade deferred-tx shape this branch eliminated elsewhere (availability-only 517 risk) | high | Pre-existing shape from the base branch; journal appends did not change it; out-of-scope |
| 27 | `crates/services/src/services/event_bus/mod.rs:193-208` | low | quality | `subscribe_from` returns `Result` but is currently infallible | medium | Signature reservation; ripple through all consumers not justified now |
| 28 | `crates/services/src/services/event_bus/mod.rs:216-233,262-283` | low | quality | Initializing and Lagged-refill arms duplicate the mark-then-read-range block | medium | Core replay state machine is panel-verified; extraction deferred |
| 29 | `crates/server/tests/events.rs`, `trigger_hooks.rs` tests, `emission_conformance.rs:119-167` | low | quality | Test-only duplication (SSE collect loop ×6; tx idiom ×6; counter vec) | high | Verified-test churn without behavior benefit; follow-up |
| 30 | `crates/local-deployment/src/lib.rs:417-418` | low | correctness | Hook-supervisor backoff doubles per respawn and never resets after a healthy run | high | Latency-only (journal makes restarts lossless); follow-up |

## Verified clean (for the record)
No-cursor TOCTOU (subscribe-first + fresh-mark replay + live dedup), seq ordering under SQLite single-writer, compaction boundary math, SSE frame fields, type exports, emission-conformance allowlist claims, test-harness isolation.

## Verdict: With fixes

Actionable: [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17]
