//! WebSocket session handler for node connections.
//!
//! This module handles the lifecycle of a single node WebSocket connection,
//! including authentication, message routing, and heartbeat management.

use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use sqlx::PgPool;
use tokio::{
    sync::mpsc,
    time::{self, MissedTickBehavior},
};
use tracing::{Span, instrument};
use uuid::Uuid;

use super::{
    connection::ConnectionManager,
    message::{
        AttemptSyncMessage, AuthResultMessage, BackfillResponseMessage, DeregisterMessage,
        ExecutionSyncMessage, HeartbeatMessage, HiveMessage, LinkProjectMessage, LinkedProjectInfo,
        LogsBatchMessage, NodeMessage, NodeRemovedMessage, OutboxOp, PROTOCOL_VERSION,
        ProjectSyncMessage, ProjectsSyncMessage, SwarmLabelInfo, TaskExecutionStatus,
        TaskOutputMessage, TaskProgressMessage, TaskStatusMessage, TaskSyncMessage,
        TaskSyncResponseMessage, UnlinkProjectMessage,
    },
};
use crate::{
    db::node_task_attempts::NodeTaskAttemptRepository,
    nodes::{
        BackfillService,
        backfill::BackfillRequestTracker,
        domain::NodeStatus,
        service::{NodeServiceImpl, RegisterNode},
    },
};

/// Heartbeat timeout - close connection if no heartbeat received.
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(90);

/// Lease TTL granted on renewal. Must exceed the node's heartbeat/renew cadence (task 206)
/// so a renewing node never expires between heartbeats.
const LEASE_TTL: chrono::Duration = chrono::Duration::seconds(60);

/// Channel buffer size for outgoing messages.
const OUTGOING_BUFFER_SIZE: usize = 64;

/// Returns the final path component of a git repository path.
///
/// Trims trailing '/' and '\' characters and supports both Unix and Windows separators. If the input contains no separators, or is empty, the original input is returned.
///
/// # Examples
///
/// ```
/// assert_eq!(extract_project_name("https://example.com/org/repo.git"), "repo.git");
/// assert_eq!(extract_project_name("C:\\path\\to\\project\\"), "project");
/// assert_eq!(extract_project_name("/single_component"), "single_component");
/// assert_eq!(extract_project_name(""), "");
/// ```
fn extract_project_name(git_repo_path: &str) -> String {
    let trimmed = git_repo_path.trim_end_matches(['/', '\\']);
    let candidate = if trimmed.is_empty() {
        git_repo_path
    } else {
        trimmed
    };
    candidate
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(candidate)
        .to_string()
}

/// Handle a new node WebSocket connection.
#[instrument(
    name = "node_ws.session",
    skip(socket, pool, connections, backfill),
    fields(
        node_id = tracing::field::Empty,
        org_id = tracing::field::Empty,
        machine_id = tracing::field::Empty
    )
)]
pub async fn handle(
    socket: WebSocket,
    pool: PgPool,
    connections: ConnectionManager,
    backfill: Arc<BackfillService>,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<HiveMessage>(OUTGOING_BUFFER_SIZE);

    // Wait for authentication message
    let auth_result = match wait_for_auth(&mut ws_receiver, &pool).await {
        Ok(result) => result,
        Err(error) => {
            tracing::info!(?error, "node authentication failed");
            let _ = send_message(
                &mut ws_sender,
                &HiveMessage::AuthResult(AuthResultMessage {
                    success: false,
                    node_id: None,
                    organization_id: None,
                    error: Some(error.to_string()),
                    protocol_version: PROTOCOL_VERSION,
                    linked_projects: vec![],
                    swarm_labels: vec![],
                }),
            )
            .await;
            return;
        }
    };

    // Record context in span
    Span::current().record("node_id", format_args!("{}", auth_result.node_id));
    Span::current().record("org_id", format_args!("{}", auth_result.organization_id));

    // Clone projects for broadcast before moving into response
    let projects_for_broadcast = auth_result.linked_projects.clone();

    tracing::info!(
        node_id = %auth_result.node_id,
        org_id = %auth_result.organization_id,
        linked_projects = auth_result.linked_projects.len(),
        swarm_labels = auth_result.swarm_labels.len(),
        "sending auth success with swarm labels"
    );

    // Send auth success response
    if send_message(
        &mut ws_sender,
        &HiveMessage::AuthResult(AuthResultMessage {
            success: true,
            node_id: Some(auth_result.node_id),
            organization_id: Some(auth_result.organization_id),
            error: None,
            protocol_version: PROTOCOL_VERSION,
            linked_projects: auth_result.linked_projects,
            swarm_labels: auth_result.swarm_labels,
        }),
    )
    .await
    .is_err()
    {
        return;
    }

    // Register connection
    connections
        .register(auth_result.node_id, auth_result.organization_id, tx)
        .await;

    // Broadcast this node's projects to other nodes in the organization
    broadcast_node_projects(
        auth_result.node_id,
        auth_result.organization_id,
        &auth_result.node_name,
        auth_result.node_public_url.as_deref(),
        &projects_for_broadcast,
        &pool,
        &connections,
    )
    .await;

    // Trigger backfill for incomplete attempts (non-blocking)
    let node_id_for_backfill = auth_result.node_id;
    let backfill_service = Arc::clone(&backfill);
    tokio::spawn(async move {
        match backfill_service
            .trigger_reconnect_backfill(node_id_for_backfill)
            .await
        {
            Ok(count) => {
                if count > 0 {
                    tracing::info!(
                        node_id = %node_id_for_backfill,
                        count = count,
                        "triggered backfill for incomplete attempts on reconnect"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    node_id = %node_id_for_backfill,
                    error = %e,
                    "failed to trigger backfill on reconnect"
                );
            }
        }
    });

    // Set up heartbeat timeout
    let mut heartbeat_timeout = time::interval(HEARTBEAT_TIMEOUT);
    heartbeat_timeout.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut last_heartbeat = Utc::now();

    tracing::info!(
        node_id = %auth_result.node_id,
        organization_id = %auth_result.organization_id,
        "node session started"
    );

    // Get tracker for correlating backfill responses
    let tracker = backfill.tracker();

    // Main message loop
    loop {
        tokio::select! {
            // Handle outgoing messages from hive
            Some(msg) = rx.recv() => {
                if send_message(&mut ws_sender, &msg).await.is_err() {
                    break;
                }
            }

            // Handle incoming messages from node
            maybe_message = ws_receiver.next() => {
                match maybe_message {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<NodeMessage>(&text) {
                            Ok(msg) => {
                                if let Err(error) = handle_node_message(
                                    &msg,
                                    auth_result.node_id,
                                    auth_result.organization_id,
                                    &auth_result.node_name,
                                    &pool,
                                    &connections,
                                    &mut ws_sender,
                                    &mut last_heartbeat,
                                    &tracker,
                                ).await {
                                    tracing::warn!(?error, "error handling node message");
                                }
                            }
                            Err(error) => {
                                tracing::debug!(?error, "invalid node message");
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::debug!("node sent close frame");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if ws_sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(_)) => {
                        // Ignore other message types
                    }
                    Some(Err(error)) => {
                        tracing::debug!(?error, "websocket receive error");
                        break;
                    }
                    None => break,
                }
            }

            // Check heartbeat timeout
            _ = heartbeat_timeout.tick() => {
                let elapsed = Utc::now().signed_duration_since(last_heartbeat);
                if elapsed > chrono::Duration::from_std(HEARTBEAT_TIMEOUT).unwrap() {
                    tracing::warn!(
                        node_id = %auth_result.node_id,
                        elapsed_secs = elapsed.num_seconds(),
                        "node heartbeat timeout"
                    );
                    let _ = send_message(
                        &mut ws_sender,
                        &HiveMessage::Close {
                            reason: "heartbeat timeout".to_string(),
                        },
                    ).await;
                    break;
                }
            }
        }
    }

    // Clean up
    connections.unregister(auth_result.node_id).await;

    // Clear in-memory backfill tracking for this node.
    // NOTE: We intentionally do NOT reset attempts to partial here. The backfill_request_id
    // stored in the database allows delayed responses to be processed correctly even after
    // the in-memory tracker is cleared. The stale timeout mechanism will eventually reset
    // attempts that truly failed.
    let cleared_count = tracker.clear_node(auth_result.node_id).await.len();
    if cleared_count > 0 {
        tracing::info!(
            node_id = %auth_result.node_id,
            count = cleared_count,
            "cleared in-memory backfill tracking on disconnect"
        );
    }

    // Update node status to offline
    let service = NodeServiceImpl::new(pool.clone());
    if let Err(error) = service
        .update_node_status(auth_result.node_id, NodeStatus::Offline)
        .await
    {
        tracing::warn!(?error, "failed to update node status to offline");
    }

    tracing::info!(
        node_id = %auth_result.node_id,
        "node session ended"
    );
}

/// Result of successful authentication.
struct AuthResult {
    node_id: Uuid,
    organization_id: Uuid,
    node_name: String,
    node_public_url: Option<String>,
    linked_projects: Vec<LinkedProjectInfo>,
    /// Swarm labels for the organization (synced to nodes on connect)
    swarm_labels: Vec<SwarmLabelInfo>,
}

/// Authentication error.
#[derive(Debug, thiserror::Error)]
enum AuthError {
    #[error("timeout waiting for auth message")]
    Timeout,
    #[error("connection closed before auth")]
    ConnectionClosed,
    #[error("invalid auth message: {0}")]
    InvalidMessage(String),
    #[error("invalid API key")]
    InvalidApiKey,
    #[error("API key revoked")]
    ApiKeyRevoked,
    #[error("API key blocked: {0}")]
    ApiKeyBlocked(String),
    #[error("takeover detected: {0}")]
    TakeoverDetected(String),
    #[error("protocol version mismatch: client={client}, server={server}")]
    ProtocolMismatch { client: u32, server: u32 },
    #[error("registration failed: {0}")]
    RegistrationFailed(String),
}

/// Wait for and process the authentication message.
async fn wait_for_auth(
    receiver: &mut futures::stream::SplitStream<WebSocket>,
    pool: &PgPool,
) -> Result<AuthResult, AuthError> {
    // Wait for auth message with timeout
    let auth_timeout = Duration::from_secs(30);
    let message = tokio::time::timeout(auth_timeout, receiver.next())
        .await
        .map_err(|_| AuthError::Timeout)?
        .ok_or(AuthError::ConnectionClosed)?
        .map_err(|e| AuthError::InvalidMessage(e.to_string()))?;

    let text = match message {
        Message::Text(text) => text,
        _ => {
            return Err(AuthError::InvalidMessage(
                "expected text message".to_string(),
            ));
        }
    };

    let node_msg: NodeMessage =
        serde_json::from_str(&text).map_err(|e| AuthError::InvalidMessage(e.to_string()))?;

    let auth = match node_msg {
        NodeMessage::Auth(auth) => auth,
        _ => {
            return Err(AuthError::InvalidMessage(
                "expected auth message".to_string(),
            ));
        }
    };

    Span::current().record("machine_id", &auth.machine_id);

    // Validate protocol version
    if auth.protocol_version != PROTOCOL_VERSION {
        return Err(AuthError::ProtocolMismatch {
            client: auth.protocol_version,
            server: PROTOCOL_VERSION,
        });
    }

    // Validate API key and get organization
    let service = NodeServiceImpl::new(pool.clone());
    let api_key = service.validate_api_key(&auth.api_key).await.map_err(|e| {
        tracing::debug!(?e, "API key validation failed");
        match e {
            crate::nodes::service::NodeError::ApiKeyRevoked => AuthError::ApiKeyRevoked,
            crate::nodes::service::NodeError::ApiKeyBlocked(reason) => {
                AuthError::ApiKeyBlocked(reason)
            }
            _ => AuthError::InvalidApiKey,
        }
    })?;

    // Check if API key is blocked (additional check after validation)
    if let Some(reason) = &api_key.blocked_reason {
        tracing::info!(
            key_id = %api_key.id,
            key_name = %api_key.name,
            reason = %reason,
            "Rejecting blocked API key"
        );
        return Err(AuthError::ApiKeyBlocked(reason.clone()));
    }

    // Store name and public_url for later use in broadcasts
    let node_name = auth.name.clone();
    let node_public_url = auth.public_url.clone();

    // Register or update the node using API key-based identity
    let register_data = RegisterNode {
        name: auth.name,
        machine_id: auth.machine_id,
        capabilities: auth.capabilities,
        public_url: auth.public_url,
    };

    let node = service
        .register_node_with_api_key(&api_key, register_data)
        .await
        .map_err(|e| {
            tracing::debug!(?e, "Node registration failed");
            match e {
                crate::nodes::service::NodeError::TakeoverDetected(msg) => {
                    AuthError::TakeoverDetected(msg)
                }
                crate::nodes::service::NodeError::ApiKeyBlocked(reason) => {
                    AuthError::ApiKeyBlocked(reason)
                }
                _ => AuthError::RegistrationFailed(e.to_string()),
            }
        })?;

    // Get only swarm projects this node is linked to (not all org projects)
    // This prevents visibility leak where nodes see other nodes' unlinked projects
    use crate::db::swarm_projects::SwarmProjectRepository;
    let swarm_projects = SwarmProjectRepository::list_for_node_auth(pool, node.id)
        .await
        .map_err(|e| AuthError::RegistrationFailed(e.to_string()))?;

    // Convert to LinkedProjectInfo with ownership info
    // In the new swarm architecture, all returned projects are "owned" by this node
    // since we only return projects the node is linked to
    let linked_projects = swarm_projects
        .into_iter()
        .map(|p| LinkedProjectInfo {
            link_id: p.link_id,
            project_id: p.swarm_project_id, // Use swarm_project_id as project_id
            local_project_id: p.local_project_id,
            git_repo_path: p.git_repo_path,
            default_branch: p.default_branch,
            project_name: p.project_name,
            source_node_id: p.source_node_id,
            source_node_name: p.source_node_name,
            is_owned: true, // Node is always linked to projects it receives
        })
        .collect();

    // Fetch swarm labels for the organization (org-global labels with project_id = NULL)
    // These are synced to nodes on connection so they can use them for swarm tasks
    let swarm_labels = {
        use crate::db::labels::LabelRepository;
        let label_repo = LabelRepository::new(pool);
        label_repo
            .find_swarm_labels(node.organization_id)
            .await
            .map_err(|e| AuthError::RegistrationFailed(format!("failed to fetch labels: {}", e)))?
            .into_iter()
            .map(|l| SwarmLabelInfo {
                id: l.id,
                name: l.name,
                icon: l.icon,
                color: l.color,
                version: l.version,
            })
            .collect()
    };

    Ok(AuthResult {
        node_id: node.id,
        organization_id: node.organization_id,
        node_name,
        node_public_url,
        linked_projects,
        swarm_labels,
    })
}

/// Handle an incoming message from a node.
#[allow(clippy::too_many_arguments)]
async fn handle_node_message(
    msg: &NodeMessage,
    node_id: Uuid,
    organization_id: Uuid,
    node_name: &str,
    pool: &PgPool,
    connections: &ConnectionManager,
    ws_sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    last_heartbeat: &mut chrono::DateTime<Utc>,
    tracker: &BackfillRequestTracker,
) -> Result<(), HandleError> {
    match msg {
        NodeMessage::Heartbeat(heartbeat) => {
            handle_heartbeat(
                node_id,
                heartbeat,
                pool,
                connections,
                ws_sender,
                last_heartbeat,
            )
            .await
        }
        NodeMessage::TaskStatus(status) => {
            handle_task_status(node_id, organization_id, status, pool).await
        }
        NodeMessage::TaskOutput(output) => handle_task_output(node_id, output, pool).await,
        NodeMessage::TaskProgress(progress) => handle_task_progress(node_id, progress, pool).await,
        NodeMessage::LinkProject(link) => {
            handle_link_project(node_id, organization_id, link, pool, connections).await
        }
        NodeMessage::UnlinkProject(unlink) => {
            handle_unlink_project(node_id, organization_id, unlink, pool, connections).await
        }
        NodeMessage::Deregister(deregister) => {
            handle_deregister(node_id, organization_id, deregister, pool, connections).await
        }
        NodeMessage::AttemptSync(attempt) => handle_attempt_sync(node_id, attempt, pool).await,
        NodeMessage::ExecutionSync(execution) => {
            handle_execution_sync(node_id, execution, pool).await
        }
        NodeMessage::LogsBatch(logs) => handle_logs_batch(node_id, logs, pool).await,
        NodeMessage::LabelSync(_label) => {
            // Labels are no longer synced from nodes to hive.
            // Labels are managed centrally on the hive and synced DOWN to nodes.
            // Ignore incoming label sync messages from nodes.
            tracing::debug!(
                node_id = %node_id,
                "ignoring deprecated label sync from node - labels are now hive-managed"
            );
            Ok(())
        }
        NodeMessage::TaskSync(task) => {
            handle_task_sync(node_id, organization_id, node_name, task, pool, ws_sender).await
        }
        NodeMessage::ProjectsSync(projects) => handle_projects_sync(node_id, projects, pool).await,
        NodeMessage::Ack { message_id } => {
            tracing::trace!(node_id = %node_id, message_id = %message_id, "received ack");
            Ok(())
        }
        NodeMessage::Error { message_id, error } => {
            tracing::warn!(
                node_id = %node_id,
                message_id = ?message_id,
                error = %error,
                "received error from node"
            );
            Ok(())
        }
        NodeMessage::Auth(_) => {
            // Auth should only happen once at connection start
            tracing::warn!(node_id = %node_id, "received unexpected auth message");
            Ok(())
        }
        NodeMessage::BackfillResponse(response) => {
            handle_backfill_response(node_id, response, pool, tracker).await
        }
        NodeMessage::OpBatch { ops } => {
            handle_op_batch(node_id, organization_id, node_name, ops, pool, ws_sender).await
        }
        NodeMessage::LeaseHeartbeat { assignment_ids } => {
            handle_lease_heartbeat(node_id, assignment_ids, pool, ws_sender).await
        }
        NodeMessage::Digest { entries } => {
            // STUB — filled by task 503 (compare against shared_tasks + node_op_log, reply DigestResult).
            // Logs so the exhaustive match compiles now; 503 replaces the body with handle_digest(...).
            tracing::debug!(node_id = %node_id, entry_count = entries.len(), "received digest (compare TODO: task 503)");
            Ok(())
        }
    }
}

/// Handle a heartbeat message.
async fn handle_heartbeat(
    node_id: Uuid,
    heartbeat: &HeartbeatMessage,
    pool: &PgPool,
    connections: &ConnectionManager,
    ws_sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    last_heartbeat: &mut chrono::DateTime<Utc>,
) -> Result<(), HandleError> {
    *last_heartbeat = Utc::now();

    // Update connection status
    connections
        .update_status(node_id, heartbeat.status, heartbeat.active_tasks)
        .await;

    // Update database
    let service = NodeServiceImpl::new(pool.clone());
    service
        .update_node_status(node_id, heartbeat.status)
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?;

    // Send heartbeat acknowledgement
    send_message(
        ws_sender,
        &HiveMessage::HeartbeatAck {
            server_time: Utc::now(),
        },
    )
    .await
    .map_err(|_| HandleError::Send)?;

    tracing::trace!(
        node_id = %node_id,
        status = ?heartbeat.status,
        active_tasks = heartbeat.active_tasks,
        "processed heartbeat"
    );

    Ok(())
}

/// Handle a task status update.
async fn handle_task_status(
    node_id: Uuid,
    _organization_id: Uuid,
    status: &TaskStatusMessage,
    pool: &PgPool,
) -> Result<(), HandleError> {
    use crate::db::task_assignments::TaskAssignmentRepository;
    use crate::db::tasks::{SharedTaskRepository, TaskStatus};

    let service = NodeServiceImpl::new(pool.clone());

    // Map execution status to database status string
    let db_status = match status.status {
        TaskExecutionStatus::Pending => "pending",
        TaskExecutionStatus::Starting => "starting",
        TaskExecutionStatus::Running => "running",
        TaskExecutionStatus::Completed => "completed",
        TaskExecutionStatus::Failed => "failed",
        TaskExecutionStatus::Cancelled => "cancelled",
    };

    // Guard the entire handler on an active, unexpired lease (tournament R1/A + R2/A).
    // The legacy `find_by_id` filtered only on `id`, so any node that knew the
    // assignment_id could mutate the assignment row AND propagate a shared-task status.
    // `find_active_lease_for_node` guards on `node_id`, `completed_at IS NULL`, AND
    // `lease_expires_at > NOW()` — only the current, unexpired lease-holder may write.
    // This gate precedes the assignment-row writes below so a node whose lease has
    // expired (or was never held) cannot mutate `execution_status` or `local_task_id`.
    let assignment_repo = TaskAssignmentRepository::new(pool);
    let assignment = match assignment_repo
        .find_active_lease_for_node(status.assignment_id, node_id)
        .await
    {
        Ok(Some(a)) => a,
        Ok(None) => {
            tracing::warn!(
                node_id = %node_id,
                assignment_id = %status.assignment_id,
                "rejected task status update — no active lease for this node (ADR-0010 §D)"
            );
            return Ok(());
        }
        Err(e) => return Err(HandleError::Database(e.to_string())),
    };

    // Update local IDs if provided
    if status.local_task_id.is_some() || status.local_attempt_id.is_some() {
        service
            .update_assignment_local_ids(
                status.assignment_id,
                status.local_task_id,
                status.local_attempt_id,
            )
            .await
            .map_err(|e| HandleError::Database(e.to_string()))?;
    }

    // Update execution status on the assignment
    service
        .update_assignment_status(status.assignment_id, db_status)
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?;

    // Also update the shared task status
    {
        // Map execution status to the proposed shared task status.
        let proposed = match status.status {
            TaskExecutionStatus::Pending | TaskExecutionStatus::Starting => TaskStatus::Todo,
            TaskExecutionStatus::Running => TaskStatus::InProgress,
            TaskExecutionStatus::Completed => TaskStatus::InReview,
            TaskExecutionStatus::Failed | TaskExecutionStatus::Cancelled => TaskStatus::Todo,
        };

        let task_repo = SharedTaskRepository::new(pool);
        // Guard via the single-author matrix (ADR-0010 §D). The legacy path is node-reported, so only a
        // no-op or a node-authored transition (with an active lease) may write — never a hive-authored
        // or illegal transition (the old `*→Todo` clobber).
        match task_repo.find_by_id(assignment.task_id).await {
            Ok(Some(current_task)) if current_task.status == proposed => {
                // no-op — nothing to write
            }
            Ok(Some(current_task))
                if crate::nodes::ws::status_machine::node_may_author(
                    current_task.status,
                    proposed,
                ) =>
            {
                if let Err(e) = task_repo
                    .update_status_from_node(assignment.task_id, proposed)
                    .await
                {
                    tracing::warn!(task_id = %assignment.task_id, error = %e,
                        "failed to update shared task status");
                }
            }
            Ok(Some(current_task)) => {
                tracing::warn!(task_id = %assignment.task_id, from = ?current_task.status,
                    to = ?proposed,
                    "rejected non-node-authored status transition on legacy path (ADR-0010)");
            }
            Ok(None) => {
                tracing::warn!(task_id = %assignment.task_id, "shared task not found for status update");
            }
            Err(e) => {
                tracing::warn!(task_id = %assignment.task_id, error = %e,
                    "failed to read shared task status");
            }
        }
    }

    tracing::info!(
        node_id = %node_id,
        assignment_id = %status.assignment_id,
        status = ?status.status,
        "task status updated"
    );

    Ok(())
}

/// Handle task output/log messages from a node.
async fn handle_task_output(
    node_id: Uuid,
    output: &TaskOutputMessage,
    pool: &PgPool,
) -> Result<(), HandleError> {
    use crate::db::task_output_logs::{CreateTaskOutputLog, TaskOutputLogRepository};

    let output_type = match output.output_type {
        super::message::TaskOutputType::Stdout => "stdout",
        super::message::TaskOutputType::Stderr => "stderr",
        super::message::TaskOutputType::System => "system",
    };

    let repo = TaskOutputLogRepository::new(pool);
    repo.create(CreateTaskOutputLog {
        assignment_id: output.assignment_id,
        output_type: output_type.to_string(),
        content: output.content.clone(),
        timestamp: output.timestamp,
    })
    .await
    .map_err(|e| HandleError::Database(e.to_string()))?;

    tracing::trace!(
        node_id = %node_id,
        assignment_id = %output.assignment_id,
        output_type = %output_type,
        content_len = output.content.len(),
        "stored task output"
    );

    Ok(())
}

/// Handle task progress events from a node.
async fn handle_task_progress(
    node_id: Uuid,
    progress: &TaskProgressMessage,
    pool: &PgPool,
) -> Result<(), HandleError> {
    use crate::db::task_progress_events::{CreateTaskProgressEvent, TaskProgressEventRepository};

    let event_type = format!("{:?}", progress.event_type).to_lowercase();

    let repo = TaskProgressEventRepository::new(pool);
    repo.create(CreateTaskProgressEvent {
        assignment_id: progress.assignment_id,
        event_type,
        message: progress.message.clone(),
        metadata: progress.metadata.clone(),
        timestamp: progress.timestamp,
    })
    .await
    .map_err(|e| HandleError::Database(e.to_string()))?;

    tracing::debug!(
        node_id = %node_id,
        assignment_id = %progress.assignment_id,
        event_type = ?progress.event_type,
        "stored task progress event"
    );

    Ok(())
}

/// Create a link between a node and a project, persist the link, and broadcast a ProjectSync
/// message to all other nodes in the organization.
///
/// This links a node's local project to a swarm project by:
/// 1. Creating/updating the swarm_project_nodes record
/// 2. Updating node_local_projects.swarm_project_id
/// 3. Broadcasting the link to other nodes
///
/// Note: The `project_id` in LinkProjectMessage is now interpreted as `swarm_project_id`.
///
/// The broadcast excludes the originating node. The project name included in the broadcast is
/// derived from the provided `git_repo_path`. Returns an error if the database operation or the
/// broadcast fails.
async fn handle_link_project(
    node_id: Uuid,
    organization_id: Uuid,
    link: &LinkProjectMessage,
    pool: &PgPool,
    connections: &ConnectionManager,
) -> Result<(), HandleError> {
    use crate::db::node_local_projects::NodeLocalProjectRepository;
    use crate::db::swarm_projects::{LinkSwarmProjectNodeData, SwarmProjectRepository};

    let swarm_project_id = link.project_id; // project_id is now swarm_project_id

    // Verify the swarm project exists and belongs to this organization
    let swarm_project = SwarmProjectRepository::find_by_id(pool, swarm_project_id)
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?
        .ok_or_else(|| {
            HandleError::Database(format!("swarm project {} not found", swarm_project_id))
        })?;

    if swarm_project.organization_id != organization_id {
        return Err(HandleError::Database(format!(
            "swarm project {} does not belong to organization {}",
            swarm_project_id, organization_id
        )));
    }

    // Create/update swarm_project_nodes record
    let swarm_link = SwarmProjectRepository::link_node_pool(
        pool,
        LinkSwarmProjectNodeData {
            swarm_project_id,
            node_id,
            local_project_id: link.local_project_id,
            git_repo_path: link.git_repo_path.clone(),
            os_type: None, // OS type can be added to LinkProjectMessage later if needed
        },
    )
    .await
    .map_err(|e| HandleError::Database(e.to_string()))?;

    // Update node_local_projects.swarm_project_id (upserts if record doesn't exist)
    if let Err(e) = NodeLocalProjectRepository::link_to_swarm_with_upsert(
        pool,
        node_id,
        link.local_project_id,
        swarm_project_id,
        &link.git_repo_path,
        &link.default_branch,
    )
    .await
    {
        tracing::warn!(
            node_id = %node_id,
            swarm_project_id = %swarm_project_id,
            local_project_id = %link.local_project_id,
            error = ?e,
            "failed to update node_local_projects.swarm_project_id (non-fatal)"
        );
    }

    tracing::info!(
        node_id = %node_id,
        swarm_project_id = %swarm_project_id,
        local_project_id = %link.local_project_id,
        git_repo_path = %link.git_repo_path,
        "linked project to swarm"
    );

    // Broadcast the new project link to other nodes
    let service = NodeServiceImpl::new(pool.clone());
    let node = service
        .get_node(node_id)
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?;

    let project_name = extract_project_name(&link.git_repo_path);

    let sync_msg = HiveMessage::ProjectSync(ProjectSyncMessage {
        message_id: Uuid::new_v4(),
        link_id: swarm_link.id,
        project_id: swarm_project_id, // This is now the swarm_project_id
        project_name,
        local_project_id: link.local_project_id,
        git_repo_path: link.git_repo_path.clone(),
        default_branch: link.default_branch.clone(),
        source_node_id: node_id,
        source_node_name: node.name,
        source_node_public_url: node.public_url,
        is_new: true,
    });

    // Only broadcast to nodes that are linked to the same swarm project
    let linked_nodes = SwarmProjectRepository::get_linked_node_ids(pool, swarm_project_id)
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?;

    let target_nodes: Vec<_> = linked_nodes
        .into_iter()
        .filter(|&id| id != node_id)
        .collect();

    if target_nodes.is_empty() {
        tracing::debug!(
            node_id = %node_id,
            swarm_project_id = %swarm_project_id,
            "no other nodes linked to swarm project, skipping broadcast"
        );
        return Ok(());
    }

    let failed = connections.send_to_nodes(&target_nodes, sync_msg).await;

    if !failed.is_empty() {
        tracing::warn!(
            node_id = %node_id,
            swarm_project_id = %swarm_project_id,
            failed_count = failed.len(),
            "failed to broadcast project link to some nodes"
        );
    }

    Ok(())
}

/// Unlinks a node's local project from its swarm project and broadcasts the removal to other nodes in the organization.
///
/// This removes the swarm_project_nodes link, clears the corresponding node_local_projects.swarm_project_id when the local project is known, and — if the link existed — sends a `ProjectSync` message with `is_new = false` containing the project's name, default branch (falls back to `"main"`), and source node information to all other nodes in the same organization.
///
/// # Returns
///
/// `Ok(())` on success, `Err(HandleError)` if a database or send error occurs.
///
/// # Examples
///
/// ```no_run
/// use uuid::Uuid;
/// # async fn doc() {
/// let pool = /* PgPool */ todo!();
/// let connections = /* ConnectionManager */ todo!();
/// let unlink = /* UnlinkProjectMessage */ todo!();
/// handle_unlink_project(Uuid::nil(), Uuid::nil(), &unlink, &pool, &connections).await.unwrap();
/// # }
/// ```
async fn handle_unlink_project(
    node_id: Uuid,
    _organization_id: Uuid,
    unlink: &UnlinkProjectMessage,
    pool: &PgPool,
    connections: &ConnectionManager,
) -> Result<(), HandleError> {
    use crate::db::node_local_projects::NodeLocalProjectRepository;
    use crate::db::swarm_projects::SwarmProjectRepository;

    let swarm_project_id = unlink.project_id; // project_id is now swarm_project_id

    let service = NodeServiceImpl::new(pool.clone());

    // Get node info before unlink for the broadcast
    let node = service
        .get_node(node_id)
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?;

    // Find the swarm_project_nodes link to get the local_project_id
    let link_info = SwarmProjectRepository::find_node_link(pool, swarm_project_id, node_id)
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?;

    // Remove the swarm_project_nodes link
    if let Err(e) = SwarmProjectRepository::unlink_node_pool(pool, swarm_project_id, node_id).await
    {
        tracing::warn!(
            node_id = %node_id,
            swarm_project_id = %swarm_project_id,
            error = ?e,
            "failed to unlink swarm_project_nodes (may not exist)"
        );
    }

    // Clear node_local_projects.swarm_project_id if we know the local_project_id
    if let Some(ref link) = link_info
        && let Err(e) =
            NodeLocalProjectRepository::unlink_from_swarm_pool(pool, node_id, link.local_project_id)
                .await
    {
        tracing::warn!(
            node_id = %node_id,
            local_project_id = %link.local_project_id,
            error = ?e,
            "failed to clear node_local_projects.swarm_project_id (non-fatal)"
        );
    }

    tracing::info!(
        node_id = %node_id,
        swarm_project_id = %swarm_project_id,
        "unlinked project from swarm"
    );

    // Broadcast the unlink to other nodes (only if we found the link info)
    if let Some(link) = link_info {
        let project_name = extract_project_name(&link.git_repo_path);

        // Look up default_branch from node_local_projects (or use "main" as fallback)
        let default_branch = NodeLocalProjectRepository::find_by_node_and_project(
            pool,
            node_id,
            link.local_project_id,
        )
        .await
        .ok()
        .flatten()
        .map(|nlp| nlp.default_branch)
        .unwrap_or_else(|| "main".to_string());

        let sync_msg = HiveMessage::ProjectSync(ProjectSyncMessage {
            message_id: Uuid::new_v4(),
            link_id: link.id,
            project_id: swarm_project_id, // This is now the swarm_project_id
            project_name,
            local_project_id: link.local_project_id,
            git_repo_path: link.git_repo_path,
            default_branch,
            source_node_id: node_id,
            source_node_name: node.name,
            source_node_public_url: node.public_url,
            is_new: false, // false indicates removal
        });

        // Only broadcast to nodes that are still linked to this swarm project
        // (the unlinking node was already removed at this point)
        let linked_nodes = SwarmProjectRepository::get_linked_node_ids(pool, swarm_project_id)
            .await
            .unwrap_or_default();

        if linked_nodes.is_empty() {
            tracing::debug!(
                node_id = %node_id,
                swarm_project_id = %swarm_project_id,
                "no other nodes linked to swarm project, skipping unlink broadcast"
            );
        } else {
            let failed = connections.send_to_nodes(&linked_nodes, sync_msg).await;

            if !failed.is_empty() {
                tracing::warn!(
                    node_id = %node_id,
                    swarm_project_id = %swarm_project_id,
                    failed_count = failed.len(),
                    "failed to broadcast project unlink to some nodes"
                );
            }
        }
    }

    Ok(())
}

/// Deregisters a node by removing its database records and notifying the organization.
///
/// Deletes the node and its related data, then broadcasts a `NodeRemoved` message to other nodes in the same organization. Any failures to notify peers are logged but do not prevent completion of the deregistration.
///
/// # Examples
///
/// ```
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # // placeholders for required values
/// # let pool = todo!("PgPool instance");
/// # let connections = todo!("ConnectionManager instance");
/// use uuid::Uuid;
/// let node_id = Uuid::new_v4();
/// let org_id = Uuid::new_v4();
/// let dereg_msg = DeregisterMessage {
///     message_id: Uuid::new_v4(),
///     reason: Some("decommission".into()),
/// };
/// // Call the async function (this doc example is illustrative and uses placeholders)
/// let _ = handle_deregister(node_id, org_id, &dereg_msg, &pool, &connections).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Returns
///
/// `Ok(())` on success, or `Err(HandleError::Database(_))` if deleting the node from the database fails.
async fn handle_deregister(
    node_id: Uuid,
    organization_id: Uuid,
    deregister: &DeregisterMessage,
    pool: &PgPool,
    connections: &ConnectionManager,
) -> Result<(), HandleError> {
    tracing::info!(
        node_id = %node_id,
        message_id = %deregister.message_id,
        reason = ?deregister.reason,
        "node requesting deregistration"
    );

    let service = NodeServiceImpl::new(pool.clone());

    // Delete the node (cascades all related data: swarm_project_nodes, task_assignments)
    service
        .delete_node(node_id)
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?;

    // Broadcast node removal to all other nodes in the organization
    let removal_msg = HiveMessage::NodeRemoved(NodeRemovedMessage {
        node_id,
        reason: deregister
            .reason
            .clone()
            .unwrap_or_else(|| "Node deregistered".to_string()),
    });

    let failed = connections
        .broadcast_to_org(organization_id, removal_msg)
        .await;
    if !failed.is_empty() {
        tracing::warn!(
            node_id = %node_id,
            failed_count = failed.len(),
            "failed to notify some nodes of deregistration"
        );
    }

    tracing::info!(
        node_id = %node_id,
        "node deregistered successfully"
    );

    Ok(())
}

/// Error when handling a node message.
#[derive(Debug, thiserror::Error)]
enum HandleError {
    #[error("database error: {0}")]
    Database(String),
    #[error("failed to send message")]
    Send,
}

/// Sanitize a string by removing null bytes (0x00).
///
/// PostgreSQL does not allow null bytes in text fields, so this function
/// strips them to prevent "invalid byte sequence for encoding UTF8: 0x00" errors.
fn sanitize_string(s: &str) -> String {
    s.replace('\0', "")
}

/// Sanitize an optional string by removing null bytes.
fn sanitize_option_string(s: Option<String>) -> Option<String> {
    s.map(|v| sanitize_string(&v))
}

/// Send a message to the WebSocket.
async fn send_message(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    msg: &HiveMessage,
) -> Result<(), ()> {
    match serde_json::to_string(msg) {
        Ok(json) => sender
            .send(Message::Text(json.into()))
            .await
            .map_err(|error| {
                tracing::debug!(?error, "failed to send websocket message");
            }),
        Err(error) => {
            tracing::error!(?error, "failed to serialize message");
            Err(())
        }
    }
}

/// Upserts a node task attempt and, when the attempt has no `assignment_id`, tries to create a synthetic assignment for locally-started tasks.
///
/// If `attempt.assignment_id` is provided, that value is used. If it is `None`, the function looks up the shared task's `swarm_project_id`, finds the node's `swarm_project_nodes` link for that project, and attempts to create or find a synthetic assignment tied to that link. Failures to create or find a synthetic assignment are logged and do not prevent the attempt from being upserted; the attempt will be stored without an `assignment_id` in that case. The upsert writes the attempt data into the `node_task_attempts` table.
///
/// # Examples
///
/// ```
/// # async fn example(pool: &sqlx::PgPool) -> Result<(), Box<dyn std::error::Error>> {
/// use uuid::Uuid;
/// use chrono::Utc;
/// use crate::nodes::ws::messages::AttemptSyncMessage;
///
/// let node_id = Uuid::new_v4();
///
/// let attempt = AttemptSyncMessage {
///     attempt_id: Uuid::new_v4(),
///     shared_task_id: Uuid::new_v4(),
///     assignment_id: None,
///     executor: None,
///     executor_variant: None,
///     branch: None,
///     target_branch: None,
///     container_ref: None,
///     worktree_deleted: false,
///     setup_completed_at: None,
///     created_at: Utc::now(),
///     updated_at: Utc::now(),
/// };
///
/// crate::nodes::ws::session::handle_attempt_sync(node_id, &attempt, pool).await?;
/// # Ok(())
/// # }
/// ```
async fn handle_attempt_sync(
    node_id: Uuid,
    attempt: &AttemptSyncMessage,
    pool: &PgPool,
) -> Result<(), HandleError> {
    use crate::db::node_task_attempts::{NodeTaskAttemptRepository, UpsertNodeTaskAttempt};
    use crate::db::swarm_projects::SwarmProjectRepository;
    use crate::db::task_assignments::TaskAssignmentRepository;
    use crate::db::tasks::SharedTaskRepository;

    // Determine assignment_id: use provided one or create a synthetic one
    let assignment_id = match attempt.assignment_id {
        Some(id) => Some(id),
        None => {
            // For locally-started tasks, we need to create a synthetic assignment
            // First, find the swarm_project_id from the shared_task
            let shared_task_repo = SharedTaskRepository::new(pool);
            let shared_task = shared_task_repo
                .find_by_id(attempt.shared_task_id)
                .await
                .map_err(|e| HandleError::Database(e.to_string()))?;

            if let Some(task) = shared_task {
                // Find the swarm_project_nodes link for this project and node
                // swarm_project_id is the source of truth
                if let Some(swarm_project_id) = task.swarm_project_id {
                    let node_link =
                        SwarmProjectRepository::find_node_link(pool, swarm_project_id, node_id)
                            .await
                            .map_err(|e| HandleError::Database(e.to_string()))?;

                    if let Some(link) = node_link {
                        // Create or find a synthetic assignment
                        // Use the swarm_project_nodes link ID as the node_project_id
                        let assignment_repo = TaskAssignmentRepository::new(pool);
                        match assignment_repo
                            .create_or_find_synthetic(attempt.shared_task_id, node_id, link.id)
                            .await
                        {
                            Ok(assignment) => {
                                tracing::info!(
                                    node_id = %node_id,
                                    attempt_id = %attempt.attempt_id,
                                    assignment_id = %assignment.id,
                                    "created synthetic assignment for locally-started task"
                                );
                                Some(assignment.id)
                            }
                            Err(e) => {
                                tracing::warn!(
                                    node_id = %node_id,
                                    attempt_id = %attempt.attempt_id,
                                    error = %e,
                                    "failed to create synthetic assignment, proceeding without"
                                );
                                None
                            }
                        }
                    } else {
                        tracing::debug!(
                            node_id = %node_id,
                            swarm_project_id = %swarm_project_id,
                            "no swarm_project_nodes link found for synthetic assignment"
                        );
                        None
                    }
                } else {
                    tracing::debug!(
                        node_id = %node_id,
                        shared_task_id = %attempt.shared_task_id,
                        "task has no swarm_project_id, skipping synthetic assignment"
                    );
                    None
                }
            } else {
                tracing::debug!(
                    shared_task_id = %attempt.shared_task_id,
                    "shared task not found for synthetic assignment"
                );
                None
            }
        }
    };

    let repo = NodeTaskAttemptRepository::new(pool);
    repo.upsert(&UpsertNodeTaskAttempt {
        id: attempt.attempt_id,
        assignment_id,
        shared_task_id: attempt.shared_task_id,
        node_id,
        executor: attempt.executor.clone(),
        executor_variant: attempt.executor_variant.clone(),
        branch: attempt.branch.clone(),
        target_branch: attempt.target_branch.clone(),
        container_ref: attempt.container_ref.clone(),
        worktree_deleted: attempt.worktree_deleted,
        setup_completed_at: attempt.setup_completed_at,
        created_at: attempt.created_at,
        updated_at: attempt.updated_at,
    })
    .await
    .map_err(|e| HandleError::Database(e.to_string()))?;

    tracing::debug!(
        node_id = %node_id,
        attempt_id = %attempt.attempt_id,
        shared_task_id = %attempt.shared_task_id,
        assignment_id = ?assignment_id,
        "synced task attempt from node"
    );

    Ok(())
}

/// Handle an execution sync message from a node.
///
/// Upserts the execution process into node_execution_processes.
/// If the referenced attempt doesn't exist yet (race condition during sync),
/// we log a warning and skip - the client will retry on the next sync cycle.
async fn handle_execution_sync(
    node_id: Uuid,
    execution: &ExecutionSyncMessage,
    pool: &PgPool,
) -> Result<(), HandleError> {
    use crate::db::node_execution_processes::{
        NodeExecutionProcessError, NodeExecutionProcessRepository, UpsertNodeExecutionProcess,
    };
    use crate::db::node_task_attempts::NodeTaskAttemptRepository;

    // Check if the parent attempt exists first to avoid FK constraint errors
    // This can happen when ExecutionSync arrives before AttemptSync is processed
    let attempt_repo = NodeTaskAttemptRepository::new(pool);
    let attempt_exists = attempt_repo
        .find_by_id(execution.attempt_id)
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?
        .is_some();

    if !attempt_exists {
        tracing::warn!(
            node_id = %node_id,
            execution_id = %execution.execution_id,
            attempt_id = %execution.attempt_id,
            "execution sync skipped: parent attempt not yet synced (will retry)"
        );
        // Return Ok - the client should retry on next sync cycle
        // This is a race condition, not a real error
        return Ok(());
    }

    let repo = NodeExecutionProcessRepository::new(pool);
    match repo
        .upsert(&UpsertNodeExecutionProcess {
            id: execution.execution_id,
            attempt_id: execution.attempt_id,
            node_id,
            run_reason: execution.run_reason.clone(),
            executor_action: execution.executor_action.clone(),
            before_head_commit: execution.before_head_commit.clone(),
            after_head_commit: execution.after_head_commit.clone(),
            status: execution.status.clone(),
            exit_code: execution.exit_code,
            dropped: execution.dropped,
            pid: execution.pid,
            started_at: execution.started_at,
            completed_at: execution.completed_at,
            created_at: execution.created_at,
        })
        .await
    {
        Ok(_) => {
            tracing::debug!(
                node_id = %node_id,
                execution_id = %execution.execution_id,
                attempt_id = %execution.attempt_id,
                status = %execution.status,
                "synced execution process from node"
            );
            Ok(())
        }
        Err(NodeExecutionProcessError::AttemptNotFound(attempt_id)) => {
            // The attempt doesn't exist on the hive (perhaps deleted or not yet synced).
            // Log a warning but don't fail - this is a transient sync issue.
            tracing::warn!(
                node_id = %node_id,
                execution_id = %execution.execution_id,
                attempt_id = %attempt_id,
                "skipping execution sync: attempt not found on hive"
            );
            Ok(())
        }
        Err(e) => Err(HandleError::Database(e.to_string())),
    }
}

/// Handle a logs batch message from a node.
///
/// Stores the log entries in node_task_output_logs with execution_process_id.
/// If the assignment doesn't exist yet (race condition with AttemptSync), creates
/// a synthetic assignment using the shared_task_id.
async fn handle_logs_batch(
    node_id: Uuid,
    logs: &LogsBatchMessage,
    pool: &PgPool,
) -> Result<(), HandleError> {
    use crate::db::swarm_projects::SwarmProjectRepository;
    use crate::db::task_assignments::TaskAssignmentRepository;
    use crate::db::task_output_logs::{CreateTaskOutputLog, TaskOutputLogRepository};
    use crate::db::tasks::SharedTaskRepository;

    // First, ensure the assignment exists. If not, try to create a synthetic one.
    let assignment_repo = TaskAssignmentRepository::new(pool);
    let assignment_id = match assignment_repo.find_by_id(logs.assignment_id).await {
        Ok(Some(_)) => {
            // Assignment exists, use it
            logs.assignment_id
        }
        Ok(None) => {
            // Assignment doesn't exist - try to create synthetic if we have shared_task_id
            if let Some(shared_task_id) = logs.shared_task_id {
                // Look up the task to find swarm_project_id
                let shared_task_repo = SharedTaskRepository::new(pool);
                let shared_task = shared_task_repo
                    .find_by_id(shared_task_id)
                    .await
                    .map_err(|e| HandleError::Database(e.to_string()))?;

                if let Some(task) = shared_task {
                    if let Some(swarm_project_id) = task.swarm_project_id {
                        // Find the node's link to this project
                        let node_link =
                            SwarmProjectRepository::find_node_link(pool, swarm_project_id, node_id)
                                .await
                                .map_err(|e| HandleError::Database(e.to_string()))?;

                        if let Some(link) = node_link {
                            // Create synthetic assignment
                            match assignment_repo
                                .create_or_find_synthetic(shared_task_id, node_id, link.id)
                                .await
                            {
                                Ok(assignment) => {
                                    tracing::info!(
                                        node_id = %node_id,
                                        original_assignment_id = %logs.assignment_id,
                                        new_assignment_id = %assignment.id,
                                        shared_task_id = %shared_task_id,
                                        "created synthetic assignment for logs batch"
                                    );
                                    assignment.id
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        node_id = %node_id,
                                        assignment_id = %logs.assignment_id,
                                        error = %e,
                                        "failed to create synthetic assignment for logs"
                                    );
                                    return Err(HandleError::Database(format!(
                                        "failed to create synthetic assignment: {}",
                                        e
                                    )));
                                }
                            }
                        } else {
                            tracing::warn!(
                                node_id = %node_id,
                                swarm_project_id = %swarm_project_id,
                                "no node link found for logs batch - cannot create assignment"
                            );
                            return Err(HandleError::Database(
                                "no node link found for project".to_string(),
                            ));
                        }
                    } else {
                        // Task exists but has no swarm_project_id yet - can happen during sync races
                        // or for legacy tasks. Log at debug level and skip this batch gracefully.
                        tracing::debug!(
                            node_id = %node_id,
                            shared_task_id = %shared_task_id,
                            "task has no swarm_project_id - skipping logs batch (will retry after task sync)"
                        );
                        return Ok(());
                    }
                } else {
                    tracing::warn!(
                        node_id = %node_id,
                        shared_task_id = %shared_task_id,
                        "shared task not found - cannot create assignment"
                    );
                    return Err(HandleError::Database("shared task not found".to_string()));
                }
            } else {
                // No shared_task_id provided, cannot create synthetic assignment
                tracing::warn!(
                    node_id = %node_id,
                    assignment_id = %logs.assignment_id,
                    "assignment not found and no shared_task_id provided"
                );
                return Err(HandleError::Database(format!(
                    "assignment {} not found and no shared_task_id provided",
                    logs.assignment_id
                )));
            }
        }
        Err(e) => {
            return Err(HandleError::Database(e.to_string()));
        }
    };

    // Now insert the logs with the valid assignment_id
    let log_repo = TaskOutputLogRepository::new(pool);

    for entry in &logs.entries {
        let output_type = match entry.output_type {
            super::message::TaskOutputType::Stdout => "stdout",
            super::message::TaskOutputType::Stderr => "stderr",
            super::message::TaskOutputType::System => "system",
        };

        // Create log with optional execution_process_id
        log_repo
            .create_with_execution_process(
                CreateTaskOutputLog {
                    assignment_id,
                    output_type: output_type.to_string(),
                    content: entry.content.clone(),
                    timestamp: entry.timestamp,
                },
                logs.execution_process_id,
            )
            .await
            .map_err(|e| HandleError::Database(e.to_string()))?;
    }

    tracing::trace!(
        node_id = %node_id,
        assignment_id = %assignment_id,
        execution_process_id = ?logs.execution_process_id,
        entry_count = logs.entries.len(),
        "stored logs batch from node"
    );

    Ok(())
}

// NOTE: handle_label_sync has been removed.
// Labels are now managed centrally on the hive and synced DOWN to nodes.
// NodeMessage::LabelSync is ignored (see route_message above).

/// Handle a task sync message from a node.
///
/// The new flow looks up the swarm project via node_local_projects → swarm_project_nodes,
/// using the node's local_project_id to find the linked swarm_project_id.
///
/// Race condition handling:
/// - If node_local_projects doesn't exist → ProjectsSync hasn't arrived, send RETRY
/// - If node_local_projects exists but swarm_project_id is NULL → not linked, send error
/// - If fully linked → proceed with sync
async fn handle_task_sync(
    node_id: Uuid,
    organization_id: Uuid,
    node_name: &str,
    task_sync: &TaskSyncMessage,
    pool: &PgPool,
    ws_sender: &mut futures::stream::SplitSink<WebSocket, Message>,
) -> Result<(), HandleError> {
    use crate::db::node_local_projects::NodeLocalProjectRepository;
    use crate::db::tasks::{SharedTaskRepository, UpsertTaskFromNodeData};

    // Canonicalize the node-reported status via the single boundary helper (302 / tournament R1/B).
    // The node serializes lowercase (`inprogress`/`inreview`); the old manual match only checked
    // kebab-case forms and fell through to `Todo` — silently corrupting `InProgress`/`InReview`.
    // An unknown wire value is a node-side serialization bug; we surface it as a `success: false`
    // response rather than `?`-propagating (which would leave the node waiting on a response).
    // This mirrors `handle_op_batch_apply`'s SKIP+ADVANCE for the identical failure class (R2/E).
    let status = match crate::nodes::ws::status_machine::canonical_status_from_node(&task_sync.status)
    {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                node_id = %node_id,
                local_task_id = %task_sync.local_task_id,
                status = %task_sync.status,
                error = %e,
                "task_sync: rejected unknown status wire value"
            );
            let r = TaskSyncResponseMessage {
                local_task_id: task_sync.local_task_id,
                shared_task_id: Uuid::nil(),
                success: false,
                error: Some(format!("REJECTED: unknown status wire value: {}", e)),
            };
            let _ = send_message(ws_sender, &HiveMessage::TaskSyncResponse(r)).await;
            return Ok(());
        }
    };

    // First check if the node_local_projects record exists
    // This distinguishes between "ProjectsSync hasn't arrived" and "project not linked"
    let local_project = NodeLocalProjectRepository::find_by_node_and_project(
        pool,
        node_id,
        task_sync.local_project_id,
    )
    .await
    .map_err(|e| HandleError::Database(e.to_string()))?;

    let local_project = match local_project {
        Some(p) => p,
        None => {
            // Race condition: ProjectsSync hasn't arrived yet
            // Tell the node to retry after its ProjectsSync completes
            tracing::debug!(
                node_id = %node_id,
                local_project_id = %task_sync.local_project_id,
                local_task_id = %task_sync.local_task_id,
                "task sync: project not found in node_local_projects (ProjectsSync race, retry)"
            );
            let r = TaskSyncResponseMessage {
                local_task_id: task_sync.local_task_id,
                shared_task_id: Uuid::nil(),
                success: false,
                error: Some(
                    "RETRY: Project not synced yet, please retry after ProjectsSync completes"
                        .to_string(),
                ),
            };
            let _ = send_message(ws_sender, &HiveMessage::TaskSyncResponse(r)).await;
            return Ok(());
        }
    };

    // Check if the project is linked to a swarm project
    let swarm_project_id = match local_project.swarm_project_id {
        Some(id) => id,
        None => {
            // Project exists in node_local_projects but isn't linked to swarm
            // This is intentional - the project needs to be linked via UI first
            tracing::debug!(
                node_id = %node_id,
                local_project_id = %task_sync.local_project_id,
                local_task_id = %task_sync.local_task_id,
                "task sync: project not linked to swarm (link via UI first)"
            );
            let r = TaskSyncResponseMessage {
                local_task_id: task_sync.local_task_id,
                shared_task_id: Uuid::nil(),
                success: false,
                error: Some(
                    "Project not linked to swarm - link it via the swarm settings UI first"
                        .to_string(),
                ),
            };
            let _ = send_message(ws_sender, &HiveMessage::TaskSyncResponse(r)).await;
            return Ok(());
        }
    };

    // Verify the swarm project belongs to this organization and node has a link
    let swarm_link: Option<SwarmProjectLink> = sqlx::query_as(
        r#"
        SELECT
            sp.organization_id
        FROM swarm_project_nodes spn
        JOIN swarm_projects sp ON spn.swarm_project_id = sp.id
        WHERE spn.node_id = $1
          AND spn.local_project_id = $2
          AND spn.swarm_project_id = $3
          AND sp.organization_id = $4
        "#,
    )
    .bind(node_id)
    .bind(task_sync.local_project_id)
    .bind(swarm_project_id)
    .bind(organization_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| HandleError::Database(e.to_string()))?;

    let org_id = match swarm_link {
        Some(link) => link.organization_id,
        None => {
            tracing::warn!(
                node_id = %node_id,
                local_project_id = %task_sync.local_project_id,
                swarm_project_id = %swarm_project_id,
                "task sync failed: swarm_project_nodes link missing or org mismatch"
            );
            let r = TaskSyncResponseMessage {
                local_task_id: task_sync.local_task_id,
                shared_task_id: Uuid::nil(),
                success: false,
                error: Some("Swarm project link invalid - re-link the project via UI".to_string()),
            };
            let _ = send_message(ws_sender, &HiveMessage::TaskSyncResponse(r)).await;
            return Ok(());
        }
    };

    // Sanitize string fields to remove null bytes (PostgreSQL doesn't allow them)
    let sanitized_title = sanitize_string(&task_sync.title);
    let sanitized_description = sanitize_option_string(task_sync.description.clone());

    let repo = SharedTaskRepository::new(pool);

    // SC4 author guard (ADR-0010 §D / tournament R1/C): mirror the guard in
    // `handle_op_batch_apply` — a node may not author hive-only status transitions
    // (todo→in-progress, in-review→done, in-review→in-progress, *→cancelled) nor any
    // out-of-matrix transition. `handle_task_sync` is the third write site (in addition
    // to 303's `handle_op_batch_apply` and 304's `handle_task_status`); without this
    // guard, a node could bypass the matrix via a TaskSync message.
    if let Some(existing) = repo
        .find_by_source_task_id(node_id, task_sync.local_task_id)
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?
    {
        let current = existing.status;
        if status != current {
            use crate::nodes::ws::status_machine::{TransitionAuthor, author_of_transition};
            match author_of_transition(current, status) {
                Some(TransitionAuthor::Node) => {
                    // Node-authored transition; proceed to upsert.
                }
                Some(TransitionAuthor::Hive) => {
                    tracing::warn!(
                        node_id = %node_id,
                        local_task_id = %task_sync.local_task_id,
                        shared_task_id = %existing.id,
                        from = ?current,
                        to = ?status,
                        "task_sync: reject (hive-authored transition reported by node)"
                    );
                    let r = TaskSyncResponseMessage {
                        local_task_id: task_sync.local_task_id,
                        shared_task_id: existing.id,
                        success: false,
                        error: Some(format!(
                            "Status transition {:?}→{:?} is hive-only — not authorizable from a node",
                            current, status
                        )),
                    };
                    let _ = send_message(ws_sender, &HiveMessage::TaskSyncResponse(r)).await;
                    return Ok(());
                }
                None => {
                    tracing::warn!(
                        node_id = %node_id,
                        local_task_id = %task_sync.local_task_id,
                        shared_task_id = %existing.id,
                        from = ?current,
                        to = ?status,
                        "task_sync: reject (illegal status transition)"
                    );
                    let r = TaskSyncResponseMessage {
                        local_task_id: task_sync.local_task_id,
                        shared_task_id: existing.id,
                        success: false,
                        error: Some(format!(
                            "Status transition {:?}→{:?} is not in the transition matrix",
                            current, status
                        )),
                    };
                    let _ = send_message(ws_sender, &HiveMessage::TaskSyncResponse(r)).await;
                    return Ok(());
                }
            }
        }
        // incoming == current → no-op (metadata-only update); proceed to upsert.
    }
    // Task not found → creation path; no `from` status, no guard. Proceed to upsert.

    match repo
        .upsert_from_node(UpsertTaskFromNodeData {
            swarm_project_id,
            project_id: swarm_project_id, // Use swarm_project_id for both (backwards compat)
            organization_id: org_id,
            origin_node_id: node_id,
            local_task_id: task_sync.local_task_id,
            title: sanitized_title,
            description: sanitized_description,
            status,
            version: task_sync.version,
            owner_node_id: task_sync.owner_node_id.or(Some(node_id)),
            owner_name: task_sync
                .owner_name
                .clone()
                .or_else(|| Some(node_name.to_string())),
            assignee_user_id: task_sync.assignee_user_id,
        })
        .await
    {
        Ok((task, was_created)) => {
            // Sync labels if provided
            if !task_sync.label_ids.is_empty() {
                use crate::db::labels::LabelRepository;
                let label_repo = LabelRepository::new(pool);
                if let Err(e) = label_repo
                    .set_task_labels(task.id, &task_sync.label_ids)
                    .await
                {
                    tracing::warn!(
                        shared_task_id = %task.id,
                        label_count = task_sync.label_ids.len(),
                        error = ?e,
                        "failed to sync task labels (task sync succeeded)"
                    );
                } else {
                    tracing::debug!(
                        shared_task_id = %task.id,
                        label_count = task_sync.label_ids.len(),
                        "synced task labels"
                    );
                }
            }

            tracing::info!(
                node_id = %node_id,
                shared_task_id = %task.id,
                swarm_project_id = %swarm_project_id,
                was_created = was_created,
                label_count = task_sync.label_ids.len(),
                "synced task from node"
            );
            let r = TaskSyncResponseMessage {
                local_task_id: task_sync.local_task_id,
                shared_task_id: task.id,
                success: true,
                error: None,
            };
            let _ = send_message(ws_sender, &HiveMessage::TaskSyncResponse(r)).await;
        }
        Err(e) => {
            tracing::error!(node_id = %node_id, error = ?e, "failed to sync task");
            let r = TaskSyncResponseMessage {
                local_task_id: task_sync.local_task_id,
                shared_task_id: Uuid::nil(),
                success: false,
                error: Some(e.to_string()),
            };
            let _ = send_message(ws_sender, &HiveMessage::TaskSyncResponse(r)).await;
        }
    }
    Ok(())
}

/// Apply a batch of node outbox ops to the hive op-log + `shared_tasks` and return the new
/// `applied_through_seq` high-water (SC2). WS-free core so the unit test can exercise the apply
/// path without constructing a `SplitSink` (see task 106 STOP note on ws_sender in test).
///
/// Park-vs-skip split mirrors `handle_task_sync`'s three-branch context resolution:
/// - `node_local_projects` row absent → **PARK** (transient, ProjectsSync race): break, no advance.
/// - row present but not swarm-linked, or swarm-link/org lookup absent → **SKIP + ADVANCE**
///   (permanent): record the op in `node_op_log` and advance; do NOT call `upsert_from_node`.
/// - otherwise → **APPLY** (apply-then-record): `upsert_from_node` first, then insert the dedup row.
async fn handle_op_batch_apply(
    node_id: Uuid,
    organization_id: Uuid,
    node_name: &str,
    ops: &[OutboxOp],
    pool: &PgPool,
) -> Result<(i64, Vec<(Uuid, String)>), HandleError> {
    use crate::db::node_local_projects::NodeLocalProjectRepository;
    use crate::db::tasks::{SharedTaskRepository, UpsertTaskFromNodeData};

    // Revoke queue (ws-free split, option (a)): (assignment_id, reason) pairs surfaced
    // out of the apply loop for `handle_op_batch` to emit as `HiveMessage::LeaseRevoked`.
    let mut revokes: Vec<(Uuid, String)> = Vec::new();

    let mut applied_through_seq: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) FROM node_op_log WHERE node_id = $1")
            .bind(node_id)
            .fetch_one(pool)
            .await
            .map_err(|e| HandleError::Database(e.to_string()))?;

    for op in ops {
        // (a) Tracer scope guard: only task.upsert is handled in this phase.
        if op.op_type != "task.upsert" {
            applied_through_seq = op.seq;
            continue;
        }

        let local_project_id: Uuid = op
            .payload
            .get("project_id")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<Uuid>().ok())
            .ok_or_else(|| {
                HandleError::Database(format!(
                    "op_batch: op seq {} missing payload.project_id",
                    op.seq
                ))
            })?;
        let local_task_id: Uuid = op
            .payload
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<Uuid>().ok())
            .ok_or_else(|| {
                HandleError::Database(format!("op_batch: op seq {} missing payload.id", op.seq))
            })?;

        // (b) Resolve context — copy handle_task_sync's three-branch resolution exactly.
        let local_project =
            NodeLocalProjectRepository::find_by_node_and_project(pool, node_id, local_project_id)
                .await
                .map_err(|e| HandleError::Database(e.to_string()))?;

        let local_project = match local_project {
            Some(p) => p,
            None => {
                // TRANSIENT (ProjectsSync race) → PARK: break, do NOT advance, do NOT record.
                tracing::debug!(
                    node_id = %node_id,
                    local_project_id = %local_project_id,
                    local_task_id = %local_task_id,
                    seq = op.seq,
                    "op_batch: park (node_local_projects row absent, ProjectsSync race)"
                );
                break;
            }
        };

        let swarm_project_id = match local_project.swarm_project_id {
            Some(id) => id,
            None => {
                // PERMANENT (not swarm-linked) → SKIP + ADVANCE.
                sqlx::query(
                    r#"
                    INSERT INTO node_op_log (node_id, idempotency_key, seq, op_type, entity_id)
                    VALUES ($1, $2, $3, $4, $5)
                    ON CONFLICT (node_id, idempotency_key) DO NOTHING
                    "#,
                )
                .bind(node_id)
                .bind(&op.idempotency_key)
                .bind(op.seq)
                .bind(&op.op_type)
                .bind(op.entity_id)
                .execute(pool)
                .await
                .map_err(|e| HandleError::Database(e.to_string()))?;
                applied_through_seq = op.seq;
                tracing::debug!(
                    node_id = %node_id,
                    local_project_id = %local_project_id,
                    seq = op.seq,
                    "op_batch: skip+advance (project not swarm-linked)"
                );
                continue;
            }
        };

        // Verify the swarm project belongs to this organization and node has a link.
        let swarm_link: Option<SwarmProjectLink> = sqlx::query_as(
            r#"
            SELECT
                sp.organization_id
            FROM swarm_project_nodes spn
            JOIN swarm_projects sp ON spn.swarm_project_id = sp.id
            WHERE spn.node_id = $1
              AND spn.local_project_id = $2
              AND spn.swarm_project_id = $3
              AND sp.organization_id = $4
            "#,
        )
        .bind(node_id)
        .bind(local_project_id)
        .bind(swarm_project_id)
        .bind(organization_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?;

        let org_id = match swarm_link {
            Some(link) => link.organization_id,
            None => {
                // PERMANENT (bad link) → SKIP + ADVANCE.
                sqlx::query(
                    r#"
                    INSERT INTO node_op_log (node_id, idempotency_key, seq, op_type, entity_id)
                    VALUES ($1, $2, $3, $4, $5)
                    ON CONFLICT (node_id, idempotency_key) DO NOTHING
                    "#,
                )
                .bind(node_id)
                .bind(&op.idempotency_key)
                .bind(op.seq)
                .bind(&op.op_type)
                .bind(op.entity_id)
                .execute(pool)
                .await
                .map_err(|e| HandleError::Database(e.to_string()))?;
                applied_through_seq = op.seq;
                tracing::warn!(
                    node_id = %node_id,
                    local_project_id = %local_project_id,
                    swarm_project_id = %swarm_project_id,
                    seq = op.seq,
                    "op_batch: skip+advance (swarm_project_nodes link missing or org mismatch)"
                );
                continue;
            }
        };

        // (c) Idempotent apply — APPLY FIRST, RECORD SECOND (tournament R1/F1).
        let seen: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM node_op_log WHERE node_id = $1 AND idempotency_key = $2)",
        )
        .bind(node_id)
        .bind(&op.idempotency_key)
        .fetch_one(pool)
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?;

        if seen {
            // Already applied in a prior committed pass: skip the upsert, advance.
            applied_through_seq = op.seq;
            continue;
        }

        // (c.0) Fencing guard (CONTRACT §C / ADR-0009 SC3): for ops against a hive-assigned
        // task, reject stale-token writes BEFORE applying. The assignment row is keyed on
        // `node_task_assignments.task_id` = `shared_tasks.id`, so resolve the hive shared id
        // from `payload.shared_task_id` DIRECTLY (the load-bearing, reassignment-proof key —
        // a creator-keyed `find_by_source_task_id` would return None for a task ASSIGNED to
        // the sender but CREATED elsewhere, silently disabling the fence = the SC3 bug).
        let shared_id = op
            .payload
            .get("shared_task_id")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<Uuid>().ok());

        if let Some(shared_id) = shared_id {
            // Node-owned task bypass (tournament R1/G): a task created by THIS node
            // (`owner_node_id == node_id`) has no `node_task_assignments` row — the
            // fence lookup below would find no assignment and `break`, rejecting the
            // owner's own writes. Query `owner_node_id` and if it matches `node_id`,
            // skip the lease+token fence (the owner does not need a lease to write its
            // own task). Only hive-assigned tasks (owner_node_id != node_id or NULL)
            // require the fence.
            let owner_node_id: Option<Uuid> = sqlx::query_scalar(
                r#"SELECT owner_node_id FROM shared_tasks WHERE id = $1 AND deleted_at IS NULL"#,
            )
            .bind(shared_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| HandleError::Database(e.to_string()))?;

            if owner_node_id == Some(node_id) {
                // Node-owned task — bypass the fence. Proceed to status mapping + upsert.
                tracing::trace!(
                    node_id = %node_id,
                    seq = op.seq,
                    shared_task_id = %shared_id,
                    "op_batch: node-owned task, skipping lease+token fence"
                );
            } else {
                // Narrow read: do NOT use `NodeTaskAssignment` FromRow (the `fencing_token` column
                // is not on that struct — 203's judgment call). Read only `id` + `fencing_token`.
                let assignment: Option<(Uuid, i64)> = sqlx::query_as(
                    r#"
                SELECT id, fencing_token
                FROM node_task_assignments
                WHERE task_id = $1 AND completed_at IS NULL
                "#,
                )
                .bind(shared_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| HandleError::Database(e.to_string()))?;

                if let Some((assignment_id, current_token)) = assignment {
                    let stale = match op.fencing_token {
                        None => true,
                        Some(tok) => tok < current_token,
                    };
                    if stale {
                        // REJECT (permanent): do NOT upsert, do NOT record node_op_log, do NOT
                        // advance applied_through_seq past this op. Mirror 106's PARK control-flow
                        // of NOT advancing (break, not continue), but this is a permanent reject,
                        // not a transient park. Emit LeaseRevoked so the partitioned writer learns
                        // its lease is gone.
                        tracing::warn!(
                            node_id = %node_id,
                            seq = op.seq,
                            assignment_id = %assignment_id,
                            op_token = ?op.fencing_token,
                            current_token = current_token,
                            "op_batch: reject (stale fencing token) — LeaseRevoked"
                        );
                        revokes.push((assignment_id, "stale fencing token".to_string()));
                        break;
                    }
                } else {
                    // No active assignment for a hive-managed task (shared_task_id present):
                    // the lease was reclaimed or the task was completed/cancelled. A late write
                    // from a partitioned node MUST NOT overwrite the completed task (SC3). Drop
                    // the op and stop processing this batch — the node's self-fence watchdog
                    // and the reclaim sweep's LeaseRevoked event will halt the node.
                    //
                    // Emit LeaseRevoked so the partitioned writer learns its lease is gone
                    // (tournament R1/D — without this, the node retries unacked ops forever).
                    // The active-lease query above filtered `completed_at IS NULL`, so we look
                    // up the assignment WITHOUT that filter to obtain the `assignment_id` for
                    // the revoke signal.
                    tracing::warn!(
                        node_id = %node_id,
                        seq = op.seq,
                        shared_task_id = %shared_id,
                        op_token = ?op.fencing_token,
                        "op_batch: reject (no active assignment for hive-managed task) — lease revoked or completed"
                    );
                    let assignment_id: Option<Uuid> = sqlx::query_scalar(
                        r#"SELECT id FROM node_task_assignments
                       WHERE task_id = $1 AND node_id = $2
                       ORDER BY completed_at DESC NULLS LAST
                       LIMIT 1"#,
                    )
                    .bind(shared_id)
                    .bind(node_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| HandleError::Database(e.to_string()))?;
                    if let Some(aid) = assignment_id {
                        revokes.push((
                            aid,
                            "no active assignment (lease revoked or completed)".to_string(),
                        ));
                    }
                    break;
                }
            }
        }
        // `shared_id` None → creator's first pre-link write / node-owned work: no fence.
        // `shared_id` Some + owner_node_id == node_id → node-owned task, fence bypassed above.

        // (d) Status value mapping (tournament R1/F5): node serializes lowercase
        // (inprogress/inreview); canonicalize via the single boundary helper (302).
        let status_raw = op
            .payload
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let status = match crate::nodes::ws::status_machine::canonical_status_from_node(status_raw)
        {
            Ok(s) => s,
            Err(e) => {
                // Unknown status value → SKIP+ADVANCE (tournament R1/E): propagating `?`
                // would return a Database error → no OpAck sent → node retries forever.
                // Skip the op permanently (record in node_op_log + advance cursor) so the
                // node's outbox drains past the malformed op.
                tracing::warn!(
                    node_id = %node_id,
                    seq = op.seq,
                    status_raw,
                    "op_batch: skip+advance (unknown status from node): {}", e
                );
                sqlx::query(
                    r#"
                    INSERT INTO node_op_log (node_id, idempotency_key, seq, op_type, entity_id)
                    VALUES ($1, $2, $3, $4, $5)
                    ON CONFLICT (node_id, idempotency_key) DO NOTHING
                    "#,
                )
                .bind(node_id)
                .bind(&op.idempotency_key)
                .bind(op.seq)
                .bind(&op.op_type)
                .bind(op.entity_id)
                .execute(pool)
                .await
                .map_err(|e| HandleError::Database(e.to_string()))?;
                applied_through_seq = op.seq;
                continue;
            }
        };

        let title = op
            .payload
            .get("title")
            .and_then(|v| v.as_str())
            .map(sanitize_string)
            .ok_or_else(|| {
                HandleError::Database(format!("op_batch: op seq {} missing payload.title", op.seq))
            })?;
        let description = op
            .payload
            .get("description")
            .and_then(|v| v.as_str())
            .map(sanitize_string);

        let repo = SharedTaskRepository::new(pool);

        // (e) Status transition author guard (ADR-0010 §D / SC4): a hive-managed shared
        // task's status may be changed ONLY by its sole authoritative author. Governs
        // only ops carrying a `shared_task_id` (hive-managed) — node-owned work
        // (`shared_id` None) skips the guard, mirroring P2's fence scope. A no-op
        // (`incoming == current`) is NOT a transition: the metadata-only upsert proceeds.
        // An illegal transition or a hive-authored-from-node transition is REJECTED via
        // SKIP+ADVANCE (same node_op_log write + cursor advance as 106's permanent-skip
        // branches above), so a rejected status does not wedge the op-log. Rides P2's
        // lease+token fence above: by the time we reach here, a `shared_id`-bearing op
        // has already passed the stale-token check (or broken out before this point).
        if let Some(shared_id) = shared_id {
            // Creation path: if the shared_task row does not yet exist, there is no `from`
            // status — the matrix governs transitions of an EXISTING row. Skip the guard
            // and let 106's upsert create it.
            if let Some(existing) = repo
                .find_by_id(shared_id)
                .await
                .map_err(|e| HandleError::Database(e.to_string()))?
            {
                let current = existing.status;
                if status != current {
                    use crate::nodes::ws::status_machine::{
                        TransitionAuthor, author_of_transition,
                    };
                    match author_of_transition(current, status) {
                        Some(TransitionAuthor::Node) => {
                            // Node-authored transition; P2's lease+token fence already
                            // validated above → proceed to upsert_from_node.
                        }
                        Some(TransitionAuthor::Hive) => {
                            // A hive-authored transition arriving FROM A NODE → REJECT
                            // (SKIP+ADVANCE): a node may not author `todo→in-progress`,
                            // `in-review→done`, `in-review→in-progress`, or `*→cancelled`.
                            tracing::warn!(
                                node_id = %node_id,
                                seq = op.seq,
                                shared_task_id = %shared_id,
                                from = ?current,
                                to = ?status,
                                "op_batch: skip+advance (hive-authored transition reported by node)"
                            );
                            sqlx::query(
                                r#"
                                INSERT INTO node_op_log (node_id, idempotency_key, seq, op_type, entity_id)
                                VALUES ($1, $2, $3, $4, $5)
                                ON CONFLICT (node_id, idempotency_key) DO NOTHING
                                "#,
                            )
                            .bind(node_id)
                            .bind(&op.idempotency_key)
                            .bind(op.seq)
                            .bind(&op.op_type)
                            .bind(op.entity_id)
                            .execute(pool)
                            .await
                            .map_err(|e| HandleError::Database(e.to_string()))?;
                            applied_through_seq = op.seq;
                            continue;
                        }
                        None => {
                            // Illegal transition (in no author's column) → REJECT
                            // (SKIP+ADVANCE): do not merge an out-of-matrix status.
                            tracing::warn!(
                                node_id = %node_id,
                                seq = op.seq,
                                shared_task_id = %shared_id,
                                from = ?current,
                                to = ?status,
                                "op_batch: skip+advance (illegal status transition)"
                            );
                            sqlx::query(
                                r#"
                                INSERT INTO node_op_log (node_id, idempotency_key, seq, op_type, entity_id)
                                VALUES ($1, $2, $3, $4, $5)
                                ON CONFLICT (node_id, idempotency_key) DO NOTHING
                                "#,
                            )
                            .bind(node_id)
                            .bind(&op.idempotency_key)
                            .bind(op.seq)
                            .bind(&op.op_type)
                            .bind(op.entity_id)
                            .execute(pool)
                            .await
                            .map_err(|e| HandleError::Database(e.to_string()))?;
                            applied_through_seq = op.seq;
                            continue;
                        }
                    }
                }
                // incoming == current → no-op short-circuit: NOT a transition, proceed
                // to upsert_from_node (metadata-only update; other fields may change).
            }
        }

        repo.upsert_from_node(UpsertTaskFromNodeData {
            swarm_project_id,
            project_id: swarm_project_id,
            organization_id: org_id,
            origin_node_id: node_id,
            local_task_id,
            title,
            description,
            status,
            version: 1,
            owner_node_id: Some(node_id),
            owner_name: Some(node_name.to_string()),
            assignee_user_id: None,
        })
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?;

        // ONLY AFTER upsert succeeds → record the dedup row.
        sqlx::query(
            r#"
            INSERT INTO node_op_log (node_id, idempotency_key, seq, op_type, entity_id)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (node_id, idempotency_key) DO NOTHING
            "#,
        )
        .bind(node_id)
        .bind(&op.idempotency_key)
        .bind(op.seq)
        .bind(&op.op_type)
        .bind(op.entity_id)
        .execute(pool)
        .await
        .map_err(|e| HandleError::Database(e.to_string()))?;
        applied_through_seq = op.seq;
    }

    Ok((applied_through_seq, revokes))
}

/// Handle a `NodeMessage::OpBatch` (SC2): apply each op idempotently to `node_op_log` +
/// `shared_tasks` and ack with `applied_through_seq`. Wraps `handle_op_batch_apply` (WS-free core)
/// and sends the durable `HiveMessage::OpAck`.
async fn handle_op_batch(
    node_id: Uuid,
    organization_id: Uuid,
    node_name: &str,
    ops: &[OutboxOp],
    pool: &PgPool,
    ws_sender: &mut futures::stream::SplitSink<WebSocket, Message>,
) -> Result<(), HandleError> {
    let (applied_through_seq, revokes) =
        handle_op_batch_apply(node_id, organization_id, node_name, ops, pool).await?;
    // Emit LeaseRevoked for each rejected op (ws-free split, option (a)).
    for (assignment_id, reason) in revokes {
        send_message(
            ws_sender,
            &HiveMessage::LeaseRevoked {
                assignment_id,
                reason,
            },
        )
        .await
        .map_err(|_| HandleError::Send)?;
    }
    send_message(
        ws_sender,
        &HiveMessage::OpAck {
            applied_through_seq,
        },
    )
    .await
    .map_err(|_| HandleError::Send)?;
    Ok(())
}

/// Renew held leases for the given assignment_ids (pure DB, no WebSocket send).
///
/// Returns the `LeaseClaim` for each assignment this node still holds (renew_lease Some).
/// Foreign/missing assignments are skipped (renew_lease None → no entry).
async fn handle_lease_heartbeat_renew(
    node_id: Uuid,
    assignment_ids: &[Uuid],
    pool: &PgPool,
) -> Result<Vec<crate::db::task_assignments::LeaseClaim>, HandleError> {
    let repo = crate::db::task_assignments::TaskAssignmentRepository::new(pool);
    let mut grants = Vec::new();
    for assignment_id in assignment_ids {
        match repo.renew_lease(*assignment_id, node_id, LEASE_TTL).await {
            Ok(Some(claim)) => grants.push(claim),
            Ok(None) => {} // not held by this node — skip, no grant
            Err(e) => return Err(HandleError::Database(e.to_string())),
        }
    }
    Ok(grants)
}

/// Handle a lease heartbeat: renew held leases and reply LeaseGrant per assignment.
///
/// For each assignment_id, renews the lease via `TaskAssignmentRepository::renew_lease`.
/// Replies `HiveMessage::LeaseGrant` only for assignments the node actually holds
/// (renew_lease returns Some); skips foreign/missing assignments (no grant).
async fn handle_lease_heartbeat(
    node_id: Uuid,
    assignment_ids: &[Uuid],
    pool: &PgPool,
    ws_sender: &mut futures::stream::SplitSink<WebSocket, Message>,
) -> Result<(), HandleError> {
    let grants = handle_lease_heartbeat_renew(node_id, assignment_ids, pool).await?;
    for claim in grants {
        send_message(
            ws_sender,
            &HiveMessage::LeaseGrant {
                assignment_id: claim.assignment_id,
                fencing_token: claim.fencing_token,
                lease_expires_at: claim.lease_expires_at,
            },
        )
        .await
        .map_err(|_| HandleError::Send)?;
    }
    Ok(())
}

/// Helper struct for swarm project link lookup
#[derive(sqlx::FromRow)]
struct SwarmProjectLink {
    organization_id: Uuid,
}

/// Handle a projects sync message from a node.
///
/// This upserts all local projects from the node into the `node_local_projects`
/// table, enabling the swarm settings UI to show all projects for linking.
async fn handle_projects_sync(
    node_id: Uuid,
    projects: &ProjectsSyncMessage,
    pool: &PgPool,
) -> Result<(), HandleError> {
    use crate::db::node_local_projects::{NodeLocalProjectRepository, UpsertLocalProjectData};

    let projects_data: Vec<UpsertLocalProjectData> = projects
        .projects
        .iter()
        .map(|p| UpsertLocalProjectData {
            node_id,
            local_project_id: p.local_project_id,
            name: p.name.clone(),
            git_repo_path: p.git_repo_path.clone(),
            default_branch: p.default_branch.clone(),
        })
        .collect();

    let count = projects_data.len();
    match NodeLocalProjectRepository::bulk_upsert(pool, node_id, projects_data).await {
        Ok(upserted) => {
            tracing::info!(
                node_id = %node_id,
                total = count,
                upserted = upserted,
                "synced local projects from node"
            );
        }
        Err(e) => {
            tracing::error!(
                node_id = %node_id,
                error = ?e,
                "failed to sync local projects"
            );
        }
    }

    Ok(())
}

/// Broadcast a node's owned projects to other nodes linked to the same swarm projects.
///
/// This is called when a node connects to notify other nodes about the newly
/// connected node's available projects. Only projects owned by this node
/// (is_owned == true) are broadcast, and only to nodes linked to the same swarm project.
async fn broadcast_node_projects(
    node_id: Uuid,
    _organization_id: Uuid,
    node_name: &str,
    node_public_url: Option<&str>,
    linked_projects: &[LinkedProjectInfo],
    pool: &PgPool,
    connections: &ConnectionManager,
) {
    use crate::db::swarm_projects::SwarmProjectRepository;

    // Only broadcast projects owned by this node
    let owned_projects: Vec<_> = linked_projects.iter().filter(|p| p.is_owned).collect();

    if owned_projects.is_empty() {
        return;
    }

    let mut broadcast_count = 0;

    for project_info in &owned_projects {
        // Get nodes linked to this swarm project (project_id is swarm_project_id)
        let linked_nodes = match SwarmProjectRepository::get_linked_node_ids(
            pool,
            project_info.project_id,
        )
        .await
        {
            Ok(nodes) => nodes,
            Err(e) => {
                tracing::warn!(
                    node_id = %node_id,
                    project_id = %project_info.project_id,
                    error = ?e,
                    "failed to get linked nodes for project, skipping broadcast"
                );
                continue;
            }
        };

        // Filter out the current node
        let target_nodes: Vec<_> = linked_nodes
            .into_iter()
            .filter(|&id| id != node_id)
            .collect();

        if target_nodes.is_empty() {
            tracing::debug!(
                node_id = %node_id,
                project_id = %project_info.project_id,
                "no other nodes linked to swarm project, skipping broadcast"
            );
            continue;
        }

        let sync_msg = HiveMessage::ProjectSync(ProjectSyncMessage {
            message_id: Uuid::new_v4(),
            link_id: project_info.link_id,
            project_id: project_info.project_id,
            project_name: project_info.project_name.clone(),
            local_project_id: project_info.local_project_id,
            git_repo_path: project_info.git_repo_path.clone(),
            default_branch: project_info.default_branch.clone(),
            source_node_id: node_id,
            source_node_name: node_name.to_string(),
            source_node_public_url: node_public_url.map(String::from),
            is_new: true,
        });

        let failed = connections.send_to_nodes(&target_nodes, sync_msg).await;
        broadcast_count += 1;

        if !failed.is_empty() {
            tracing::warn!(
                node_id = %node_id,
                project_id = %project_info.project_id,
                failed_count = failed.len(),
                "failed to broadcast project to some nodes"
            );
        }
    }

    tracing::info!(
        node_id = %node_id,
        owned_project_count = owned_projects.len(),
        broadcast_count = broadcast_count,
        "broadcast node's owned projects to linked nodes"
    );
}

/// Handle a backfill response from a node.
///
/// When a node responds to a backfill request, it sends the actual data via
/// normal AttemptSync/ExecutionSync/LogsBatch messages. This response just
/// confirms completion and allows us to mark attempts as complete.
async fn handle_backfill_response(
    node_id: Uuid,
    response: &BackfillResponseMessage,
    pool: &PgPool,
    tracker: &BackfillRequestTracker,
) -> Result<(), HandleError> {
    tracing::info!(
        node_id = %node_id,
        request_id = %response.request_id,
        success = response.success,
        entities_sent = response.entities_sent,
        error = ?response.error,
        "received backfill response from node"
    );

    let repo = NodeTaskAttemptRepository::new(pool);

    // Look up the attempt IDs - first try in-memory tracker, then fall back to database
    let attempt_ids = match tracker.complete(response.request_id).await {
        Some(ids) => ids,
        None => {
            // In-memory tracker doesn't have this request - try database fallback.
            // This handles the race condition where the node disconnects (clearing the tracker)
            // before the delayed backfill response arrives.
            match repo.find_by_backfill_request_id(response.request_id).await {
                Ok(ids) if !ids.is_empty() => {
                    tracing::info!(
                        node_id = %node_id,
                        request_id = %response.request_id,
                        attempt_count = ids.len(),
                        "using DB fallback for backfill correlation"
                    );
                    ids
                }
                Ok(_) => {
                    // No attempts found in DB either - genuinely unknown request
                    tracing::warn!(
                        node_id = %node_id,
                        request_id = %response.request_id,
                        "no attempts found for backfill response (not in tracker or DB)"
                    );
                    return Ok(());
                }
                Err(e) => {
                    tracing::error!(
                        node_id = %node_id,
                        request_id = %response.request_id,
                        error = %e,
                        "failed to query DB for backfill request correlation"
                    );
                    return Ok(());
                }
            }
        }
    };

    if attempt_ids.is_empty() {
        return Ok(());
    }

    if response.success {
        // Mark each tracked attempt as complete
        tracing::debug!(
            node_id = %node_id,
            request_id = %response.request_id,
            attempt_count = attempt_ids.len(),
            "marking tracked attempts as complete"
        );

        for attempt_id in attempt_ids {
            if let Err(e) = repo.mark_complete(attempt_id).await {
                tracing::error!(
                    node_id = %node_id,
                    attempt_id = %attempt_id,
                    error = %e,
                    "failed to mark attempt as complete"
                );
            }
        }
    } else {
        // Log the error - attempts need to be reset to partial state
        tracing::warn!(
            node_id = %node_id,
            request_id = %response.request_id,
            error = ?response.error,
            "backfill request failed"
        );

        // Reset tracked attempts to partial state so they can be retried
        tracing::debug!(
            node_id = %node_id,
            request_id = %response.request_id,
            attempt_count = attempt_ids.len(),
            "resetting tracked attempts to partial"
        );

        for attempt_id in attempt_ids {
            if let Err(e) = repo.reset_attempt_to_partial(attempt_id).await {
                tracing::error!(
                    node_id = %node_id,
                    attempt_id = %attempt_id,
                    error = %e,
                    "failed to reset attempt to partial"
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod op_batch_tests {
    use super::*;
    use chrono::Utc;
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::nodes::ws::message::OutboxOp;

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
        sqlx::PgPool::connect(&database_url().unwrap())
            .await
            .expect("connect")
    }

    async fn create_test_organization(pool: &PgPool) -> Uuid {
        let org_id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO organizations (id, name, slug, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(org_id)
        .bind(format!("Test Org {}", org_id))
        .bind(format!("test-org-{}", org_id))
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("Failed to create test organization");
        org_id
    }

    async fn create_test_node(pool: &PgPool, org_id: Uuid) -> Uuid {
        let node_id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO nodes (id, organization_id, name, machine_id, last_heartbeat_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(node_id)
        .bind(org_id)
        .bind(format!("node-{}", node_id))
        .bind(format!("machine-{}", node_id))
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("Failed to create test node");
        node_id
    }

    async fn create_swarm_project(pool: &PgPool, org_id: Uuid) -> Uuid {
        let sp_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO swarm_projects (id, organization_id, name)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(sp_id)
        .bind(org_id)
        .bind(format!("Swarm Project {}", sp_id))
        .execute(pool)
        .await
        .expect("Failed to create swarm project");
        sp_id
    }

    async fn create_node_local_project(
        pool: &PgPool,
        node_id: Uuid,
        local_project_id: Uuid,
        swarm_project_id: Option<Uuid>,
    ) {
        let res = sqlx::query(
            r#"
            INSERT INTO node_local_projects (node_id, local_project_id, name, git_repo_path, swarm_project_id)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(node_id)
        .bind(local_project_id)
        .bind("local-proj")
        .bind("/repo/path")
        .bind(swarm_project_id)
        .execute(pool)
        .await;
        if let Err(e) = res {
            eprintln!("create_node_local_project (non-fatal): {}", e);
        }
    }

    async fn create_swarm_project_node(
        pool: &PgPool,
        swarm_project_id: Uuid,
        node_id: Uuid,
        local_project_id: Uuid,
    ) {
        sqlx::query(
            r#"
            INSERT INTO swarm_project_nodes (swarm_project_id, node_id, local_project_id, git_repo_path)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(swarm_project_id)
        .bind(node_id)
        .bind(local_project_id)
        .bind("/repo/path")
        .execute(pool)
        .await
        .expect("Failed to create swarm_project_nodes link");
    }

    async fn cleanup_node_op_log(pool: &PgPool, node_id: Uuid) {
        let _ = sqlx::query("DELETE FROM node_op_log WHERE node_id = $1")
            .bind(node_id)
            .execute(pool)
            .await;
    }

    async fn cleanup_shared_task(pool: &PgPool, source_node_id: Uuid, source_task_id: Uuid) {
        let _ = sqlx::query(
            "DELETE FROM shared_tasks WHERE source_node_id = $1 AND source_task_id = $2",
        )
        .bind(source_node_id)
        .bind(source_task_id)
        .execute(pool)
        .await;
    }

    async fn cleanup_node_local_projects(pool: &PgPool, node_id: Uuid) {
        let _ = sqlx::query("DELETE FROM node_local_projects WHERE node_id = $1")
            .bind(node_id)
            .execute(pool)
            .await;
    }

    async fn cleanup_swarm_project_nodes(pool: &PgPool, swarm_project_id: Uuid) {
        let _ = sqlx::query("DELETE FROM swarm_project_nodes WHERE swarm_project_id = $1")
            .bind(swarm_project_id)
            .execute(pool)
            .await;
    }

    async fn cleanup_swarm_project(pool: &PgPool, swarm_project_id: Uuid) {
        let _ = sqlx::query("DELETE FROM swarm_projects WHERE id = $1")
            .bind(swarm_project_id)
            .execute(pool)
            .await;
    }

    async fn cleanup_node(pool: &PgPool, node_id: Uuid) {
        let _ = sqlx::query("DELETE FROM nodes WHERE id = $1")
            .bind(node_id)
            .execute(pool)
            .await;
    }

    async fn cleanup_org(pool: &PgPool, org_id: Uuid) {
        let _ = sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(org_id)
            .execute(pool)
            .await;
    }

    async fn node_op_log_count_for_key(pool: &PgPool, node_id: Uuid, key: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM node_op_log WHERE node_id = $1 AND idempotency_key = $2",
        )
        .bind(node_id)
        .bind(key)
        .fetch_one(pool)
        .await
        .expect("count")
    }

    async fn node_op_log_max_seq(pool: &PgPool, node_id: Uuid) -> i64 {
        sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) FROM node_op_log WHERE node_id = $1")
            .bind(node_id)
            .fetch_one(pool)
            .await
            .expect("max seq")
    }

    async fn shared_task_status(
        pool: &PgPool,
        source_node_id: Uuid,
        source_task_id: Uuid,
    ) -> String {
        sqlx::query_scalar("SELECT status::text FROM shared_tasks WHERE source_node_id = $1 AND source_task_id = $2")
            .bind(source_node_id)
            .bind(source_task_id)
            .fetch_one(pool)
            .await
            .expect("shared task status")
    }

    fn make_op(
        seq: i64,
        local_task_id: Uuid,
        local_project_id: Uuid,
        status: &str,
        idempotency_key: &str,
    ) -> OutboxOp {
        OutboxOp {
            seq,
            op_type: "task.upsert".to_string(),
            entity_type: "task".to_string(),
            entity_id: local_task_id,
            payload: serde_json::json!({
                "id": local_task_id,
                "project_id": local_project_id,
                "title": "t",
                "description": null,
                "status": status,
            }),
            idempotency_key: idempotency_key.to_string(),
            fencing_token: None,
        }
    }

    #[tokio::test]
    async fn op_batch_applies_swarm_linked_task_idempotently_and_acks() {
        skip_without_db!();
        let pool = create_pool().await;
        let org_id = create_test_organization(&pool).await;
        let node_id = create_test_node(&pool, org_id).await;
        let local_project_id = Uuid::new_v4();
        let local_task_id = Uuid::new_v4();
        let swarm_project_id = create_swarm_project(&pool, org_id).await;
        create_node_local_project(&pool, node_id, local_project_id, Some(swarm_project_id)).await;
        create_swarm_project_node(&pool, swarm_project_id, node_id, local_project_id).await;

        let key = format!("task:{}:{}", local_project_id, local_task_id);
        let op = make_op(1, local_task_id, local_project_id, "done", &key);
        let ops = vec![op.clone()];

        let (seq, _revokes) = handle_op_batch_apply(node_id, org_id, "node-name", &ops, &pool)
            .await
            .expect("first apply");
        assert_eq!(
            seq, 1,
            "applied_through_seq advances to 1 after first apply"
        );
        assert_eq!(
            node_op_log_count_for_key(&pool, node_id, &key).await,
            1,
            "exactly one node_op_log row after first apply"
        );
        assert_eq!(
            shared_task_status(&pool, node_id, local_task_id).await,
            "done",
            "shared_tasks has the task with mapped status"
        );
        assert_eq!(
            node_op_log_max_seq(&pool, node_id).await,
            1,
            "max seq in node_op_log is 1"
        );

        let (seq2, _revokes2) = handle_op_batch_apply(node_id, org_id, "node-name", &ops, &pool)
            .await
            .expect("second apply");
        assert_eq!(seq2, 1, "applied_through_seq stays at 1 after duplicate");
        assert_eq!(
            node_op_log_count_for_key(&pool, node_id, &key).await,
            1,
            "still ONE node_op_log row (ON CONFLICT DO NOTHING)"
        );

        cleanup_shared_task(&pool, node_id, local_task_id).await;
        cleanup_node_op_log(&pool, node_id).await;
        cleanup_swarm_project_nodes(&pool, swarm_project_id).await;
        cleanup_node_local_projects(&pool, node_id).await;
        cleanup_swarm_project(&pool, swarm_project_id).await;
        cleanup_node(&pool, node_id).await;
        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    #[allow(non_snake_case)]
    async fn op_batch_PARKS_when_local_project_link_absent() {
        skip_without_db!();
        let pool = create_pool().await;
        let org_id = create_test_organization(&pool).await;
        let node_id = create_test_node(&pool, org_id).await;
        let local_project_id = Uuid::new_v4();
        let local_task_id = Uuid::new_v4();

        let key = format!("task:{}:{}", local_project_id, local_task_id);
        let op = make_op(1, local_task_id, local_project_id, "done", &key);
        let ops = vec![op.clone()];

        let (seq, _revokes) = handle_op_batch_apply(node_id, org_id, "node-name", &ops, &pool)
            .await
            .expect("apply should not error on park");
        assert_eq!(
            seq, 0,
            "applied_through_seq does NOT advance to 1 (stays at high-water 0) on PARK"
        );
        assert_eq!(
            node_op_log_count_for_key(&pool, node_id, &key).await,
            0,
            "NO node_op_log row for the key → node re-sends"
        );

        cleanup_node_op_log(&pool, node_id).await;
        cleanup_node(&pool, node_id).await;
        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    #[allow(non_snake_case)]
    async fn op_batch_SKIPS_AND_ADVANCES_when_project_present_but_not_swarm_linked() {
        skip_without_db!();
        let pool = create_pool().await;
        let org_id = create_test_organization(&pool).await;
        let node_id = create_test_node(&pool, org_id).await;
        let local_project_id = Uuid::new_v4();
        let local_task_id = Uuid::new_v4();
        create_node_local_project(&pool, node_id, local_project_id, None).await;

        let key = format!("task:{}:{}", local_project_id, local_task_id);
        let op = make_op(1, local_task_id, local_project_id, "done", &key);
        let ops = vec![op.clone()];

        let (seq, _revokes) = handle_op_batch_apply(node_id, org_id, "node-name", &ops, &pool)
            .await
            .expect("apply should not error on skip+advance");
        assert_eq!(
            seq, 1,
            "applied_through_seq DOES advance to 1 (op acked/skipped, NOT parked)"
        );
        assert_eq!(
            node_op_log_count_for_key(&pool, node_id, &key).await,
            1,
            "node_op_log records the skipped op (cursor + dedup consistent)"
        );

        cleanup_node_op_log(&pool, node_id).await;
        cleanup_node_local_projects(&pool, node_id).await;
        cleanup_node(&pool, node_id).await;
        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    async fn op_batch_maps_node_lowercase_status_explicitly() {
        skip_without_db!();
        let pool = create_pool().await;
        let org_id = create_test_organization(&pool).await;
        let node_id = create_test_node(&pool, org_id).await;
        let local_project_id = Uuid::new_v4();
        let swarm_project_id = create_swarm_project(&pool, org_id).await;
        create_node_local_project(&pool, node_id, local_project_id, Some(swarm_project_id)).await;
        create_swarm_project_node(&pool, swarm_project_id, node_id, local_project_id).await;

        let local_task_id_1 = Uuid::new_v4();
        let key_1 = format!("task:{}:{}", local_project_id, local_task_id_1);
        let op_1 = make_op(1, local_task_id_1, local_project_id, "inprogress", &key_1);
        let (seq, _revokes) = handle_op_batch_apply(node_id, org_id, "node-name", &[op_1], &pool)
            .await
            .expect("apply inprogress");
        assert_eq!(seq, 1);
        assert_eq!(
            shared_task_status(&pool, node_id, local_task_id_1).await,
            "in-progress",
            "node 'inprogress' maps to hive 'in-progress', NOT the wrong fallback"
        );

        let local_task_id_2 = Uuid::new_v4();
        let key_2 = format!("task:{}:{}", local_project_id, local_task_id_2);
        let op_2 = make_op(2, local_task_id_2, local_project_id, "inreview", &key_2);
        let (seq, _revokes) = handle_op_batch_apply(node_id, org_id, "node-name", &[op_2], &pool)
            .await
            .expect("apply inreview");
        assert_eq!(seq, 2);
        assert_eq!(
            shared_task_status(&pool, node_id, local_task_id_2).await,
            "in-review",
            "node 'inreview' maps to hive 'in-review', NOT the wrong fallback"
        );

        cleanup_shared_task(&pool, node_id, local_task_id_1).await;
        cleanup_shared_task(&pool, node_id, local_task_id_2).await;
        cleanup_node_op_log(&pool, node_id).await;
        cleanup_swarm_project_nodes(&pool, swarm_project_id).await;
        cleanup_node_local_projects(&pool, node_id).await;
        cleanup_swarm_project(&pool, swarm_project_id).await;
        cleanup_node(&pool, node_id).await;
        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    async fn op_batch_does_not_lose_apply_when_upsert_fails_then_retried() {
        skip_without_db!();
        let pool = create_pool().await;
        let org_id = create_test_organization(&pool).await;
        let node_id = create_test_node(&pool, org_id).await;
        let local_project_id = Uuid::new_v4();
        let local_task_id = Uuid::new_v4();
        let swarm_project_id = create_swarm_project(&pool, org_id).await;
        create_node_local_project(&pool, node_id, local_project_id, Some(swarm_project_id)).await;
        create_swarm_project_node(&pool, swarm_project_id, node_id, local_project_id).await;

        let key = format!("task:{}:{}", local_project_id, local_task_id);
        let op = make_op(1, local_task_id, local_project_id, "done", &key);

        // Weaker invariant (per task note): a node_op_log row exists ONLY for ops whose
        // shared_tasks apply is present — never a dedup row without its task. Injecting an
        // upsert failure is impractical at this seam (upsert_from_node lives in tasks.rs and
        // is not mockable here without touching an unlisted file). Apply-then-record ordering
        // guarantees this invariant structurally: the dedup INSERT runs only after upsert Ok.
        let (seq, _revokes) = handle_op_batch_apply(node_id, org_id, "node-name", &[op], &pool)
            .await
            .expect("apply");
        assert_eq!(seq, 1);

        let log_count = node_op_log_count_for_key(&pool, node_id, &key).await;
        let task_status = shared_task_status(&pool, node_id, local_task_id).await;
        assert_eq!(log_count, 1, "dedup row present");
        assert_eq!(task_status, "done", "task apply present");
    }
}

#[cfg(test)]
mod fencing_tests {
    use super::*;
    use crate::db::task_assignments::TaskAssignmentRepository;
    use chrono::Utc;
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::nodes::ws::message::OutboxOp;

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
        sqlx::PgPool::connect(&url)
            .await
            .expect("Failed to connect to database")
    }

    async fn create_test_organization(pool: &PgPool) -> Uuid {
        let org_id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO organizations (id, name, slug, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(org_id)
        .bind(format!("Test Org {}", org_id))
        .bind(format!("test-org-{}", org_id))
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("Failed to create test organization");
        org_id
    }

    async fn create_test_node(pool: &PgPool, org_id: Uuid) -> Uuid {
        let node_id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO nodes (id, organization_id, name, machine_id, last_heartbeat_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(node_id)
        .bind(org_id)
        .bind(format!("node-{}", node_id))
        .bind(format!("machine-{}", node_id))
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("Failed to create test node");
        node_id
    }

    async fn create_swarm_project(pool: &PgPool, org_id: Uuid) -> Uuid {
        let sp_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO swarm_projects (id, organization_id, name)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(sp_id)
        .bind(org_id)
        .bind(format!("Swarm Project {}", sp_id))
        .execute(pool)
        .await
        .expect("Failed to create swarm project");
        sp_id
    }

    async fn create_node_local_project(
        pool: &PgPool,
        node_id: Uuid,
        local_project_id: Uuid,
        swarm_project_id: Option<Uuid>,
    ) {
        let res = sqlx::query(
            r#"
            INSERT INTO node_local_projects (node_id, local_project_id, name, git_repo_path, swarm_project_id)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(node_id)
        .bind(local_project_id)
        .bind("local-proj")
        .bind("/repo/path")
        .bind(swarm_project_id)
        .execute(pool)
        .await;
        if let Err(e) = res {
            eprintln!("create_node_local_project (non-fatal): {}", e);
        }
    }

    async fn create_swarm_project_node(
        pool: &PgPool,
        swarm_project_id: Uuid,
        node_id: Uuid,
        local_project_id: Uuid,
    ) -> Uuid {
        let row = sqlx::query(
            r#"
            INSERT INTO swarm_project_nodes (swarm_project_id, node_id, local_project_id, git_repo_path)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(swarm_project_id)
        .bind(node_id)
        .bind(local_project_id)
        .bind("/repo/path")
        .fetch_one(pool)
        .await
        .expect("Failed to create swarm_project_nodes link");
        sqlx::Row::get(&row, "id")
    }

    async fn cleanup_org(pool: &PgPool, org_id: Uuid) {
        let _ = sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(org_id)
            .execute(pool)
            .await;
    }

    async fn node_op_log_count_for_key(pool: &PgPool, node_id: Uuid, key: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM node_op_log WHERE node_id = $1 AND idempotency_key = $2",
        )
        .bind(node_id)
        .bind(key)
        .fetch_one(pool)
        .await
        .expect("count")
    }

    async fn shared_task_status_by_id(pool: &PgPool, shared_id: Uuid) -> Option<String> {
        sqlx::query_scalar("SELECT status::text FROM shared_tasks WHERE id = $1")
            .bind(shared_id)
            .fetch_optional(pool)
            .await
            .expect("shared task status by id")
    }

    async fn shared_task_status_by_source(
        pool: &PgPool,
        source_node_id: Uuid,
        source_task_id: Uuid,
    ) -> String {
        sqlx::query_scalar(
            "SELECT status::text FROM shared_tasks WHERE source_node_id = $1 AND source_task_id = $2",
        )
        .bind(source_node_id)
        .bind(source_task_id)
        .fetch_one(pool)
        .await
        .expect("shared task status by source")
    }

    /// Insert a shared_tasks row directly (created by `creator_node` with local id
    /// `creator_local_task_id`), returning the shared task id. Used to seed a task the
    /// sender did NOT create (the ASSIGNED-NOT-CREATED reassignment scenario, R2/F2).
    async fn insert_shared_task(
        pool: &PgPool,
        org_id: Uuid,
        swarm_project_id: Uuid,
        creator_node: Uuid,
        creator_local_task_id: Uuid,
        status: &str,
    ) -> Uuid {
        let now = Utc::now();
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO shared_tasks (
                id, organization_id, project_id, swarm_project_id,
                source_node_id, source_task_id,
                title, status, version, shared_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $3, $4, $5, $6, $7::task_status, 1, $8, $8, $8)
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(swarm_project_id)
        .bind(creator_node)
        .bind(creator_local_task_id)
        .bind("seeded task")
        .bind(status)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert shared task");
        id
    }

    /// Build an OutboxOp literally (make_op in op_batch_tests does not set shared_task_id
    /// or fencing_token; build the struct directly here to avoid changing make_op, which
    /// 4 existing tests depend on).
    fn make_fence_op(
        seq: i64,
        local_task_id: Uuid,
        local_project_id: Uuid,
        shared_task_id: Option<Uuid>,
        fencing_token: Option<i64>,
        status: &str,
        idempotency_key: &str,
    ) -> OutboxOp {
        let payload = match shared_task_id {
            Some(sid) => serde_json::json!({
                "id": local_task_id,
                "project_id": local_project_id,
                "shared_task_id": sid,
                "title": "t",
                "description": null,
                "status": status,
            }),
            None => serde_json::json!({
                "id": local_task_id,
                "project_id": local_project_id,
                "title": "t",
                "description": null,
                "status": status,
            }),
        };
        OutboxOp {
            seq,
            op_type: "task.upsert".to_string(),
            entity_type: "task".to_string(),
            entity_id: local_task_id,
            payload,
            idempotency_key: idempotency_key.to_string(),
            fencing_token,
        }
    }

    #[tokio::test]
    async fn op_against_assigned_task_with_stale_token_is_rejected_not_applied() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_c = create_test_node(&pool, org_id).await; // CREATOR
        let node_a = create_test_node(&pool, org_id).await; // first holder (stale)
        let node_b = create_test_node(&pool, org_id).await; // current holder

        let swarm_project_id = create_swarm_project(&pool, org_id).await;
        // Each node needs a swarm_project_nodes link (provides node_project_id for try_claim)
        // and a node_local_projects row (context resolution in handle_op_batch_apply).
        let local_proj_c = Uuid::new_v4();
        let local_proj_a = Uuid::new_v4();
        let local_proj_b = Uuid::new_v4();
        create_node_local_project(&pool, node_c, local_proj_c, Some(swarm_project_id)).await;
        create_node_local_project(&pool, node_a, local_proj_a, Some(swarm_project_id)).await;
        create_node_local_project(&pool, node_b, local_proj_b, Some(swarm_project_id)).await;
        let _np_c = create_swarm_project_node(&pool, swarm_project_id, node_c, local_proj_c).await;
        let np_a = create_swarm_project_node(&pool, swarm_project_id, node_a, local_proj_a).await;
        let _np_b = create_swarm_project_node(&pool, swarm_project_id, node_b, local_proj_b).await;

        // Shared task CREATED BY node_c (source_node_id = node_c). node_a did NOT create it,
        // so find_by_source_task_id(node_a, a_local) resolves NOTHING — only payload.shared_task_id
        // resolves the assignment (the SC3 guard).
        let c_local_task_id = Uuid::new_v4();
        let shared_id = insert_shared_task(
            &pool,
            org_id,
            swarm_project_id,
            node_c,
            c_local_task_id,
            "todo",
        )
        .await;

        // node_a claims (T1) with a PAST TTL so the lease is already expired.
        let claim_a = repo
            .try_claim(shared_id, node_a, np_a, chrono::Duration::seconds(-300))
            .await
            .expect("claim a")
            .expect("node_a claimed");
        let t1 = claim_a.fencing_token;
        assert!(t1 > 0, "first claim bumps a positive token");

        // node_b reclaims (T2 > T1) — the expired lease lets try_claim bump the token.
        let claim_b = repo
            .try_claim(shared_id, node_b, _np_b, chrono::Duration::seconds(300))
            .await
            .expect("claim b")
            .expect("node_b reclaimed");
        let t2 = claim_b.fencing_token;
        assert!(t2 > t1, "reassignment bumps the fencing token (T2 > T1)");

        // Sanity: node_a's old assignment row is the SAME row, now held by node_b. Its id is
        // claim_b.assignment_id (the row was UPDATEd in place by try_claim). This is the id
        // the LeaseRevoked must reference.
        let assignment_id = claim_b.assignment_id;

        // node_a (partitioned-but-alive) sends a stale op stamped fencing_token = T1.
        let a_local_task_id = Uuid::new_v4();
        let key = format!("task:{}:{}", local_proj_a, a_local_task_id);
        let op = make_fence_op(
            1,
            a_local_task_id,
            local_proj_a,
            Some(shared_id),
            Some(t1),
            "done",
            &key,
        );

        let pre_status = shared_task_status_by_id(&pool, shared_id)
            .await
            .expect("task exists pre-apply");

        let (seq, revokes) = handle_op_batch_apply(node_a, org_id, "node-a", &[op], &pool)
            .await
            .expect("apply");

        // (a) shared_tasks NOT updated by the stale op (status unchanged).
        assert_eq!(
            shared_task_status_by_id(&pool, shared_id).await,
            Some(pre_status),
            "stale-token op MUST NOT update shared_tasks"
        );
        // (b) node_op_log has NO row for the op's idempotency_key.
        assert_eq!(
            node_op_log_count_for_key(&pool, node_a, &key).await,
            0,
            "rejected op MUST NOT record a node_op_log dedup row"
        );
        // (c) returned seq does NOT advance past the rejected op's seq (high-water stays at
        // the pre-reject value — break, not continue).
        assert_eq!(
            seq, 0,
            "applied_through_seq MUST NOT advance past the rejected op (break, not continue)"
        );
        // (d) the revoke vec contains (assignment_id, "stale fencing token").
        assert_eq!(revokes.len(), 1, "exactly one LeaseRevoked emitted");
        assert_eq!(revokes[0].0, assignment_id, "revoked assignment_id matches");
        assert_eq!(
            revokes[0].1, "stale fencing token",
            "revoke reason matches the contract"
        );

        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    async fn op_with_current_token_against_assigned_task_applies_normally() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        // node_b is BOTH the creator AND the current holder — so its op's upsert keys on
        // (source_node_id=node_b, source_task_id=b_local) and UPDATEs the existing row (not
        // a new INSERT). This isolates the test to the fence behavior, not 106's source-key
        // semantics. The fence still guards: an active assignment exists, the op carries the
        // current token T2, so it MUST apply.
        let node_b = create_test_node(&pool, org_id).await;

        let swarm_project_id = create_swarm_project(&pool, org_id).await;
        let local_proj_b = Uuid::new_v4();
        create_node_local_project(&pool, node_b, local_proj_b, Some(swarm_project_id)).await;
        let np_b = create_swarm_project_node(&pool, swarm_project_id, node_b, local_proj_b).await;

        let b_local_task_id = Uuid::new_v4();
        let shared_id = insert_shared_task(
            &pool,
            org_id,
            swarm_project_id,
            node_b,
            b_local_task_id,
            "in-progress",
        )
        .await;

        // node_b claims (T2) — the rightful current holder.
        let claim_b = repo
            .try_claim(shared_id, node_b, np_b, chrono::Duration::seconds(300))
            .await
            .expect("claim b")
            .expect("node_b claimed");
        let t2 = claim_b.fencing_token;
        assert!(t2 > 0, "claim bumps a positive token");

        // node_b sends an op stamped fencing_token = T2 (current) → applies. The transition
        // in-progress→done is node-authored (ADR-0010 §D), so 303's author guard also
        // accepts it (seeding as 'in-progress' rather than 'todo' keeps this fence test
        // matrix-compliant after 303 landed).
        let key = format!("task:{}:{}", local_proj_b, b_local_task_id);
        let op = make_fence_op(
            1,
            b_local_task_id,
            local_proj_b,
            Some(shared_id),
            Some(t2),
            "done",
            &key,
        );

        let (seq, revokes) = handle_op_batch_apply(node_b, org_id, "node-b", &[op], &pool)
            .await
            .expect("apply");

        assert_eq!(
            seq, 1,
            "applied_through_seq advances to op.seq on a current-token apply"
        );
        assert_eq!(
            node_op_log_count_for_key(&pool, node_b, &key).await,
            1,
            "current-token op records a node_op_log dedup row"
        );
        assert_eq!(
            shared_task_status_by_id(&pool, shared_id).await,
            Some("done".to_string()),
            "shared_tasks is updated by the current-token op"
        );
        assert!(revokes.is_empty(), "no LeaseRevoked for a current-token op");

        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    async fn op_with_null_token_node_owned_work_is_unaffected_by_the_fence() {
        skip_without_db!();
        let pool = create_pool().await;

        let org_id = create_test_organization(&pool).await;
        let node_id = create_test_node(&pool, org_id).await;

        let swarm_project_id = create_swarm_project(&pool, org_id).await;
        let local_project_id = Uuid::new_v4();
        create_node_local_project(&pool, node_id, local_project_id, Some(swarm_project_id)).await;
        create_swarm_project_node(&pool, swarm_project_id, node_id, local_project_id).await;

        let local_task_id = Uuid::new_v4();
        let key = format!("task:{}:{}", local_project_id, local_task_id);
        // No shared_task_id, no fencing_token — node-owned work (CONTRACT §C / ADR-0009).
        // No active assignment exists for this task, so the fence does not apply.
        let op = make_fence_op(1, local_task_id, local_project_id, None, None, "done", &key);

        let (seq, revokes) = handle_op_batch_apply(node_id, org_id, "node-name", &[op], &pool)
            .await
            .expect("apply");

        assert_eq!(
            seq, 1,
            "node-owned op applies and advances applied_through_seq"
        );
        assert_eq!(
            node_op_log_count_for_key(&pool, node_id, &key).await,
            1,
            "node-owned op records a node_op_log dedup row"
        );
        // shared_tasks row is created by the apply (origin_node_id = node_id).
        assert_eq!(
            shared_task_status_by_source(&pool, node_id, local_task_id).await,
            "done",
            "node-owned shared_tasks row is created with the mapped status"
        );
        assert!(revokes.is_empty(), "no LeaseRevoked for node-owned work");

        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    async fn op_for_completed_task_with_no_active_assignment_is_rejected_not_applied() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_a = create_test_node(&pool, org_id).await; // partitioned writer (stale)
        let node_b = create_test_node(&pool, org_id).await; // rightful holder who completed

        let swarm_project_id = create_swarm_project(&pool, org_id).await;
        let local_proj_a = Uuid::new_v4();
        let local_proj_b = Uuid::new_v4();
        create_node_local_project(&pool, node_a, local_proj_a, Some(swarm_project_id)).await;
        create_node_local_project(&pool, node_b, local_proj_b, Some(swarm_project_id)).await;
        let np_a = create_swarm_project_node(&pool, swarm_project_id, node_a, local_proj_a).await;
        let np_b = create_swarm_project_node(&pool, swarm_project_id, node_b, local_proj_b).await;

        let a_local_task_id = Uuid::new_v4();
        let shared_id = insert_shared_task(
            &pool,
            org_id,
            swarm_project_id,
            node_a,
            a_local_task_id,
            "todo",
        )
        .await;

        // node_a claims (T1), then its lease expires and node_b reclaims (T2 > T1).
        let claim_a = repo
            .try_claim(shared_id, node_a, np_a, chrono::Duration::seconds(-300))
            .await
            .expect("claim a")
            .expect("node_a claimed");
        let t1 = claim_a.fencing_token;

        let claim_b = repo
            .try_claim(shared_id, node_b, np_b, chrono::Duration::seconds(300))
            .await
            .expect("claim b")
            .expect("node_b reclaimed");
        let t2 = claim_b.fencing_token;
        assert!(t2 > t1, "reassignment bumps token");

        // node_b COMPLETES the task → completed_at is set on the assignment row.
        repo.complete(claim_b.assignment_id, "done")
            .await
            .expect("complete");

        // Now there is NO active assignment (completed_at IS NULL returns nothing).
        // node_a (partitioned, late) sends an op stamped with its stale token T1.
        let key = format!("task:{}:{}", local_proj_a, a_local_task_id);
        let op = make_fence_op(
            1,
            a_local_task_id,
            local_proj_a,
            Some(shared_id),
            Some(t1),
            "done",
            &key,
        );

        let pre_status = shared_task_status_by_id(&pool, shared_id)
            .await
            .expect("task exists pre-apply");

        let (seq, _revokes) = handle_op_batch_apply(node_a, org_id, "node-a", &[op], &pool)
            .await
            .expect("apply");

        // (a) shared_tasks NOT updated — the late op must not overwrite the completed task.
        assert_eq!(
            shared_task_status_by_id(&pool, shared_id).await,
            Some(pre_status),
            "late op for completed task MUST NOT update shared_tasks (SC3)"
        );
        // (b) node_op_log has NO row for the op's idempotency_key.
        assert_eq!(
            node_op_log_count_for_key(&pool, node_a, &key).await,
            0,
            "rejected op MUST NOT record a node_op_log dedup row"
        );
        // (c) seq does NOT advance past the rejected op.
        assert_eq!(
            seq, 0,
            "applied_through_seq MUST NOT advance past the rejected op (break, not continue)"
        );

        cleanup_org(&pool, org_id).await;
    }
}

#[cfg(test)]
mod lease_heartbeat_tests {
    use super::*;
    use crate::db::task_assignments::TaskAssignmentRepository;
    use chrono::Utc;
    use sqlx::PgPool;
    use sqlx::Row;
    use uuid::Uuid;

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
        sqlx::PgPool::connect(&url)
            .await
            .expect("Failed to connect to database")
    }

    async fn create_test_organization(pool: &PgPool) -> Uuid {
        let org_id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO organizations (id, name, slug, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(org_id)
        .bind(format!("Test Org {}", org_id))
        .bind(format!("test-org-{}", org_id))
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("Failed to create test organization");

        org_id
    }

    async fn create_test_node(pool: &PgPool, org_id: Uuid) -> Uuid {
        let node_id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO nodes (id, organization_id, name, machine_id, status, capabilities, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 'online', '{}'::jsonb, $5, $6)
            "#,
        )
        .bind(node_id)
        .bind(org_id)
        .bind(format!("node-{}", node_id))
        .bind(format!("machine-{}", node_id))
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("Failed to create test node");

        node_id
    }

    async fn create_test_swarm_project(pool: &PgPool, org_id: Uuid) -> Uuid {
        let sp_id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO swarm_projects (id, organization_id, name, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(sp_id)
        .bind(org_id)
        .bind(format!("Swarm Project {}", sp_id))
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("Failed to create test swarm project");

        sp_id
    }

    async fn create_test_swarm_project_node(
        pool: &PgPool,
        swarm_project_id: Uuid,
        node_id: Uuid,
    ) -> Uuid {
        let local_project_id = Uuid::new_v4();

        let row = sqlx::query(
            r#"
            INSERT INTO swarm_project_nodes (swarm_project_id, node_id, local_project_id, git_repo_path)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(swarm_project_id)
        .bind(node_id)
        .bind(local_project_id)
        .bind("test-repo")
        .fetch_one(pool)
        .await
        .expect("Failed to create test swarm project node");

        row.get("id")
    }

    async fn create_test_shared_task(pool: &PgPool, org_id: Uuid) -> Uuid {
        let task_id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO shared_tasks (id, organization_id, title, status, created_at, updated_at)
            VALUES ($1, $2, $3, 'todo'::task_status, $4, $5)
            "#,
        )
        .bind(task_id)
        .bind(org_id)
        .bind(format!("Test Task {}", task_id))
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("Failed to create test shared task");

        task_id
    }

    async fn cleanup_org(pool: &PgPool, org_id: Uuid) {
        let _ = sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(org_id)
            .execute(pool)
            .await;
    }

    #[tokio::test]
    async fn renew_extends_held_leases_and_returns_a_grant_per_assignment() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_a = create_test_node(&pool, org_id).await;
        let swarm_project = create_test_swarm_project(&pool, org_id).await;
        let np_id = create_test_swarm_project_node(&pool, swarm_project, node_a).await;
        let task_1 = create_test_shared_task(&pool, org_id).await;
        let task_2 = create_test_shared_task(&pool, org_id).await;

        let claim_1 = repo
            .try_claim(task_1, node_a, np_id, chrono::Duration::seconds(30))
            .await
            .unwrap()
            .expect("claim 1");
        let claim_2 = repo
            .try_claim(task_2, node_a, np_id, chrono::Duration::seconds(30))
            .await
            .unwrap()
            .expect("claim 2");

        let pre_token_1 = claim_1.fencing_token;
        let pre_token_2 = claim_2.fencing_token;

        let grants = handle_lease_heartbeat_renew(
            node_a,
            &[claim_1.assignment_id, claim_2.assignment_id],
            &pool,
        )
        .await
        .expect("renew");

        assert_eq!(grants.len(), 2, "one grant per held assignment");
        for g in &grants {
            assert!(
                g.lease_expires_at > chrono::Utc::now(),
                "renewed lease in the future"
            );
            assert!(g.fencing_token > 0, "token present");
        }
        let grant_1 = grants
            .iter()
            .find(|g| g.assignment_id == claim_1.assignment_id)
            .expect("grant 1");
        let grant_2 = grants
            .iter()
            .find(|g| g.assignment_id == claim_2.assignment_id)
            .expect("grant 2");
        assert_eq!(
            grant_1.fencing_token, pre_token_1,
            "renewal does NOT bump fencing token (1)"
        );
        assert_eq!(
            grant_2.fencing_token, pre_token_2,
            "renewal does NOT bump fencing token (2)"
        );

        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    async fn renew_skips_assignments_not_held_by_this_node() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_a = create_test_node(&pool, org_id).await;
        let node_b = create_test_node(&pool, org_id).await;
        let swarm_project = create_test_swarm_project(&pool, org_id).await;
        let np_id_a = create_test_swarm_project_node(&pool, swarm_project, node_a).await;
        let _np_id_b = create_test_swarm_project_node(&pool, swarm_project, node_b).await;
        let task_1 = create_test_shared_task(&pool, org_id).await;

        let claim_a = repo
            .try_claim(task_1, node_a, np_id_a, chrono::Duration::seconds(300))
            .await
            .unwrap()
            .expect("node_a claims");

        let grants = handle_lease_heartbeat_renew(node_b, &[claim_a.assignment_id], &pool)
            .await
            .expect("renew");
        assert!(grants.is_empty(), "no grant for a foreign assignment");

        cleanup_org(&pool, org_id).await;
    }
}

#[cfg(test)]
mod status_guard_tests {
    use super::*;
    use crate::db::task_assignments::TaskAssignmentRepository;
    use chrono::Utc;
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::nodes::ws::message::OutboxOp;

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
        sqlx::PgPool::connect(&url)
            .await
            .expect("Failed to connect to database")
    }

    async fn create_test_organization(pool: &PgPool) -> Uuid {
        let org_id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO organizations (id, name, slug, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(org_id)
        .bind(format!("Test Org {}", org_id))
        .bind(format!("test-org-{}", org_id))
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("Failed to create test organization");
        org_id
    }

    async fn create_test_node(pool: &PgPool, org_id: Uuid) -> Uuid {
        let node_id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO nodes (id, organization_id, name, machine_id, last_heartbeat_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(node_id)
        .bind(org_id)
        .bind(format!("node-{}", node_id))
        .bind(format!("machine-{}", node_id))
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("Failed to create test node");
        node_id
    }

    async fn create_swarm_project(pool: &PgPool, org_id: Uuid) -> Uuid {
        let sp_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO swarm_projects (id, organization_id, name)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(sp_id)
        .bind(org_id)
        .bind(format!("Swarm Project {}", sp_id))
        .execute(pool)
        .await
        .expect("Failed to create swarm project");
        sp_id
    }

    async fn create_node_local_project(
        pool: &PgPool,
        node_id: Uuid,
        local_project_id: Uuid,
        swarm_project_id: Option<Uuid>,
    ) {
        let res = sqlx::query(
            r#"
            INSERT INTO node_local_projects (node_id, local_project_id, name, git_repo_path, swarm_project_id)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(node_id)
        .bind(local_project_id)
        .bind("local-proj")
        .bind("/repo/path")
        .bind(swarm_project_id)
        .execute(pool)
        .await;
        if let Err(e) = res {
            eprintln!("create_node_local_project (non-fatal): {}", e);
        }
    }

    async fn create_swarm_project_node(
        pool: &PgPool,
        swarm_project_id: Uuid,
        node_id: Uuid,
        local_project_id: Uuid,
    ) -> Uuid {
        let row = sqlx::query(
            r#"
            INSERT INTO swarm_project_nodes (swarm_project_id, node_id, local_project_id, git_repo_path)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(swarm_project_id)
        .bind(node_id)
        .bind(local_project_id)
        .bind("/repo/path")
        .fetch_one(pool)
        .await
        .expect("Failed to create swarm_project_nodes link");
        sqlx::Row::get(&row, "id")
    }

    async fn cleanup_org(pool: &PgPool, org_id: Uuid) {
        let _ = sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(org_id)
            .execute(pool)
            .await;
    }

    async fn node_op_log_count_for_key(pool: &PgPool, node_id: Uuid, key: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM node_op_log WHERE node_id = $1 AND idempotency_key = $2",
        )
        .bind(node_id)
        .bind(key)
        .fetch_one(pool)
        .await
        .expect("count")
    }

    async fn node_op_log_max_seq(pool: &PgPool, node_id: Uuid) -> i64 {
        sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) FROM node_op_log WHERE node_id = $1")
            .bind(node_id)
            .fetch_one(pool)
            .await
            .expect("max seq")
    }

    async fn shared_task_status_by_id(pool: &PgPool, shared_id: Uuid) -> Option<String> {
        sqlx::query_scalar("SELECT status::text FROM shared_tasks WHERE id = $1")
            .bind(shared_id)
            .fetch_optional(pool)
            .await
            .expect("shared task status by id")
    }

    /// Insert a shared_tasks row directly (created by `creator_node` with local id
    /// `creator_local_task_id`), returning the shared task id. Used to seed a task the
    /// sender did NOT create (the ASSIGNED-NOT-CREATED reassignment scenario, R2/F2).
    async fn insert_shared_task(
        pool: &PgPool,
        org_id: Uuid,
        swarm_project_id: Uuid,
        creator_node: Uuid,
        creator_local_task_id: Uuid,
        status: &str,
    ) -> Uuid {
        let now = Utc::now();
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO shared_tasks (
                id, organization_id, project_id, swarm_project_id,
                source_node_id, source_task_id,
                title, status, version, shared_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $3, $4, $5, $6, $7::task_status, 1, $8, $8, $8)
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(swarm_project_id)
        .bind(creator_node)
        .bind(creator_local_task_id)
        .bind("seeded task")
        .bind(status)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert shared task");
        id
    }

    /// Build an OutboxOp literally (make_op in op_batch_tests does not set shared_task_id
    /// or fencing_token; build the struct directly here to avoid changing make_op, which
    /// 4 existing tests depend on).
    fn make_fence_op(
        seq: i64,
        local_task_id: Uuid,
        local_project_id: Uuid,
        shared_task_id: Option<Uuid>,
        fencing_token: Option<i64>,
        status: &str,
        idempotency_key: &str,
    ) -> OutboxOp {
        let payload = match shared_task_id {
            Some(sid) => serde_json::json!({
                "id": local_task_id,
                "project_id": local_project_id,
                "shared_task_id": sid,
                "title": "t",
                "description": null,
                "status": status,
            }),
            None => serde_json::json!({
                "id": local_task_id,
                "project_id": local_project_id,
                "title": "t",
                "description": null,
                "status": status,
            }),
        };
        OutboxOp {
            seq,
            op_type: "task.upsert".to_string(),
            entity_type: "task".to_string(),
            entity_id: local_task_id,
            payload,
            idempotency_key: idempotency_key.to_string(),
            fencing_token,
        }
    }

    #[tokio::test]
    async fn node_reported_in_progress_to_done_accepted_with_valid_lease_and_token() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        // node_b is BOTH the creator AND the current holder — so its op's upsert keys on
        // (source_node_id=node_b, source_task_id=b_local) and UPDATEs the seeded row
        // (isolating the test to the guard's behavior, not 106's source-key semantics).
        let node_b = create_test_node(&pool, org_id).await;
        let swarm_project_id = create_swarm_project(&pool, org_id).await;
        let local_proj_b = Uuid::new_v4();
        create_node_local_project(&pool, node_b, local_proj_b, Some(swarm_project_id)).await;
        let np_b = create_swarm_project_node(&pool, swarm_project_id, node_b, local_proj_b).await;

        let b_local_task_id = Uuid::new_v4();
        let shared_id = insert_shared_task(
            &pool,
            org_id,
            swarm_project_id,
            node_b,
            b_local_task_id,
            "in-progress",
        )
        .await;

        // node_b holds an active assignment with token T.
        let claim_b = repo
            .try_claim(shared_id, node_b, np_b, chrono::Duration::seconds(300))
            .await
            .expect("claim b")
            .expect("node_b claimed");
        let t = claim_b.fencing_token;
        assert!(t > 0, "claim bumps a positive token");

        // node→hive op: in-progress→done (node-authored), stamped with the current token T.
        let key = format!("task:{}:{}", local_proj_b, b_local_task_id);
        let op = make_fence_op(
            1,
            b_local_task_id,
            local_proj_b,
            Some(shared_id),
            Some(t),
            "done",
            &key,
        );

        let (seq, revokes) = handle_op_batch_apply(node_b, org_id, "node-b", &[op], &pool)
            .await
            .expect("apply");

        assert_eq!(
            seq, 1,
            "applied_through_seq advances to op.seq on an accepted node-authored transition"
        );
        assert_eq!(
            node_op_log_count_for_key(&pool, node_b, &key).await,
            1,
            "accepted transition records a node_op_log dedup row"
        );
        assert_eq!(
            node_op_log_max_seq(&pool, node_b).await,
            1,
            "max seq in node_op_log is 1"
        );
        assert_eq!(
            shared_task_status_by_id(&pool, shared_id).await,
            Some("done".to_string()),
            "shared_tasks.status is updated by the accepted op"
        );
        assert!(revokes.is_empty(), "no LeaseRevoked for an accepted op");

        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    async fn node_reported_done_rejected_without_lease_or_current_token() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_a = create_test_node(&pool, org_id).await; // partitioned writer (stale)
        let node_b = create_test_node(&pool, org_id).await; // current holder

        let swarm_project_id = create_swarm_project(&pool, org_id).await;
        let local_proj_a = Uuid::new_v4();
        let local_proj_b = Uuid::new_v4();
        create_node_local_project(&pool, node_a, local_proj_a, Some(swarm_project_id)).await;
        create_node_local_project(&pool, node_b, local_proj_b, Some(swarm_project_id)).await;
        let np_a = create_swarm_project_node(&pool, swarm_project_id, node_a, local_proj_a).await;
        let np_b = create_swarm_project_node(&pool, swarm_project_id, node_b, local_proj_b).await;

        let a_local_task_id = Uuid::new_v4();
        let shared_id = insert_shared_task(
            &pool,
            org_id,
            swarm_project_id,
            node_a,
            a_local_task_id,
            "in-progress",
        )
        .await;

        // (a) NO active assignment: P2's fence breaks before 303's guard runs.
        let key_a = format!("task:{}:{}", local_proj_a, a_local_task_id);
        let op_a = make_fence_op(
            1,
            a_local_task_id,
            local_proj_a,
            Some(shared_id),
            None,
            "done",
            &key_a,
        );

        let pre_status = shared_task_status_by_id(&pool, shared_id)
            .await
            .expect("task exists pre-apply (a)");

        let (seq_a, _revokes_a) = handle_op_batch_apply(node_a, org_id, "node-a", &[op_a], &pool)
            .await
            .expect("apply (a)");

        assert_eq!(
            shared_task_status_by_id(&pool, shared_id).await,
            Some(pre_status.clone()),
            "(a) no-assignment op MUST NOT update shared_tasks (P2 fence reject)"
        );
        assert_eq!(
            node_op_log_count_for_key(&pool, node_a, &key_a).await,
            0,
            "(a) rejected op MUST NOT record a node_op_log dedup row"
        );
        assert_eq!(
            seq_a, 0,
            "(a) applied_through_seq MUST NOT advance past the rejected op (break)"
        );

        // (b) Stale token: node_a claims (T1), node_b reclaims (T2 > T1). node_a sends an
        // op stamped T1 → P2's stale-token check breaks before 303's guard runs.
        let claim_a = repo
            .try_claim(shared_id, node_a, np_a, chrono::Duration::seconds(-300))
            .await
            .expect("claim a")
            .expect("node_a claimed");
        let t1 = claim_a.fencing_token;

        let claim_b = repo
            .try_claim(shared_id, node_b, np_b, chrono::Duration::seconds(300))
            .await
            .expect("claim b")
            .expect("node_b reclaimed");
        let t2 = claim_b.fencing_token;
        assert!(t2 > t1, "reassignment bumps the fencing token (T2 > T1)");

        let key_b = format!("task:{}:{}-b", local_proj_a, a_local_task_id);
        let op_b = make_fence_op(
            2,
            a_local_task_id,
            local_proj_a,
            Some(shared_id),
            Some(t1),
            "done",
            &key_b,
        );

        let pre_status_b = shared_task_status_by_id(&pool, shared_id)
            .await
            .expect("task exists pre-apply (b)");

        let (seq_b, revokes_b) = handle_op_batch_apply(node_a, org_id, "node-a", &[op_b], &pool)
            .await
            .expect("apply (b)");

        assert_eq!(
            shared_task_status_by_id(&pool, shared_id).await,
            Some(pre_status_b),
            "(b) stale-token op MUST NOT update shared_tasks (P2 fence reject)"
        );
        assert_eq!(
            node_op_log_count_for_key(&pool, node_a, &key_b).await,
            0,
            "(b) rejected op MUST NOT record a node_op_log dedup row"
        );
        assert_eq!(
            seq_b, 0,
            "(b) applied_through_seq MUST NOT advance past the rejected op (break)"
        );
        assert_eq!(revokes_b.len(), 1, "(b) exactly one LeaseRevoked emitted");
        assert_eq!(
            revokes_b[0].0, claim_b.assignment_id,
            "(b) revoked assignment matches"
        );
        assert_eq!(
            revokes_b[0].1, "stale fencing token",
            "(b) revoke reason matches the contract"
        );

        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    async fn hive_authored_transition_rejected_when_reported_by_node() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_b = create_test_node(&pool, org_id).await;
        let swarm_project_id = create_swarm_project(&pool, org_id).await;
        let local_proj_b = Uuid::new_v4();
        create_node_local_project(&pool, node_b, local_proj_b, Some(swarm_project_id)).await;
        let np_b = create_swarm_project_node(&pool, swarm_project_id, node_b, local_proj_b).await;

        let b_local_task_id = Uuid::new_v4();
        let shared_id = insert_shared_task(
            &pool,
            org_id,
            swarm_project_id,
            node_b,
            b_local_task_id,
            "in-review",
        )
        .await;

        // Active assignment with a current token — the fence passes; only the author
        // guard (303) decides the rejection.
        let claim_b = repo
            .try_claim(shared_id, node_b, np_b, chrono::Duration::seconds(300))
            .await
            .expect("claim b")
            .expect("node_b claimed");
        let t = claim_b.fencing_token;
        assert!(t > 0, "claim bumps a positive token");

        // (1) in-review→cancelled: `*→cancelled` is HIVE-authored. A node may not author
        // it even with a valid lease+token → REJECTED (SKIP+ADVANCE).
        let key1 = format!("task:{}:{}-cancel", local_proj_b, b_local_task_id);
        let op1 = make_fence_op(
            1,
            b_local_task_id,
            local_proj_b,
            Some(shared_id),
            Some(t),
            "cancelled",
            &key1,
        );

        let (seq1, revokes1) = handle_op_batch_apply(node_b, org_id, "node-b", &[op1], &pool)
            .await
            .expect("apply (1)");

        assert_eq!(
            seq1, 1,
            "(1) rejected hive-authored transition SKIP+ADVANCEs the cursor (no wedge)"
        );
        assert_eq!(
            node_op_log_count_for_key(&pool, node_b, &key1).await,
            1,
            "(1) rejected transition records a node_op_log dedup row (SKIP+ADVANCE)"
        );
        assert_eq!(
            shared_task_status_by_id(&pool, shared_id).await,
            Some("in-review".to_string()),
            "(1) shared_tasks.status stays 'in-review' (hive-authored-from-node rejected)"
        );
        assert!(
            revokes1.is_empty(),
            "(1) no LeaseRevoked (this is a 303 reject, not a P2 reject)"
        );

        // (2) in-review→done: also HIVE-authored (operator approve). A node may not author
        // it → REJECTED (SKIP+ADVANCE). Current status is still 'inreview'.
        let key2 = format!("task:{}:{}-done", local_proj_b, b_local_task_id);
        let op2 = make_fence_op(
            2,
            b_local_task_id,
            local_proj_b,
            Some(shared_id),
            Some(t),
            "done",
            &key2,
        );

        let (seq2, revokes2) = handle_op_batch_apply(node_b, org_id, "node-b", &[op2], &pool)
            .await
            .expect("apply (2)");

        assert_eq!(
            seq2, 2,
            "(2) rejected hive-authored transition SKIP+ADVANCEs the cursor"
        );
        assert_eq!(
            node_op_log_count_for_key(&pool, node_b, &key2).await,
            1,
            "(2) rejected transition records a node_op_log dedup row"
        );
        assert_eq!(
            shared_task_status_by_id(&pool, shared_id).await,
            Some("in-review".to_string()),
            "(2) shared_tasks.status still 'in-review' (in-review→done is hive-authored)"
        );
        assert!(revokes2.is_empty(), "(2) no LeaseRevoked");

        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    async fn illegal_transition_rejected_from_either_party() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_b = create_test_node(&pool, org_id).await;
        let swarm_project_id = create_swarm_project(&pool, org_id).await;
        let local_proj_b = Uuid::new_v4();
        create_node_local_project(&pool, node_b, local_proj_b, Some(swarm_project_id)).await;
        let np_b = create_swarm_project_node(&pool, swarm_project_id, node_b, local_proj_b).await;

        let b_local_task_id = Uuid::new_v4();
        let shared_id = insert_shared_task(
            &pool,
            org_id,
            swarm_project_id,
            node_b,
            b_local_task_id,
            "done",
        )
        .await;

        let claim_b = repo
            .try_claim(shared_id, node_b, np_b, chrono::Duration::seconds(300))
            .await
            .expect("claim b")
            .expect("node_b claimed");
        let t = claim_b.fencing_token;
        assert!(t > 0, "claim bumps a positive token");

        // done→in-progress is in NO author's column (illegal). Even with a valid
        // lease+token, the node may not author it → REJECTED (SKIP+ADVANCE).
        let key = format!("task:{}:{}-illegal", local_proj_b, b_local_task_id);
        let op = make_fence_op(
            1,
            b_local_task_id,
            local_proj_b,
            Some(shared_id),
            Some(t),
            "inprogress",
            &key,
        );

        let (seq, revokes) = handle_op_batch_apply(node_b, org_id, "node-b", &[op], &pool)
            .await
            .expect("apply");

        assert_eq!(
            seq, 1,
            "illegal transition SKIP+ADVANCEs the cursor (no wedge)"
        );
        assert_eq!(
            node_op_log_count_for_key(&pool, node_b, &key).await,
            1,
            "illegal transition records a node_op_log dedup row (SKIP+ADVANCE)"
        );
        assert_eq!(
            shared_task_status_by_id(&pool, shared_id).await,
            Some("done".to_string()),
            "illegal transition MUST NOT change shared_tasks.status (stays 'done')"
        );
        assert!(
            revokes.is_empty(),
            "no LeaseRevoked (303 reject, not a P2 reject)"
        );

        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    async fn noop_same_status_is_not_a_rejected_transition() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_b = create_test_node(&pool, org_id).await;
        let swarm_project_id = create_swarm_project(&pool, org_id).await;
        let local_proj_b = Uuid::new_v4();
        create_node_local_project(&pool, node_b, local_proj_b, Some(swarm_project_id)).await;
        let np_b = create_swarm_project_node(&pool, swarm_project_id, node_b, local_proj_b).await;

        let b_local_task_id = Uuid::new_v4();
        let shared_id = insert_shared_task(
            &pool,
            org_id,
            swarm_project_id,
            node_b,
            b_local_task_id,
            "in-progress",
        )
        .await;

        let claim_b = repo
            .try_claim(shared_id, node_b, np_b, chrono::Duration::seconds(300))
            .await
            .expect("claim b")
            .expect("node_b claimed");
        let t = claim_b.fencing_token;
        assert!(t > 0, "claim bumps a positive token");

        // from==to (in-progress→in-progress): a no-op is NOT a transition. The guard's
        // short-circuit lets the upsert proceed (metadata-only), the cursor advances, and
        // the status stays 'in-progress'. A no-op must NOT be treated as an illegal
        // transition that wedges the op-log.
        let key = format!("task:{}:{}-noop", local_proj_b, b_local_task_id);
        let op = make_fence_op(
            1,
            b_local_task_id,
            local_proj_b,
            Some(shared_id),
            Some(t),
            "inprogress",
            &key,
        );

        let (seq, revokes) = handle_op_batch_apply(node_b, org_id, "node-b", &[op], &pool)
            .await
            .expect("apply");

        assert_eq!(
            seq, 1,
            "no-op op advances the cursor (applied as idempotent upsert)"
        );
        assert_eq!(
            node_op_log_count_for_key(&pool, node_b, &key).await,
            1,
            "no-op op records a node_op_log dedup row"
        );
        assert_eq!(
            shared_task_status_by_id(&pool, shared_id).await,
            Some("in-progress".to_string()),
            "no-op op leaves status as 'in-progress' (from==to, not a transition)"
        );
        assert!(revokes.is_empty(), "no LeaseRevoked for a no-op");

        cleanup_org(&pool, org_id).await;
    }
}

#[cfg(test)]
mod legacy_status_guard_tests {
    use super::*;
    use crate::db::task_assignments::TaskAssignmentRepository;
    use chrono::Utc;
    use sqlx::PgPool;
    use uuid::Uuid;

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
        sqlx::PgPool::connect(&url)
            .await
            .expect("Failed to connect to database")
    }

    async fn create_test_organization(pool: &PgPool) -> Uuid {
        let org_id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO organizations (id, name, slug, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(org_id)
        .bind(format!("Test Org {}", org_id))
        .bind(format!("test-org-{}", org_id))
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("Failed to create test organization");
        org_id
    }

    async fn create_test_node(pool: &PgPool, org_id: Uuid) -> Uuid {
        let node_id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO nodes (id, organization_id, name, machine_id, last_heartbeat_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(node_id)
        .bind(org_id)
        .bind(format!("node-{}", node_id))
        .bind(format!("machine-{}", node_id))
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("Failed to create test node");
        node_id
    }

    async fn create_swarm_project(pool: &PgPool, org_id: Uuid) -> Uuid {
        let sp_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO swarm_projects (id, organization_id, name)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(sp_id)
        .bind(org_id)
        .bind(format!("Swarm Project {}", sp_id))
        .execute(pool)
        .await
        .expect("Failed to create swarm project");
        sp_id
    }

    async fn create_node_local_project(
        pool: &PgPool,
        node_id: Uuid,
        local_project_id: Uuid,
        swarm_project_id: Option<Uuid>,
    ) {
        let res = sqlx::query(
            r#"
            INSERT INTO node_local_projects (node_id, local_project_id, name, git_repo_path, swarm_project_id)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(node_id)
        .bind(local_project_id)
        .bind("local-proj")
        .bind("/repo/path")
        .bind(swarm_project_id)
        .execute(pool)
        .await;
        if let Err(e) = res {
            eprintln!("create_node_local_project (non-fatal): {}", e);
        }
    }

    async fn create_swarm_project_node(
        pool: &PgPool,
        swarm_project_id: Uuid,
        node_id: Uuid,
        local_project_id: Uuid,
    ) -> Uuid {
        let row = sqlx::query(
            r#"
            INSERT INTO swarm_project_nodes (swarm_project_id, node_id, local_project_id, git_repo_path)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(swarm_project_id)
        .bind(node_id)
        .bind(local_project_id)
        .bind("/repo/path")
        .fetch_one(pool)
        .await
        .expect("Failed to create swarm_project_nodes link");
        sqlx::Row::get(&row, "id")
    }

    async fn cleanup_org(pool: &PgPool, org_id: Uuid) {
        let _ = sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(org_id)
            .execute(pool)
            .await;
    }

    #[allow(dead_code)]
    async fn node_op_log_count_for_key(pool: &PgPool, node_id: Uuid, key: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM node_op_log WHERE node_id = $1 AND idempotency_key = $2",
        )
        .bind(node_id)
        .bind(key)
        .fetch_one(pool)
        .await
        .expect("count")
    }

    async fn shared_task_status_by_id(pool: &PgPool, shared_id: Uuid) -> Option<String> {
        sqlx::query_scalar("SELECT status::text FROM shared_tasks WHERE id = $1")
            .bind(shared_id)
            .fetch_optional(pool)
            .await
            .expect("shared task status by id")
    }

    async fn insert_shared_task(
        pool: &PgPool,
        org_id: Uuid,
        swarm_project_id: Uuid,
        creator_node: Uuid,
        creator_local_task_id: Uuid,
        status: &str,
    ) -> Uuid {
        let now = Utc::now();
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO shared_tasks (
                id, organization_id, project_id, swarm_project_id,
                source_node_id, source_task_id,
                title, status, version, shared_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $3, $4, $5, $6, $7::task_status, 1, $8, $8, $8)
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(swarm_project_id)
        .bind(creator_node)
        .bind(creator_local_task_id)
        .bind("seeded task")
        .bind(status)
        .bind(now)
        .execute(pool)
        .await
        .expect("insert shared task");
        id
    }

    #[tokio::test]
    async fn legacy_path_applies_node_authored_transition_with_lease() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_id = create_test_node(&pool, org_id).await;

        let swarm_project_id = create_swarm_project(&pool, org_id).await;
        let local_project_id = Uuid::new_v4();
        create_node_local_project(&pool, node_id, local_project_id, Some(swarm_project_id)).await;
        let np =
            create_swarm_project_node(&pool, swarm_project_id, node_id, local_project_id).await;

        let creator_local_task_id = Uuid::new_v4();
        let shared_id = insert_shared_task(
            &pool,
            org_id,
            swarm_project_id,
            node_id,
            creator_local_task_id,
            "in-progress",
        )
        .await;

        // Active assignment (valid lease) held by node_id — the legacy path's lease context.
        let claim = repo
            .try_claim(shared_id, node_id, np, chrono::Duration::seconds(300))
            .await
            .expect("claim")
            .expect("node claimed");

        // Completed → InReview. in-progress→in-review is node-authored (ADR-0010 §D) → APPLIED.
        let msg = TaskStatusMessage {
            assignment_id: claim.assignment_id,
            local_task_id: Some(creator_local_task_id),
            local_attempt_id: None,
            status: TaskExecutionStatus::Completed,
            message: None,
            timestamp: Utc::now(),
        };
        handle_task_status(node_id, org_id, &msg, &pool)
            .await
            .expect("handle_task_status");

        assert_eq!(
            shared_task_status_by_id(&pool, shared_id).await,
            Some("in-review".to_string()),
            "node-authored in-progress→in-review transition is applied on the legacy path"
        );

        cleanup_org(&pool, org_id).await;
    }

    #[tokio::test]
    async fn legacy_path_rejects_hive_authored_or_illegal_transition_from_node() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);

        let org_id = create_test_organization(&pool).await;
        let node_id = create_test_node(&pool, org_id).await;

        let swarm_project_id = create_swarm_project(&pool, org_id).await;
        let local_project_id = Uuid::new_v4();
        create_node_local_project(&pool, node_id, local_project_id, Some(swarm_project_id)).await;
        let np =
            create_swarm_project_node(&pool, swarm_project_id, node_id, local_project_id).await;

        let creator_local_task_id = Uuid::new_v4();
        let shared_id = insert_shared_task(
            &pool,
            org_id,
            swarm_project_id,
            node_id,
            creator_local_task_id,
            "done",
        )
        .await;

        let claim = repo
            .try_claim(shared_id, node_id, np, chrono::Duration::seconds(300))
            .await
            .expect("claim")
            .expect("node claimed");

        // Failed → Todo. done→todo is illegal (no author) → REJECTED: status stays 'done'.
        // This is the concrete clobber the old `*→Todo` map caused (Failed/Cancelled reset).
        let msg = TaskStatusMessage {
            assignment_id: claim.assignment_id,
            local_task_id: Some(creator_local_task_id),
            local_attempt_id: None,
            status: TaskExecutionStatus::Failed,
            message: None,
            timestamp: Utc::now(),
        };
        handle_task_status(node_id, org_id, &msg, &pool)
            .await
            .expect("handle_task_status");

        assert_eq!(
            shared_task_status_by_id(&pool, shared_id).await,
            Some("done".to_string()),
            "illegal done→todo reset MUST NOT clobber shared_tasks.status (ADR-0010)"
        );

        cleanup_org(&pool, org_id).await;
    }
}
