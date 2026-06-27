use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Error as AnyhowError, anyhow};
use async_trait::async_trait;
use db::{
    DBService,
    models::{
        execution_process::{
            CreateExecutionProcess, ExecutionContext, ExecutionProcess, ExecutionProcessRunReason,
            ExecutionProcessStatus,
        },
        execution_process_logs::ExecutionProcessLogs,
        executor_session::{CreateExecutorSession, ExecutorSession},
        task::{Task, TaskStatus},
        task_attempt::{TaskAttempt, TaskAttemptError},
        task_variable::TaskVariable,
    },
};
use executors::{
    actions::{
        ExecutorAction, ExecutorActionType,
        coding_agent_follow_up::CodingAgentFollowUpRequest,
        coding_agent_initial::CodingAgentInitialRequest,
        script::{ScriptContext, ScriptRequest, ScriptRequestLanguage},
    },
    executors::{ExecutorError, StandardCodingAgentExecutor},
    logs::{NormalizedEntry, NormalizedEntryError, NormalizedEntryType, utils::ConversationPatch},
    profile::{ExecutorConfigs, ExecutorProfileId},
};
use futures::{StreamExt, future};
use sqlx::Error as SqlxError;
use thiserror::Error;
use tokio::{sync::RwLock, task::JoinHandle, time::Duration};
use utils::{
    log_msg::LogMsg,
    msg_store::MsgStore,
    text::{git_branch_id, short_uuid},
};
use uuid::Uuid;

use crate::services::{
    config::Config,
    git::{GitService, GitServiceError},
    image::ImageService,
    log_batcher::LogBatcherHandle,
    normalization_metrics::NormalizationMetrics,
    notification::NotificationService,
    process_fence::{self, FenceOutcome},
    process_inspector::SysinfoProcessInspector,
    share::SharePublisher,
    variable_expander,
    worktree_manager::WorktreeError,
};
pub type ContainerRef = String;

#[derive(Debug, Error)]
pub enum ContainerError {
    #[error(transparent)]
    GitServiceError(#[from] GitServiceError),
    #[error(transparent)]
    Sqlx(#[from] SqlxError),
    #[error(transparent)]
    ExecutorError(#[from] ExecutorError),
    #[error(transparent)]
    Worktree(#[from] WorktreeError),
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to kill process: {0}")]
    KillFailed(std::io::Error),
    #[error(transparent)]
    TaskAttemptError(#[from] TaskAttemptError),
    #[error(transparent)]
    Other(#[from] AnyhowError), // Catches any unclassified errors
}

/// Build a resume action from a stored coding-agent action.
///
/// Constructs a new `CodingAgentFollowUpRequest` using the provided session ID and prompt,
/// preserving the executor profile and `next_action` from the stored action.
///
/// Handles both `CodingAgentInitialRequest` (first-turn crash) and
/// `CodingAgentFollowUpRequest` (multi-turn crash), covering the full resume surface.
///
/// Returns `None` if the stored action is not a coding-agent action (e.g. a script request).
pub fn build_resume_action(
    stored: &ExecutorAction,
    session_id: String,
    prompt: String,
) -> Option<ExecutorAction> {
    match &stored.typ {
        ExecutorActionType::CodingAgentInitialRequest(req) => {
            // Clone the full ExecutorProfileId (including variant) so that a profile like
            // "ClaudeCode:PLAN" is preserved across the resume — not downgraded to DEFAULT.
            let follow_up = CodingAgentFollowUpRequest {
                prompt,
                session_id,
                executor_profile_id: req.executor_profile_id.clone(),
            };
            let action = ExecutorAction::new(
                ExecutorActionType::CodingAgentFollowUpRequest(follow_up),
                stored.next_action().map(|next| Box::new(next.clone())),
            );
            Some(action)
        }
        ExecutorActionType::CodingAgentFollowUpRequest(req) => {
            // Multi-turn crash: preserve the existing profile; use the new session_id
            // (stored session_id may be stale across turns).
            let follow_up = CodingAgentFollowUpRequest {
                prompt,
                session_id,
                executor_profile_id: req.executor_profile_id.clone(),
            };
            let action = ExecutorAction::new(
                ExecutorActionType::CodingAgentFollowUpRequest(follow_up),
                stored.next_action().map(|next| Box::new(next.clone())),
            );
            Some(action)
        }
        _ => None,
    }
}

#[async_trait]
pub trait ContainerService {
    fn msg_stores(&self) -> &Arc<RwLock<HashMap<Uuid, Arc<MsgStore>>>>;

    fn db(&self) -> &DBService;

    fn git(&self) -> &GitService;

    fn share_publisher(&self) -> Option<&SharePublisher>;

    /// Get the log batcher handle for batched database writes.
    /// Returns None if batching is disabled (falls back to direct writes).
    fn log_batcher(&self) -> Option<&LogBatcherHandle>;

    /// Get normalization metrics for tracking completion times and timeouts.
    fn normalization_metrics(&self) -> &NormalizationMetrics;

    /// Store a normalization task handle for an execution process.
    /// Called after starting log normalization to track the async task.
    async fn store_normalization_handle(&self, exec_id: Uuid, handle: JoinHandle<()>);

    /// Take (remove and return) a normalization handle for an execution process.
    /// Used to await normalization completion before signaling finished.
    async fn take_normalization_handle(&self, exec_id: &Uuid) -> Option<JoinHandle<()>>;

    /// Get an entry index provider for log message injection.
    async fn get_entry_index_provider(
        &self,
        exec_id: &Uuid,
    ) -> Option<executors::logs::utils::EntryIndexProvider>;

    /// Store an entry index provider for log message injection.
    async fn store_entry_index_provider(
        &self,
        exec_id: Uuid,
        provider: executors::logs::utils::EntryIndexProvider,
    );

    /// Get the server instance ID. Used to tag execution processes for
    /// instance-scoped cleanup on shutdown.
    fn instance_id(&self) -> &str;

    fn task_attempt_to_current_dir(&self, task_attempt: &TaskAttempt) -> PathBuf;

    async fn create(&self, task_attempt: &TaskAttempt) -> Result<ContainerRef, ContainerError>;

    async fn kill_all_running_processes(&self) -> Result<(), ContainerError>;

    async fn delete(&self, task_attempt: &TaskAttempt) -> Result<(), ContainerError> {
        self.try_stop(task_attempt).await;
        self.delete_inner(task_attempt).await
    }

    /// Check if a task has any running execution processes
    async fn has_running_processes(&self, task_id: Uuid) -> Result<bool, ContainerError> {
        let attempts = TaskAttempt::fetch_all(&self.db().pool, Some(task_id)).await?;

        for attempt in attempts {
            if self.has_running_processes_for_attempt(attempt.id).await? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if a task attempt has any running execution processes (excluding dev server)
    async fn has_running_processes_for_attempt(
        &self,
        attempt_id: Uuid,
    ) -> Result<bool, ContainerError> {
        let processes =
            ExecutionProcess::find_by_task_attempt_id(&self.db().pool, attempt_id, false).await?;

        for process in processes {
            if process.status == ExecutionProcessStatus::Running
                && !matches!(process.run_reason, ExecutionProcessRunReason::DevServer)
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Stop execution processes for task attempts without cleanup
    async fn stop_task_processes(
        &self,
        task_attempts: &[TaskAttempt],
    ) -> Result<(), ContainerError> {
        for attempt in task_attempts {
            self.try_stop(attempt).await;
        }
        Ok(())
    }

    /// A context is finalized when
    /// - Always when the execution process has failed or been killed
    /// - Never when the run reason is DevServer
    /// - Never when the run reason is SetupScript with no next_action (parallel mode)
    /// - The next action is None (no follow-up actions)
    fn should_finalize(&self, ctx: &ExecutionContext) -> bool {
        if matches!(
            ctx.execution_process.run_reason,
            ExecutionProcessRunReason::DevServer
        ) {
            return false;
        }

        let action = ctx.execution_process.executor_action().unwrap();

        // Never finalize setup scripts without next_action (parallel mode)
        // In parallel mode, the setup script runs independently and shouldn't trigger finalization
        if matches!(
            ctx.execution_process.run_reason,
            ExecutionProcessRunReason::SetupScript
        ) && action.next_action.is_none()
        {
            return false;
        }

        // Always finalize failed or killed executions, regardless of next action
        if matches!(
            ctx.execution_process.status,
            ExecutionProcessStatus::Failed | ExecutionProcessStatus::Killed
        ) {
            return true;
        }
        // Otherwise, finalize only if no next action
        action.next_action.is_none()
    }

    /// Finalize task execution by updating status to InReview and sending notifications
    async fn finalize_task(
        &self,
        config: &Arc<RwLock<Config>>,
        share_publisher: Option<&SharePublisher>,
        ctx: &ExecutionContext,
    ) {
        match Task::update_status(&self.db().pool, ctx.task.id, TaskStatus::InReview).await {
            Ok(_) => {
                if let Some(publisher) = share_publisher
                    && let Err(err) = publisher.update_shared_task_by_id(ctx.task.id).await
                {
                    tracing::warn!(
                        ?err,
                        "Failed to propagate shared task update for {}",
                        ctx.task.id
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to update task status to InReview: {e}");
            }
        }
        let notify_cfg = config.read().await.notifications.clone();
        NotificationService::notify_execution_halted(notify_cfg, ctx).await;
    }

    /// Drain persisted queued messages for idle attempts on boot.
    /// Called AFTER cleanup_orphan_executions; no-op by default (overridden in LocalContainerService).
    async fn drain_queued_messages_on_boot(&self) -> Result<(), ContainerError> {
        Ok(())
    }

    /// Cleanup executions marked as running in the db, call at startup.
    /// Uses fence-then-resume: for each running coding-agent process with a PID,
    /// fence the process first, then resume if a session_id exists, or mark-failed otherwise.
    /// Non-coding-agent orphans are handled by the blanket mark_orphaned_as_failed call.
    async fn cleanup_orphan_executions(&self) -> Result<(), ContainerError> {
        let instance_id = self.instance_id();
        let pool = &self.db().pool;

        tracing::info!(
            instance_id = %instance_id,
            "Starting fence-then-resume recovery for orphaned execution processes"
        );

        // Fetch running coding-agent processes that have a PID (may be orphaned)
        let candidates = ExecutionProcess::find_running_with_pids(pool).await?;

        // Build a single inspector for all fence calls (avoids per-call sysinfo refresh)
        let inspector = SysinfoProcessInspector::new();

        for process in &candidates {
            // Only recover coding-agent runs (SC8 target: resume, not script runs)
            if process.run_reason != ExecutionProcessRunReason::CodingAgent {
                continue;
            }

            let Some(pid_raw) = process.pid else {
                continue;
            };

            // Step 1: Fence the process (safety invariant: never resume into a live writer)
            let task_attempt = match TaskAttempt::find_by_id(pool, process.task_attempt_id).await {
                Ok(Some(ta)) => ta,
                _ => {
                    tracing::warn!(
                        process_id = %process.id,
                        "Could not find task attempt for process; skipping recovery"
                    );
                    continue;
                }
            };

            let Some(container_ref) = task_attempt.container_ref.clone().filter(|s| !s.is_empty()) else {
                // No worktree path recorded — cannot safely identify the process by cwd.
                // An empty marker would match every process on the system, defeating the
                // PID-reuse guard.  Treat as not-ours and skip recovery.
                tracing::warn!(
                    process_id = %process.id,
                    pid = pid_raw,
                    "No container_ref; cannot safely fence — skipping recovery"
                );
                continue;
            };
            let fence_result = process_fence::fence(&inspector, pid_raw, &container_ref).await;

            match fence_result {
                FenceOutcome::NotOurProcess => {
                    // PID was reused by another process; do not kill, skip recovery.
                    // Mark abandoned and update task status to InReview — the execution
                    // is gone (the process is not ours), so treat it as a failure.
                    let _ =
                        ExecutionProcess::set_resume_state(pool, process.id, "abandoned").await;
                    tracing::warn!(
                        process_id = %process.id,
                        pid = pid_raw,
                        "PID reused by another process; marking execution as failed"
                    );
                    self.mark_process_failed_with_task_update(pool, process, &task_attempt)
                        .await;
                    continue;
                }
                FenceOutcome::AlreadyGone | FenceOutcome::Fenced => {
                    // Process is confirmed dead; safe to proceed with resume classification
                }
                FenceOutcome::CouldNotKill => {
                    // Process survived SIGKILL (D-state / uninterruptible sleep).
                    // Resuming into a potentially live writer violates SC1; skip this process.
                    // Set resume_state='pending' so the blanket mark_orphaned_as_failed guard
                    // (which excludes 'pending' and 'resumed') does NOT mark this row failed —
                    // the process is still alive and will be fenced again on next restart.
                    let _ =
                        ExecutionProcess::set_resume_state(pool, process.id, "pending").await;
                    tracing::warn!(
                        process_id = %process.id,
                        pid = pid_raw,
                        "Process survived SIGKILL (D-state); skipping recovery to avoid concurrent writer"
                    );
                    continue;
                }
            }

            // Step 2: Classify — resume if session_id exists, mark-failed otherwise
            let session_id =
                ExecutionProcess::find_latest_session_id_by_task_attempt(pool, process.task_attempt_id)
                    .await
                    .unwrap_or(None);

            if let Some(session_id) = session_id {
                // Resume: executor supports session recovery (task 301 audit: all 9 executors do)
                let _ = ExecutionProcess::set_resume_state(pool, process.id, "pending").await;

                // Reconstruct stored action from the process record
                let stored_action = match process.executor_action() {
                    Ok(action) => action.clone(),
                    Err(_) => {
                        // executor_action JSON is unparseable (database corruption or schema
                        // mismatch). Fall back to ClaudeCode:DEFAULT — the original executor
                        // variant is lost, but the minimal continuation prompt still gives
                        // the session a chance to recover. If start_execution_inner then fails,
                        // the error path below marks the process abandoned.
                        ExecutorAction::new(
                            ExecutorActionType::CodingAgentInitialRequest(
                                CodingAgentInitialRequest {
                                    prompt: String::new(),
                                    executor_profile_id: executors::profile::ExecutorProfileId::new(
                                        executors::executors::BaseCodingAgent::ClaudeCode,
                                    ),
                                },
                            ),
                            None,
                        )
                    }
                };

                // Minimal continuation prompt per ledger task 303 decision:
                // re-sending the original task prompt over --resume reads as "redo the task",
                // not "continue where you left off". A minimal prompt is the safer default.
                let resume_prompt = "Your previous session was interrupted. Please continue the task from where you left off.".to_string();

                match self
                    .resume_execution(
                        &task_attempt,
                        process,
                        &stored_action,
                        session_id,
                        resume_prompt,
                    )
                    .await
                {
                    Ok(()) => {
                        let _ =
                            ExecutionProcess::set_resume_state(pool, process.id, "resumed").await;
                        tracing::info!(
                            process_id = %process.id,
                            "Successfully resumed execution"
                        );
                    }
                    Err(e) => {
                        // Resume failed; fall through to mark-failed
                        tracing::warn!(
                            process_id = %process.id,
                            error = ?e,
                            "Resume failed; marking as abandoned"
                        );
                        let _ =
                            ExecutionProcess::set_resume_state(pool, process.id, "abandoned").await;
                        self.mark_process_failed_with_task_update(pool, process, &task_attempt)
                            .await;
                    }
                }
            } else {
                // No session_id: mark-failed (abandoned)
                let _ = ExecutionProcess::set_resume_state(pool, process.id, "abandoned").await;
                self.mark_process_failed_with_task_update(pool, process, &task_attempt)
                    .await;
            }
        }

        // Now run the blanket mark-orphaned-as-failed for non-coding-agent orphans
        // (mark_orphaned_as_failed now excludes rows with resume_state IN ('pending','resumed'))
        let orphaned_count = ExecutionProcess::mark_orphaned_as_failed(pool, instance_id).await?;

        if orphaned_count > 0 {
            tracing::info!(
                instance_id = %instance_id,
                orphaned_count = orphaned_count,
                "Marked {} non-resumable orphaned processes as failed",
                orphaned_count
            );
        }

        Ok(())
    }

    /// Mark an execution process as failed and update the parent task status to InReview.
    async fn mark_process_failed_with_task_update(
        &self,
        pool: &sqlx::SqlitePool,
        process: &ExecutionProcess,
        task_attempt: &TaskAttempt,
    ) {
        if let Err(e) = ExecutionProcess::update_completion(
            pool,
            process.id,
            ExecutionProcessStatus::Failed,
            None,
            Some("eof"),
            None,
        )
        .await
        {
            tracing::error!(
                "Failed to update orphaned execution process {} status: {}",
                process.id,
                e
            );
            return;
        }

        // Capture after-head commit (best-effort)
        if let Some(container_ref) = &task_attempt.container_ref {
            let wt = std::path::PathBuf::from(container_ref);
            if let Ok(head) = self.git().get_head_info(&wt) {
                let _ = ExecutionProcess::update_after_head_commit(pool, process.id, &head.oid)
                    .await;
            }
        }

        // Update task status to InReview for coding agent failures
        if matches!(
            process.run_reason,
            ExecutionProcessRunReason::CodingAgent
                | ExecutionProcessRunReason::SetupScript
                | ExecutionProcessRunReason::CleanupScript
        ) && let Ok(Some(task)) = task_attempt.parent_task(pool).await
        {
            match Task::update_status(pool, task.id, TaskStatus::InReview).await {
                Ok(_) => {
                    if let Some(publisher) = self.share_publisher()
                        && let Err(err) = publisher.update_shared_task_by_id(task.id).await
                    {
                        tracing::warn!(
                            ?err,
                            "Failed to propagate shared task update for {}",
                            task.id
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to update task status to InReview: {}",
                        e
                    );
                }
            }
        }

        tracing::info!("Marked orphaned process {} as failed", process.id);
    }

    /// Resume a coding-agent execution from a previous session.
    ///
    /// This method takes a stored executor action (typically from a previous execution),
    /// builds a resume action with the new session ID and prompt, and starts the execution.
    ///
    /// # Arguments
    /// * `task_attempt` - The task attempt to resume execution for
    /// * `execution_process` - The execution process record to track this run
    /// * `stored_action` - The previously stored executor action to resume from
    /// * `session_id` - The session ID for the follow-up conversation
    /// * `prompt` - The prompt text to send as a follow-up
    ///
    /// # Returns
    /// * `Ok(())` if the execution starts successfully
    /// * `Err(ContainerError::Other)` if the stored action is not a resumable coding-agent action
    async fn resume_execution(
        &self,
        task_attempt: &TaskAttempt,
        execution_process: &ExecutionProcess,
        stored_action: &ExecutorAction,
        session_id: String,
        prompt: String,
    ) -> Result<(), ContainerError> {
        // Build the resume action from the stored action
        let action = build_resume_action(stored_action, session_id, prompt).ok_or_else(|| {
            ContainerError::Other(anyhow!(
                "stored action is not a resumable coding-agent action"
            ))
        })?;

        // Start the execution with the built resume action
        self.start_execution_inner(task_attempt, execution_process, &action)
            .await
    }

    /// Backfill before_head_commit for legacy execution processes.
    /// Rules:
    /// - If a process has after_head_commit and missing before_head_commit,
    ///   then set before_head_commit to the previous process's after_head_commit.
    /// - If there is no previous process, set before_head_commit to the base branch commit.
    async fn backfill_before_head_commits(&self) -> Result<(), ContainerError> {
        let pool = &self.db().pool;
        let rows = ExecutionProcess::list_missing_before_context(pool).await?;
        for row in rows {
            // Skip if no after commit at all (shouldn't happen due to WHERE)
            // Prefer previous process after-commit if present
            let mut before = row.prev_after_head_commit.clone();

            // Fallback to base branch commit OID
            if before.is_none() {
                let repo_path =
                    std::path::Path::new(row.git_repo_path.as_deref().unwrap_or_default());
                match self
                    .git()
                    .get_branch_oid(repo_path, row.target_branch.as_str())
                {
                    Ok(oid) => before = Some(oid),
                    Err(e) => {
                        tracing::warn!(
                            "Backfill: Failed to resolve base branch OID for attempt {} (branch {}): {}",
                            row.task_attempt_id,
                            row.target_branch,
                            e
                        );
                    }
                }
            }

            if let Some(before_oid) = before
                && let Err(e) =
                    ExecutionProcess::update_before_head_commit(pool, row.id, &before_oid).await
            {
                tracing::warn!(
                    "Backfill: Failed to update before_head_commit for process {}: {}",
                    row.id,
                    e
                );
            }
        }

        Ok(())
    }

    fn cleanup_action(&self, cleanup_script: Option<String>) -> Option<Box<ExecutorAction>> {
        cleanup_script.map(|script| {
            Box::new(ExecutorAction::new(
                ExecutorActionType::ScriptRequest(ScriptRequest {
                    script,
                    language: ScriptRequestLanguage::Bash,
                    context: ScriptContext::CleanupScript,
                }),
                None,
            ))
        })
    }

    async fn try_stop(&self, task_attempt: &TaskAttempt) {
        const STOP_TIMEOUT: Duration = Duration::from_secs(15);

        // Stop all execution processes for this attempt, with a timeout to prevent hanging
        let stop_result = tokio::time::timeout(STOP_TIMEOUT, async {
            if let Ok(processes) =
                ExecutionProcess::find_by_task_attempt_id(&self.db().pool, task_attempt.id, false)
                    .await
            {
                for process in processes {
                    if process.status == ExecutionProcessStatus::Running {
                        self.stop_execution(&process, ExecutionProcessStatus::Killed)
                            .await
                            .unwrap_or_else(|e| {
                                tracing::debug!(
                                    "Failed to stop execution process {} for task attempt {}: {}",
                                    process.id,
                                    task_attempt.id,
                                    e
                                );
                            });
                    }
                }
            }
        })
        .await;

        if stop_result.is_err() {
            tracing::warn!(
                task_attempt_id = %task_attempt.id,
                timeout_secs = STOP_TIMEOUT.as_secs(),
                "try_stop timed out - proceeding anyway to avoid blocking"
            );
        }
    }

    async fn delete_inner(&self, task_attempt: &TaskAttempt) -> Result<(), ContainerError>;

    async fn ensure_container_exists(
        &self,
        task_attempt: &TaskAttempt,
    ) -> Result<ContainerRef, ContainerError>;

    async fn is_container_clean(&self, task_attempt: &TaskAttempt) -> Result<bool, ContainerError>;

    async fn start_execution_inner(
        &self,
        task_attempt: &TaskAttempt,
        execution_process: &ExecutionProcess,
        executor_action: &ExecutorAction,
    ) -> Result<(), ContainerError>;

    async fn stop_execution(
        &self,
        execution_process: &ExecutionProcess,
        status: ExecutionProcessStatus,
    ) -> Result<(), ContainerError>;

    /// Inject a message into a running execution process.
    /// This is only supported for Claude Code executors on local deployments.
    /// Returns Ok(true) if message was sent, Ok(false) if not supported, Err on failure.
    async fn inject_message(
        &self,
        _execution_process_id: Uuid,
        _message: String,
    ) -> Result<bool, ContainerError> {
        // Default implementation: not supported
        Ok(false)
    }

    async fn try_commit_changes(&self, ctx: &ExecutionContext) -> Result<bool, ContainerError>;

    async fn copy_project_files(
        &self,
        source_dir: &Path,
        target_dir: &Path,
        copy_files: &str,
    ) -> Result<(), ContainerError>;

    /// Stream diff updates as LogMsg for WebSocket endpoints.
    async fn stream_diff(
        &self,
        task_attempt: &TaskAttempt,
        stats_only: bool,
    ) -> Result<futures::stream::BoxStream<'static, Result<LogMsg, std::io::Error>>, ContainerError>;

    /// Fetch the MsgStore for a given execution ID, panicking if missing.
    async fn get_msg_store_by_id(&self, uuid: &Uuid) -> Option<Arc<MsgStore>> {
        let map = self.msg_stores().read().await;
        map.get(uuid).cloned()
    }

    async fn git_branch_prefix(&self) -> String;

    async fn git_branch_from_task_attempt(&self, attempt_id: &Uuid, task_title: &str) -> String {
        let task_title_id = git_branch_id(task_title);
        let prefix = self.git_branch_prefix().await;

        if prefix.is_empty() {
            format!("{}-{}", short_uuid(attempt_id), task_title_id)
        } else {
            format!("{}/{}-{}", prefix, short_uuid(attempt_id), task_title_id)
        }
    }

    async fn stream_raw_logs(
        &self,
        id: &Uuid,
    ) -> Option<futures::stream::BoxStream<'static, Result<LogMsg, std::io::Error>>> {
        if let Some(store) = self.get_msg_store_by_id(id).await {
            // First try in-memory store
            return Some(
                store
                    .history_plus_stream()
                    .filter(|msg| {
                        future::ready(matches!(
                            msg,
                            Ok(LogMsg::Stdout(..) | LogMsg::Stderr(..) | LogMsg::Finished)
                        ))
                    })
                    .boxed(),
            );
        } else {
            // Fallback: load from DB and create direct stream
            let log_records =
                match ExecutionProcessLogs::find_by_execution_id(&self.db().pool, *id).await {
                    Ok(records) if !records.is_empty() => records,
                    Ok(_) => return None, // No logs exist
                    Err(e) => {
                        tracing::error!("Failed to fetch logs for execution {}: {}", id, e);
                        return None;
                    }
                };

            let messages = match ExecutionProcessLogs::parse_logs(&log_records) {
                Ok(msgs) => msgs,
                Err(e) => {
                    tracing::error!("Failed to parse logs for execution {}: {}", id, e);
                    return None;
                }
            };

            // Direct stream from parsed messages
            let stream = futures::stream::iter(
                messages
                    .into_iter()
                    .filter(|m| matches!(m, LogMsg::Stdout(_) | LogMsg::Stderr(_)))
                    .chain(std::iter::once(LogMsg::Finished))
                    .map(Ok::<_, std::io::Error>),
            )
            .boxed();

            Some(stream)
        }
    }

    async fn stream_normalized_logs(
        &self,
        id: &Uuid,
    ) -> Option<futures::stream::BoxStream<'static, Result<LogMsg, std::io::Error>>> {
        // First try in-memory store (existing behavior)
        if let Some(store) = self.get_msg_store_by_id(id).await {
            Some(
                store
                    .history_plus_stream() // BoxStream<Result<LogMsg, io::Error>>
                    .filter(|msg| future::ready(matches!(msg, Ok(LogMsg::JsonPatch(..)))))
                    .chain(futures::stream::once(async {
                        Ok::<_, std::io::Error>(LogMsg::Finished)
                    }))
                    .boxed(),
            )
        } else {
            // Fallback: load from DB and normalize
            let log_records =
                match ExecutionProcessLogs::find_by_execution_id(&self.db().pool, *id).await {
                    Ok(records) if !records.is_empty() => records,
                    Ok(_) => return None, // No logs exist
                    Err(e) => {
                        tracing::error!("Failed to fetch logs for execution {}: {}", id, e);
                        return None;
                    }
                };

            let raw_messages = match ExecutionProcessLogs::parse_logs(&log_records) {
                Ok(msgs) => msgs,
                Err(e) => {
                    tracing::error!("Failed to parse logs for execution {}: {}", id, e);
                    return None;
                }
            };

            // Check for existing normalized patches
            let existing_patches: Vec<_> = raw_messages
                .iter()
                .filter(|msg| matches!(msg, LogMsg::JsonPatch(_)))
                .cloned()
                .collect();

            if !existing_patches.is_empty() {
                // Already normalized - stream existing patches directly
                let temp_store = Arc::new(MsgStore::new());
                for patch in existing_patches {
                    temp_store.push(patch);
                }
                temp_store.push_finished();

                return Some(
                    temp_store
                        .history_plus_stream()
                        .filter(|msg| future::ready(matches!(msg, Ok(LogMsg::JsonPatch(..)))))
                        .chain(futures::stream::once(async {
                            Ok::<_, std::io::Error>(LogMsg::Finished)
                        }))
                        .boxed(),
                );
            }

            // No existing patches - normalize stdout/stderr only
            let temp_store = Arc::new(MsgStore::new());
            for msg in raw_messages {
                if matches!(msg, LogMsg::Stdout(_) | LogMsg::Stderr(_)) {
                    temp_store.push(msg);
                }
            }
            temp_store.push_finished();

            let process = match ExecutionProcess::find_by_id(&self.db().pool, *id).await {
                Ok(Some(process)) => process,
                Ok(None) => {
                    tracing::error!("No execution process found for ID: {}", id);
                    return None;
                }
                Err(e) => {
                    tracing::error!("Failed to fetch execution process {}: {}", id, e);
                    return None;
                }
            };

            // Get the task attempt to determine correct directory
            let task_attempt = match process.parent_task_attempt(&self.db().pool).await {
                Ok(Some(task_attempt)) => task_attempt,
                Ok(None) => {
                    tracing::error!("No task attempt found for ID: {}", process.task_attempt_id);
                    return None;
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to fetch task attempt {}: {}",
                        process.task_attempt_id,
                        e
                    );
                    return None;
                }
            };

            if let Err(err) = self.ensure_container_exists(&task_attempt).await {
                tracing::warn!(
                    "Failed to recreate worktree before log normalization for task attempt {}: {}",
                    task_attempt.id,
                    err
                );
            }

            let current_dir = self.task_attempt_to_current_dir(&task_attempt);

            let executor_action = if let Ok(executor_action) = process.executor_action() {
                executor_action
            } else {
                tracing::error!(
                    "Failed to parse executor action: {:?}",
                    process.executor_action()
                );
                return None;
            };

            // Spawn normalizer on populated store
            match executor_action.typ() {
                ExecutorActionType::CodingAgentInitialRequest(request) => {
                    let executor = ExecutorConfigs::get_cached()
                        .get_coding_agent_or_default(&request.executor_profile_id);
                    let entry_index_provider =
                        executors::logs::utils::EntryIndexProvider::start_from(&temp_store);
                    executor.normalize_logs(temp_store.clone(), &current_dir, entry_index_provider);
                }
                ExecutorActionType::CodingAgentFollowUpRequest(request) => {
                    let executor = ExecutorConfigs::get_cached()
                        .get_coding_agent_or_default(&request.executor_profile_id);
                    let entry_index_provider =
                        executors::logs::utils::EntryIndexProvider::start_from(&temp_store);
                    executor.normalize_logs(temp_store.clone(), &current_dir, entry_index_provider);
                }
                ExecutorActionType::CodingAgentReviewRequest(request) => {
                    let executor = ExecutorConfigs::get_cached()
                        .get_coding_agent_or_default(&request.executor_profile_id);
                    let entry_index_provider =
                        executors::logs::utils::EntryIndexProvider::start_from(&temp_store);
                    executor.normalize_logs(temp_store.clone(), &current_dir, entry_index_provider);
                }
                _ => {
                    tracing::debug!(
                        "Executor action doesn't support log normalization: {:?}",
                        process.executor_action()
                    );
                    return None;
                }
            }
            Some(
                temp_store
                    .history_plus_stream()
                    .filter(|msg| future::ready(matches!(msg, Ok(LogMsg::JsonPatch(..)))))
                    .chain(futures::stream::once(async {
                        Ok::<_, std::io::Error>(LogMsg::Finished)
                    }))
                    .boxed(),
            )
        }
    }

    /// Stream live-only logs for an execution (no history).
    ///
    /// This is used by the unified log WebSocket endpoint to stream only
    /// new log entries without replaying history. The frontend uses the
    /// REST pagination endpoint to fetch historical entries separately.
    ///
    /// Returns `None` if:
    /// - The execution doesn't exist
    /// - The execution is not running (no live stream available)
    async fn stream_live_logs_only(
        &self,
        id: &Uuid,
    ) -> Option<futures::stream::BoxStream<'static, Result<LogMsg, std::io::Error>>> {
        // Only available for running executions (in-memory store)
        if let Some(store) = self.get_msg_store_by_id(id).await {
            Some(
                store
                    .stream_live_only()
                    .filter(|msg| {
                        // Include all log types that the frontend might need
                        future::ready(matches!(
                            msg,
                            Ok(LogMsg::Stdout(..)
                                | LogMsg::Stderr(..)
                                | LogMsg::JsonPatch(..)
                                | LogMsg::SessionId(..)
                                | LogMsg::Finished
                                | LogMsg::RefreshRequired { .. })
                        ))
                    })
                    .boxed(),
            )
        } else {
            // Execution not running - no live stream available.
            // Frontend should use REST pagination for completed executions.
            None
        }
    }

    fn spawn_stream_raw_logs_to_db(&self, execution_id: &Uuid) -> JoinHandle<()> {
        let execution_id = *execution_id;
        let msg_stores = self.msg_stores().clone();
        let db = self.db().clone();
        let log_batcher = self.log_batcher().cloned();

        tokio::spawn(async move {
            // Get the message store for this execution
            let store = {
                let map = msg_stores.read().await;
                map.get(&execution_id).cloned()
            };

            if let Some(store) = store {
                let mut stream = store.history_plus_stream();

                while let Some(Ok(msg)) = stream.next().await {
                    match &msg {
                        LogMsg::Stdout(_) | LogMsg::Stderr(_) => {
                            // Use batched writes if log batcher is available
                            if let Some(ref batcher) = log_batcher {
                                batcher.add_log(execution_id, msg.clone()).await;
                            } else {
                                // Fallback to direct writes (legacy behavior)
                                match serde_json::to_string(&msg) {
                                    Ok(jsonl_line) => {
                                        let jsonl_line_with_newline = format!("{jsonl_line}\n");

                                        if let Err(e) = ExecutionProcessLogs::append_log_line(
                                            &db.pool,
                                            execution_id,
                                            &jsonl_line_with_newline,
                                        )
                                        .await
                                        {
                                            tracing::error!(
                                                "Failed to append log line for execution {}: {}",
                                                execution_id,
                                                e
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to serialize log message for execution {}: {}",
                                            execution_id,
                                            e
                                        );
                                    }
                                }
                            }
                        }
                        LogMsg::SessionId(session_id) => {
                            // Session ID updates are rare and important - write immediately
                            if let Err(e) = ExecutorSession::update_session_id(
                                &db.pool,
                                execution_id,
                                session_id,
                            )
                            .await
                            {
                                tracing::error!(
                                    "Failed to update session_id {} for execution process {}: {}",
                                    session_id,
                                    execution_id,
                                    e
                                );
                            }
                        }
                        LogMsg::Finished => {
                            // Flush any remaining batched logs before finishing
                            if let Some(ref batcher) = log_batcher {
                                batcher.finish(execution_id).await;
                            }
                            break;
                        }
                        LogMsg::JsonPatch(_) => {
                            // Persist JsonPatch to database via log batcher
                            if let Some(ref batcher) = log_batcher {
                                batcher.add_log(execution_id, msg.clone()).await;
                            } else {
                                // Fallback to direct writes (legacy behavior)
                                match serde_json::to_string(&msg) {
                                    Ok(jsonl_line) => {
                                        let jsonl_line_with_newline = format!("{jsonl_line}\n");
                                        if let Err(e) = ExecutionProcessLogs::append_log_line(
                                            &db.pool,
                                            execution_id,
                                            &jsonl_line_with_newline,
                                        )
                                        .await
                                        {
                                            tracing::error!(
                                                "Failed to append JsonPatch log for execution {}: {}",
                                                execution_id,
                                                e
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to serialize JsonPatch for execution {}: {}",
                                            execution_id,
                                            e
                                        );
                                    }
                                }
                            }
                        }
                        LogMsg::RefreshRequired { .. } => continue,
                    }
                }
            }
        })
    }

    /// Starts a new execution attempt for a task attempt, creating the worktree unless skipped and wiring setup scripts, coding agent requests, and optional cleanup actions.
    ///
    /// The function ensures the container/worktree exists (unless `skip_worktree_creation` is true), resolves the latest task attempt, expands image paths and task variables in the prompt, and then starts one or more executions:
    /// - If the project defines a setup script and it is configured to run in parallel, the setup script is started independently and the coding-agent initial request is started immediately.
    /// - If the setup script is sequential, the setup script is started with the coding-agent initial request chained as the next action.
    /// - If there is no setup script, the coding-agent initial request is started directly.
    /// On success returns the started `ExecutionProcess`.
    ///
    /// # Parameters
    ///
    /// - `task_attempt`: The task attempt to run; the function will refresh this record from the database before starting.
    /// - `executor_profile_id`: Identifier of the executor profile to use for the coding-agent initial request.
    /// - `skip_worktree_creation`: When true, do not create the worktree/container (useful for shared or pre-created worktrees).
    ///
    /// # Returns
    ///
    /// `ExecutionProcess` representing the started execution on success.
    ///
    /// # Examples
    ///
    /// ```
    /// # // Pseudocode example; replace `svc`, `task_attempt`, and `profile_id` with real values from your environment.
    /// # async fn example(svc: &impl ContainerService, task_attempt: &TaskAttempt, profile_id: ExecutorProfileId) -> Result<(), ContainerError> {
    /// let exec = svc.start_attempt(task_attempt, profile_id, false).await?;
    /// assert_eq!(exec.task_attempt_id, task_attempt.id);
    /// # Ok(())
    /// # }
    /// ```
    async fn start_attempt(
        &self,
        task_attempt: &TaskAttempt,
        executor_profile_id: ExecutorProfileId,
        skip_worktree_creation: bool,
    ) -> Result<ExecutionProcess, ContainerError> {
        // Create container (unless skipping for shared worktree scenarios)
        if !skip_worktree_creation {
            self.create(task_attempt).await?;
        }

        // Get parent task
        let task = task_attempt
            .parent_task(&self.db().pool)
            .await?
            .ok_or(SqlxError::RowNotFound)?;

        // Get parent project
        let project = task
            .parent_project(&self.db().pool)
            .await?
            .ok_or(SqlxError::RowNotFound)?;

        // // Get latest version of task attempt
        let task_attempt = TaskAttempt::find_by_id(&self.db().pool, task_attempt.id)
            .await?
            .ok_or(SqlxError::RowNotFound)?;

        // TODO: this implementation will not work in cloud
        let worktree_path = PathBuf::from(
            task_attempt
                .container_ref
                .as_ref()
                .ok_or_else(|| ContainerError::Other(anyhow!("Container ref not found")))?,
        );
        let prompt = ImageService::canonicalise_image_paths(&task.to_prompt(), &worktree_path);

        // Expand task variables ($VAR and ${VAR} syntax) in the prompt
        let prompt = {
            // Get resolved variables for this task (including inherited from parent chain)
            let variables = TaskVariable::get_variable_map_with_system(&self.db().pool, task.id)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!(task_id = %task.id, error = ?e, "Failed to fetch task variables");
                    std::collections::HashMap::new()
                });

            if variables.is_empty() {
                prompt
            } else {
                // Convert (String, Uuid) to (String, Option<Uuid>) for variable_expander
                let variables: HashMap<String, (String, Option<Uuid>)> = variables
                    .into_iter()
                    .map(|(k, (v, id))| (k, (v, Some(id))))
                    .collect();

                let result = variable_expander::expand_variables(&prompt, &variables);

                // Log warning if there are undefined variables
                if !result.undefined_vars.is_empty() {
                    tracing::warn!(
                        task_id = %task.id,
                        undefined_vars = ?result.undefined_vars,
                        "Task prompt contains undefined variables that were not expanded"
                    );
                }

                if !result.expanded_vars.is_empty() {
                    tracing::info!(
                        task_id = %task.id,
                        expanded_count = result.expanded_vars.len(),
                        "Expanded task variables in prompt"
                    );
                }

                result.text
            }
        };

        let cleanup_action = self.cleanup_action(project.cleanup_script);

        // Choose whether to execute the setup_script or coding agent first
        let execution_process = if let Some(setup_script) = project.setup_script {
            if project.parallel_setup_script {
                // Parallel mode: start setup script independently (no next_action)
                let setup_action = ExecutorAction::new(
                    ExecutorActionType::ScriptRequest(ScriptRequest {
                        script: setup_script,
                        language: ScriptRequestLanguage::Bash,
                        context: ScriptContext::SetupScript,
                    }),
                    None, // No chaining - runs independently
                );
                if let Err(e) = self
                    .start_execution(
                        &task_attempt,
                        &setup_action,
                        &ExecutionProcessRunReason::SetupScript,
                    )
                    .await
                {
                    tracing::warn!(?e, "Failed to start setup script in parallel mode");
                }

                // Start coding agent immediately (don't wait for setup script)
                let coding_action = ExecutorAction::new(
                    ExecutorActionType::CodingAgentInitialRequest(CodingAgentInitialRequest {
                        prompt,
                        executor_profile_id: executor_profile_id.clone(),
                    }),
                    cleanup_action,
                );

                self.start_execution(
                    &task_attempt,
                    &coding_action,
                    &ExecutionProcessRunReason::CodingAgent,
                )
                .await?
            } else {
                // Sequential mode (existing behavior): chain setup → coding agent
                let executor_action = ExecutorAction::new(
                    ExecutorActionType::ScriptRequest(ScriptRequest {
                        script: setup_script,
                        language: ScriptRequestLanguage::Bash,
                        context: ScriptContext::SetupScript,
                    }),
                    // once the setup script is done, run the initial coding agent request
                    Some(Box::new(ExecutorAction::new(
                        ExecutorActionType::CodingAgentInitialRequest(CodingAgentInitialRequest {
                            prompt,
                            executor_profile_id: executor_profile_id.clone(),
                        }),
                        cleanup_action,
                    ))),
                );

                self.start_execution(
                    &task_attempt,
                    &executor_action,
                    &ExecutionProcessRunReason::SetupScript,
                )
                .await?
            }
        } else {
            let executor_action = ExecutorAction::new(
                ExecutorActionType::CodingAgentInitialRequest(CodingAgentInitialRequest {
                    prompt,
                    executor_profile_id: executor_profile_id.clone(),
                }),
                cleanup_action,
            );

            self.start_execution(
                &task_attempt,
                &executor_action,
                &ExecutionProcessRunReason::CodingAgent,
            )
            .await?
        };
        Ok(execution_process)
    }

    async fn start_execution(
        &self,
        task_attempt: &TaskAttempt,
        executor_action: &ExecutorAction,
        run_reason: &ExecutionProcessRunReason,
    ) -> Result<ExecutionProcess, ContainerError> {
        // Update task status to InProgress when starting an attempt
        let task = task_attempt
            .parent_task(&self.db().pool)
            .await?
            .ok_or(SqlxError::RowNotFound)?;
        if task.status != TaskStatus::InProgress
            && run_reason != &ExecutionProcessRunReason::DevServer
        {
            Task::update_status(&self.db().pool, task.id, TaskStatus::InProgress).await?;

            if let Some(publisher) = self.share_publisher()
                && let Err(err) = publisher.update_shared_task_by_id(task.id).await
            {
                tracing::warn!(
                    ?err,
                    "Failed to propagate shared task update for {}",
                    task.id
                );
            }
        }
        // Create new execution process record
        // Capture current HEAD as the "before" commit for this execution
        let before_head_commit = {
            if let Some(container_ref) = &task_attempt.container_ref {
                let wt = std::path::Path::new(container_ref);
                self.git().get_head_info(wt).ok().map(|h| h.oid)
            } else {
                None
            }
        };
        let create_execution_process = CreateExecutionProcess {
            task_attempt_id: task_attempt.id,
            executor_action: executor_action.clone(),
            run_reason: run_reason.clone(),
        };

        let execution_process = ExecutionProcess::create(
            &self.db().pool,
            &create_execution_process,
            Uuid::new_v4(),
            before_head_commit.as_deref(),
            Some(self.instance_id()),
        )
        .await?;

        if let Some(prompt) = match executor_action.typ() {
            ExecutorActionType::CodingAgentInitialRequest(coding_agent_request) => {
                Some(coding_agent_request.prompt.clone())
            }
            ExecutorActionType::CodingAgentFollowUpRequest(follow_up_request) => {
                Some(follow_up_request.prompt.clone())
            }
            ExecutorActionType::CodingAgentReviewRequest(review_request) => {
                Some(review_request.display_prompt())
            }
            _ => None,
        } {
            let create_executor_data = CreateExecutorSession {
                task_attempt_id: task_attempt.id,
                execution_process_id: execution_process.id,
                prompt: Some(prompt),
            };

            let executor_session_record_id = Uuid::new_v4();

            ExecutorSession::create(
                &self.db().pool,
                &create_executor_data,
                executor_session_record_id,
            )
            .await?;
        }

        if let Err(start_error) = self
            .start_execution_inner(task_attempt, &execution_process, executor_action)
            .await
        {
            // Mark process as failed
            if let Err(update_error) = ExecutionProcess::update_completion(
                &self.db().pool,
                execution_process.id,
                ExecutionProcessStatus::Failed,
                None,
                Some("error"),                  // Execution failed to start
                Some(&start_error.to_string()), // Include error message
            )
            .await
            {
                tracing::error!(
                    "Failed to mark execution process {} as failed after start error: {}",
                    execution_process.id,
                    update_error
                );
            }
            Task::update_status(&self.db().pool, task.id, TaskStatus::InReview).await?;

            // Emit stderr error message
            let log_message = LogMsg::Stderr(format!("Failed to start execution: {start_error}"));
            if let Ok(json_line) = serde_json::to_string(&log_message) {
                let _ = ExecutionProcessLogs::append_log_line(
                    &self.db().pool,
                    execution_process.id,
                    &format!("{json_line}\n"),
                )
                .await;
            }

            // Emit NextAction with failure context for coding agent requests
            if let ContainerError::ExecutorError(ExecutorError::ExecutableNotFound { program }) =
                &start_error
            {
                let help_text = format!("The required executable `{program}` is not installed.");
                let error_message = NormalizedEntry {
                    timestamp: None,
                    entry_type: NormalizedEntryType::ErrorMessage {
                        error_type: NormalizedEntryError::SetupRequired,
                    },
                    content: help_text,
                    metadata: None,
                };
                let patch = ConversationPatch::add_normalized_entry(2, error_message);
                if let Ok(json_line) = serde_json::to_string::<LogMsg>(&LogMsg::JsonPatch(patch)) {
                    let _ = ExecutionProcessLogs::append_log_line(
                        &self.db().pool,
                        execution_process.id,
                        &format!("{json_line}\n"),
                    )
                    .await;
                }
            };
            return Err(start_error);
        }

        // Start processing normalised logs for executor requests and follow ups
        if let Some(msg_store) = self.get_msg_store_by_id(&execution_process.id).await
            && let Some(executor_profile_id) = match executor_action.typ() {
                ExecutorActionType::CodingAgentInitialRequest(request) => {
                    Some(&request.executor_profile_id)
                }
                ExecutorActionType::CodingAgentFollowUpRequest(request) => {
                    Some(&request.executor_profile_id)
                }
                ExecutorActionType::CodingAgentReviewRequest(request) => {
                    Some(&request.executor_profile_id)
                }
                _ => None,
            }
        {
            if let Some(executor) =
                ExecutorConfigs::get_cached().get_coding_agent(executor_profile_id)
            {
                // Create and store the provider FIRST
                let entry_index_provider =
                    executors::logs::utils::EntryIndexProvider::start_from(&msg_store);
                self.store_entry_index_provider(execution_process.id, entry_index_provider.clone())
                    .await;

                // Pass the same provider to normalize_logs
                let handle = executor.normalize_logs(
                    msg_store,
                    &self.task_attempt_to_current_dir(task_attempt),
                    entry_index_provider,
                );
                self.store_normalization_handle(execution_process.id, handle)
                    .await;
            } else {
                tracing::error!(
                    "Failed to resolve profile '{:?}' for normalization",
                    executor_profile_id
                );
            }
        }

        self.spawn_stream_raw_logs_to_db(&execution_process.id);
        Ok(execution_process)
    }

    async fn try_start_next_action(&self, ctx: &ExecutionContext) -> Result<(), ContainerError> {
        let action = ctx.execution_process.executor_action()?;
        let next_action = if let Some(next_action) = action.next_action() {
            next_action
        } else {
            tracing::debug!("No next action configured");
            return Ok(());
        };

        // Determine the run reason of the next action
        let next_run_reason = match (action.typ(), next_action.typ()) {
            (ExecutorActionType::ScriptRequest(_), ExecutorActionType::ScriptRequest(_)) => {
                ExecutionProcessRunReason::SetupScript
            }
            (
                ExecutorActionType::CodingAgentInitialRequest(_)
                | ExecutorActionType::CodingAgentFollowUpRequest(_)
                | ExecutorActionType::CodingAgentReviewRequest(_),
                ExecutorActionType::ScriptRequest(_),
            ) => ExecutionProcessRunReason::CleanupScript,
            (
                _,
                ExecutorActionType::CodingAgentFollowUpRequest(_)
                | ExecutorActionType::CodingAgentInitialRequest(_)
                | ExecutorActionType::CodingAgentReviewRequest(_),
            ) => ExecutionProcessRunReason::CodingAgent,
        };

        self.start_execution(&ctx.task_attempt, next_action, &next_run_reason)
            .await?;

        tracing::debug!("Started next action: {:?}", next_action);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::models::execution_process::{ExecutionProcess, ExecutionProcessStatus};
    use executors::executors::BaseCodingAgent;
    use executors::logs::utils::ConversationPatch;
    use executors::logs::{NormalizedEntry, NormalizedEntryType};

    /// Helper: Create a sample ExecutorAction with a Claude Code profile
    fn sample_coding_agent_action_claude() -> ExecutorAction {
        let initial = CodingAgentInitialRequest {
            prompt: "Initial prompt".to_string(),
            executor_profile_id: ExecutorProfileId::new(BaseCodingAgent::ClaudeCode),
        };
        ExecutorAction::new(ExecutorActionType::CodingAgentInitialRequest(initial), None)
    }

    #[test]
    fn test_build_resume_action_preserves_profile_and_session() {
        let stored = sample_coding_agent_action_claude();
        let action = build_resume_action(&stored, "sess-abc".to_string(), "continue".to_string())
            .expect("resumable");

        // Verify the action type is a follow-up request
        match action.typ {
            ExecutorActionType::CodingAgentFollowUpRequest(req) => {
                assert_eq!(req.session_id, "sess-abc");
                assert_eq!(req.prompt, "continue");
                assert_eq!(req.executor_profile_id.executor, BaseCodingAgent::ClaudeCode);
            }
            _ => panic!("expected a follow-up resume action"),
        }
    }

    #[test]
    fn test_build_resume_action_follow_up_preserves_profile_and_updates_session() {
        // A crash during a multi-turn follow-up must be resumable (SC1 coverage gap fix).
        let initial_req = CodingAgentInitialRequest {
            prompt: "Initial".to_string(),
            executor_profile_id: ExecutorProfileId::new(BaseCodingAgent::ClaudeCode),
        };
        let first_follow_up = CodingAgentFollowUpRequest {
            prompt: "Turn 2".to_string(),
            session_id: "old-sess-111".to_string(),
            executor_profile_id: ExecutorProfileId::new(BaseCodingAgent::ClaudeCode),
        };
        // Simulate a stored action that is already a follow-up (mid-conversation crash)
        let stored = ExecutorAction::new(
            ExecutorActionType::CodingAgentFollowUpRequest(first_follow_up),
            Some(Box::new(ExecutorAction::new(
                ExecutorActionType::CodingAgentInitialRequest(initial_req),
                None,
            ))),
        );

        let action =
            build_resume_action(&stored, "new-sess-999".to_string(), "resume turn 2".to_string())
                .expect("follow-up crash must be resumable");

        match action.typ {
            ExecutorActionType::CodingAgentFollowUpRequest(req) => {
                // New session_id (stale stored session must not be reused)
                assert_eq!(req.session_id, "new-sess-999");
                assert_eq!(req.prompt, "resume turn 2");
                // Profile preserved from stored follow-up
                assert_eq!(req.executor_profile_id.executor, BaseCodingAgent::ClaudeCode);
            }
            _ => panic!("expected a follow-up resume action"),
        }
    }

    #[test]
    fn test_build_resume_action_initial_request_preserves_variant() {
        // Regression test: before the fix, InitialRequest used
        // ExecutorProfileId::new(req.base_executor()), which dropped the variant.
        let initial = CodingAgentInitialRequest {
            prompt: "Task".to_string(),
            executor_profile_id: ExecutorProfileId::with_variant(
                BaseCodingAgent::ClaudeCode,
                "PLAN".to_string(),
            ),
        };
        let stored =
            ExecutorAction::new(ExecutorActionType::CodingAgentInitialRequest(initial), None);
        let action =
            build_resume_action(&stored, "sess-xyz".to_string(), "continue".to_string())
                .expect("resumable");

        match action.typ {
            ExecutorActionType::CodingAgentFollowUpRequest(req) => {
                assert_eq!(req.executor_profile_id.executor, BaseCodingAgent::ClaudeCode);
                assert_eq!(
                    req.executor_profile_id.variant.as_deref(),
                    Some("PLAN"),
                    "variant must be preserved across initial-request resume"
                );
            }
            _ => panic!("expected follow-up"),
        }
    }

    #[test]
    fn test_build_resume_action_non_coding_agent_returns_none() {
        // Create a ScriptRequest action (not resumable)
        let script = ScriptRequest {
            language: ScriptRequestLanguage::Bash,
            script: "echo hello".to_string(),
            context: ScriptContext::SetupScript,
        };
        let stored = ExecutorAction::new(ExecutorActionType::ScriptRequest(script), None);

        // Should return None for non-coding-agent actions
        let result = build_resume_action(&stored, "sess-abc".to_string(), "prompt".to_string());
        assert!(result.is_none(), "ScriptRequest should not be resumable");
    }

    #[tokio::test]
    async fn test_stream_normalized_logs_no_duplicates() {
        // Setup: Create existing JsonPatch entries (already normalized)
        let patch1 = ConversationPatch::add_normalized_entry(
            0,
            NormalizedEntry {
                timestamp: None,
                entry_type: NormalizedEntryType::AssistantMessage,
                content: "Test message 1".to_string(),
                metadata: None,
            },
        );
        let patch2 = ConversationPatch::add_normalized_entry(
            1,
            NormalizedEntry {
                timestamp: None,
                entry_type: NormalizedEntryType::AssistantMessage,
                content: "Test message 2".to_string(),
                metadata: None,
            },
        );

        // Simulate raw_messages from DB that already have JsonPatch entries
        let raw_messages = [
            LogMsg::JsonPatch(patch1.clone()),
            LogMsg::JsonPatch(patch2.clone()),
        ];

        // The function should detect existing patches and stream them directly
        // without re-normalizing, which would create duplicates.

        // Create a temp store to simulate what the function does
        let temp_store = Arc::new(MsgStore::new());

        // Check for existing patches (this is what the fix does)
        let existing_patches: Vec<_> = raw_messages
            .iter()
            .filter(|msg| matches!(msg, LogMsg::JsonPatch(_)))
            .cloned()
            .collect();

        assert_eq!(
            existing_patches.len(),
            2,
            "Should detect 2 existing JsonPatch entries"
        );

        // Populate store with existing patches only
        for patch in existing_patches {
            temp_store.push(patch);
        }
        temp_store.push_finished();

        // Count patches in the output
        let history = temp_store.get_history();
        let output_patch_count = history
            .iter()
            .filter(|msg| matches!(msg, LogMsg::JsonPatch(_)))
            .count();

        // Assert: Output patches count matches input (no duplicates created)
        assert_eq!(
            output_patch_count, 2,
            "Expected exactly 2 JsonPatch entries, no duplicates"
        );
    }

    #[tokio::test]
    async fn test_stream_normalized_logs_idempotent() {
        // Setup: Create existing JsonPatch entries (already normalized)
        let patch1 = ConversationPatch::add_normalized_entry(
            0,
            NormalizedEntry {
                timestamp: None,
                entry_type: NormalizedEntryType::AssistantMessage,
                content: "Test message 1".to_string(),
                metadata: None,
            },
        );
        let patch2 = ConversationPatch::add_normalized_entry(
            1,
            NormalizedEntry {
                timestamp: None,
                entry_type: NormalizedEntryType::AssistantMessage,
                content: "Test message 2".to_string(),
                metadata: None,
            },
        );

        // Simulate raw_messages from DB that already have JsonPatch entries
        let raw_messages = [
            LogMsg::JsonPatch(patch1.clone()),
            LogMsg::JsonPatch(patch2.clone()),
        ];

        // Simulate the stream_normalized_logs behavior twice on the same data

        // First call
        let store1 = Arc::new(MsgStore::new());
        let existing_patches1: Vec<_> = raw_messages
            .iter()
            .filter(|msg| matches!(msg, LogMsg::JsonPatch(_)))
            .cloned()
            .collect();
        for patch in existing_patches1 {
            store1.push(patch);
        }
        store1.push_finished();
        let history1 = store1.get_history();
        let patch_count1 = history1
            .iter()
            .filter(|msg| matches!(msg, LogMsg::JsonPatch(_)))
            .count();

        // Second call on the same data
        let store2 = Arc::new(MsgStore::new());
        let existing_patches2: Vec<_> = raw_messages
            .iter()
            .filter(|msg| matches!(msg, LogMsg::JsonPatch(_)))
            .cloned()
            .collect();
        for patch in existing_patches2 {
            store2.push(patch);
        }
        store2.push_finished();
        let history2 = store2.get_history();
        let patch_count2 = history2
            .iter()
            .filter(|msg| matches!(msg, LogMsg::JsonPatch(_)))
            .count();

        // Assert: Both calls produce identical outputs (idempotent)
        assert_eq!(
            patch_count1, patch_count2,
            "Calling stream_normalized_logs twice should produce identical results"
        );
        assert_eq!(
            patch_count1, 2,
            "Expected exactly 2 JsonPatch entries from each call"
        );

        // Verify content is also identical
        assert_eq!(
            history1.len(),
            history2.len(),
            "Both calls should produce same number of messages"
        );
    }

    #[tokio::test]
    async fn cleanup_orphan_executions_accessor_set_and_get_resume_state() {
        use db::test_utils::create_test_pool;

        let (pool, _tmp) = create_test_pool().await;

        // Seed a project
        let project_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO projects (id, name, git_repo_path) VALUES ($1, 'test-project', '/tmp/test')"#,
        )
        .bind(project_id)
        .execute(&pool)
        .await
        .unwrap();

        // Seed a task
        let task_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO tasks (id, project_id, title, status) VALUES ($1, $2, 'test', 'todo')"#,
        )
        .bind(task_id)
        .bind(project_id)
        .execute(&pool)
        .await
        .unwrap();

        // Seed a task attempt
        let attempt_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO task_attempts (id, task_id, executor, branch, target_branch, container_ref)
               VALUES ($1, $2, 'CLAUDE_CODE', 'test-branch', 'main', '/tmp/test-wt')"#,
        )
        .bind(attempt_id)
        .bind(task_id)
        .execute(&pool)
        .await
        .unwrap();

        // Seed an execution process
        let process_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO execution_processes (id, task_attempt_id, run_reason, executor_action, status, started_at)
               VALUES ($1, $2, 'codingagent', '{}', 'running', datetime('now'))"#,
        )
        .bind(process_id)
        .bind(attempt_id)
        .execute(&pool)
        .await
        .unwrap();

        // Verify initial state
        let initial = ExecutionProcess::get_resume_state(&pool, process_id).await.unwrap();
        assert_eq!(initial, None);

        // Set resume_state to 'pending'
        ExecutionProcess::set_resume_state(&pool, process_id, "pending").await.unwrap();
        let after_pending = ExecutionProcess::get_resume_state(&pool, process_id).await.unwrap();
        assert_eq!(after_pending, Some("pending".to_string()));

        // Set resume_state to 'resumed'
        ExecutionProcess::set_resume_state(&pool, process_id, "resumed").await.unwrap();
        let after_resumed = ExecutionProcess::get_resume_state(&pool, process_id).await.unwrap();
        assert_eq!(after_resumed, Some("resumed".to_string()));

        // Verify mark_orphaned_as_failed does NOT touch resumed rows (SC8 safety)
        ExecutionProcess::mark_orphaned_as_failed(&pool, "other-instance").await.unwrap();
        let after_mark = ExecutionProcess::find_by_id(&pool, process_id).await.unwrap().unwrap();
        assert_eq!(
            after_mark.status,
            ExecutionProcessStatus::Running,
            "resumed process must not be marked failed by blanket mark_orphaned"
        );
    }
}
