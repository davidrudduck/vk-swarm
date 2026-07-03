-- SC6 / ADR-0011 — one-time hive-only-state cutover (DATA-CLEAR leg). IRREVERSIBLE (data loss). Gate
-- behind a pre-cutover backup (ADR-0011 Consequences). Forward-only.
--
-- Interpretation (plan.md Phase 7 note; ratified judgment call): the rebuild is IN-PLACE and the cutover
-- is a DATA operation, not a schema teardown. Every REGENERABLE/DISCARDABLE table still has surviving
-- query references in crates/remote/src (NOT removed by this workstream — that is P4/P5), and the node
-- re-ingest path INSERTs into the existing table rather than recreating it. So we TRUNCATE the data and
-- KEEP the schema. MUST-MIGRATE tables (shared_tasks incl. the source_task_id/source_node_id id bridge,
-- node_api_keys, nodes, swarm_projects/_nodes, swarm_templates, labels/shared_task_labels,
-- identity/tenancy) are NOT touched here; their preservation is asserted by 702.

-- REGENERABLE — node-mirror caches / logs / sync bookkeeping; data rebuilt by node re-ingest (ADR-0008
-- outbox + the existing handle_attempt_sync path). Clear data, keep schema. These are listed TOGETHER in
-- one TRUNCATE so their intra-set FKs (node_execution_processes.attempt_id → node_task_attempts;
-- *_logs/_events.assignment_id → node_task_assignments stays UNtruncated) are satisfied WITHOUT CASCADE —
-- CASCADE is deliberately OMITTED so the operation can NEVER silently reach a MUST-MIGRATE table (the
-- seed test asserts shared_tasks survives). No RESTART IDENTITY: these are UUID-PK tables, no serials.
-- (sync_state / backfill_request_id / last_full_sync_at are COLUMNS on node_task_attempts — cleared with
-- its rows.) NOTE: node_task_output_logs/_events FK node_task_assignments(id) ON DELETE CASCADE, but
-- assignments are not TRUNCATEd here (only DELETEd by WHERE below), so no cross-set dependency.
TRUNCATE TABLE node_execution_processes, node_task_output_logs, node_task_progress_events,
               node_task_attempts, node_local_projects, project_activity_counters;

-- DISCARDABLE — not migrated (ADR-0011). The tables stay (kept auth/activity code references them — the
-- code removal is out of this workstream's scope); their history is cleared. activity is partitioned;
-- TRUNCATE empties all partitions. No CASCADE / no RESTART IDENTITY: these are leaf, UUID-PK tables.
TRUNCATE TABLE activity, auth_sessions, oauth_handoffs, revoked_refresh_tokens;

-- DISCARDABLE rows inside a MUST-MIGRATE table: completed assignments (ADR-0011). Keep active
-- (completed_at IS NULL) — the only record of which node owns which in-flight task.
DELETE FROM node_task_assignments WHERE completed_at IS NOT NULL;