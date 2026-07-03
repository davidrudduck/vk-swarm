//! Service for syncing local task attempts, execution processes, and logs to the Hive.
//!
//! This module provides a background sync service that periodically finds unsynced
//! entities in the local database and sends them to the Hive via WebSocket.
//!
//! # Sync Strategy
//!
//! The sync service uses the following strategy:
//! 1. **Task Sync**: First sync tasks (to get shared_task_id for attempts)
//! 2. **Attempt Sync**: Sync task attempts, as they are the parent entities
//! 3. **Execution Sync**: Sync execution processes, which belong to attempts
//! 4. **Log Batch Sync**: Finally sync log entries in batches, grouped by execution
//!
//! Entities are marked with `hive_synced_at` timestamp after successful sync.
//! On reconnection, all unsynced entities are retried.
//!
//! # Labels
//!
//! Labels are NOT synced from nodes to hive. Labels flow in one direction only:
//! from hive down to nodes. They are sent in the auth response and via broadcast
//! messages when updated on the hive.

use std::collections::HashMap;
use std::time::Duration;

use db::models::execution_process::ExecutionProcess;
use db::models::label::Label;
use db::models::log_entry::DbLogEntry;
use db::models::project::Project;
use db::models::task::Task;
use db::models::task_attempt::TaskAttempt;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tokio::time::{self, MissedTickBehavior};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::hive_client::{
    AttemptSyncMessage, DigestEntry, ExecutionSyncMessage, LocalProjectSyncInfo, LogsBatchMessage,
    NodeMessage, OutboxOp, ProjectsSyncMessage, SyncLogEntry, TaskOutputType, TaskSyncMessage,
};

/// Configuration for the Hive sync service.
#[derive(Debug, Clone)]
pub struct HiveSyncConfig {
    /// How often to check for unsynced entities
    pub sync_interval: Duration,
    /// Maximum number of tasks to sync in one batch
    pub max_tasks_per_batch: i64,
    /// Maximum number of attempts to sync in one batch
    pub max_attempts_per_batch: i64,
    /// Maximum number of executions to sync in one batch
    pub max_executions_per_batch: i64,
    /// Maximum number of log entries to sync in one batch
    pub max_logs_per_batch: i64,
}

impl Default for HiveSyncConfig {
    /// Creates a HiveSyncConfig populated with sensible defaults for the Hive sync service.
    ///
    /// Defaults:
    /// - `sync_interval`: 30 seconds
    /// - `max_tasks_per_batch`: 50
    /// - `max_attempts_per_batch`: 50
    /// - `max_executions_per_batch`: 100
    /// - `max_logs_per_batch`: 500
    ///
    /// # Examples
    ///
    /// ```
    /// let cfg = HiveSyncConfig::default();
    /// assert_eq!(cfg.sync_interval, std::time::Duration::from_secs(30));
    /// assert_eq!(cfg.max_tasks_per_batch, 50);
    /// assert_eq!(cfg.max_attempts_per_batch, 50);
    /// assert_eq!(cfg.max_executions_per_batch, 100);
    /// assert_eq!(cfg.max_logs_per_batch, 500);
    /// ```
    fn default() -> Self {
        Self {
            // 30 seconds is appropriate for project/task sync
            // Active execution (logs/attempts) happens more frequently via direct messages
            sync_interval: Duration::from_secs(30),
            max_tasks_per_batch: 50,
            max_attempts_per_batch: 50,
            max_executions_per_batch: 100,
            max_logs_per_batch: 500,
        }
    }
}

/// Errors from the Hive sync service.
#[derive(Debug, thiserror::Error)]
pub enum HiveSyncError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("send error: {0}")]
    Send(String),
    #[error("task not found for attempt: {0}")]
    TaskNotFound(Uuid),
    #[error("task not linked to Hive: {0}")]
    TaskNotLinked(Uuid),
}

/// Service for syncing local entities to the Hive.
pub struct HiveSyncService {
    pool: SqlitePool,
    command_tx: mpsc::Sender<NodeMessage>,
    config: HiveSyncConfig,
    /// Optional node runner state, attached only in the running node so outbox ops for
    /// hive-assigned tasks are stamped with the current lease fencing token (SC3).
    node_state:
        Option<std::sync::Arc<tokio::sync::RwLock<crate::services::node_runner::NodeRunnerState>>>,
}

impl HiveSyncService {
    /// Create a new Hive sync service.
    pub fn new(
        pool: SqlitePool,
        command_tx: mpsc::Sender<NodeMessage>,
        config: HiveSyncConfig,
    ) -> Self {
        Self {
            pool,
            command_tx,
            config,
            node_state: None,
        }
    }

    /// Attach the node runner state so outbox ops against hive-assigned tasks can be stamped with the
    /// current fencing token (SC3). Without it, ops pass through unstamped (tracer/back-compat).
    pub fn with_node_state(
        mut self,
        state: std::sync::Arc<tokio::sync::RwLock<crate::services::node_runner::NodeRunnerState>>,
    ) -> Self {
        self.node_state = Some(state);
        self
    }

    /// Run the sync service in a loop.
    ///
    /// This spawns a background task that periodically syncs unsynced entities.
    /// The task runs until the channel is closed.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = time::interval(self.config.sync_interval);
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                interval.tick().await;

                if let Err(e) = self.sync_once().await {
                    warn!(error = ?e, "Hive sync cycle failed");
                }
            }
        })
    }

    /// Perform one sync cycle.
    ///
    /// This syncs local projects first (so hive knows available projects), then
    /// tasks (to get shared_task_id), then attempts, executions, and logs in order.
    ///
    /// Note: Labels are NOT synced from nodes to hive. Labels are managed centrally
    /// on the hive and synced DOWN to nodes via the auth response and broadcast messages.
    pub async fn sync_once(&self) -> Result<(), HiveSyncError> {
        // Sync local projects first (so hive knows what's available on this node)
        if let Err(e) = self.sync_local_projects().await {
            // Log but don't fail the entire sync - other syncs can continue
            warn!(error = ?e, "Failed to sync local projects");
        }

        // Sync tasks first (to ensure shared_task_id exists for attempts)
        let tasks_synced = self.sync_tasks().await?;
        if tasks_synced > 0 {
            debug!(count = tasks_synced, "Synced tasks to Hive");
        }

        // Sync attempts next (parent entities for executions)
        let attempts_synced = self.sync_attempts().await?;
        if attempts_synced > 0 {
            debug!(count = attempts_synced, "Synced task attempts to Hive");
        }

        // Sync executions next
        let executions_synced = self.sync_executions().await?;
        if executions_synced > 0 {
            debug!(
                count = executions_synced,
                "Synced execution processes to Hive"
            );
        }

        // Sync logs
        let logs_synced = self.sync_logs().await?;
        if logs_synced > 0 {
            debug!(count = logs_synced, "Synced log entries to Hive");
        }

        // Labels are NOT synced from nodes to hive - they flow hive->nodes only

        // Drain the node_outbox op-log (SC2 tracer): send unacked ops in seq order as a single
        // OpBatch. Does NOT mark them acked — the cursor advances only on the hive's durable OpAck
        // (task 108). Runs ALONGSIDE the legacy sync above (additive; hive apply is idempotent).
        if let Err(e) = self.sync_outbox().await {
            warn!(error = ?e, "Failed to drain node_outbox op-log");
        }

        // Emit the anti-entropy digest (SC5): a per-entity version snapshot the hive compares against
        // its own state to detect silent divergence the ack cursor misses, then replies DigestResult
        // (503). Read-only — does NOT mark anything synced/acked. Runs every cycle (so "on reconnect" =
        // the first cycle after the channel is re-established); the heal is applied by 504.
        if let Err(e) = self.sync_digest().await {
            warn!(error = ?e, "Failed to send anti-entropy digest");
        }

        Ok(())
    }

    /// Drain unacked node_outbox ops and push them to the hive as one ordered `OpBatch`.
    /// Best-effort: an empty outbox sends nothing. Does NOT advance the ack cursor (108 owns that).
    async fn sync_outbox(&self) -> Result<(), HiveSyncError> {
        use db::models::node_outbox::OutboxRepository;
        let rows =
            OutboxRepository::peek_unacked(&self.pool, self.config.max_tasks_per_batch).await?;
        if rows.is_empty() {
            return Ok(());
        }
        // Build a local_task_id -> fencing_token lookup from active assignments, if the node runner
        // state is attached. Only task-type ops are mapped; the tracer is task-only today.
        let token_by_task: Option<HashMap<Uuid, i64>> = if let Some(state) = &self.node_state {
            let s = state.read().await;
            Some(
                s.active_assignments
                    .values()
                    .filter_map(|a| Some((a.local_task_id?, a.fencing_token?)))
                    .collect(),
            )
        } else {
            None
        };
        let ops: Vec<OutboxOp> = rows
            .into_iter()
            .map(|r| OutboxOp {
                seq: r.seq,
                op_type: r.op_type.clone(),
                entity_type: r.entity_type.clone(),
                entity_id: r.entity_id,
                payload: r.payload,
                idempotency_key: r.idempotency_key,
                fencing_token: if r.entity_type == "task" {
                    token_by_task
                        .as_ref()
                        .and_then(|m| m.get(&r.entity_id).copied())
                } else {
                    r.fencing_token
                },
            })
            .collect();
        self.command_tx
            .send(NodeMessage::OpBatch { ops })
            .await
            .map_err(|e| HiveSyncError::Send(e.to_string()))?;
        Ok(())
    }

    /// Build and push the SC5 anti-entropy digest: one `DigestEntry` per swarm-linked task
    /// (`shared_task_id IS NOT NULL`) carrying its `remote_version`. Best-effort, read-only; an empty
    /// set sends nothing. Does NOT advance any cursor (it is divergence DETECTION, not sync).
    async fn sync_digest(&self) -> Result<(), HiveSyncError> {
        use db::models::task::Task;
        let rows = Task::find_digest_entries(&self.pool).await?;
        if rows.is_empty() {
            return Ok(());
        }
        let entries: Vec<DigestEntry> = rows
            .into_iter()
            .map(|r| DigestEntry {
                entity_type: "task".to_string(),
                entity_id: r.id,
                version: r.remote_version,
            })
            .collect();
        self.command_tx
            .send(NodeMessage::Digest { entries })
            .await
            .map_err(|e| HiveSyncError::Send(e.to_string()))?;
        Ok(())
    }

    /// Synchronizes local tasks that require a Hive-side shared_task_id.
    ///
    /// This finds tasks that:
    /// 1. Don't have a shared_task_id (new tasks)
    /// 2. Have shared_task_id but need resync (remote_last_synced_at IS NULL)
    /// 3. Belong to local projects (not remote) with a remote_project_id (linked to swarm)
    ///
    /// This includes:
    /// - Tasks created before the project was linked to swarm
    /// - Tasks that failed initial sync
    /// - Tasks without any attempts yet
    /// - Tasks marked for force resync via mark_for_resync_by_project
    ///
    /// # Returns
    ///
    /// The number of tasks for which a `TaskSync` message was sent.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // `service` is an instance of `HiveSyncService`.
    /// # async fn example(service: &crate::hive::HiveSyncService) {
    /// let synced = service.sync_tasks().await.unwrap();
    /// println!("sent {} task sync messages", synced);
    /// # }
    /// ```
    async fn sync_tasks(&self) -> Result<usize, HiveSyncError> {
        // Find ALL tasks in swarm-linked projects that are missing shared_task_id
        // This captures tasks created before project was linked, failed syncs, etc.
        let mut tasks =
            Task::find_missing_shared_task_id(&self.pool, self.config.max_tasks_per_batch).await?;

        // Also find tasks that have shared_task_id but need resync (force resync scenario)
        let resync_tasks =
            Task::find_needing_resync(&self.pool, self.config.max_tasks_per_batch).await?;
        tasks.extend(resync_tasks);

        if tasks.is_empty() {
            return Ok(0);
        }

        let mut synced_count = 0;

        for task in &tasks {
            // Look up the project to check if it's linked to swarm
            let project = match Project::find_by_id(&self.pool, task.project_id).await? {
                Some(p) => p,
                None => {
                    debug!(
                        task_id = %task.id,
                        project_id = %task.project_id,
                        "Skipping task sync - project not found"
                    );
                    continue;
                }
            };

            // Skip if project isn't linked to swarm (remote_project_id is set when linked)
            if project.remote_project_id.is_none() {
                debug!(
                    task_id = %task.id,
                    project_id = %task.project_id,
                    "Skipping task sync - project not linked to swarm"
                );
                continue;
            }

            // Serialize status to string
            let status = serde_json::to_value(&task.status)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "todo".to_string());

            // Fetch labels for this task and collect their shared_label_ids
            // Only include labels that have been synced to the hive (have shared_label_id)
            let label_ids: Vec<Uuid> = match Label::find_by_task_id(&self.pool, task.id).await {
                Ok(labels) => labels
                    .into_iter()
                    .filter_map(|l| l.shared_label_id)
                    .collect(),
                Err(e) => {
                    debug!(
                        task_id = %task.id,
                        error = ?e,
                        "Failed to fetch labels for task, syncing without labels"
                    );
                    Vec::new()
                }
            };

            // Send local_project_id - the hive looks up swarm_project_id via node_local_projects
            // Owner fields are set by the hive based on which node sent the message
            let message = TaskSyncMessage {
                local_task_id: task.id,
                shared_task_id: task.shared_task_id,
                local_project_id: task.project_id, // Send LOCAL project ID, not remote
                title: task.title.clone(),
                description: task.description.clone(),
                status,
                version: 1, // Initial version for new sync
                is_update: task.shared_task_id.is_some(),
                // Owner fields are set by the hive based on which node sent the sync message
                owner_node_id: None,
                owner_name: None,
                created_at: task.created_at,
                updated_at: task.updated_at,
                label_ids,
                // Assignee fields from remote_assignee_* (synced from hive)
                assignee_user_id: task.remote_assignee_user_id,
                assignee_name: task.remote_assignee_name.clone(),
                assignee_username: task.remote_assignee_username.clone(),
            };

            if let Err(e) = self.command_tx.send(NodeMessage::TaskSync(message)).await {
                error!(error = ?e, task_id = %task.id, "Failed to send task sync");
                return Err(HiveSyncError::Send(e.to_string()));
            }

            synced_count += 1;

            info!(
                task_id = %task.id,
                title = %task.title,
                local_project_id = %task.project_id,
                "Sent task sync to Hive"
            );
        }

        Ok(synced_count)
    }

    /// Sync unsynced task attempts to the Hive.
    async fn sync_attempts(&self) -> Result<usize, HiveSyncError> {
        let attempts =
            TaskAttempt::find_unsynced(&self.pool, self.config.max_attempts_per_batch).await?;

        if attempts.is_empty() {
            return Ok(0);
        }

        let mut synced_count = 0;
        let mut synced_ids = Vec::new();

        for attempt in &attempts {
            // Look up the task to get the shared_task_id
            let task = Task::find_by_id(&self.pool, attempt.task_id)
                .await?
                .ok_or(HiveSyncError::TaskNotFound(attempt.task_id))?;

            // Skip if task isn't linked to Hive
            let shared_task_id = match task.shared_task_id {
                Some(id) => id,
                None => {
                    debug!(
                        task_id = %attempt.task_id,
                        "Skipping attempt sync - task not linked to Hive"
                    );
                    continue;
                }
            };

            let message = AttemptSyncMessage {
                attempt_id: attempt.id,
                assignment_id: attempt.hive_assignment_id, // Use stored assignment_id if available
                shared_task_id,
                executor: attempt.executor.clone(),
                executor_variant: None, // TODO: Add executor_variant to TaskAttempt model
                branch: attempt.branch.clone(),
                target_branch: attempt.target_branch.clone(),
                container_ref: attempt.container_ref.clone(),
                worktree_deleted: attempt.worktree_deleted,
                setup_completed_at: attempt.setup_completed_at,
                created_at: attempt.created_at,
                updated_at: attempt.updated_at,
            };

            if let Err(e) = self
                .command_tx
                .send(NodeMessage::AttemptSync(message))
                .await
            {
                error!(error = ?e, attempt_id = %attempt.id, "Failed to send attempt sync");
                return Err(HiveSyncError::Send(e.to_string()));
            }

            synced_ids.push(attempt.id);
            synced_count += 1;
        }

        // Mark all synced attempts
        if !synced_ids.is_empty() {
            TaskAttempt::mark_hive_synced_batch(&self.pool, &synced_ids).await?;
        }

        Ok(synced_count)
    }

    /// Sync unsynced execution processes to the Hive.
    async fn sync_executions(&self) -> Result<usize, HiveSyncError> {
        let executions =
            ExecutionProcess::find_unsynced(&self.pool, self.config.max_executions_per_batch)
                .await?;

        if executions.is_empty() {
            return Ok(0);
        }

        let mut synced_count = 0;
        let mut synced_ids = Vec::new();

        for execution in &executions {
            // Serialize the executor_action to JSON
            let executor_action_json = serde_json::to_value(&execution.executor_action.0)
                .ok()
                .filter(|v| !v.is_null());

            // Serialize run_reason and status to their serde representations
            let run_reason = serde_json::to_value(&execution.run_reason)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "codingagent".to_string());
            let status = serde_json::to_value(&execution.status)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "running".to_string());

            let message = ExecutionSyncMessage {
                execution_id: execution.id,
                attempt_id: execution.task_attempt_id,
                run_reason,
                executor_action: executor_action_json,
                before_head_commit: execution.before_head_commit.clone(),
                after_head_commit: execution.after_head_commit.clone(),
                status,
                exit_code: execution.exit_code.map(|c| c as i32),
                dropped: execution.dropped,
                pid: execution.pid,
                started_at: execution.started_at,
                completed_at: execution.completed_at,
                created_at: execution.created_at,
            };

            if let Err(e) = self
                .command_tx
                .send(NodeMessage::ExecutionSync(message))
                .await
            {
                error!(error = ?e, execution_id = %execution.id, "Failed to send execution sync");
                return Err(HiveSyncError::Send(e.to_string()));
            }

            synced_ids.push(execution.id);
            synced_count += 1;
        }

        // Mark all synced executions
        if !synced_ids.is_empty() {
            ExecutionProcess::mark_hive_synced_batch(&self.pool, &synced_ids).await?;
        }

        Ok(synced_count)
    }

    /// Sync unsynced log entries to the Hive in batches.
    async fn sync_logs(&self) -> Result<usize, HiveSyncError> {
        let logs = DbLogEntry::find_unsynced(&self.pool, self.config.max_logs_per_batch).await?;

        if logs.is_empty() {
            return Ok(0);
        }

        // Group logs by execution_id for batching
        let mut batches: HashMap<Uuid, Vec<&DbLogEntry>> = HashMap::new();
        for log in &logs {
            batches.entry(log.execution_id).or_default().push(log);
        }

        let mut synced_count = 0;
        let mut synced_ids = Vec::new();

        for (execution_id, batch_logs) in batches {
            // Look up the execution process to get the task_attempt_id
            let execution = match ExecutionProcess::find_by_id(&self.pool, execution_id).await? {
                Some(ep) => ep,
                None => {
                    debug!(
                        execution_id = %execution_id,
                        "Skipping log sync - execution process not found"
                    );
                    continue;
                }
            };

            // Look up the task attempt to get hive_assignment_id
            let attempt =
                match TaskAttempt::find_by_id(&self.pool, execution.task_attempt_id).await? {
                    Some(ta) => ta,
                    None => {
                        debug!(
                            execution_id = %execution_id,
                            task_attempt_id = %execution.task_attempt_id,
                            "Skipping log sync - task attempt not found"
                        );
                        continue;
                    }
                };

            // Get the assignment_id and shared_task_id from the attempt
            // For locally-started tasks, use attempt.id as assignment_id and include shared_task_id
            // so the Hive can create a synthetic assignment if needed
            let (assignment_id, shared_task_id) = match attempt.hive_assignment_id {
                Some(id) => {
                    // Hive-dispatched task - use the real assignment_id
                    (id, None)
                }
                None => {
                    // Locally-started task - look up shared_task_id from the task
                    let task = match Task::find_by_id(&self.pool, attempt.task_id).await? {
                        Some(t) => t,
                        None => {
                            debug!(
                                execution_id = %execution_id,
                                task_id = %attempt.task_id,
                                "Skipping log sync - task not found"
                            );
                            continue;
                        }
                    };

                    match task.shared_task_id {
                        Some(shared_id) => {
                            // Use attempt.id as assignment_id (Hive will create synthetic assignment)
                            debug!(
                                execution_id = %execution_id,
                                attempt_id = %attempt.id,
                                shared_task_id = %shared_id,
                                "Syncing logs for locally-started task with shared_task_id"
                            );
                            (attempt.id, Some(shared_id))
                        }
                        None => {
                            // Don't mark as synced - shared_task_id may arrive via TaskSync later.
                            // Logs will be retried on subsequent sync cycles until the task is linked.
                            debug!(
                                execution_id = %execution_id,
                                attempt_id = %attempt.id,
                                "Skipping log sync - no shared_task_id yet (waiting for TaskSync)"
                            );
                            continue;
                        }
                    }
                }
            };

            // Convert logs to sync format
            let entries: Vec<SyncLogEntry> = batch_logs
                .iter()
                .map(|log| SyncLogEntry {
                    output_type: parse_output_type(&log.output_type),
                    content: log.content.clone(),
                    timestamp: log.timestamp,
                })
                .collect();

            let message = LogsBatchMessage {
                assignment_id,
                shared_task_id,
                execution_process_id: Some(execution_id),
                entries,
                compressed: false,
            };

            if let Err(e) = self.command_tx.send(NodeMessage::LogsBatch(message)).await {
                error!(error = ?e, execution_id = %execution_id, "Failed to send logs batch");
                return Err(HiveSyncError::Send(e.to_string()));
            }

            // Collect synced log IDs
            for log in batch_logs {
                synced_ids.push(log.id);
                synced_count += 1;
            }
        }

        // Mark all synced logs
        if !synced_ids.is_empty() {
            DbLogEntry::mark_hive_synced_batch(&self.pool, &synced_ids).await?;
        }

        Ok(synced_count)
    }

    // NOTE: sync_labels has been removed.
    // Labels are now managed centrally on the hive and synced DOWN to nodes.
    // See the auth response and HiveMessage::LabelSync broadcast for the new flow.

    // NOTE: Force resync is handled via database flags (mark_for_resync_by_project).
    // Tasks marked for resync are picked up by sync_tasks() via find_needing_resync().
    // The API endpoint sets the flag, and the sync loop does the actual work.

    /// Sync all local projects to the Hive.
    ///
    /// This sends a snapshot of all local projects to the Hive so it knows
    /// what projects are available on this node for linking to swarm projects.
    /// Called on each sync cycle to keep the Hive's view of node projects current.
    pub async fn sync_local_projects(&self) -> Result<usize, HiveSyncError> {
        // Fetch only local projects (is_remote=false) - don't sync remote projects back to hive
        let projects = Project::find_local_projects(&self.pool).await?;

        if projects.is_empty() {
            return Ok(0);
        }

        // Convert to sync format
        // Note: Project model doesn't have default_branch field, so we use "main" as default
        let project_infos: Vec<LocalProjectSyncInfo> = projects
            .iter()
            .map(|p| LocalProjectSyncInfo {
                local_project_id: p.id,
                name: p.name.clone(),
                git_repo_path: p.git_repo_path.to_string_lossy().into_owned(),
                default_branch: "main".to_string(),
            })
            .collect();

        let count = project_infos.len();

        let message = ProjectsSyncMessage {
            projects: project_infos,
        };

        if let Err(e) = self
            .command_tx
            .send(NodeMessage::ProjectsSync(message))
            .await
        {
            error!(error = ?e, "Failed to send projects sync");
            return Err(HiveSyncError::Send(e.to_string()));
        }

        debug!(count = count, "Synced local projects to Hive");
        Ok(count)
    }
}

/// Parse output type string to TaskOutputType.
fn parse_output_type(s: &str) -> TaskOutputType {
    match s.to_lowercase().as_str() {
        "stdout" => TaskOutputType::Stdout,
        "stderr" => TaskOutputType::Stderr,
        "system" => TaskOutputType::System,
        _ => TaskOutputType::System,
    }
}

/// Spawn the Hive sync service.
///
/// This is a convenience function that creates and spawns the sync service
/// in a background task.
pub fn spawn_hive_sync_service(
    pool: SqlitePool,
    command_tx: mpsc::Sender<NodeMessage>,
    config: Option<HiveSyncConfig>,
    node_state: Option<
        std::sync::Arc<tokio::sync::RwLock<crate::services::node_runner::NodeRunnerState>>,
    >,
) -> tokio::task::JoinHandle<()> {
    let mut service = HiveSyncService::new(pool, command_tx, config.unwrap_or_default());
    if let Some(state) = node_state {
        service = service.with_node_state(state);
    }
    service.spawn()
}

#[cfg(test)]
mod tests {
    use super::HiveSyncConfig;
    use super::HiveSyncService;
    use crate::services::hive_client::{NodeMessage, TaskExecutionStatus};
    use crate::services::node_runner::{ActiveAssignment, NodeRunnerState};

    #[tokio::test]
    async fn sync_outbox_sends_unacked_ops_as_op_batch_in_seq_order() {
        let (pool, _tmp) = db::test_utils::create_test_pool().await;
        use db::models::node_outbox::{NewOutboxOp, OutboxRepository};
        let mk = |k: &str| NewOutboxOp {
            op_type: "task.upsert".into(),
            entity_type: "task".into(),
            entity_id: uuid::Uuid::new_v4(),
            payload: serde_json::json!({}),
            idempotency_key: k.into(),
            fencing_token: None,
        };
        OutboxRepository::enqueue_op(&pool, mk("task:a:1"))
            .await
            .unwrap();
        OutboxRepository::enqueue_op(&pool, mk("task:b:1"))
            .await
            .unwrap();

        let (command_tx, mut command_rx) = tokio::sync::mpsc::channel(8);
        let service = HiveSyncService::new(pool.clone(), command_tx, HiveSyncConfig::default());
        service.sync_outbox().await.unwrap();

        let msg = command_rx.try_recv().expect("an OpBatch was sent");
        match msg {
            NodeMessage::OpBatch { ops } => {
                assert_eq!(ops.len(), 2);
                assert!(ops[1].seq > ops[0].seq, "seq order preserved");
                assert!(ops.iter().all(|o| o.op_type == "task.upsert"));
            }
            other => panic!("expected OpBatch, got {other:?}"),
        }

        assert_eq!(
            OutboxRepository::peek_unacked(&pool, 10)
                .await
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn sync_outbox_stamps_fencing_token_for_hive_assigned_tasks_only() {
        let (pool, _tmp) = db::test_utils::create_test_pool().await;
        use db::models::node_outbox::{NewOutboxOp, OutboxRepository};

        let assigned_task = uuid::Uuid::new_v4();
        let owned_task = uuid::Uuid::new_v4();
        let mk = |tid: uuid::Uuid, k: &str| NewOutboxOp {
            op_type: "task.upsert".into(),
            entity_type: "task".into(),
            entity_id: tid,
            payload: serde_json::json!({}),
            idempotency_key: k.into(),
            fencing_token: None,
        };
        OutboxRepository::enqueue_op(&pool, mk(assigned_task, "task:a:1"))
            .await
            .unwrap();
        OutboxRepository::enqueue_op(&pool, mk(owned_task, "task:b:1"))
            .await
            .unwrap();

        let state = std::sync::Arc::new(tokio::sync::RwLock::new(NodeRunnerState::default()));
        {
            let aid = uuid::Uuid::new_v4();
            state.write().await.active_assignments.insert(
                aid,
                ActiveAssignment {
                    assignment_id: aid,
                    task_id: uuid::Uuid::new_v4(),
                    local_task_id: Some(assigned_task),
                    local_attempt_id: None,
                    status: TaskExecutionStatus::Pending,
                    fencing_token: Some(5),
                    lease_expires_at: Some(chrono::Utc::now() + chrono::Duration::seconds(60)),
                },
            );
        }

        let (command_tx, mut command_rx) = tokio::sync::mpsc::channel(8);
        let service = HiveSyncService::new(pool.clone(), command_tx, HiveSyncConfig::default())
            .with_node_state(state.clone());
        service.sync_outbox().await.unwrap();

        match command_rx.try_recv().expect("an OpBatch was sent") {
            NodeMessage::OpBatch { ops } => {
                let assigned = ops.iter().find(|o| o.entity_id == assigned_task).unwrap();
                let owned = ops.iter().find(|o| o.entity_id == owned_task).unwrap();
                assert_eq!(
                    assigned.fencing_token,
                    Some(5),
                    "hive-assigned op carries the lease token"
                );
                assert_eq!(
                    owned.fencing_token, None,
                    "node-owned op carries no token (CONTRACT §C)"
                );
            }
            other => panic!("expected OpBatch, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn sync_outbox_without_node_state_passes_token_through_unchanged() {
        let (pool, _tmp) = db::test_utils::create_test_pool().await;
        use db::models::node_outbox::{NewOutboxOp, OutboxRepository};
        OutboxRepository::enqueue_op(
            &pool,
            NewOutboxOp {
                op_type: "task.upsert".into(),
                entity_type: "task".into(),
                entity_id: uuid::Uuid::new_v4(),
                payload: serde_json::json!({}),
                idempotency_key: "task:c:1".into(),
                fencing_token: None,
            },
        )
        .await
        .unwrap();
        let (command_tx, mut command_rx) = tokio::sync::mpsc::channel(8);
        HiveSyncService::new(pool.clone(), command_tx, HiveSyncConfig::default())
            .sync_outbox()
            .await
            .unwrap();
        match command_rx.try_recv().unwrap() {
            NodeMessage::OpBatch { ops } => assert_eq!(ops[0].fencing_token, None),
            other => panic!("expected OpBatch, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn sync_digest_sends_one_entry_per_swarm_linked_task() {
        let (pool, _tmp) = db::test_utils::create_test_pool().await;
        use db::models::task::{CreateTask, Task};
        let project_id = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO projects (id, name, git_repo_path) VALUES (?, 'p', '/tmp/p')",
        )
        .bind(project_id)
        .execute(&pool)
        .await
        .unwrap();

        let linked_id = uuid::Uuid::new_v4();
        let linked = Task::create(
            &pool,
            &CreateTask {
                project_id,
                title: "linked".into(),
                description: None,
                status: None,
                parent_task_id: None,
                image_ids: None,
                shared_task_id: Some(uuid::Uuid::new_v4()),
            },
            linked_id,
        )
        .await
        .unwrap();
        sqlx::query("UPDATE tasks SET remote_version = 1 WHERE id = ?")
            .bind(linked.id)
            .execute(&pool)
            .await
            .unwrap();

        let (command_tx, mut command_rx) = tokio::sync::mpsc::channel(8);
        let service = HiveSyncService::new(pool.clone(), command_tx, HiveSyncConfig::default());
        service.sync_digest().await.unwrap();

        let msg = command_rx.try_recv().expect("a Digest was sent");
        match msg {
            NodeMessage::Digest { entries } => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].entity_type, "task");
            }
            other => panic!("expected Digest, got {other:?}"),
        }
    }
}
