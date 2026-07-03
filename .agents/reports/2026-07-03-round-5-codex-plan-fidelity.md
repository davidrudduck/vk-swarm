I could not write `/data/Code/vk-swarm/.worktrees/vk-swarm-hive-redesign-p47/.agents/reports/2026-07-03-round-5-codex-plan-fidelity.md` because your instructions also said not to modify files or repository state, and the sandbox is read-only.

Report content:

```markdown
# Round 5 Adversarial Review — Plan Fidelity

Reviewed diff `3769a874..67996ab8`. Orientation commands run read-only:
`git diff --stat 3769a874..67996ab8` and `git log --oneline 3769a874..67996ab8`.

### [BLOCKING] Ratified P7 follow-up DROP task is not present
- **Phase/Task:** P7/701 follow-up
- **Citation:** `docs/plans/vk-swarm-hive-redesign/decisions-ledger.md:819`
- **Claim:** The ledger says Gate 2 ratified a follow-up DROP task to honor TS6 literally, but the phase-7 plan still only lists 701-703.
- **Evidence:** Ledger says the follow-up migration “drops them” and “adds a P7 follow-up task to the plan” (`decisions-ledger.md:819-822`), while the plan’s P7 table contains only 701, 702, and 703 (`plan.md:190-192`). The actual migration truncates discardable tables, not drops them (`crates/remote/migrations/20260201000000_hive_cutover_clear_regenerable_discardable.sql:24-31`).
- **Divergence type:** unjustified-divergence
- **Impact if shipped:** medium
- **Remediation:** Add the promised tracked follow-up task/workstream, or update the ledger to explicitly ratify keep-empty as final. Under the repo rules, this cannot be silently deferred.

### [SHOULD-FIX] Deleted Electric task path left stale docs behind
- **Phase/Task:** P4/405
- **Citation:** `crates/services/src/services/share.rs:9`
- **Claim:** The dead `ElectricTaskSyncService` was removed, but module docs still say Hive task sync is handled by ElectricSQL.
- **Evidence:** Task 405 says `share` should be documented as the WebSocket activity stream path (`phase-4/405-remove-dead-electric-task-sync.md:76-85`). The current docs still say “Task sync from Hive to local is now handled by ElectricSQL” (`share.rs:9`) and `mod.rs` still labels `share` under modules “being replaced” (`mod.rs:10-14`).
- **Divergence type:** regression-risk
- **Impact if shipped:** low
- **Remediation:** Update the stale docs to say task inbound sync is the WS activity stream plus connect-time reconcile.

### [INFO] P4 dirty guard follows the ratified entity-level divergence
- **Phase/Task:** P4/403
- **Citation:** `crates/db/src/models/task/sync.rs:268`
- **Claim:** The implementation matches the approved entity-level dirty guard, not a field-level guard.
- **Evidence:** `upsert_remote_task` returns the existing row when the linked local task has any unacked outbox op (`sync.rs:268-276`), using `has_unacked_for_entity` with `acked_at IS NULL` (`node_outbox.rs:140-158`). The ledger records this as ratified and more conservative than field-level (`decisions-ledger.md:547-554`).
- **Divergence type:** justified-divergence
- **Impact if shipped:** none
- **Remediation:** None.

### [INFO] P4 soft-unlink and tombstone routing follows the plan
- **Phase/Task:** P4/402, P4/401
- **Citation:** `crates/db/src/models/task/sync.rs:436`
- **Claim:** Both inbound delete legs now share one soft-unlink helper and retain local attempts.
- **Evidence:** `unlink_by_shared_task_id` clears `shared_task_id` rather than deleting (`sync.rs:436-451`); WS delete uses it (`processor.rs:436-440`), reconcile delete uses it (`node_runner.rs:1300-1304`), and TS5 asserts both executor forms retain the task attempt (`sync.rs:1660-1718`).
- **Divergence type:** followed-plan
- **Impact if shipped:** none
- **Remediation:** None.

### [INFO] P5 digest compare correctly uses existence, tombstones, and conservative replay floor
- **Phase/Task:** P5/503
- **Citation:** `crates/remote/src/nodes/ws/session.rs:2429`
- **Claim:** Digest comparison follows the recorded existence-based design and avoids both known replay bugs.
- **Evidence:** The compare builds node and hive task-id sets (`session.rs:2429-2448`), routes soft-deleted node-reported tasks to `pull_entities` (`session.rs:2455-2461`), and uses a single `.any()` requiring an id absent from both active and deleted hive sets before returning `Some(1)` (`session.rs:2474-2479`). Tests cover the floor and mixed tombstone/in-sync case (`session.rs:3509-3514`, `session.rs:3668-3715`).
- **Divergence type:** justified-divergence
- **Impact if shipped:** none
- **Remediation:** None.

### [INFO] P5 post-review remediations are correct and recorded
- **Phase/Task:** P5/503, P5/504
- **Citation:** `docs/plans/vk-swarm-hive-redesign/decisions-ledger.md:1485`
- **Claim:** Soft-deleted tombstone routing and re-stream pagination were post-review fixes, not original task scope, and both are sound.
- **Evidence:** Ledger records F1/F2 remediation (`decisions-ledger.md:1485-1486`). Code paginates `peek_from_seq` by advancing `from_seq = last_seq + 1` until short batch (`node_runner.rs:1105-1134`), and `peek_from_seq` intentionally includes acked rows (`node_outbox.rs:114-138`).
- **Divergence type:** justified-divergence
- **Impact if shipped:** none
- **Remediation:** None.

### [INFO] CF1 re-stream fencing-token propagation is the right local fix
- **Phase/Task:** P5/504 plus P2 fencing interaction
- **Citation:** `crates/services/src/services/node_runner.rs:1185`
- **Claim:** Passing `&HashMap<Uuid, i64>` into `restream_row_to_ws_op` mirrors live-send stamping and fixes the broken re-stream path.
- **Evidence:** The live send path stamps task tokens from `active_assignments` (`hive_sync.rs:229-257`); the re-stream path now builds the same task-token map (`node_runner.rs:1098-1104`) and applies it in `restream_row_to_ws_op` (`node_runner.rs:1185-1202`). Tests cover restamping, non-task preservation, and no-assignment fallback (`node_runner.rs:1880-1911`).
- **Divergence type:** justified-divergence
- **Impact if shipped:** none
- **Remediation:** Optional future cleanup: factor the duplicated row-to-WS mapping only if it stays stable.

### [INFO] P6 no-fanout guard follows the verify-and-fence plan
- **Phase/Task:** P6/601, P6/602
- **Citation:** `crates/remote/tests/no_fanout_invariant.rs:41`
- **Claim:** Phase 6 correctly asserts the existing no-fanout invariant rather than removing a non-existent fanout path.
- **Evidence:** The test exhaustively classifies every `HiveMessage` without a wildcard (`no_fanout_invariant.rs:41-64`) and rejects any `TaskStatePush` classification (`no_fanout_invariant.rs:167-179`). The plan explicitly made P6 a guard, not a removal (`plan.md:157-166`).
- **Divergence type:** followed-plan
- **Impact if shipped:** none
- **Remediation:** None.

### [INFO] WS variants appended after current tails were justified
- **Phase/Task:** P5/501, cross-phase P2/P5
- **Citation:** `docs/plans/vk-swarm-hive-redesign/decisions-ledger.md:836`
- **Claim:** Appending variants after the actual current enum tails was correct because the task’s Before snapshot was stale.
- **Evidence:** Ledger records the stale-tail adjustment (`decisions-ledger.md:836-838`). Current duplicated enums include `OpBatch`, `LeaseHeartbeat`, then `Digest` on both sides (`message.rs:87-96`, `hive_client.rs:118-127`) and explicit node handling for `DigestResult` before the later wildcard path (`hive_client.rs:1167-1188`).
- **Divergence type:** justified-divergence
- **Impact if shipped:** none
- **Remediation:** None.

### [INFO] P7 in-place TRUNCATE preserves schema identity as intended
- **Phase/Task:** P7/701-703
- **Citation:** `crates/remote/migrations/20260201000000_hive_cutover_clear_regenerable_discardable.sql:21`
- **Claim:** The migration implements the ratified in-place data-clear, preserving OIDs and MUST-MIGRATE rows.
- **Evidence:** Migration uses two `TRUNCATE TABLE` statements and one scoped `DELETE`, with no `DROP`, `CASCADE`, or MUST-MIGRATE table mutation (`migration.sql:21-31`). Tests assert seeded rows are cleared/retained (`hive_cutover_migration.rs:93-109`), table OIDs stay stable (`hive_cutover_migration.rs:130-160`), id bridge/status round-trip (`hive_cutover_must_migrate.rs:71-93`), and re-ingest refills `node_task_attempts` (`hive_cutover_reingest.rs:98-120`).
- **Divergence type:** justified-divergence
- **Impact if shipped:** none, except the missing follow-up tracked above
- **Remediation:** None for the in-place migration itself.

## Overall Verdict

`REJECT` — Mechanics for P4-P7 are largely faithful and the known CF1/CF2/P5 remediation bugs are fixed correctly. The remaining blocker is plan-fidelity/process: the ledger says the ratified P7 path adds a follow-up DROP task to satisfy literal TS6 “discardable tables are absent,” but no such task/workstream is present in the plan or diff. There is also a small stale-doc issue from P4/405.
```