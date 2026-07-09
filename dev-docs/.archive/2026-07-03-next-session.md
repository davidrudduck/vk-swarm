# Next Session: hive-redesign — Ship, E2E Test, Observability, Property Tests

**Date:** 2026-07-03
**PR:** https://github.com/davidrudduck/vk-swarm/pull/451
**Branch:** `vk-swarm-hive-redesign-p47` in worktree `.worktrees/vk-swarm-hive-redesign-p47`

## Context

PR #451 (Phases 4-7: inbound collapse, anti-entropy, no-fan-out guard, hive cutover) is open and ready to merge. All 7 phases are implemented; 5 rounds of 3-model adversarial review (Claude/Codex/Gemini) are complete; all gate checks are green (clippy, 196+94+205 tests, lint, tsc).

The frozen spec is at `docs/superpowers/specs/2026-06-26-vk-swarm-hive-redesign.md`. The plan is at `docs/plans/vk-swarm-hive-redesign/plan.md`. The decisions ledger is at `docs/plans/vk-swarm-hive-redesign/decisions-ledger.md`. CONTRACT.md has the frozen WS variant shape.

### Key architectural facts
- Task sync uses WS activity stream + durable outbox + op-batch/ack/digest model (ADR-0007/SC7)
- ElectricSQL still exists for non-task sync (projects, nodes, logs)
- Fencing tokens prevent stale writes from partitioned nodes (SC3)
- Digest anti-entropy heals divergence on reconnect (SC5)
- SKIP+ADVANCE is an invariant: every permanent reject in `handle_op_batch_apply` MUST record in `node_op_log` + advance `applied_through_seq` (round-5 CF1)
- `peek_unacked` is head-of-line (seq-ordered, filters `acked_at IS NULL`); a rejected op that doesn't SKIP+ADVANCE wedges ALL subsequent ops for that node

### Key code locations
- `crates/remote/src/nodes/ws/session.rs` — `handle_op_batch_apply`, `handle_digest`/`handle_digest_compare`, fencing checks, SKIP+ADVANCE pattern
- `crates/remote/src/db/tasks.rs` — `SharedTaskRepository`, `list_source_task_versions_for_node`, `list_soft_deleted_source_task_ids_for_node`
- `crates/db/src/models/node_outbox.rs` — `peek_unacked`, `peek_from_seq`, `has_unacked_for_entity`, `OutboxOp` struct
- `crates/db/src/models/task/sync.rs` — unlink helpers, dirty guard
- `crates/services/src/services/node_runner.rs` — `ActiveAssignment`, `restream_row_to_ws_op` (takes `&HashMap<Uuid, i64>` for token re-stamp), heal branch
- `crates/services/src/services/hive_sync.rs` — `sync_digest` helper, live-send token stamping
- `crates/services/src/services/connection.rs` — send-site comment fence
- `crates/remote/migrations/20260201000000_hive_cutover_clear_regenerable_discardable.sql` — cutover migration
- `docs/plans/vk-swarm-hive-redesign/CONTRACT.md` — frozen WS variant shape, schemas, fence semantics

### Standing debt (accepted with evidence)
- `renew_lease` does NOT check `lease_expires_at > now()` — an already-expired-but-not-reclaimed holder can renew. Matches the task file's literal contract. Accepted in ledger.
- `NodeTaskAttemptRepository` lacks txn support (Gemini round-4 INFO). Accepted as standing debt.
- Test boilerplate duplication in fencing tests (Gemini round-4 INFO). Accepted as standing debt.

## Tasks (in order)

### 1. Merge PR #451 and ship the workstream
- Merge the PR
- Run `/wai:ship` to graduate staging docs to `dev-docs/` and mark the spec `status:shipped`

### 2. Fix the test-utils feature flaw
- `cargo test --workspace` can't run because integration tests in `crates/remote/tests/` don't compile due to a pre-existing Cargo feature issue with `test-utils`
- Fix the feature so `cargo test --workspace` works — this unblocks CI for the hive-redesign E2E tests
- The E2E tests (`backfill_e2e.rs`, `lease_partition_e2e.rs`, `hive_cutover_*.rs`) use `skip_without_db!` and need `DATABASE_URL`, but they must at least COMPILE in CI

### 3. Build a full E2E partition test
- Write a test that spins up 2 nodes + hive (or uses the existing test harness in `crates/remote/tests/`)
- Scenario: node_a holds a task lease → network partition → hive reassigns to node_b → node_b completes → partition heals → node_a sends stale op (fenced, SKIP+ADVANCE) → digest runs → node_a converges (unlinks stale task, pulls node_b's completed state)
- Assert: no data loss, no wedge, op-log drains, digest converges
- This is THE scenario the redesign was built for. It currently has NO end-to-end test.

### 4. Add observability/telemetry
- Add metrics (using whatever tracing/metrics crate the repo already uses) for:
  - `node_outbox_depth` (gauge) — unacked ops per node
  - `hive_fence_reject_total` (counter) — stale-token rejections
  - `hive_lease_revoke_total` (counter) — lease revocations emitted
  - `digest_convergence_latency_ms` (histogram) — time from Digest to converged state
  - `op_batch_apply_total` (counter) — ops applied vs rejected
- Surface in the existing admin/health endpoints

### 5. Add a property-based test for partition/reconnect sequences
- Use proptest or quickcheck (check `Cargo.toml` for existing deps)
- Generate random sequences of: {op_batch, partition, reconnect, digest, reassign, complete}
- Assert invariants: op-log eventually drains, digest eventually converges, no permanent wedge, fencing always prevents stale writes
- This catches timing-dependent edge cases that 5 rounds of adversarial review couldn't reach

## Rules
- Follow CLAUDE.md and AGENTS.md
- Run all 4 mandatory gate checks before any commit: `cargo clippy --all --all-targets --all-features -- -D warnings`, `cargo test --workspace`, `cd frontend && npm run lint`, `cd frontend && npx tsc --noEmit`
- No deferred remediation — fix findings in-session
- Open PRs only against `davidrudduck/vk-swarm`
- Gate env: `SQLX_OFFLINE=true` + `DATABASE_URL=sqlite://<worktree>/dev_assets/db.sqlite` (db/services crates); `DATABASE_URL=postgres://postgres:postgres@localhost:5435/vibe_remote_dev` (remote crate)
- Use `cargo test -p <crate> --lib <filter>` for per-crate tests if workspace tests don't compile yet
- Remind me what stage is next when you complete each task
