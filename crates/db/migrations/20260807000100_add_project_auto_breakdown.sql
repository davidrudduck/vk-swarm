-- P3 auto-trigger opt-in (default OFF; SC5 requires unchanged behaviour when disabled).
ALTER TABLE projects ADD COLUMN auto_breakdown_enabled INTEGER NOT NULL DEFAULT 0;
