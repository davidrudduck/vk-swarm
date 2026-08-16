---
id: "021"
phase: 3
title: "Add the emission conformance guard (architecture fitness test)"
status: ready
depends_on: ["006", "007", "020", "022"]
parallel: false
conflicts_with: []
files:
  - "crates/db/tests/emission_conformance.rs"
irreversible: false
scope_test: "crates/db"
allowed_change: create
covers_criteria: ["SC1", "SC2"]
covers_tests: ["TS3"]
---
## Why (context, not action)
Spec Design "Coverage invariant" (third amendment, 2026-08-16): every production task/execution-
process lifecycle write journals exactly one event, enforced by a fitness test rather than review
discipline. Motivation: during this run, manual enumeration of write sites was wrong FOUR times —
each time a `grep` was filtered, truncated, or not followed by caller analysis. This test is the
mechanical replacement. It is read-only over the repo tree and touches no database.

## Failing test (write first)
This task IS a test. Write it with an empty `EXPECTED` table first, run
`cargo test -p db --test emission_conformance`, and confirm it FAILS printing the full actual
site inventory. Then fill `EXPECTED` from that output, classifying every entry per the table
below. It must then pass — and a re-run after appending a dummy
`// UPDATE tasks SET x` does NOT count as a mutation check (comments are stripped); instead
verify the net by TEMPORARILY adding `sqlx::query("UPDATE tasks SET title = 'x'")` to any
production fn in `crates/db/src/models/task/archive.rs`, observing the failure, and REMOVING it
(record the observed failure output in the ledger; the temporary line must not be committed).

## Change
Create `crates/db/tests/emission_conformance.rs` — plain `#[test]` (not tokio), std only.

**Scan rules (all dictated):**
1. Workspace root: `env!("CARGO_MANIFEST_DIR")` + `/../..`. Walk `crates/**/*.rs` recursively with
   `std::fs` (no walkdir dependency).
2. Skip any path containing a `/tests/` segment (integration tests) and any path under a
   `/target/` segment.
3. **Test-region stripping:** inside each file, truncate from the first line that trims exactly to
   `#[cfg(test)]` AND whose next non-empty line's trimmed form starts with `mod ` — from that line
   to EOF is test code (repo convention: terminal test modules). Item-level `#[cfg(test)]`
   attributes on fields/functions (e.g. `crates/local-deployment/src/container.rs:108`) do NOT
   trigger truncation — that is exactly why the `mod` lookahead is required.
4. Strip `//`-comment suffixes from lines before matching (a commented-out query is not a site).
5. Patterns counted (plain substring match on the stripped text): `INSERT INTO tasks`,
   `UPDATE tasks`, `DELETE FROM tasks`, `INSERT INTO execution_processes`,
   `UPDATE execution_processes`, `DELETE FROM execution_processes`.
6. Build a sorted inventory of `"<repo-relative-path> <pattern> x<count>"` lines and compare with
   `EXPECTED` (a `&[&str]` in the test). On ANY difference, fail with a message printing both the
   full diff and this instruction text verbatim: "New or changed task/execution_process lifecycle
   write site. Per spec Design 'Coverage invariant' (docs/superpowers/specs/
   2026-08-07-vk-swarm-event-bus.md) every such write must journal a NodeEvent or carry a reviewed
   allowlist entry here. Instrument it (see Task::update / ExecutionProcess::update_completion for
   the pattern) or add the entry WITH a written reason — do not silently bump a count."

**Classification (what your generated counts must reconcile to — one comment per EXPECTED entry).**
If your generated inventory contains ANY file not listed here, STOP and report it — do not
classify it yourself:

| file | pattern | classification |
|---|---|---|
| db/src/models/task/queries.rs | INSERT x1, UPDATE x1, DELETE x1 | INSTRUMENTED (task 006) |
| db/src/models/task/hierarchy.rs | UPDATE x2 | :50 INSTRUMENTED (006 update_status); :90 parent_task_id nullify — metadata, ALLOWLISTED |
| db/src/models/task/archive.rs | UPDATE x4 | archived_at only — outside event vocabulary, ALLOWLISTED |
| db/src/models/task/sync.rs | INSERT x2 | :283 INSTRUMENTED (task 022); :32 sync_from_shared_task dead (zero callers), ALLOWLISTED |
| db/src/models/task/sync.rs | UPDATE x13 | sync metadata only, ALLOWLISTED |
| db/src/models/task/sync.rs | DELETE x2 | dead/test-only (ADR-0007 soft-unlink), ALLOWLISTED |
| db/src/models/task/cleanup.rs | DELETE x1 | retention purge of archived terminal tasks, ALLOWLISTED |
| db/src/models/task_breakdown/queries.rs | INSERT x1 | INSTRUMENTED (task 020) |
| server/src/bin/cleanup_duplicate_tasks.rs | DELETE x1 | one-off ops binary, ALLOWLISTED |
| db/src/models/execution_process/queries.rs | INSERT x1, UPDATE x3, DELETE x1 | INSERT :473 + UPDATE :169 INSTRUMENTED (task 007); UPDATE :231/:262 metadata; DELETE :533 post-terminal cleanup, ALLOWLISTED |
| db/src/models/execution_process/lifecycle.rs | UPDATE x6 | :126 INSTRUMENTED (task 007 update_completion); :231/:249/:263/:282/:303 metadata, ALLOWLISTED |
| db/src/models/execution_process/sync.rs | UPDATE x3 | hive_synced_at metadata, ALLOWLISTED |

Counts were enumerated 2026-08-16 on this branch; if a count differs by the time you run (a task
landed in between), reconcile against the classifications above — a NEW file or a count HIGHER
than explainable by the named lines is a STOP, not a shrug.

## Allowed moves
ONLY: creating the single new test file. No production code, no other test files, no Cargo.toml
edits (std only — verify no new dependency is needed before writing).

## STOP triggers
- The generated inventory contains a file absent from the classification table.
- Test-region stripping cannot be applied cleanly to some file (production code AFTER a terminal
  test module) — report the file; do not special-case silently.
- The mutation check (temporary archive.rs line) does NOT fail the test — the scanner is broken;
  do not ship a guard that cannot catch its target.

## Manual verification (record in decisions-ledger)
Gate invocation: Rust crate — override the runner. Use
WAI_TYPECHECK_CMD="cargo fmt --all -- --check && cargo check --workspace --all-targets" with
WAI_TEST_CMD="cargo test -p db --test emission_conformance".
Record in the ledger: the mutation-check failure output, and confirmation that the temporary line
was removed (git diff of the file clean).

## Done when
`WAI_TYPECHECK_CMD="<typecheck>" WAI_TEST_CMD="<test>" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-event-bus 021` exits 0
