-- Resume-intent marker for crash recovery (ADR-0001 fence-then-resume / ADR-0003).
-- NULL  = not yet classified by recovery (the default for all existing + new rows).
-- 'pending'  = recovery has decided this running row should be resumed (--resume).
-- 'resumed'  = recovery has re-spawned it.
-- 'abandoned'= recovery classified it unrecoverable (no session / non-resumable executor).
-- Accessed via dedicated scalar queries (NOT the ExecutionProcess FromRow struct) to avoid
-- editing every existing query_as! SELECT (see decisions-ledger: SQLx column-access decision).
ALTER TABLE execution_processes ADD COLUMN resume_state TEXT;

-- Recovery scans running rows needing classification; partial index keeps it cheap.
CREATE INDEX IF NOT EXISTS idx_execution_processes_resume_state
    ON execution_processes(resume_state)
    WHERE status = 'running';
