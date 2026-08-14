---
workstream: normalize-fast-execution-lost-logs-flake
doc_type: readme
status: draft
title: "normalize_sync_test.rs is load-sensitive — possible real lost-log race"
depends_on: []
adrs: []
staging_pointers:
  - docs/plans/vk-swarm-event-bus/decisions-ledger.md
---

# normalize-fast-execution-lost-logs-flake

**Origin:** discovered 2026-08-12 during `/wai:execute vk-swarm-event-bus`, when task 013's Stage-1
gate rejected on a test that has nothing to do with the event bus. Split out rather than carried
silently, per CLAUDE.md "No Deferred Remediation" — it is pre-existing, it is not fixable inside task
013's file set, and it may be a real product bug rather than test flakiness.

## The finding

`crates/services/tests/normalize_sync_test.rs::test_fast_execution_no_lost_logs` fails
intermittently. Measured rate on **pristine pre-013 code** (worktree at `6077d670`, task 005's
commit, nothing from task 013 present):

```text
1: ok   2: FAILED   3: ok   4: ok   5: ok   6: ok   7: ok   8: ok
=== pristine pre-013 failures: 1/8 ===
```

It also failed 2 of 3 full `cargo test -p services` runs on the task-013 branch at one point, and
1 of 4 later, so the rate is load-sensitive.

## It is NOT caused by the event-bus work — established by controlled A/B

- Pristine pre-013 code (`6077d670`): **fails 1/8**. This is the decisive datum.
- Same base worktree with task 013 attempt 7's `tailer.rs` + `mod.rs` copied in: passes 3/3.
- Main worktree with attempt 7: fails ~1/4 to 2/3 depending on machine load.

Same code passing in one worktree and failing in another, plus the failure reproducing on code that
predates task 013 entirely, rules out the event bus as the cause. The variable is machine load.

## Why this may be a REAL bug, not test debt

The test's name is the point: it was written because fast executions were **losing logs**. It
asserts `patch_count >= 1` after pushing one message, pushing finished, and awaiting the
normalization handle with a 5s timeout:

```rust
let _ = tokio::time::timeout(Duration::from_secs(5), norm_handle).await;
let patch_count = count_json_patches(&msg_store);
assert!(patch_count >= 1, "Expected at least 1 JsonPatch entry for fast execution, got {} ...");
```

A failure means normalization produced NO patches for a single message within five seconds. Under
load that could be a slow machine — or it could be the original race, still live, surfacing rarely.
**It was deliberately NOT marked `#[ignore]`**: silencing it would remove the only guard against a
lost-log bug in production log handling, which is a worse trade than an occasional red run.

## It is the WHOLE FILE, not one test — and we may be aggravating it

A later gate run failed on a DIFFERENT test in the same file,
`test_normalization_malformed_input`, so the unit of flakiness is
`crates/services/tests/normalize_sync_test.rs` as a whole, not `test_fast_execution_no_lost_logs`
alone.

Measured rates, which only make sense together:

| condition | result |
|---|---|
| standalone, quiet machine | **0/12 failed** |
| standalone, machine loaded by other agents | **1/8 failed** |
| inside full `cargo test -p services` | failed 2/3, then 1/4, then again on two gate runs |

So the trigger is **machine load**, and the file is far more likely to fail inside the full crate run
than standalone.

**The nuance worth stating plainly:** the event-bus branch does not CAUSE this, but it plausibly
AGGRAVATES it. `cargo test -p services` now runs a lib suite of 267 tests that spawns nine tailer
tests, each with polling tokio tasks and its own SQLite pool, in the same invocation. That is more
load in the same command than before. Pre-existing cause, possibly increased frequency — both
statements are true and the second should not be hidden behind the first.

## What is needed

1. Reproduce with the failure message captured (10 targeted runs did not reproduce it; it needs
   sustained load, or a stress harness that pins CPU while looping the test).
2. Determine from the captured output whether the handle timed out or completed with zero patches —
   those are different bugs. Timeout = slow; completed-with-zero = a genuine lost-log race.
3. If it is a real race, fix it in the normalizer and keep the test as the regression guard.
4. If it is purely timing, make the test wait on an observable condition rather than a 5s wall-clock
   budget — the same fix pattern task 013 applied to its own flaky tailer tests (a readiness/
   happens-before edge instead of a deadline).

## Impact while open

`cargo test -p services` (and therefore the WAI Stage-1 gate for any task scoped to
`crates/services`) can reject spuriously at roughly a 1-in-8 rate under load. A rejection citing ONLY
`test_fast_execution_no_lost_logs` is this issue and not the task under gate — verify by re-running
the crate's own tests and confirming no other failure.
