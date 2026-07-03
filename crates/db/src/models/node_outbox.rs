use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct OutboxOp {
    pub id: Uuid,
    pub seq: i64,
    pub op_type: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub payload: serde_json::Value,
    pub idempotency_key: String,
    pub fencing_token: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub acked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct NewOutboxOp {
    pub op_type: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub payload: serde_json::Value,
    pub idempotency_key: String,
    pub fencing_token: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct OutboxOpRow {
    pub id: Uuid,
    pub seq: i64,
    pub op_type: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub payload: String,
    pub idempotency_key: String,
    pub fencing_token: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub acked_at: Option<DateTime<Utc>>,
}

impl From<OutboxOpRow> for OutboxOp {
    fn from(r: OutboxOpRow) -> Self {
        let payload = match serde_json::from_str(&r.payload) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    op_id = %r.id,
                    seq = r.seq,
                    idempotency_key = %r.idempotency_key,
                    error = %e,
                    "node_outbox: payload JSON decode failed — substituting Value::Null"
                );
                serde_json::Value::Null
            }
        };
        OutboxOp {
            id: r.id,
            seq: r.seq,
            op_type: r.op_type,
            entity_type: r.entity_type,
            entity_id: r.entity_id,
            payload,
            idempotency_key: r.idempotency_key,
            fencing_token: r.fencing_token,
            created_at: r.created_at,
            acked_at: r.acked_at,
        }
    }
}

pub struct OutboxRepository;

impl OutboxRepository {
    pub async fn enqueue_op(pool: &SqlitePool, op: NewOutboxOp) -> Result<OutboxOp, sqlx::Error> {
        let id = Uuid::new_v4();
        let payload = serde_json::to_string(&op.payload)
            .map_err(|e| sqlx::Error::Protocol(format!("failed to serialize payload: {e}")))?;

        let row = sqlx::query_as::<_, OutboxOpRow>(
            r#"INSERT INTO node_outbox (id, seq, op_type, entity_type, entity_id, payload, idempotency_key, fencing_token)
               VALUES (?, (SELECT COALESCE(MAX(seq),0)+1 FROM node_outbox), ?, ?, ?, ?, ?, ?)
               RETURNING id, seq, op_type, entity_type, entity_id, payload, idempotency_key, fencing_token, created_at, acked_at"#,
        )
        .bind(id)
        .bind(op.op_type)
        .bind(op.entity_type)
        .bind(op.entity_id)
        .bind(payload)
        .bind(op.idempotency_key)
        .bind(op.fencing_token)
        .fetch_one(pool)
        .await?;

        Ok(OutboxOp::from(row))
    }

    pub async fn peek_unacked(pool: &SqlitePool, limit: i64) -> Result<Vec<OutboxOp>, sqlx::Error> {
        let rows = sqlx::query_as::<_, OutboxOpRow>(
            r#"SELECT id, seq, op_type, entity_type, entity_id, payload, idempotency_key, fencing_token, created_at, acked_at
               FROM node_outbox
              WHERE acked_at IS NULL
              ORDER BY seq ASC
              LIMIT ?"#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(OutboxOp::from).collect())
    }

    pub async fn mark_acked_through(pool: &SqlitePool, seq: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE node_outbox
                 SET acked_at = datetime('now','subsec')
               WHERE acked_at IS NULL AND seq <= ?"#,
        )
        .bind(seq)
        .execute(pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_pool;

    #[tokio::test]
    async fn outbox_enqueue_peek_ack_roundtrip() {
        let (pool, _tmp) = create_test_pool().await;

        let mk = |k: &str| NewOutboxOp {
            op_type: "task.upsert".into(),
            entity_type: "task".into(),
            entity_id: uuid::Uuid::new_v4(),
            payload: serde_json::json!({"title": "x"}),
            idempotency_key: k.into(),
            fencing_token: None,
        };

        let a = OutboxRepository::enqueue_op(&pool, mk("task:a:1"))
            .await
            .unwrap();
        let b = OutboxRepository::enqueue_op(&pool, mk("task:b:1"))
            .await
            .unwrap();
        assert!(b.seq > a.seq, "seq must be per-node monotonic");

        let unacked = OutboxRepository::peek_unacked(&pool, 10).await.unwrap();
        assert_eq!(unacked.len(), 2);
        assert_eq!(unacked[0].seq, a.seq, "peek_unacked is seq-ordered");
        assert_eq!(unacked[1].seq, b.seq);

        // Ack through the first op's seq → only b remains unacked.
        OutboxRepository::mark_acked_through(&pool, a.seq)
            .await
            .unwrap();
        let remaining = OutboxRepository::peek_unacked(&pool, 10).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].seq, b.seq);
    }

    #[tokio::test]
    async fn enqueue_op_rejects_duplicate_idempotency_key() {
        let (pool, _tmp) = create_test_pool().await;
        let op = NewOutboxOp {
            op_type: "task.upsert".into(),
            entity_type: "task".into(),
            entity_id: uuid::Uuid::new_v4(),
            payload: serde_json::json!({}),
            idempotency_key: "task:dup:1".into(),
            fencing_token: None,
        };
        OutboxRepository::enqueue_op(&pool, op.clone())
            .await
            .unwrap();
        assert!(
            OutboxRepository::enqueue_op(&pool, op).await.is_err(),
            "UNIQUE(idempotency_key) must reject a re-enqueue"
        );
    }
}
