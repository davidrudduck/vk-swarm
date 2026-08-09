-- DV-2: task 201 added the Breakdown run_reason variant in Rust/TS, but the
-- execution_processes CHECK constraint still enumerated only the four original
-- values, so inserting run_reason='breakdown' failed at runtime. Mirror of
-- 20250720000000_add_cleanupscript_to_process_type_constraint.sql (column-swap,
-- SQLite cannot alter a CHECK in place).

-- 1. Add the replacement column with the wider CHECK
ALTER TABLE execution_processes
  ADD COLUMN run_reason_new TEXT NOT NULL DEFAULT 'setupscript'
    CHECK (run_reason_new IN ('setupscript',
                              'cleanupscript',
                              'codingagent',
                              'devserver',
                              'breakdown'));   -- new value

-- 2. Copy existing values across
UPDATE execution_processes
  SET run_reason_new = run_reason;

-- 3. Drop any indexes AND views that mention the old column
--    (v_workstream_state projects ep.run_reason; DROP COLUMN refuses while it exists)
DROP INDEX IF EXISTS idx_execution_processes_type;
DROP VIEW IF EXISTS v_workstream_state;

-- 4. Remove the old column (requires 3.35+)
ALTER TABLE execution_processes DROP COLUMN run_reason;

-- 5. Rename the new column back to the canonical name
ALTER TABLE execution_processes
  RENAME COLUMN run_reason_new TO run_reason;

-- 6. Re-create the index
CREATE INDEX idx_execution_processes_type
        ON execution_processes(run_reason);

-- 7. Re-create the view verbatim from 20260201000200_add_workstream_state_view.sql
CREATE VIEW IF NOT EXISTS v_workstream_state AS
SELECT
    ep.id                AS execution_process_id,
    ep.task_attempt_id   AS task_attempt_id,
    ta.container_ref     AS container_ref,
    ta.branch            AS branch,
    ta.target_branch     AS target_branch,
    ep.run_reason        AS run_reason,
    ep.status            AS status,
    ep.resume_state      AS resume_state,
    ep.pid               AS pid,
    ep.before_head_commit AS before_head_commit,
    ep.after_head_commit  AS after_head_commit,
    es.session_id        AS session_id,
    ep.created_at        AS created_at
FROM execution_processes ep
JOIN task_attempts ta ON ep.task_attempt_id = ta.id
LEFT JOIN executor_sessions es ON es.execution_process_id = ep.id;
