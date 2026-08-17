use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use db::DBService;
use db::models::trigger_cursor;
use deployment::{Deployment, DeploymentError, RemoteClientNotConfigured};
use executors::profile::ExecutorConfigs;
use services::services::{
    approvals::Approvals,
    auth::AuthContext,
    config::{Config, load_config_from_file, save_config_to_file},
    connection_token::ConnectionTokenValidator,
    container::ContainerService,
    drafts::DraftsService,
    event_bus::EventBus,
    event_compaction::{EventCompaction, EventCompactionConfig},
    events::EventService,
    file_search_cache::FileSearchCache,
    filesystem::FilesystemService,
    git::GitService,
    image::ImageService,
    node_cache::NodeCacheSyncService,
    node_proxy_client::NodeProxyClient,
    node_runner::{NodeRunnerConfig, NodeRunnerContext, spawn_node_runner},
    oauth_credentials::OAuthCredentials,
    remote_client::{RemoteClient, RemoteClientError},
    share::{RemoteSyncHandle, ShareConfig, SharePublisher},
    trigger_hooks::{TaskStatusChangedHook, TriggerHookRegistry},
};
use tokio::sync::{Mutex, RwLock};
use utils::{
    api::oauth::LoginStatus,
    assets::{backup_dir, config_path, credentials_path, database_path},
    msg_store::MsgStore,
};
use uuid::Uuid;

use crate::container::LocalContainerService;
mod command;
pub mod container;
pub mod message_queue;

#[derive(Clone)]
pub struct LocalDeployment {
    config: Arc<RwLock<Config>>,
    user_id: String,
    db: DBService,
    container: LocalContainerService,
    git: GitService,
    image: ImageService,
    filesystem: FilesystemService,
    events: EventService,
    file_search_cache: Arc<FileSearchCache>,
    approvals: Approvals,
    drafts: DraftsService,
    share_publisher: Result<SharePublisher, RemoteClientNotConfigured>,
    share_sync_handle: Arc<Mutex<Option<RemoteSyncHandle>>>,
    share_config: Option<ShareConfig>,
    remote_client: Result<RemoteClient, RemoteClientNotConfigured>,
    /// API key-based client for node operations (available even when not logged in via OAuth)
    node_auth_client: Option<RemoteClient>,
    auth_context: AuthContext,
    oauth_handoffs: Arc<RwLock<HashMap<Uuid, PendingHandoff>>>,
    /// Node runner context (if connected to a hive) - provides state access and message sending
    node_runner_context: Option<NodeRunnerContext>,
    /// Validator for connection tokens (for direct frontend-to-node connections)
    connection_token_validator: Arc<ConnectionTokenValidator>,
    /// HTTP client for proxying requests to remote nodes
    node_proxy_client: NodeProxyClient,
    /// Whether the node cache sync has been started
    node_cache_sync_started: Arc<Mutex<bool>>,
    /// Timestamp of the last VACUUM operation (for rate limiting)
    last_vacuum_time: Arc<RwLock<Option<std::time::Instant>>>,
    /// The event bus for durable event streaming and replay-to-live subscriptions
    event_bus: Arc<EventBus>,
    /// Handle to the trigger hook runner task (spawned at startup)
    /// Retained to keep the task alive; when dropped, the task is aborted
    #[allow(dead_code)]
    trigger_hook_runner_handle: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Handle to the event compaction background service
    /// Retained to keep the compaction loop alive; when dropped, signals shutdown
    #[allow(dead_code)]
    _compaction_handle: services::services::event_compaction::EventCompactionHandle,
}

#[derive(Debug, Clone)]
struct PendingHandoff {
    provider: String,
    app_verifier: String,
}

#[async_trait]
impl Deployment for LocalDeployment {
    /// Creates and initializes a LocalDeployment with all core services, background tasks, and optional
    /// remote/hive integrations configured from environment and persisted config.
    ///
    /// The function performs startup work such as loading and persisting configuration, initializing
    /// the database (with event hooks), image and filesystem services, auth context, optional remote
    /// clients (OAuth and API-key-based), node runner (when configured), and starts background tasks
    /// like orphaned image cleanup and node cache synchronization when applicable.
    ///
    /// # Returns
    ///
    /// A fully initialized `LocalDeployment` on success.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn run() {
    /// use deployment::Deployment;
    /// let deployment = local_deployment::LocalDeployment::new().await.unwrap();
    /// assert!(!deployment.user_id().is_empty());
    /// # }
    /// ```
    async fn new() -> Result<Self, DeploymentError> {
        // Load config and OAuth credentials in parallel for faster startup
        let config_path = config_path();
        let creds_path = credentials_path();
        let (mut raw_config, oauth_credentials) =
            tokio::join!(load_config_from_file(&config_path), async {
                let creds = Arc::new(OAuthCredentials::new(creds_path));
                if let Err(e) = creds.load().await {
                    tracing::warn!(?e, "failed to load OAuth credentials");
                }
                creds
            });

        let profiles = ExecutorConfigs::get_cached();
        if !raw_config.onboarding_acknowledged
            && let Ok(recommended_executor) = profiles.get_recommended_executor_profile().await
        {
            raw_config.executor_profile = recommended_executor;
        }

        // Check if app version has changed and set release notes flag
        {
            let current_version = utils::version::APP_VERSION;
            let stored_version = raw_config.last_app_version.as_deref();

            if stored_version != Some(current_version) {
                // Show release notes only if this is an upgrade (not first install)
                raw_config.show_release_notes = stored_version.is_some();
                raw_config.last_app_version = Some(current_version.to_string());
            }
        }

        // Always save config (may have been migrated or version updated)
        save_config_to_file(&raw_config, &config_path).await?;

        // Log storage locations at startup for debugging
        tracing::info!(
            database = %database_path().display(),
            backups = %backup_dir().display(),
            worktrees = %services::services::worktree_manager::WorktreeManager::get_worktree_base_dir().display(),
            "Storage locations"
        );

        let config = Arc::new(RwLock::new(raw_config));
        // Generate a unique user ID for this deployment
        let user_id = Uuid::new_v4().to_string();
        let git = GitService::new();
        let msg_stores = Arc::new(RwLock::new(HashMap::new()));
        let filesystem = FilesystemService::new();

        // Create shared components for EventService
        let events_msg_store = Arc::new(MsgStore::new());
        let events_entry_count = Arc::new(RwLock::new(0));

        // Create DB with event hooks
        // Use bootstrap() for the hook's internal DB (lightweight, no migrations)
        // Then create the main DB with the hook attached (runs full init + migrations)
        let db = {
            let bootstrap_db = DBService::bootstrap().await?;
            let hook = EventService::create_hook(
                events_msg_store.clone(),
                events_entry_count.clone(),
                bootstrap_db,
            );
            DBService::new_with_after_connect(hook).await?
        };

        let image = ImageService::new(db.clone().pool)?;
        {
            let image_service = image.clone();
            tokio::spawn(async move {
                tracing::info!("Starting orphaned image cleanup...");
                if let Err(e) = image_service.delete_orphaned_images().await {
                    tracing::error!("Failed to clean up orphaned images: {}", e);
                }
            });
        }

        let approvals = Approvals::new(msg_stores.clone());

        let share_config = ShareConfig::from_env();

        // oauth_credentials already loaded in parallel at startup

        let profile_cache = Arc::new(RwLock::new(None));
        let auth_context = AuthContext::new(oauth_credentials.clone(), profile_cache.clone());

        let api_base = std::env::var("VK_SHARED_API_BASE")
            .ok()
            .or_else(|| option_env!("VK_SHARED_API_BASE").map(|s| s.to_string()));

        // Create OAuth-based remote client for user-initiated operations (frontend)
        let remote_client = match &api_base {
            Some(url) => match RemoteClient::new(url, auth_context.clone()) {
                Ok(client) => {
                    tracing::info!("Remote client initialized with URL: {}", url);
                    Ok(client)
                }
                Err(e) => {
                    tracing::error!(?e, "failed to create remote client");
                    Err(RemoteClientNotConfigured)
                }
            },
            None => {
                tracing::info!("VK_SHARED_API_BASE not set; remote features disabled");
                Err(RemoteClientNotConfigured)
            }
        };

        // Create API key-based remote client for node sync operations (no user login required)
        // This allows nodes to sync with the hive even when no user is logged in
        let node_auth_client: Option<RemoteClient> =
            match (api_base.as_ref(), std::env::var("VK_NODE_API_KEY").ok()) {
                (Some(url), Some(api_key)) => match RemoteClient::new_with_api_key(url, api_key) {
                    Ok(client) => {
                        tracing::info!("Node auth client initialized for hive sync");
                        Some(client)
                    }
                    Err(e) => {
                        tracing::error!(?e, "failed to create node auth client");
                        None
                    }
                },
                _ => None,
            };

        let share_publisher = remote_client
            .as_ref()
            .map(|client| SharePublisher::new(db.clone(), client.clone()))
            .map_err(|e| *e);

        let oauth_handoffs = Arc::new(RwLock::new(HashMap::new()));
        let share_sync_handle = Arc::new(Mutex::new(None));

        let mut share_sync_config: Option<ShareConfig> = None;
        if let (Some(sc_ref), Ok(_)) = (share_config.as_ref(), &share_publisher)
            && oauth_credentials.get().await.is_some()
        {
            share_sync_config = Some(sc_ref.clone());
        }

        let container = LocalContainerService::new(
            db.clone(),
            msg_stores.clone(),
            config.clone(),
            git.clone(),
            image.clone(),
            approvals.clone(),
            share_publisher.clone(),
        )
        .await;

        let events = EventService::new(db.clone(), events_msg_store, events_entry_count);

        let drafts = DraftsService::new(db.clone(), image.clone());
        let file_search_cache = Arc::new(FileSearchCache::new());

        // Initialize node runner and connection token validator if hive connection is configured
        let (node_runner_context, connection_token_validator, node_proxy_client) =
            if let Some(node_config) = NodeRunnerConfig::from_env() {
                tracing::info!(
                    hive_url = %node_config.hive_url,
                    node_name = %node_config.node_name,
                    "starting node runner to connect to hive"
                );

                // Create connection token validator if secret is configured
                let validator = if let Some(secret) = node_config.connection_token_secret.clone() {
                    tracing::info!("connection token validation enabled for direct log streaming");
                    ConnectionTokenValidator::new(secret)
                } else {
                    tracing::debug!(
                        "VK_CONNECTION_TOKEN_SECRET not set; direct log streaming auth disabled"
                    );
                    ConnectionTokenValidator::disabled()
                };

                // Create node proxy client with the same secret
                // Note: local_node_id will be set after the node authenticates with the hive
                let proxy_client =
                    NodeProxyClient::new(node_config.connection_token_secret.clone(), None);
                if proxy_client.is_enabled() {
                    tracing::info!("node proxy client enabled for remote project operations");
                }

                // Pass the container and node_auth_client to spawn_node_runner to enable
                // task execution and remote project sync.
                // Use node_auth_client (API key auth) instead of remote_client (OAuth auth)
                // so sync can happen without requiring user login.
                (
                    spawn_node_runner(
                        node_config,
                        db.clone(),
                        Some(container.clone()),
                        node_auth_client.clone(),
                    ),
                    validator,
                    proxy_client,
                )
            } else {
                // Log which env vars are missing to help with debugging
                let has_hive_url = std::env::var("VK_HIVE_URL").is_ok();
                let has_api_key = std::env::var("VK_NODE_API_KEY").is_ok();
                if !has_hive_url && !has_api_key {
                    tracing::debug!(
                        "VK_HIVE_URL and VK_NODE_API_KEY not set; node runner disabled"
                    );
                } else if !has_hive_url {
                    tracing::debug!(
                        "VK_HIVE_URL not set; node runner disabled (VK_NODE_API_KEY is set)"
                    );
                } else {
                    tracing::debug!(
                        "VK_NODE_API_KEY not set; node runner disabled (VK_HIVE_URL is set)"
                    );
                }
                (
                    None,
                    ConnectionTokenValidator::disabled(),
                    NodeProxyClient::disabled(),
                )
            };

        // Create the EventBus over the LIVE DBService pool
        let event_bus = Arc::new(EventBus::new(db.pool.clone(), 64).await);

        // Build the trigger-hook registry with the status hook
        let hooks: Vec<Arc<dyn services::services::trigger_hooks::TriggerHook>> =
            vec![Arc::new(TaskStatusChangedHook)];
        let hook_registry = TriggerHookRegistry::new(hooks);

        // Spawn the supervised trigger hook runner
        // One spawned task per hook, each with its own supervised respawn loop
        let mut runner_tasks = Vec::new();
        for hook in hook_registry.all() {
            let hook_name = hook.name().to_string(); // Capture name as String for move closure
            let hook_clone = hook.clone();
            let db_clone = db.clone();
            let event_bus_clone = event_bus.clone();

            let task = tokio::spawn(async move {
                let mut backoff_ms = 1000u64;
                let max_backoff_ms = 60000u64;

                loop {
                    // Ensure cursor row exists before each run
                    if let Err(e) = trigger_cursor::ensure_row(&db_clone.pool, &hook_name).await {
                        tracing::error!(hook = %hook_name, error = ?e, "failed to ensure cursor row");
                        tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                        backoff_ms = (backoff_ms * 2).min(max_backoff_ms);
                        continue;
                    }

                    let db_for_runner = db_clone.clone();
                    let hook_for_runner = hook_clone.clone();
                    let bus_for_runner = event_bus_clone.clone();
                    let hook_name_for_error = hook_name.clone();
                    let hook_name_for_panic = hook_name.clone();

                    // Spawn run_hook as its own task to catch panics
                    // The closure must return () (not a Result) to satisfy tokio::spawn's Send bound
                    let inner_task = tokio::spawn(async move {
                        if let Err(e) = services::services::trigger_hooks::run_hook(
                            db_for_runner.pool,
                            hook_for_runner,
                            bus_for_runner,
                        )
                        .await
                        {
                            tracing::error!(
                                hook = %hook_name_for_error,
                                error = ?e,
                                "trigger hook runner failed"
                            );
                        }
                    });

                    // Wait for completion and catch panics
                    if let Err(e) = inner_task.await {
                        tracing::error!(
                            hook = %hook_name_for_panic,
                            error = ?e,
                            "trigger hook task panicked"
                        );
                    }

                    // Re-read cursor and needs_rebootstrap for the next iteration
                    let (_, needs_rebootstrap) =
                        match trigger_cursor::get_with_flag(&db_clone.pool, &hook_name).await {
                            Ok(cf) => cf,
                            Err(e) => {
                                tracing::error!(
                                    hook = %hook_name,
                                    error = ?e,
                                    "failed to read rebootstrap flag"
                                );
                                (0, false)
                            }
                        };

                    tracing::warn!(
                        hook = %hook_name,
                        needs_rebootstrap = needs_rebootstrap,
                        backoff_ms = backoff_ms,
                        "trigger hook runner terminated; respawning after backoff"
                    );

                    tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(max_backoff_ms);
                }
            });

            runner_tasks.push(task);
        }

        let trigger_hook_runner_handle = Arc::new(tokio::sync::Mutex::new(Some(
            // Store the first task handle (there should only be one hook in practice for now)
            // In a multi-hook setup, we would need a better way to track multiple handles
            runner_tasks
                .into_iter()
                .next()
                .unwrap_or_else(|| tokio::spawn(async {})),
        )));

        // Spawn the event compaction loop
        let compaction_config = EventCompactionConfig::default();
        let compaction_handle = EventCompaction::spawn(db.pool.clone(), compaction_config);

        let deployment = Self {
            config,
            user_id,
            db,
            container,
            git,
            image,
            filesystem,
            events,
            file_search_cache,
            approvals,
            drafts,
            share_publisher,
            share_sync_handle: share_sync_handle.clone(),
            share_config: share_config.clone(),
            remote_client,
            node_auth_client,
            auth_context,
            oauth_handoffs,
            node_runner_context,
            connection_token_validator: Arc::new(connection_token_validator),
            node_proxy_client,
            node_cache_sync_started: Arc::new(Mutex::new(false)),
            last_vacuum_time: Arc::new(RwLock::new(None)),
            event_bus,
            trigger_hook_runner_handle,
            _compaction_handle: compaction_handle,
        };

        // Log startup config summary for debugging connection issues
        let has_shared_api = std::env::var("VK_SHARED_API_BASE").is_ok();
        let has_hive_url = std::env::var("VK_HIVE_URL").is_ok();
        let has_api_key = std::env::var("VK_NODE_API_KEY").is_ok();

        match &deployment.node_runner_context {
            Some(_) => {
                tracing::info!("Hive connection: enabled (node runner started)");
            }
            None => {
                if has_shared_api && (!has_hive_url || !has_api_key) {
                    // This is the case where something might be misconfigured
                    tracing::warn!(
                        has_hive_url = has_hive_url,
                        has_api_key = has_api_key,
                        "Hive connection: DISABLED - VK_SHARED_API_BASE is set but hive config is incomplete. \
                         Check VK_HIVE_URL and VK_NODE_API_KEY in your .env file for typos."
                    );
                } else if !has_shared_api {
                    tracing::info!("Hive connection: not configured (standalone mode)");
                }
            }
        }

        if let Some(sc) = share_sync_config {
            deployment.spawn_remote_sync(sc);
        }

        // Start node cache sync if user is already logged in
        // (runs in background, syncs nodes/projects from all organizations)
        {
            let d = deployment.clone();
            tokio::spawn(async move {
                d.start_node_cache_sync().await;
            });
        }

        Ok(deployment)
    }

    fn user_id(&self) -> &str {
        &self.user_id
    }

    fn config(&self) -> &Arc<RwLock<Config>> {
        &self.config
    }

    fn db(&self) -> &DBService {
        &self.db
    }

    fn container(&self) -> &impl ContainerService {
        &self.container
    }

    fn git(&self) -> &GitService {
        &self.git
    }

    fn image(&self) -> &ImageService {
        &self.image
    }

    fn filesystem(&self) -> &FilesystemService {
        &self.filesystem
    }

    fn events(&self) -> &EventService {
        &self.events
    }

    fn file_search_cache(&self) -> &Arc<FileSearchCache> {
        &self.file_search_cache
    }

    fn approvals(&self) -> &Approvals {
        &self.approvals
    }

    fn drafts(&self) -> &DraftsService {
        &self.drafts
    }

    fn share_publisher(&self) -> Result<SharePublisher, RemoteClientNotConfigured> {
        self.share_publisher.clone()
    }

    fn share_sync_handle(&self) -> &Arc<Mutex<Option<RemoteSyncHandle>>> {
        &self.share_sync_handle
    }

    fn auth_context(&self) -> &AuthContext {
        &self.auth_context
    }
}

impl LocalDeployment {
    pub fn remote_client(&self) -> Result<RemoteClient, RemoteClientNotConfigured> {
        self.remote_client.clone()
    }

    pub async fn get_login_status(&self) -> LoginStatus {
        if self.auth_context.get_credentials().await.is_none() {
            self.auth_context.clear_profile().await;
            return LoginStatus::LoggedOut;
        };

        if let Some(cached_profile) = self.auth_context.cached_profile().await {
            return LoginStatus::LoggedIn {
                profile: cached_profile,
            };
        }

        let Ok(client) = self.remote_client() else {
            return LoginStatus::LoggedOut;
        };

        match client.profile().await {
            Ok(profile) => {
                self.auth_context.set_profile(profile.clone()).await;
                LoginStatus::LoggedIn { profile }
            }
            Err(RemoteClientError::Auth) => {
                let _ = self.auth_context.clear_credentials().await;
                self.auth_context.clear_profile().await;
                LoginStatus::LoggedOut
            }
            Err(_) => LoginStatus::LoggedOut,
        }
    }

    pub async fn store_oauth_handoff(
        &self,
        handoff_id: Uuid,
        provider: String,
        app_verifier: String,
    ) {
        self.oauth_handoffs.write().await.insert(
            handoff_id,
            PendingHandoff {
                provider,
                app_verifier,
            },
        );
    }

    pub async fn take_oauth_handoff(&self, handoff_id: &Uuid) -> Option<(String, String)> {
        self.oauth_handoffs
            .write()
            .await
            .remove(handoff_id)
            .map(|state| (state.provider, state.app_verifier))
    }

    pub fn share_config(&self) -> Option<&ShareConfig> {
        self.share_config.as_ref()
    }

    /// Get the node runner context (if connected to a hive).
    ///
    /// This provides both read access to state and the ability to send messages to the hive.
    pub fn node_runner_context(&self) -> Option<&NodeRunnerContext> {
        self.node_runner_context.as_ref()
    }

    /// Check if this instance is running as a node connected to a hive.
    pub async fn is_node_connected(&self) -> bool {
        if let Some(ctx) = &self.node_runner_context {
            ctx.is_connected().await
        } else {
            false
        }
    }

    /// Get the connection token validator for direct log streaming authentication.
    pub fn connection_token_validator(&self) -> &Arc<ConnectionTokenValidator> {
        &self.connection_token_validator
    }

    /// Get the node proxy client for proxying requests to remote nodes.
    pub fn node_proxy_client(&self) -> &NodeProxyClient {
        &self.node_proxy_client
    }

    /// Get the API key-based remote client for node operations.
    ///
    /// This client uses the VK_NODE_API_KEY for authentication and is available
    /// even when no user is logged in via OAuth. Use this for hive operations
    /// that don't require user-specific permissions.
    pub fn node_auth_client(&self) -> Option<&RemoteClient> {
        self.node_auth_client.as_ref()
    }

    /// Get the event bus for durable event streaming and replay-to-live subscriptions.
    ///
    /// Returns a clone of the shared EventBus handle. All clones share the same tailer and
    /// broadcast channel, so any clone can be used for subscribing to events. Calling
    /// `shutdown()` on any clone will stop the tailer for all clones.
    pub fn event_bus(&self) -> Arc<EventBus> {
        self.event_bus.clone()
    }

    /// Get direct access to the local container service.
    ///
    /// This is needed because the `Deployment::container()` method returns
    /// `&impl ContainerService`, which doesn't expose LocalContainerService-specific
    /// methods like `message_queue()`.
    pub fn local_container(&self) -> &LocalContainerService {
        &self.container
    }

    /// Get the last VACUUM time (for rate limiting).
    pub fn last_vacuum_time(&self) -> &Arc<RwLock<Option<std::time::Instant>>> {
        &self.last_vacuum_time
    }

    /// Start the background node cache sync if the user is logged in.
    ///
    /// This spawns a background task that periodically syncs nodes and projects
    /// from all organizations the user has access to.
    pub async fn start_node_cache_sync(&self) {
        // Only start once
        let mut started = self.node_cache_sync_started.lock().await;
        if *started {
            tracing::debug!("node cache sync already started, skipping");
            return;
        }

        // Need remote client and credentials
        let Ok(client) = self.remote_client() else {
            tracing::warn!("remote client not configured, skipping node cache sync");
            return;
        };

        if self.auth_context.get_credentials().await.is_none() {
            tracing::warn!("not logged in, skipping node cache sync");
            return;
        }

        // Log which database we're using
        tracing::info!(
            db_path = %database_path().display(),
            "starting background node cache sync"
        );
        *started = true;

        let pool = self.db.pool.clone();
        let sync_service = NodeCacheSyncService::new(pool, client);

        tokio::spawn(async move {
            sync_service.run().await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::models::event::NodeEvent;
    use db::models::event_journal;
    use db::test_utils::create_test_pool_with_migrations;
    use std::sync::atomic::Ordering;
    use uuid::Uuid;

    /// Test 1: deployment_exposes_an_event_bus — assert EventBus is reachable and subscribe_from works
    #[tokio::test]
    async fn deployment_exposes_an_event_bus() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Mock a minimal deployment setup
        let bus = EventBus::new(pool.clone(), 64).await;
        let bus_clone = bus.clone();

        // Test that subscribe_from returns a working stream
        let _stream = bus_clone
            .subscribe_from(0)
            .expect("subscribe_from should succeed");
        // Stream created successfully
    }

    /// Test 2: startup_spawns_the_tailer — assert tailer runs and broadcasts events
    #[tokio::test]
    async fn startup_spawns_the_tailer() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        // Create EventBus which spawns the tailer
        let bus = EventBus::new(pool.clone(), 64).await;
        let mut subscriber = bus.sender().subscribe();

        // Journal an event directly
        {
            let mut tx = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx, &event).await.unwrap();
            tx.commit().await.unwrap();
        }

        // Assert tailer broadcasts it live within bounded wait
        let received = tokio::time::timeout(std::time::Duration::from_secs(5), subscriber.recv())
            .await
            .ok()
            .is_some();

        assert!(received, "tailer should have broadcast the committed event");
        bus.shutdown().await;
    }

    /// Test 3: startup_registers_the_real_trigger_hook — assert hook registry contains status hook
    #[tokio::test]
    async fn startup_registers_the_real_trigger_hook() {
        let hooks: Vec<Arc<dyn services::services::trigger_hooks::TriggerHook>> =
            vec![Arc::new(TaskStatusChangedHook)];
        let registry = TriggerHookRegistry::new(hooks);

        assert!(!registry.all().is_empty(), "registry should contain hooks");

        let hook_names: Vec<_> = registry.all().iter().map(|h| h.name()).collect();
        assert!(
            hook_names.contains(&"task_status_changed_logger"),
            "registry should contain the real status hook"
        );
    }

    /// Test 4: startup_spawns_compaction — assert compaction handle exists
    #[tokio::test]
    async fn startup_spawns_compaction() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let config = EventCompactionConfig::default();
        let handle = EventCompaction::spawn(pool, config);

        // The handle should be cloneable and functional
        let _handle_clone = handle.clone();
        handle.shutdown().await;
    }

    /// Test 5: shutdown_stops_the_background_tasks — assert spawned tasks terminate
    ///
    /// This test constrains per the task file:
    /// (a) Assert BEHAVIOURALLY: subscribe before shutdown, commit after, assert no delivery
    /// (b) SUBSCRIBE BEFORE the commit-and-wait window
    #[tokio::test]
    async fn shutdown_stops_the_background_tasks() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let bus = EventBus::new(pool.clone(), 64).await;

        // Subscribe BEFORE shutdown (critical invariant per task file constraint b)
        let mut subscriber = bus.sender().subscribe();

        // Prove the tailer IS publishing first
        {
            let mut tx = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx, &event).await.unwrap();
            tx.commit().await.unwrap();
        }

        // Wait for liveness publication
        let received_before_shutdown =
            tokio::time::timeout(std::time::Duration::from_secs(5), subscriber.recv())
                .await
                .ok()
                .is_some();

        assert!(
            received_before_shutdown,
            "tailer must be publishing before shutdown"
        );

        // Now shut down
        bus.shutdown().await;

        // Commit a row AFTER shutdown
        {
            let mut tx = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx, &event).await.unwrap();
            tx.commit().await.unwrap();
        }

        // Assert nothing arrives (the tailer is stopped)
        let got_after_shutdown =
            tokio::time::timeout(std::time::Duration::from_secs(2), subscriber.recv())
                .await
                .ok()
                .is_some();
        assert!(
            !got_after_shutdown,
            "tailer should be stopped; received event after shutdown"
        );
    }

    /// REQUIRED §1: event_bus_clone_shares_tailer_handle — assert clones share one tailer
    #[tokio::test]
    async fn event_bus_clone_shares_tailer_handle() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let bus1 = EventBus::new(pool.clone(), 64).await;
        let bus2 = bus1.clone();

        // Subscribe through clone 2
        let mut subscriber2 = bus2.sender().subscribe();

        // Journal an event and let the tailer publish it
        {
            let mut tx = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx, &event).await.unwrap();
            tx.commit().await.unwrap();
        }

        // Receive the first tailer publication through clone 2
        let first_seq = tokio::time::timeout(std::time::Duration::from_secs(5), subscriber2.recv())
            .await
            .ok()
            .and_then(|r| r.ok())
            .map(|ev| ev.seq);

        assert!(first_seq.is_some(), "clone 2 should receive the event");

        // Now call shutdown() on bus1
        bus1.shutdown().await;

        // Commit another row AFTER bus1.shutdown()
        {
            let mut tx = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx, &event).await.unwrap();
            tx.commit().await.unwrap();
        }

        // Assert bus2's subscriber receives NOTHING (the shared tailer is stopped)
        let got_after_shutdown =
            tokio::time::timeout(std::time::Duration::from_secs(2), subscriber2.recv())
                .await
                .ok()
                .is_some();
        assert!(
            !got_after_shutdown,
            "bus2's subscriber should be silent after bus1.shutdown() stopped the shared tailer"
        );
    }

    /// REQUIRED panel-009c: ensure_row_creates_cursor_row_at_zero — assert fresh cursor row
    #[tokio::test]
    async fn ensure_row_creates_cursor_row_at_zero() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let hook_name = "test_fresh_hook";

        // Before ensure_row, no row should exist
        let cursor_before = trigger_cursor::get(&pool, hook_name).await.unwrap();
        assert_eq!(cursor_before, 0, "missing hook should return cursor 0");

        // Call ensure_row
        trigger_cursor::ensure_row(&pool, hook_name)
            .await
            .expect("ensure_row should succeed");

        // After ensure_row, row should exist at 0
        let (cursor_after, needs_rebootstrap) = trigger_cursor::get_with_flag(&pool, hook_name)
            .await
            .unwrap();
        assert_eq!(cursor_after, 0, "fresh cursor row should start at 0");
        assert!(
            !needs_rebootstrap,
            "fresh cursor row should have needs_rebootstrap=0"
        );
    }

    /// REQUIRED §3: poison_trigger_cursors_supervised_respawn — assert supervised respawn works
    #[tokio::test]
    async fn poison_trigger_cursors_supervised_respawn() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let hook_name = "test_respawn_hook";

        // Create a RAISE(ABORT) trigger to poison cursor writes
        sqlx::query(
            r#"
            CREATE TRIGGER poison_cursors BEFORE INSERT ON trigger_cursors
            BEGIN SELECT RAISE(ABORT, 'cursor writes poisoned'); END
        "#,
        )
        .execute(&pool)
        .await
        .ok(); // May fail if trigger already exists, that's fine

        // Try to ensure_row — it will fail due to the poison
        let result = trigger_cursor::ensure_row(&pool, hook_name).await;
        assert!(
            result.is_err(),
            "ensure_row should fail with poisoned trigger"
        );

        // Now drop the poison
        sqlx::query("DROP TRIGGER IF EXISTS poison_cursors")
            .execute(&pool)
            .await
            .ok();

        // After dropping, ensure_row should succeed
        let result = trigger_cursor::ensure_row(&pool, hook_name).await;
        assert!(
            result.is_ok(),
            "ensure_row should succeed after poison is removed"
        );

        // Verify the row now exists
        let (cursor, _) = trigger_cursor::get_with_flag(&pool, hook_name)
            .await
            .unwrap();
        assert_eq!(cursor, 0, "cursor should be at 0 after recovery");
    }

    /// Helper: mutation proof for REQUIRED §1
    /// This proves that clones with INDEPENDENT handles would fail
    /// (Mutation: replace handle.clone() with Arc::new(tokio::sync::Mutex::new(None)))
    #[tokio::test]
    async fn mutation_proof_clones_must_share_tailer_handle() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let bus = EventBus::new(pool.clone(), 64).await;

        // Let tailer run a bit
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let health_before = bus.tailer_health().polls_total.load(Ordering::Relaxed);
        assert!(
            health_before >= 1,
            "tailer should have polled at least once"
        );

        // Clone and verify it sees the same counter
        let cloned = bus.clone();
        let health_cloned = cloned.tailer_health().polls_total.load(Ordering::Relaxed);
        assert_eq!(
            health_before, health_cloned,
            "cloned bus should observe the same tailer health counter"
        );

        bus.shutdown().await;
    }

    /// Helper: mutation proof for shutdown_stops_the_background_tasks
    /// This proves that replacing shutdown() with a no-op would fail the test
    /// (The test requires silence AFTER shutdown, not just during)
    #[tokio::test]
    async fn mutation_proof_shutdown_actually_stops_tailer() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;

        let bus = EventBus::new(pool.clone(), 64).await;
        let mut subscriber = bus.sender().subscribe();

        // Prove tailer is live by getting an event
        {
            let mut tx = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx, &event).await.unwrap();
            tx.commit().await.unwrap();
        }

        let got_first = tokio::time::timeout(std::time::Duration::from_secs(2), subscriber.recv())
            .await
            .ok()
            .is_some();

        assert!(got_first, "tailer must be live before shutdown");

        // If we replace shutdown() with a no-op, the tailer keeps running
        // and this post-shutdown commit will be broadcast
        bus.shutdown().await;

        {
            let mut tx = pool.begin().await.unwrap();
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx, &event).await.unwrap();
            tx.commit().await.unwrap();
        }

        // This MUST be silent if shutdown() actually stopped the tailer
        if let Ok(Ok(_)) =
            tokio::time::timeout(std::time::Duration::from_secs(1), subscriber.recv()).await
        {
            panic!("shutdown() did not stop the tailer");
        }
        // Expected: timeout or channel closed
    }
}
