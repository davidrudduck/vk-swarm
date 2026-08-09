-- P3 task-breakdown substrate (ADR-0016): proposals are a separate node-local entity, never a
-- TaskStatus variant. Proposals/items never enqueue node_outbox ops and never sync; only
-- acceptance creates real tasks (which sync via the existing task.upsert path).
CREATE TABLE IF NOT EXISTS task_breakdown_proposals (
    id                   BLOB PRIMARY KEY,
    task_id              BLOB NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    status               TEXT NOT NULL DEFAULT 'draft'
                         CHECK (status IN ('draft','accepted','discarded','failed')),
    execution_process_id BLOB REFERENCES execution_processes(id) ON DELETE SET NULL,
    error                TEXT,
    created_at           TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at           TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Review gate invariant: at most ONE reviewable draft per task at a time (409 at the API layer).
CREATE UNIQUE INDEX IF NOT EXISTS idx_breakdown_one_draft_per_task
    ON task_breakdown_proposals(task_id) WHERE status = 'draft';

CREATE TABLE IF NOT EXISTS task_breakdown_proposal_items (
    id                  BLOB PRIMARY KEY,
    proposal_id         BLOB NOT NULL REFERENCES task_breakdown_proposals(id) ON DELETE CASCADE,
    title               TEXT NOT NULL,
    description         TEXT,
    sort_order          INTEGER NOT NULL DEFAULT 0,
    -- JSON array of sibling item ids (within the same proposal) this item depends on.
    depends_on_item_ids TEXT NOT NULL DEFAULT '[]',
    created_at          TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_breakdown_items_proposal ON task_breakdown_proposal_items(proposal_id, sort_order);

-- First-class dependency edges between REAL tasks; written only at acceptance. P5's substrate.
CREATE TABLE IF NOT EXISTS task_dependencies (
    task_id            BLOB NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    depends_on_task_id BLOB NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    created_at         TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    PRIMARY KEY (task_id, depends_on_task_id)
);
CREATE INDEX IF NOT EXISTS idx_task_dependencies_reverse ON task_dependencies(depends_on_task_id);
