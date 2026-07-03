-- Lease + fencing-token columns on node_task_assignments, and the per-hive monotonic token source
-- (CONTRACT §B / ADR-0009 SC3). The lease is the atomic-checkout expiry; the fencing_token is the
-- partition-safety mechanism — every grant bumps it via nextval, and a stale op (token < the
-- assignment's current token) is rejected by the hive (task 205). Pre-existing rows default to token 0.
ALTER TABLE node_task_assignments
    ADD COLUMN lease_expires_at TIMESTAMPTZ,
    ADD COLUMN fencing_token    BIGINT NOT NULL DEFAULT 0;

-- Per-hive monotonic, strictly-increasing fencing token source. try_claim / renew (203) and the
-- expiry sweep (209) call nextval('node_fencing_token_seq') so a reassigned lease ALWAYS gets a
-- strictly higher token than any prior holder — the basis of stale-token rejection (205).
CREATE SEQUENCE node_fencing_token_seq AS BIGINT START WITH 1 INCREMENT BY 1;
