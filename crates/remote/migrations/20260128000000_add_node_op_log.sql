-- Hive-side dedup + per-node high-water cursor for the node→hive op-log (SC2c). The hive applies each
-- op idempotently: INSERT … ON CONFLICT (node_id, idempotency_key) DO NOTHING (see 106). The durable
-- ack the node receives (HiveMessage::OpAck) is keyed off MAX(seq) per node = the applied-through cursor.
CREATE TABLE node_op_log (
    node_id         UUID NOT NULL,
    idempotency_key TEXT NOT NULL,
    seq             BIGINT NOT NULL,
    op_type         TEXT NOT NULL,
    entity_id       UUID NOT NULL,
    applied_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (node_id, idempotency_key)
);

-- Per-node high-water lookup: SELECT MAX(seq) WHERE node_id = $1.
CREATE INDEX idx_node_op_log_node_seq ON node_op_log (node_id, seq);