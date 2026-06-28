-- Add a counter tracking how many times crash-recovery has tried and failed to fence
-- a process stuck in D-state (uninterruptible sleep). Read/incremented only on the
-- CouldNotKill path of cleanup_orphan_executions; surfaced for operator escalation.
-- See ADR-0005.
ALTER TABLE execution_processes ADD COLUMN fence_attempt_count INTEGER NOT NULL DEFAULT 0;
