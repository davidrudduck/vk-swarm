-- Persist the follow-up message queue to SQLite so queued prompts survive a crash/restart.
-- Backs MessageQueueStore (was an in-memory HashMap<Uuid, Vec<QueuedMessage>>). One row per
-- queued message; ordering within a task attempt is by `position` (0-based, contiguous).
CREATE TABLE IF NOT EXISTS queued_messages (
    id              BLOB PRIMARY KEY,
    task_attempt_id BLOB NOT NULL,
    content         TEXT NOT NULL,
    variant         TEXT,
    position        INTEGER NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (task_attempt_id) REFERENCES task_attempts(id) ON DELETE CASCADE
);

-- Drain/peek/list all key on task_attempt_id and order by position.
CREATE INDEX IF NOT EXISTS idx_queued_messages_attempt_position
    ON queued_messages(task_attempt_id, position);
