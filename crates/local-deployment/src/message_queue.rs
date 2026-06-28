//! SQLite-backed message queue for storing follow-up messages for the next turn.
//! Messages persist across restarts and are ordered by position within each task attempt.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use sqlx::types::chrono::{DateTime, Utc};
use ts_rs::TS;
use uuid::Uuid;

/// A queued message waiting to be started as a follow-up on the next turn.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct QueuedMessage {
    pub id: Uuid,
    pub task_attempt_id: Uuid,
    pub content: String,
    pub variant: Option<String>,
    pub position: usize,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
}

/// Request to add a message to the queue.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct AddQueuedMessageRequest {
    pub content: String,
    pub variant: Option<String>,
}

/// Request to update a queued message.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct UpdateQueuedMessageRequest {
    pub content: Option<String>,
    pub variant: Option<String>,
}

/// Request to reorder queued messages.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct ReorderQueuedMessagesRequest {
    pub message_ids: Vec<Uuid>,
}

/// SQLite-backed store for queued messages, persisted per task_attempt_id.
#[derive(Debug, Clone)]
pub struct MessageQueueStore {
    pool: SqlitePool,
}

impl MessageQueueStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, task_attempt_id: Uuid) -> Vec<QueuedMessage> {
        let rows = match sqlx::query!(
            r#"SELECT id as "id!: Uuid",
                      task_attempt_id as "task_attempt_id!: Uuid",
                      content, variant, position, created_at as "created_at!: DateTime<Utc>"
               FROM queued_messages
               WHERE task_attempt_id = ?
               ORDER BY position ASC"#,
            task_attempt_id
        )
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(error = ?e, task_attempt_id = ?task_attempt_id, "Failed to list queued messages");
                return vec![];
            }
        };

        rows.into_iter()
            .map(|row| QueuedMessage {
                id: row.id,
                task_attempt_id: row.task_attempt_id,
                content: row.content,
                variant: row.variant,
                position: row.position as usize,
                created_at: row.created_at,
            })
            .collect()
    }

    pub async fn add(
        &self,
        task_attempt_id: Uuid,
        content: String,
        variant: Option<String>,
    ) -> QueuedMessage {
        // Compute position as count of existing messages
        let position = match sqlx::query!(
            "SELECT COUNT(*) as count FROM queued_messages WHERE task_attempt_id = ?",
            task_attempt_id
        )
        .fetch_one(&self.pool)
        .await
        {
            Ok(row) => row.count as usize,
            Err(e) => {
                tracing::error!(error = ?e, task_attempt_id = ?task_attempt_id, "Failed to count queued messages");
                return QueuedMessage {
                    id: Uuid::new_v4(),
                    task_attempt_id,
                    content,
                    variant,
                    position: 0,
                    created_at: Utc::now(),
                };
            }
        };

        let id = Uuid::new_v4();
        let created_at = Utc::now();
        let position_i64 = position as i64;

        if let Err(e) = sqlx::query!(
            "INSERT INTO queued_messages (id, task_attempt_id, content, variant, position, created_at) VALUES (?, ?, ?, ?, ?, ?)",
            id,
            task_attempt_id,
            content,
            variant,
            position_i64,
            created_at
        )
        .execute(&self.pool)
        .await
        {
            tracing::error!(error = ?e, task_attempt_id = ?task_attempt_id, "Failed to insert queued message");
        }

        QueuedMessage {
            id,
            task_attempt_id,
            content,
            variant,
            position,
            created_at,
        }
    }

    pub async fn update(
        &self,
        task_attempt_id: Uuid,
        message_id: Uuid,
        content: Option<String>,
        variant: Option<String>,
    ) -> Option<QueuedMessage> {
        // Check if message exists first
        let existing = match sqlx::query!(
            "SELECT variant FROM queued_messages WHERE id = ? AND task_attempt_id = ?",
            message_id,
            task_attempt_id
        )
        .fetch_optional(&self.pool)
        .await
        {
            Ok(row) => row,
            Err(e) => {
                tracing::error!(error = ?e, message_id = ?message_id, task_attempt_id = ?task_attempt_id, "Failed to fetch message for update");
                return None;
            }
        };

        existing?;

        // Update content if provided
        if let Some(new_content) = content
            && let Err(e) = sqlx::query!(
                "UPDATE queued_messages SET content = ? WHERE id = ? AND task_attempt_id = ?",
                new_content,
                message_id,
                task_attempt_id
            )
            .execute(&self.pool)
            .await
        {
            tracing::error!(error = ?e, message_id = ?message_id, "Failed to update message content");
            return None;
        }

        // Update variant if explicitly provided
        if let Some(v) = variant {
            let variant_value = if v.is_empty() { None } else { Some(v) };
            if let Err(e) = sqlx::query!(
                "UPDATE queued_messages SET variant = ? WHERE id = ? AND task_attempt_id = ?",
                variant_value,
                message_id,
                task_attempt_id
            )
            .execute(&self.pool)
            .await
            {
                tracing::error!(error = ?e, message_id = ?message_id, "Failed to update message variant");
                return None;
            }
        }

        // Re-select and return the updated message
        match sqlx::query!(
            "SELECT id as \"id!: Uuid\", task_attempt_id as \"task_attempt_id!: Uuid\", content, variant, position, created_at as \"created_at!: DateTime<Utc>\" FROM queued_messages WHERE id = ? AND task_attempt_id = ?",
            message_id,
            task_attempt_id
        )
        .fetch_optional(&self.pool)
        .await
        {
            Ok(Some(row)) => Some(QueuedMessage {
                id: row.id,
                task_attempt_id: row.task_attempt_id,
                content: row.content,
                variant: row.variant,
                position: row.position as usize,
                created_at: row.created_at,
            }),
            Ok(None) => None,
            Err(e) => {
                tracing::error!(error = ?e, message_id = ?message_id, "Failed to fetch updated message");
                None
            }
        }
    }

    pub async fn remove(&self, task_attempt_id: Uuid, message_id: Uuid) -> bool {
        // Get the position of the message being removed
        let removed_msg = match sqlx::query!(
            "SELECT position FROM queued_messages WHERE id = ? AND task_attempt_id = ?",
            message_id,
            task_attempt_id
        )
        .fetch_optional(&self.pool)
        .await
        {
            Ok(row) => row,
            Err(e) => {
                tracing::error!(error = ?e, message_id = ?message_id, task_attempt_id = ?task_attempt_id, "Failed to fetch message position");
                return false;
            }
        };

        let Some(msg_row) = removed_msg else {
            return false; // Message not found
        };

        let removed_position = msg_row.position;

        // Delete the message
        let result = match sqlx::query!(
            "DELETE FROM queued_messages WHERE id = ? AND task_attempt_id = ?",
            message_id,
            task_attempt_id
        )
        .execute(&self.pool)
        .await
        {
            Ok(result) => result,
            Err(e) => {
                tracing::error!(error = ?e, message_id = ?message_id, "Failed to delete message");
                return false;
            }
        };

        if result.rows_affected() == 0 {
            return false;
        }

        // Re-pack positions: decrement all positions after the removed one
        if let Err(e) = sqlx::query!(
            "UPDATE queued_messages SET position = position - 1 WHERE task_attempt_id = ? AND position > ?",
            task_attempt_id,
            removed_position
        )
        .execute(&self.pool)
        .await
        {
            tracing::error!(error = ?e, task_attempt_id = ?task_attempt_id, removed_position = removed_position, "Failed to repack positions");
        }

        true
    }

    pub async fn reorder(
        &self,
        task_attempt_id: Uuid,
        message_ids: Vec<Uuid>,
    ) -> Option<Vec<QueuedMessage>> {
        // Fetch all current messages for validation
        let current_messages = match sqlx::query!(
            "SELECT id as \"id!: Uuid\" FROM queued_messages WHERE task_attempt_id = ? ORDER BY position ASC",
            task_attempt_id
        )
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(error = ?e, task_attempt_id = ?task_attempt_id, "Failed to fetch messages for reorder");
                return None;
            }
        };

        // Validate lengths match
        if message_ids.len() != current_messages.len() {
            return None;
        }

        // Validate that the IDs match (as sets)
        let id_set: std::collections::HashSet<Uuid> = message_ids.iter().copied().collect();
        let current_ids: std::collections::HashSet<Uuid> =
            current_messages.iter().map(|m| m.id).collect();
        if id_set != current_ids {
            return None;
        }

        // Use a transaction to update all positions atomically
        let mut tx = match self.pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!(error = ?e, task_attempt_id = ?task_attempt_id, "Failed to begin transaction for reorder");
                return None;
            }
        };

        for (new_position, message_id) in message_ids.iter().enumerate() {
            let new_position_i64 = new_position as i64;
            if let Err(e) = sqlx::query!(
                "UPDATE queued_messages SET position = ? WHERE id = ? AND task_attempt_id = ?",
                new_position_i64,
                message_id,
                task_attempt_id
            )
            .execute(&mut *tx)
            .await
            {
                tracing::error!(error = ?e, message_id = ?message_id, "Failed to update position in reorder");
                let _ = tx.rollback().await;
                return None;
            }
        }

        if let Err(e) = tx.commit().await {
            tracing::error!(error = ?e, task_attempt_id = ?task_attempt_id, "Failed to commit reorder transaction");
            return None;
        }

        // Fetch and return the reordered messages
        let rows = match sqlx::query!(
            "SELECT id as \"id!: Uuid\", task_attempt_id as \"task_attempt_id!: Uuid\", content, variant, position, created_at as \"created_at!: DateTime<Utc>\" FROM queued_messages WHERE task_attempt_id = ? ORDER BY position ASC",
            task_attempt_id
        )
        .fetch_all(&self.pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(error = ?e, task_attempt_id = ?task_attempt_id, "Failed to fetch reordered messages");
                return None;
            }
        };

        Some(
            rows.into_iter()
                .map(|row| QueuedMessage {
                    id: row.id,
                    task_attempt_id: row.task_attempt_id,
                    content: row.content,
                    variant: row.variant,
                    position: row.position as usize,
                    created_at: row.created_at,
                })
                .collect(),
        )
    }

    pub async fn peek_next(&self, task_attempt_id: Uuid) -> Option<QueuedMessage> {
        match sqlx::query!(
            "SELECT id as \"id!: Uuid\", task_attempt_id as \"task_attempt_id!: Uuid\", content, variant, position, created_at as \"created_at!: DateTime<Utc>\" FROM queued_messages WHERE task_attempt_id = ? ORDER BY position ASC LIMIT 1",
            task_attempt_id
        )
        .fetch_optional(&self.pool)
        .await
        {
            Ok(Some(row)) => Some(QueuedMessage {
                id: row.id,
                task_attempt_id: row.task_attempt_id,
                content: row.content,
                variant: row.variant,
                position: row.position as usize,
                created_at: row.created_at,
            }),
            Ok(None) => None,
            Err(e) => {
                tracing::error!(error = ?e, task_attempt_id = ?task_attempt_id, "Failed to peek next message");
                None
            }
        }
    }

    pub async fn pop_next(&self, task_attempt_id: Uuid) -> Option<QueuedMessage> {
        // Fetch the first message
        let first_msg = match sqlx::query!(
            "SELECT id as \"id!: Uuid\", task_attempt_id as \"task_attempt_id!: Uuid\", content, variant, position, created_at as \"created_at!: DateTime<Utc>\" FROM queued_messages WHERE task_attempt_id = ? ORDER BY position ASC LIMIT 1",
            task_attempt_id
        )
        .fetch_optional(&self.pool)
        .await
        {
            Ok(row) => row,
            Err(e) => {
                tracing::error!(error = ?e, task_attempt_id = ?task_attempt_id, "Failed to fetch first message for pop");
                return None;
            }
        };

        let row = first_msg?;

        let message_id = row.id;
        let popped_message = QueuedMessage {
            id: row.id,
            task_attempt_id: row.task_attempt_id,
            content: row.content,
            variant: row.variant,
            position: row.position as usize,
            created_at: row.created_at,
        };

        // Delete the message
        if let Err(e) = sqlx::query!("DELETE FROM queued_messages WHERE id = ?", message_id)
            .execute(&self.pool)
            .await
        {
            tracing::error!(error = ?e, message_id = ?message_id, "Failed to delete popped message");
            return Some(popped_message); // Still return the message even if delete fails
        }

        // Re-pack positions: decrement all positions after position 0
        if let Err(e) = sqlx::query!(
            "UPDATE queued_messages SET position = position - 1 WHERE task_attempt_id = ? AND position > 0",
            task_attempt_id
        )
        .execute(&self.pool)
        .await
        {
            tracing::error!(error = ?e, task_attempt_id = ?task_attempt_id, "Failed to repack positions after pop");
        }

        Some(popped_message)
    }

    pub async fn clear(&self, task_attempt_id: Uuid) {
        if let Err(e) = sqlx::query!(
            "DELETE FROM queued_messages WHERE task_attempt_id = ?",
            task_attempt_id
        )
        .execute(&self.pool)
        .await
        {
            tracing::error!(error = ?e, task_attempt_id = ?task_attempt_id, "Failed to clear queued messages");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::test_utils::create_test_pool;

    /// Helper to seed a task_attempt for testing.
    async fn seed_task_attempt(pool: &SqlitePool) -> Uuid {
        // Create a test project
        let project_id = Uuid::new_v4();
        let now = Utc::now();
        if let Err(e) = sqlx::query!(
            "INSERT INTO projects (id, name, git_repo_path, created_at) VALUES (?, ?, ?, ?)",
            project_id,
            "Test Project",
            "/tmp/test-repo",
            now
        )
        .execute(pool)
        .await
        {
            tracing::error!(error = ?e, "Failed to create test project");
        }

        // Create a test task
        let task_id = Uuid::new_v4();
        let now = Utc::now();
        if let Err(e) = sqlx::query!(
            "INSERT INTO tasks (id, project_id, title, created_at) VALUES (?, ?, ?, ?)",
            task_id,
            project_id,
            "Test Task",
            now
        )
        .execute(pool)
        .await
        {
            tracing::error!(error = ?e, "Failed to create test task");
        }

        // Create a test task_attempt
        let attempt_id = Uuid::new_v4();
        let now = Utc::now();
        if let Err(e) = sqlx::query!(
            "INSERT INTO task_attempts (id, task_id, executor, branch, target_branch, created_at) VALUES (?, ?, ?, ?, ?, ?)",
            attempt_id,
            task_id,
            "CLAUDE_CODE",
            "test-branch",
            "main",
            now
        )
        .execute(pool)
        .await
        {
            tracing::error!(error = ?e, "Failed to create test task_attempt");
        }

        attempt_id
    }

    #[tokio::test]
    async fn test_queue_add_returns_message_with_position_zero() {
        let (pool, _tmp) = create_test_pool().await;
        let store = MessageQueueStore::new(pool.clone());
        let attempt_id = seed_task_attempt(&pool).await;
        let msg = store.add(attempt_id, "hello".to_string(), None).await;
        assert_eq!(msg.position, 0);
    }

    #[tokio::test]
    async fn test_queue_pop_next_returns_first_and_removes() {
        let (pool, _tmp) = create_test_pool().await;
        let store = MessageQueueStore::new(pool.clone());
        let attempt_id = seed_task_attempt(&pool).await;
        store.add(attempt_id, "first".to_string(), None).await;
        store.add(attempt_id, "second".to_string(), None).await;
        let popped = store.pop_next(attempt_id).await.unwrap();
        assert_eq!(popped.content, "first");
        let remaining = store.list(attempt_id).await;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].position, 0);
    }

    #[tokio::test]
    async fn test_queue_peek_next_returns_first_without_removing() {
        let (pool, _tmp) = create_test_pool().await;
        let store = MessageQueueStore::new(pool.clone());
        let attempt_id = seed_task_attempt(&pool).await;
        store.add(attempt_id, "first".to_string(), None).await;
        store.add(attempt_id, "second".to_string(), None).await;

        let peeked = store.peek_next(attempt_id).await.unwrap();
        assert_eq!(peeked.content, "first");

        let remaining = store.list(attempt_id).await;
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0].content, "first");
        assert_eq!(remaining[0].position, 0);
        assert_eq!(remaining[1].content, "second");
        assert_eq!(remaining[1].position, 1);
    }

    #[tokio::test]
    async fn test_update_preserves_variant_when_not_provided() {
        let (pool, _tmp) = create_test_pool().await;
        let store = MessageQueueStore::new(pool.clone());
        let attempt_id = seed_task_attempt(&pool).await;
        let msg = store
            .add(attempt_id, "content".to_string(), Some("plan".to_string()))
            .await;

        // Update only content, not variant (variant is None)
        let updated = store
            .update(attempt_id, msg.id, Some("new content".to_string()), None)
            .await
            .unwrap();

        // Variant should be preserved
        assert_eq!(updated.variant, Some("plan".to_string()));
        assert_eq!(updated.content, "new content");
    }

    #[tokio::test]
    async fn test_update_clears_variant_when_empty_string() {
        let (pool, _tmp) = create_test_pool().await;
        let store = MessageQueueStore::new(pool.clone());
        let attempt_id = seed_task_attempt(&pool).await;
        let msg = store
            .add(attempt_id, "content".to_string(), Some("plan".to_string()))
            .await;

        // Explicitly clear variant by passing empty string
        let updated = store
            .update(attempt_id, msg.id, None, Some("".to_string()))
            .await
            .unwrap();

        // Variant should be cleared
        assert_eq!(updated.variant, None);
        assert_eq!(updated.content, "content");
    }

    #[tokio::test]
    async fn test_update_changes_variant_when_provided() {
        let (pool, _tmp) = create_test_pool().await;
        let store = MessageQueueStore::new(pool.clone());
        let attempt_id = seed_task_attempt(&pool).await;
        let msg = store
            .add(attempt_id, "content".to_string(), Some("plan".to_string()))
            .await;

        // Change variant to a new value
        let updated = store
            .update(attempt_id, msg.id, None, Some("research".to_string()))
            .await
            .unwrap();

        // Variant should be updated
        assert_eq!(updated.variant, Some("research".to_string()));
    }

    #[tokio::test]
    async fn test_update_adds_variant_when_previously_none() {
        let (pool, _tmp) = create_test_pool().await;
        let store = MessageQueueStore::new(pool.clone());
        let attempt_id = seed_task_attempt(&pool).await;
        let msg = store.add(attempt_id, "content".to_string(), None).await;

        // Add a variant where there was none
        let updated = store
            .update(attempt_id, msg.id, None, Some("plan".to_string()))
            .await
            .unwrap();

        // Variant should be set
        assert_eq!(updated.variant, Some("plan".to_string()));
    }

    #[tokio::test]
    async fn test_queue_persists_across_store_recreation() {
        let (pool, _tmp) = create_test_pool().await;
        // a task_attempt row must exist for the FK
        let attempt_id = seed_task_attempt(&pool).await;
        let store = MessageQueueStore::new(pool.clone());
        store.add(attempt_id, "first".to_string(), None).await;
        store.add(attempt_id, "second".to_string(), None).await;

        // Simulate a restart: drop the store, build a NEW one over the same pool.
        drop(store);
        let store2 = MessageQueueStore::new(pool.clone());
        let remaining = store2.list(attempt_id).await;
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0].content, "first");
        assert_eq!(remaining[0].position, 0);
        assert_eq!(remaining[1].position, 1);

        let popped = store2.pop_next(attempt_id).await.unwrap();
        assert_eq!(popped.content, "first");
        assert_eq!(store2.list(attempt_id).await[0].position, 0); // reindexed
    }
}
