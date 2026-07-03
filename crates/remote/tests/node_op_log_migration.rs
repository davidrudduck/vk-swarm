//! Asserts the node_op_log table + its dedup contract exist after migration.
use sqlx::PgPool;

// Inlined verbatim from crates/remote/tests/backfill_e2e.rs (no shared `common` module exists).
fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}
macro_rules! skip_without_db {
    () => {
        if database_url().is_none() {
            eprintln!("Skipping: DATABASE_URL not set");
            return;
        }
    };
}
async fn create_pool() -> PgPool {
    PgPool::connect(&database_url().unwrap())
        .await
        .expect("connect")
}

#[tokio::test]
async fn node_op_log_table_and_pk_exist() {
    skip_without_db!(); // Trap 2b: a real migrated PG MUST be set or this is a hollow pass
    let pool = create_pool().await;

    // Table exists with the dedup PRIMARY KEY (node_id, idempotency_key): inserting the same key twice
    // for the same node conflicts; ON CONFLICT DO NOTHING is what 106 relies on.
    let node_id = uuid::Uuid::new_v4();
    let entity_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO node_op_log (node_id, idempotency_key, seq, op_type, entity_id) \
         VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(node_id)
    .bind("task:1:1")
    .bind(1_i64)
    .bind("task.upsert")
    .bind(entity_id)
    .execute(&pool)
    .await
    .unwrap();

    let affected = sqlx::query(
        "INSERT INTO node_op_log (node_id, idempotency_key, seq, op_type, entity_id) \
         VALUES ($1,$2,$3,$4,$5) ON CONFLICT (node_id, idempotency_key) DO NOTHING",
    )
    .bind(node_id)
    .bind("task:1:1")
    .bind(1_i64)
    .bind("task.upsert")
    .bind(entity_id)
    .execute(&pool)
    .await
    .unwrap()
    .rows_affected();
    assert_eq!(
        affected, 0,
        "duplicate (node_id, idempotency_key) must be deduped"
    );

    // High-water = max(seq) per node.
    let hw: Option<i64> = sqlx::query_scalar("SELECT MAX(seq) FROM node_op_log WHERE node_id = $1")
        .bind(node_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(hw, Some(1));
}
