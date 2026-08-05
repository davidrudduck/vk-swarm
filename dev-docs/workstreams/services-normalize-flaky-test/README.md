---
workstream: services-normalize-flaky-test
status: active
created: 2026-08-04
parent_session: vk-swarm-node-ui-localize /wai:close code-review gate
---

# services-normalize-flaky-test

Fixes the intermittently-failing `test_fast_execution_no_lost_logs`
(`crates/services/tests/normalize_sync_test.rs:365`), discovered during the `/dr:code-review`
pre-graduation gate of the `vk-swarm-node-ui-localize` close. Filed as **F-2026-08-04-02**.

## Evidence it is a FLAKE, and PRE-EXISTING

Not a regression from `vk-swarm-node-ui-localize`:

```text
$ git diff --name-only feff74be..HEAD -- crates/services/ crates/executors/
(empty — this branch never touched services or executors)
```

Intermittent, and independent of the close's own edit:

```text
$ cargo test --workspace                       # first run
test=101   -> 45 "test result: ok" blocks
  panicked at crates/services/tests/normalize_sync_test.rs:365:5:
  Expected at least 1 JsonPatch entry for fast execution, got 0.

$ git stash                                    # remove the close's harness_smoke edit
$ cargo test -p services --test normalize_sync_test
baseline exit=101                              # STILL fails -> not caused by the edit

$ cargo test -p services --test normalize_sync_test test_fast_execution_no_lost_logs
run 1..5 exit=0                                # passes 5/5 in ISOLATION

$ cargo test -p services --test normalize_sync_test   # whole file, later
run 1..3 exit=0

$ cargo test --workspace                       # second run
test=0     -> 57 "test result: ok" blocks
```

Passes in isolation, passes on re-run, fails intermittently in a full-file / full-workspace run.
That signature is a **race between the normalizer task and the assertion**, not a logic error.

## Why a separate workstream rather than a task in `vk-swarm-node-ui-localize`

That spec is FROZEN (ADR-0001) and covers node hive-proxy routes, the API-key surface,
`ProjectWithStats`, and the hive-absent state. The log normalizer is none of those, and the branch
does not touch `crates/services` at all. This is a legitimate named scope split under CLAUDE.md
"No Deferred Remediation", tracked in THIS session rather than deferred to the next.

**It is NOT being suppressed.** No `#[ignore]`, no retry wrapper, no disabled category — the test
stays live and can still fail CI until it is fixed properly.

## The likely defect

`normalize_sync_test.rs:350-368` pushes one message, immediately calls `push_finished()`, then
awaits the normalization handle under a 5s timeout:

```rust
let _ = tokio::time::timeout(Duration::from_secs(5), norm_handle).await;
let patch_count = count_json_patches(&msg_store);
assert!(patch_count >= 1, ...);
```

The timeout result is **discarded** (`let _ =`). If the handle has not completed — which is more
likely under the CPU contention of a full-workspace run — the test proceeds to assert against a
store the normalizer has not yet written to, and reads 0 patches. The assertion message
("Fast executions should not lose logs") then misreports a test-harness race as a product defect.

## What "done" looks like

- The discarded `timeout` result is checked, so a timeout fails as a *timeout* rather than as a
  bogus "lost logs" assertion.
- The test waits on a real completion signal instead of racing the handle.
- Passes reliably under `cargo test --workspace` (the contended path), not just in isolation —
  demonstrated by repeated full-workspace runs.
- Whether the other tests in this file share the pattern is checked, not assumed.
