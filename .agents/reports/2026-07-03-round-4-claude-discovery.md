`ExitPlanMode` is not available in this session's tool set — the plan-mode shell was triggered but the exit tool was not loaded. The report content is complete; here it is directly so nothing is lost:

---

# Tournament Adversarial Review — Round 4 (Claude Discovery)
**Branch:** `vk-swarm-hive-redesign-p47` · **Date:** 2026-07-03 · **Scope:** Phases 4–7 + debt fixes

---

### FINDING F1: `resend_from_seq` false positive — two independent differences instead of one intersection

- **Tag:** [SHOULD-FIX]
- **Location:** `crates/remote/src/nodes/ws/session.rs:2469–2474`
- **Evidence:**
  ```rust
  let resend_from_seq = if node_ids.difference(&hive_ids).next().is_some()
      && node_ids.difference(&hive_deleted_ids).next().is_some()
  {
      Some(1i64)
  } else {
      None
  };
  ```
  `hive_ids` = `source_task_id` of active rows (`deleted_at IS NULL`).  
  `hive_deleted_ids` = `source_task_id` of soft-deleted rows (`deleted_at IS NOT NULL`).  
  These sets are disjoint by construction.

- **Problem:** The two `.is_some()` calls test **independent** items, not one item satisfying both predicates. Counter-example:
  - `node_ids = {A, B}`, `hive_ids = {B}` (B in-sync), `hive_deleted_ids = {A}` (A tombstoned)
  - `node_ids.difference(&hive_ids)` = `{A}` → non-empty
  - `node_ids.difference(&hive_deleted_ids)` = `{B}` → non-empty
  - **Current result:** `resend_from_seq = Some(1)` ← spurious re-stream
  - **Correct result:** `None` — A is a tombstone (goes to `pull_entities`), B is in sync; no genuine gap

  The existing test `digest_routes_soft_deleted_task_to_pull_not_restream` only exercises `node_ids = {A}`, `hive_deleted = {A}` (all-deleted case) and passes. The mixed-case (at least one deleted + at least one in-sync) is not tested and triggers the false positive.

  Consequence: spurious `OpBatch` re-streams. They are idempotent (`ON CONFLICT DO NOTHING`) so no data is corrupted, but they are semantically wrong and wasteful.

- **Remediation:** Replace the two-condition `&&` with a single `.any()`:
  ```rust
  // session.rs:2469 — replace the entire resend_from_seq block with:
  let resend_from_seq = if node_ids
      .iter()
      .any(|id| !hive_ids.contains(id) && !hive_deleted_ids.contains(id))
  {
      Some(1i64)
  } else {
      None
  };
  ```
  This correctly requires **one item** to be absent from **both** sets simultaneously — i.e., genuinely missing from the hive (neither active nor tombstoned).

- **Remediation-verification:** Add a `digest_tests` test with `node_ids = {A (tombstoned), B (active)}` and assert `resend_from_seq == None`, `pull_entities.contains(&A)`. `cargo test -p remote --test no_fanout_invariant` and the new digest test both pass.

---

### FINDING F2: Missing trailing newline in four new files

- **Tag:** [INFO]
- **Location:** `crates/remote/migrations/20260201000000_hive_cutover_clear_regenerable_discardable.sql`, `crates/remote/tests/hive_cutover_migration.rs`, `crates/remote/tests/hive_cutover_must_migrate.rs`, `crates/remote/tests/no_fanout_invariant.rs` (all at EOF)
- **Evidence:** All four show `\ No newline at end of file` in `git diff`.
- **Problem:** `cargo fmt --all -- --check` flags `.rs` files without trailing newlines.
- **Remediation:** Append `\n` to the last line of each file.
- **Remediation-verification:** `cargo fmt --all -- --check` exits 0; `git diff --check HEAD` clean.

---

## Fidelity summary (all clean)

All SC7/SC5/SC1/SC6 requirements verified in code:
- **P4 (SC7):** soft-unlink helpers (`sync.rs:424`), dirty-guard (`upsert_remote_task:271`), `task.reassigned` arm (`processor.rs:67`), dead-code deleted (`electric_task_sync.rs`), TS5 test (`sync.rs:1660`)
- **P5 (SC5):** WS variants match CONTRACT §A exactly; `handle_digest_compare` covers in-sync / node-has-hive-lacks / hive-has-node-lacks / tombstone cases; paginated re-stream loop with `RESTREAM_LIMIT=500` and cursor advancement; `resend_from_seq = Some(1)` not `MAX(seq)` (R2/F4)
- **P6 (SC1/TS7):** exhaustive 16-variant `HiveMessage` match; send-site comment fence in `connection.rs`
- **P7 (SC6/TS6):** `TRUNCATE` (not `DROP`) migration; seed→run→assert hollow-test guard; OID stability test; id-bridge round-trip; re-ingest path proven

Migration FK ordering is safe: `node_execution_processes` (FK→`node_task_attempts`) and `node_task_attempts` appear in the same `TRUNCATE TABLE` statement — PostgreSQL handles intra-statement FK cycles. `_output_logs`/`_events` FK to `node_task_assignments` (not truncated), which is fine since we're deleting child rows, not parents.

Pre-existing debt fixes are clean (`activity.rs` column list, `organization_members.rs` type cast).

---

**VERDICT: REVISE** — one [SHOULD-FIX] (F1, one-line fix) and four [INFO] newline nits (F2). No blocking defects.

---

The full report content is saved in the plan file at `/home/david/.claude/plans/do-not-modify-files-modular-river.md`. Once plan mode is exited, the execution step is to write it verbatim to `.agents/reports/2026-07-03-round-4-claude-discovery.md`.