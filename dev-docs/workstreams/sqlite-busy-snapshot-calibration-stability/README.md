---
workstream: sqlite-busy-snapshot-calibration-stability
doc_type: readme
status: active
title: "Make SQLite busy-snapshot calibration deterministic"
depends_on: []
adrs: []
staging_pointers:
  - docs/plans/local-node-browser-oauth/decisions-ledger.md
---

# sqlite-busy-snapshot-calibration-stability

**Origin:** discovered before the `local-node-browser-oauth` work and reproduced during its
integrated phase-1 adversarial review on 2026-08-22. The browser-auth diff does not touch the
execution-process subsystem, but the red database gate had to be resolved in the discovering
session under the repository's no-deferred-remediation rule.

## Finding

The negative calibration controls for the execution-process write-first tests used a background
writer with a 200-microsecond sleep and hoped that one of 200 writes would commit after a deferred
transaction's `SELECT` but before its `UPDATE`. On this host,
`control_read_then_write_shape_reproduces_busy_snapshot` repeatedly observed 0/200
`SQLITE_BUSY_SNAPSHOT` errors and failed. The sibling lifecycle control used the same probabilistic
shape and was therefore exposed to the same failure even when an individual run happened to pass.

## Resolution in this branch

Both controls now construct the SQLite schedule directly:

1. begin a deferred transaction and issue its `SELECT`, opening a WAL read snapshot;
2. perform and await a write through a separate pooled connection;
3. attempt the original transaction's `UPDATE` and require SQLite extended error code 517.

The production write-first stress tests remain unchanged. No test is ignored and no quality gate is
disabled. The two repaired controls passed together in ten consecutive focused runs, and focused DB
clippy passed with warnings denied.

## Completion criteria

- [x] Both negative controls force the conflicting commit instead of relying on scheduler timing.
- [x] Both controls assert the exact `SQLITE_BUSY_SNAPSHOT` extended code.
- [x] The real write-first tests remain live and unchanged.
- [x] Focused controls pass repeatedly and `cargo clippy -p db --all-targets --all-features -- -D
      warnings` is green.
- [ ] The branch containing this remediation is merged; mark this workstream `shipped` then.
