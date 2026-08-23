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
    browser_auth_epoch: Arc<Mutex<u64>>,
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
    /// Handles to the supervised trigger-hook runner tasks — one per registered hook.
    ///
    /// Retention does NOT keep the tasks alive: dropping a `JoinHandle` DETACHES the task, it does
    /// not abort it. The handles are retained so a future shutdown path can `abort()` them and so
    /// tests can observe that the runners were spawned at all. There is no deployment-wide
    /// shutdown method today (see the STOP trigger recorded in the decisions-ledger); the only
    /// background task this deployment can currently stop is the tailer, via
    /// `event_bus().shutdown()`.
    ///
    /// `Arc<Vec<..>>` rather than `Vec<..>` because `LocalDeployment` derives `Clone` and
    /// `JoinHandle` does not implement `Clone`.
    #[allow(dead_code)]
    trigger_hook_runner_handles: Arc<Vec<tokio::task::JoinHandle<()>>>,
    /// Handle to the event compaction background service.
    ///
    /// Dropping this handle does NOT stop the compaction loop: the loop's `select!` keeps its
    /// interval-tick arm after the command channel closes, so it runs until
    /// `EventCompactionHandle::shutdown()` is called. Retained so that shutdown — and the
    /// on-demand `compact_now()` used by this file's tests — remain reachable.
    #[allow(dead_code)]
    compaction_handle: services::services::event_compaction::EventCompactionHandle,
}

/// Default initial backoff before a dead trigger-hook runner is respawned.
const TRIGGER_RUNNER_INITIAL_BACKOFF: std::time::Duration = std::time::Duration::from_secs(1);

/// Ceiling for the doubling trigger-hook runner respawn backoff.
const TRIGGER_RUNNER_MAX_BACKOFF: std::time::Duration = std::time::Duration::from_secs(60);

/// Broadcast channel capacity for the [`EventBus`] created at startup.
const EVENT_BUS_BROADCAST_CAPACITY: usize = 64;

/// Tunables for the background loops started by [`LocalDeployment::from_parts`].
///
/// Production always uses [`StartupTuning::default`]; the fields exist so this file's tests can
/// drive the supervised respawn loop and the compaction loop on timescales a test can wait for,
/// without touching process-global environment variables.
#[derive(Clone, Debug)]
pub(crate) struct StartupTuning {
    /// First backoff after a trigger-hook runner dies, doubling up to `trigger_runner_max_backoff`.
    trigger_runner_initial_backoff: std::time::Duration,
    /// Ceiling for the doubling respawn backoff.
    trigger_runner_max_backoff: std::time::Duration,
    /// Configuration handed to the event compaction loop.
    compaction: EventCompactionConfig,
}

struct StartupRemoteConfig {
    api_base: Option<String>,
    share_config: Option<ShareConfig>,
}

impl Default for StartupTuning {
    fn default() -> Self {
        Self {
            trigger_runner_initial_backoff: TRIGGER_RUNNER_INITIAL_BACKOFF,
            trigger_runner_max_backoff: TRIGGER_RUNNER_MAX_BACKOFF,
            compaction: EventCompactionConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
struct PendingHandoff {
    provider: String,
    app_verifier: String,
}

impl LocalDeployment {
    /// Internal constructor seam: everything [`LocalDeployment::new`] does once the LIVE
    /// `DBService` exists.
    ///
    /// `new()` owns the parts that must not run in a test — reading and rewriting the config file,
    /// and opening the real database — and then delegates here. Tests construct the real
    /// deployment through this seam against a migrated test pool, so the startup wiring itself
    /// (event bus over the live pool, journal tailer, supervised trigger-hook runners, compaction
    /// loop) is covered rather than re-implemented in the test module.
    ///
    /// The public API is unchanged: this is `pub(crate)`, and the `Deployment` trait is untouched.
    pub(crate) async fn from_parts(
        db: DBService,
        config: Arc<RwLock<Config>>,
        oauth_credentials: Arc<OAuthCredentials>,
        events_msg_store: Arc<MsgStore>,
        events_entry_count: Arc<RwLock<usize>>,
        tuning: StartupTuning,
        remote_config: StartupRemoteConfig,
    ) -> Result<Self, DeploymentError> {
        let StartupRemoteConfig {
            api_base,
            share_config,
        } = remote_config;

        // Generate a unique user ID for this deployment
        let user_id = Uuid::new_v4().to_string();
        let git = GitService::new();
        let msg_stores = Arc::new(RwLock::new(HashMap::new()));
        let filesystem = FilesystemService::new();

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

        // oauth_credentials already loaded in parallel at startup

        let profile_cache = Arc::new(RwLock::new(None));
        let auth_context = AuthContext::new(oauth_credentials.clone(), profile_cache.clone());

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
        let browser_auth_epoch = Arc::new(Mutex::new(0u64));

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

        // Create the EventBus over the LIVE DBService pool (the one from
        // `new_with_after_connect`), so the tailer reads the pool the application writes to.
        let event_bus =
            Arc::new(EventBus::new(db.pool.clone(), EVENT_BUS_BROADCAST_CAPACITY).await);

        // Build the trigger-hook registry with the one real status hook
        let hooks: Vec<Arc<dyn services::services::trigger_hooks::TriggerHook>> =
            vec![Arc::new(TaskStatusChangedHook)];
        let hook_registry = TriggerHookRegistry::new(hooks);

        // One spawned task per hook, each with its own supervised respawn loop.
        let mut runner_handles = Vec::new();
        for hook in hook_registry.all() {
            // `name()` returns a `&'static str`, so it moves into the spawned task as-is.
            let hook_name = hook.name();

            // Registration-time cursor row, BEFORE the runner is spawned: a hook with no
            // `trigger_cursors` row contributes nothing to the compaction floor and matches no
            // row in compaction's flag UPDATE, so a brand-new hook mid-replay could have the
            // journal deleted underneath it and never be flagged. A failure here fails startup.
            trigger_cursor::ensure_row(&db.pool, hook_name).await?;

            let hook_clone = hook.clone();
            let db_clone = db.clone();
            let event_bus_clone = event_bus.clone();
            let initial_backoff = tuning.trigger_runner_initial_backoff;
            let max_backoff = tuning.trigger_runner_max_backoff;

            let task = tokio::spawn(async move {
                let mut backoff = initial_backoff;

                loop {
                    // Belt and braces: the row is created at registration above, but a runner
                    // whose row was lost (manual deletion, restore from an older database) must
                    // recreate it rather than silently drop off the compaction floor.
                    if let Err(e) = trigger_cursor::ensure_row(&db_clone.pool, hook_name).await {
                        tracing::error!(hook = %hook_name, error = ?e, "failed to ensure cursor row");
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(max_backoff);
                        continue;
                    }

                    let db_for_runner = db_clone.clone();
                    let hook_for_runner = hook_clone.clone();
                    let bus_for_runner = event_bus_clone.clone();

                    // Spawn run_hook as its own task so a panic in `fire()` is caught as a
                    // JoinError and respawned, rather than killing this supervisor.
                    let inner_task = tokio::spawn(async move {
                        if let Err(e) = services::services::trigger_hooks::run_hook(
                            db_for_runner.pool,
                            hook_for_runner,
                            bus_for_runner,
                        )
                        .await
                        {
                            tracing::error!(
                                hook = %hook_name,
                                error = ?e,
                                "trigger hook runner failed"
                            );
                        }
                    });

                    // Wait for completion and catch panics
                    if let Err(e) = inner_task.await {
                        tracing::error!(
                            hook = %hook_name,
                            error = ?e,
                            "trigger hook task panicked"
                        );
                    }

                    // Re-read the rebootstrap flag so a flag raised by LIVE compaction is
                    // observable in the log without a process restart; `run_hook` itself re-reads
                    // both cursor and flag on its next start.
                    let (_, needs_rebootstrap) =
                        match trigger_cursor::get_with_flag(&db_clone.pool, hook_name).await {
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
                        backoff_ms = backoff.as_millis() as u64,
                        "trigger hook runner terminated; respawning after backoff"
                    );

                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(max_backoff);
                }
            });

            runner_handles.push(task);
        }

        // Every runner handle is retained. Dropping a JoinHandle detaches its task rather than
        // aborting it, so this retention is about being able to observe and (in future) abort
        // them — not about keeping them alive.
        let trigger_hook_runner_handles = Arc::new(runner_handles);

        // Spawn the event compaction loop over the same live pool
        let compaction_handle = EventCompaction::spawn(db.pool.clone(), tuning.compaction);

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
            browser_auth_epoch: browser_auth_epoch.clone(),
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
            trigger_hook_runner_handles,
            compaction_handle,
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
            deployment.install_remote_sync(sc).await;
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

    #[cfg(test)]
    fn disable_orphan_cleanup_for_tests() {
        static DISABLE_ORPHAN_CLEANUP: std::sync::Once = std::sync::Once::new();
        DISABLE_ORPHAN_CLEANUP.call_once(|| {
            // SAFETY (partial, and stated honestly): this write is ordered before THIS call's
            // own container construction, so the sweep it spawns always observes it. It is NOT
            // race-free against sibling tests: `set_var` is unsound against any concurrent
            // `getenv`, and tests on other threads may be inside `from_parts` calling
            // `ShareConfig::from_env`, `NodeRunnerConfig::from_env` or `database_path()` at this
            // moment. Accepted deliberately: the value is written once, never unset, never read
            // for an assertion, and the alternative is a test run that deletes a developer's
            // worktrees.
            unsafe {
                std::env::set_var("DISABLE_WORKTREE_ORPHAN_CLEANUP", "1");
            }
        });
    }

    /// Construct a REAL deployment over a migrated test pool, through the same
    /// [`from_parts`](Self::from_parts) seam production uses.
    ///
    /// Only the pieces `new()` owns are substituted: the config is a default rather than the
    /// on-disk one, and the OAuth credentials point at an unwritten temp path (never loaded, so
    /// no file is read or created). Everything the wiring under test touches — the event bus over
    /// the live pool, the tailer, the supervised runners, the compaction loop — is the real thing.
    #[cfg(test)]
    pub(crate) async fn for_test(
        pool: sqlx::SqlitePool,
        tuning: StartupTuning,
    ) -> Result<Self, DeploymentError> {
        // `from_parts` builds a real LocalContainerService, whose constructor spawns
        // `cleanup_orphaned_worktrees` (crates/local-deployment/src/container.rs:320). That sweep
        // treats every directory under the worktree base dir with no matching `task_attempts` row
        // as orphaned — and a test database has no such rows — so on a machine where the base dir
        // exists it would delete real worktrees. The reach is pre-existing (container.rs:169's
        // `new_for_drain_test` already calls the same constructor), but this test module makes it
        // routine, so disable the sweep for the whole test binary before any deployment is built.
        Self::disable_orphan_cleanup_for_tests();

        let db = DBService {
            pool,
            metrics: db::DbMetrics::new(),
        };
        let creds_path = std::env::temp_dir()
            .join(format!("vk-test-credentials-{}", Uuid::new_v4()))
            .join("credentials.json");

        Self::from_parts(
            db,
            Arc::new(RwLock::new(Config::default())),
            Arc::new(OAuthCredentials::new(creds_path)),
            Arc::new(MsgStore::new()),
            Arc::new(RwLock::new(0)),
            tuning,
            StartupRemoteConfig {
                api_base: None,
                share_config: None,
            },
        )
        .await
    }
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

        let api_base = std::env::var("VK_SHARED_API_BASE")
            .ok()
            .or_else(|| option_env!("VK_SHARED_API_BASE").map(String::from));
        let share_config = ShareConfig::from_env();

        Self::from_parts(
            db,
            config,
            oauth_credentials,
            events_msg_store,
            events_entry_count,
            StartupTuning::default(),
            StartupRemoteConfig {
                api_base,
                share_config,
            },
        )
        .await
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

    fn browser_auth_epoch(&self) -> &Arc<Mutex<u64>> {
        &self.browser_auth_epoch
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

    /// Stop the event-journal background writers: the compaction loop and the bus tailer.
    ///
    /// Called from the server's shutdown path BEFORE the final WAL checkpoint. Best-effort
    /// and signal-only: the compaction shutdown returns once the command is queued (not when
    /// the worker exits) and the tailer abort is asynchronous, so a pass already in flight
    /// can still commit after the checkpoint — SQLite replays any residual WAL on next open,
    /// and `pool.close().await` waits out in-flight connections. What this DOES guarantee is
    /// that no NEW compaction pass or tail poll starts after shutdown, so the writers cannot
    /// spin on `PoolClosed` errors until process exit.
    pub async fn shutdown_event_services(&self) {
        self.compaction_handle.shutdown().await;
        self.event_bus.shutdown().await;
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
    use futures::StreamExt;
    use std::time::Duration;
    use uuid::Uuid;

    /// The name of the one real hook registered by `from_parts`.
    const REAL_HOOK: &str = "task_status_changed_logger";

    /// Tuning for deployment tests: a respawn backoff a test can wait out, and a compaction loop
    /// whose only run is the one a test asks for via `compact_now()` (the loop skips its first
    /// tick, and an hour-long interval means no second one lands during a test).
    fn test_tuning() -> StartupTuning {
        StartupTuning {
            trigger_runner_initial_backoff: Duration::from_millis(50),
            trigger_runner_max_backoff: Duration::from_millis(100),
            compaction: EventCompactionConfig {
                retention_hours: 168,
                min_rows: 1,
                max_rows: 5,
                compaction_interval_secs: 3600,
            },
        }
    }

    /// Journal one event through the real model function, committing it exactly as a write site
    /// would. Returns the assigned sequence number.
    async fn journal_event(pool: &sqlx::SqlitePool) -> i64 {
        let mut tx = pool.begin().await.expect("begin");
        let event = NodeEvent::TaskCreated {
            task_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
        };
        let seq = event_journal::append(&mut *tx, &event)
            .await
            .expect("append event");
        tx.commit().await.expect("commit");
        seq
    }

    async fn journal_row_count(pool: &sqlx::SqlitePool) -> i64 {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM event_journal")
            .fetch_one(pool)
            .await
            .expect("count journal rows")
    }

    /// The cursor row for `hook_name`, or None when no row exists. Unlike `trigger_cursor::get`,
    /// this distinguishes "no row" from "row at 0" — which is the whole point of `ensure_row`.
    async fn cursor_row(pool: &sqlx::SqlitePool, hook_name: &str) -> Option<(i64, bool)> {
        sqlx::query_as::<_, (i64, bool)>(
            "SELECT last_processed_seq, needs_rebootstrap FROM trigger_cursors WHERE hook_name = ?",
        )
        .bind(hook_name)
        .fetch_optional(pool)
        .await
        .expect("read cursor row")
    }

    /// Poll `check` until it holds, or fail the test after 10 seconds.
    async fn wait_for<F, Fut>(label: &str, mut check: F)
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            if check().await {
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for {label}"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    /// Test 1: the wired deployment exposes a WORKING event bus through the inherent accessor.
    ///
    /// The stream is polled rather than merely constructed: a bus wired over the wrong pool, or a
    /// `subscribe_from` that yields a stream nothing ever feeds, fails here.
    #[tokio::test]
    async fn deployment_exposes_an_event_bus() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let deployment = LocalDeployment::for_test(pool.clone(), test_tuning())
            .await
            .expect("deployment constructs over the test pool");

        let mut stream = deployment
            .event_bus()
            .subscribe_from(0)
            .expect("subscribe_from(0) succeeds");

        let seq = journal_event(&pool).await;

        let delivered = tokio::time::timeout(Duration::from_secs(10), stream.next())
            .await
            .expect("stream yields the journaled event within the bounded wait")
            .expect("stream does not end")
            .expect("stream does not error");

        assert_eq!(
            delivered.seq, seq,
            "the deployment's bus must deliver the event that was journaled to its pool"
        );

        deployment.event_bus().shutdown().await;
    }

    /// Test 2: startup actually spawns the tailer, over the deployment's own pool.
    ///
    /// The subscriber is a raw broadcast receiver (no journal replay), taken from the bus the
    /// deployment exposes and BEFORE the commit, so only a live tailer publication can satisfy it.
    /// A deployment that never spawns the tailer, or wires the bus to a pool nothing writes to,
    /// fails here.
    #[tokio::test]
    async fn startup_spawns_the_tailer() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let deployment = LocalDeployment::for_test(pool.clone(), test_tuning())
            .await
            .expect("deployment constructs over the test pool");

        let mut live = deployment.event_bus().sender().subscribe();

        let seq = journal_event(&pool).await;

        let delivered = tokio::time::timeout(Duration::from_secs(10), live.recv())
            .await
            .expect("the tailer publishes the committed event within the bounded wait")
            .expect("broadcast channel stays open");

        assert_eq!(
            delivered.seq, seq,
            "append -> tail -> broadcast must be connected on a real deployment"
        );

        deployment.event_bus().shutdown().await;
    }

    /// Test 3: startup registers the real trigger hook — asserted behaviourally, against the
    /// database the deployment wired itself to.
    ///
    /// `ensure_row` runs at registration, before the runner is spawned, so the row is present the
    /// moment construction returns. A deployment that builds a registry but never registers the
    /// hook (or skips `ensure_row`) leaves no row and fails here.
    #[tokio::test]
    async fn startup_registers_the_real_trigger_hook() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let deployment = LocalDeployment::for_test(pool.clone(), test_tuning())
            .await
            .expect("deployment constructs over the test pool");

        assert_eq!(
            cursor_row(&pool, REAL_HOOK).await,
            Some((0, false)),
            "registration must leave a cursor row for the real hook at seq 0, unflagged"
        );

        // Retention, not just behaviour: one handle per registered hook, none discarded.
        assert_eq!(
            deployment.trigger_hook_runner_handles.len(),
            1,
            "every registered hook's runner handle must be retained"
        );

        deployment.event_bus().shutdown().await;
    }

    /// Test 4: startup spawns the compaction loop — asserted by driving the handle the deployment
    /// retained and observing rows actually deleted.
    ///
    /// The loop skips its first tick and this test's interval is an hour, so the only compaction
    /// that can run is the one requested here: if the deployment never spawned the loop (or did
    /// not retain a working handle), nothing consumes the command and the journal stays at 20.
    #[tokio::test]
    async fn startup_spawns_compaction() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let deployment = LocalDeployment::for_test(pool.clone(), test_tuning())
            .await
            .expect("deployment constructs over the test pool");

        // Over the tuned hard cap of 5.
        for _ in 0..20 {
            journal_event(&pool).await;
        }
        assert_eq!(
            journal_row_count(&pool).await,
            20,
            "journal seeded over cap"
        );

        deployment.compaction_handle.compact_now().await;

        wait_for("compaction to trim the journal to the hard cap", || async {
            journal_row_count(&pool).await <= 5
        })
        .await;

        deployment.event_bus().shutdown().await;
    }

    /// Test 5: shutting the deployment's bus down stops the background tailer.
    ///
    /// Constraints from the task file, both learned the hard way in task 013:
    /// (a) assert BEHAVIOURALLY (a committed row is never published), not on a handle;
    /// (b) take the subscriber BEFORE the commit-and-wait window — a broadcast receiver never sees
    ///     history, so a subscriber created afterwards would report silence even from a live tailer.
    #[tokio::test]
    async fn shutdown_stops_the_background_tasks() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let deployment = LocalDeployment::for_test(pool.clone(), test_tuning())
            .await
            .expect("deployment constructs over the test pool");

        let mut live = deployment.event_bus().sender().subscribe();

        // Prove the tailer is publishing first, so the silence below means something.
        journal_event(&pool).await;
        tokio::time::timeout(Duration::from_secs(10), live.recv())
            .await
            .expect("tailer must be publishing before shutdown")
            .expect("broadcast channel open before shutdown");

        deployment.event_bus().shutdown().await;

        journal_event(&pool).await;

        let after = tokio::time::timeout(Duration::from_secs(2), live.recv()).await;
        assert!(
            after.is_err(),
            "a row committed after shutdown must never be published: {after:?}"
        );
    }

    /// REQUIRED §1: all clones of an `EventBus` share ONE tailer handle, so `shutdown()` on any
    /// clone stops the tailer for every clone.
    ///
    /// The clone here is an `EventBus::clone` (the accessor hands out `Arc` clones, which would
    /// share the handle trivially), so this pins `impl Clone for EventBus` itself: give clones an
    /// independent handle and the shutdown below becomes a no-op, the tailer survives, and the
    /// post-shutdown commit reaches `sub1`.
    #[tokio::test]
    async fn event_bus_clone_shares_tailer_handle() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let deployment = LocalDeployment::for_test(pool.clone(), test_tuning())
            .await
            .expect("deployment constructs over the test pool");

        let bus1 = deployment.event_bus();
        let bus2: EventBus = (*bus1).clone();

        let mut sub1 = bus1.sender().subscribe();

        // Liveness first: silence only proves something once publication is established.
        journal_event(&pool).await;
        tokio::time::timeout(Duration::from_secs(10), sub1.recv())
            .await
            .expect("the shared tailer must be publishing before shutdown")
            .expect("broadcast channel open before shutdown");

        // Shut down through the CLONE.
        bus2.shutdown().await;

        journal_event(&pool).await;

        let after = tokio::time::timeout(Duration::from_secs(2), sub1.recv()).await;
        assert!(
            after.is_err(),
            "shutdown() on a clone must stop the tailer observed through the original: {after:?}"
        );
    }

    /// REQUIRED panel-009c: `ensure_row`'s fresh-row INSERT path — the half task 009 left untested.
    #[tokio::test]
    async fn ensure_row_creates_cursor_row_at_zero() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let hook_name = "test_fresh_hook";

        assert_eq!(
            cursor_row(&pool, hook_name).await,
            None,
            "no cursor row exists before ensure_row"
        );

        trigger_cursor::ensure_row(&pool, hook_name)
            .await
            .expect("ensure_row succeeds");

        assert_eq!(
            cursor_row(&pool, hook_name).await,
            Some((0, false)),
            "a fresh cursor row starts at seq 0 with needs_rebootstrap = 0"
        );
    }

    /// REQUIRED §3: the supervised runner loop survives a fatal cursor-write failure and resumes
    /// processing once the failure clears — with no process restart and no reconstruction.
    ///
    /// `run_hook` returns on ANY cursor write error, so a bare `let _ = run_hook(..)` spawn would
    /// die permanently here and pin the compaction floor at a stale cursor. The RAISE(ABORT)
    /// triggers cover INSERT and UPDATE because `trigger_cursor::set` is an upsert and
    /// `ensure_row` is an INSERT OR IGNORE.
    #[tokio::test]
    async fn supervised_runner_resumes_after_poisoned_cursor_writes() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let deployment = LocalDeployment::for_test(pool.clone(), test_tuning())
            .await
            .expect("deployment constructs over the test pool");

        for stmt in [
            "CREATE TRIGGER poison_cursor_insert BEFORE INSERT ON trigger_cursors \
             BEGIN SELECT RAISE(ABORT, 'cursor writes poisoned'); END",
            "CREATE TRIGGER poison_cursor_update BEFORE UPDATE ON trigger_cursors \
             BEGIN SELECT RAISE(ABORT, 'cursor writes poisoned'); END",
        ] {
            sqlx::query(stmt)
                .execute(&pool)
                .await
                .expect("install poison trigger");
        }

        let seq = journal_event(&pool).await;

        // Poisoned: the runner keeps dying and respawning (50ms doubling to 100ms), so it gets
        // several attempts inside this window and none of them can move the cursor.
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert_eq!(
            cursor_row(&pool, REAL_HOOK).await,
            Some((0, false)),
            "the cursor cannot advance while cursor writes abort"
        );

        for stmt in [
            "DROP TRIGGER poison_cursor_insert",
            "DROP TRIGGER poison_cursor_update",
        ] {
            sqlx::query(stmt)
                .execute(&pool)
                .await
                .expect("drop poison trigger");
        }

        // No reconstruction: the same supervised loop must pick the work back up.
        wait_for(
            "the supervised runner to resume and advance the cursor",
            || async { matches!(cursor_row(&pool, REAL_HOOK).await, Some((c, _)) if c >= seq) },
        )
        .await;

        deployment.event_bus().shutdown().await;
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
        journal_event(&pool).await;

        let got_first = tokio::time::timeout(std::time::Duration::from_secs(2), subscriber.recv())
            .await
            .ok()
            .is_some();

        assert!(got_first, "tailer must be live before shutdown");

        // If we replace shutdown() with a no-op, the tailer keeps running
        // and this post-shutdown commit will be broadcast
        bus.shutdown().await;

        journal_event(&pool).await;

        // This MUST be silent if shutdown() actually stopped the tailer
        if let Ok(Ok(_)) =
            tokio::time::timeout(std::time::Duration::from_secs(1), subscriber.recv()).await
        {
            panic!("shutdown() did not stop the tailer");
        }
        // Expected: timeout or channel closed
    }

    #[tokio::test]
    async fn browser_auth_epoch_is_shared_by_deployment_clones() {
        let (pool, _temp_dir) = create_test_pool_with_migrations().await;
        let deployment = LocalDeployment::for_test(pool, test_tuning())
            .await
            .unwrap();
        let clone = deployment.clone();
        assert_eq!(*deployment.browser_auth_epoch().lock().await, 0);
        *clone.browser_auth_epoch().lock().await += 1;
        assert_eq!(*deployment.browser_auth_epoch().lock().await, 1);
        deployment.event_bus().shutdown().await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn configured_startup_sync_is_installed_before_constructor_returns() {
        LocalDeployment::disable_orphan_cleanup_for_tests();
        let (pool, temp_dir) = create_test_pool_with_migrations().await;
        let credentials = Arc::new(OAuthCredentials::new_file_backed(
            temp_dir.path().join("credentials.json"),
        ));
        credentials
            .save(&services::services::oauth_credentials::Credentials {
                access_token: Some("test-access-token".to_owned()),
                refresh_token: "test-refresh-token".to_owned(),
                expires_at: None,
            })
            .await
            .unwrap();

        let deployment = LocalDeployment::from_parts(
            DBService {
                pool,
                metrics: db::DbMetrics::new(),
            },
            Arc::new(RwLock::new(Config::default())),
            credentials,
            Arc::new(MsgStore::new()),
            Arc::new(RwLock::new(0)),
            test_tuning(),
            StartupRemoteConfig {
                api_base: Some("http://127.0.0.1:1".to_owned()),
                share_config: Some(ShareConfig {
                    api_base: "http://127.0.0.1:1".parse().unwrap(),
                    websocket_base: "ws://127.0.0.1:1".parse().unwrap(),
                    activity_page_limit: 1,
                    bulk_sync_threshold: 1,
                }),
            },
        )
        .await
        .unwrap();

        let sync_handle = deployment.share_sync_handle().lock().await.take();
        assert!(sync_handle.is_some());
        sync_handle.unwrap().shutdown().await;
        deployment.event_bus().shutdown().await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn raw_api_base_remains_available_when_share_sync_config_is_unavailable() {
        LocalDeployment::disable_orphan_cleanup_for_tests();
        let (pool, temp_dir) = create_test_pool_with_migrations().await;
        let deployment = LocalDeployment::from_parts(
            DBService {
                pool,
                metrics: db::DbMetrics::new(),
            },
            Arc::new(RwLock::new(Config::default())),
            Arc::new(OAuthCredentials::new_file_backed(
                temp_dir.path().join("credentials.json"),
            )),
            Arc::new(MsgStore::new()),
            Arc::new(RwLock::new(0)),
            test_tuning(),
            StartupRemoteConfig {
                api_base: Some("ftp://example.invalid".to_owned()),
                share_config: None,
            },
        )
        .await
        .unwrap();

        assert!(deployment.remote_client().is_ok());
        deployment.event_bus().shutdown().await;
    }
}
