-- Read-only assembling view over the run-state triple (task_attempts + execution_processes +
-- executor_sessions). The durable, queryable "workstream-state surface" recovery resumes from and
-- downstream phases (P3/P6) query (SC3). No new run entity; this is a projection of existing tables.
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
