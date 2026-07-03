//! Migration test for lease + fencing-token columns (task 201).
//!
//! Requires a PostgreSQL database at DATABASE_URL with migrations applied.

use sqlx::PgPool;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}

macro_rules! skip_without_db {
    () => {
        if database_url().is_none() {
            eprintln!("Skipping test: DATABASE_URL not set");
            return;
        }
    };
}

async fn create_pool() -> PgPool {
    let url = database_url().expect("DATABASE_URL must be set");
    PgPool::connect(&url)
        .await
        .expect("Failed to connect to database")
}

#[tokio::test]
async fn lease_columns_and_token_sequence_exist() {
    skip_without_db!();

    let pool = create_pool().await;

    sqlx::query("SELECT lease_expires_at, fencing_token FROM node_task_assignments LIMIT 0")
        .fetch_optional(&pool)
        .await
        .expect("columns exist");

    let column_default: (Option<String>,) = sqlx::query_as(
        "SELECT column_default FROM information_schema.columns \
         WHERE table_name = 'node_task_assignments' AND column_name = 'fencing_token'",
    )
    .fetch_one(&pool)
    .await
    .expect("fencing_token column info");
    let default_value = column_default.0.expect("fencing_token should have a default");
    assert!(
        default_value.trim_start_matches('(').trim_end_matches(')') == "0",
        "fencing_token default should be 0: {default_value}"
    );

    let first: (i64,) = sqlx::query_as("SELECT nextval('node_fencing_token_seq')")
        .fetch_one(&pool)
        .await
        .expect("first nextval");
    let second: (i64,) = sqlx::query_as("SELECT nextval('node_fencing_token_seq')")
        .fetch_one(&pool)
        .await
        .expect("second nextval");
    assert!(second.0 > first.0, "sequence must be strictly increasing");
}
