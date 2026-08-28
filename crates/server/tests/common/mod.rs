use db::models::{
    project::{CreateProject, Project},
    task::{CreateTask, Task, TaskStatus},
    task_attempt::{CreateTaskAttempt, TaskAttempt},
};
use deployment::Deployment;
use executors::executors::BaseCodingAgent;
use local_deployment::LocalDeployment;
use server::DeploymentImpl;
use server::auth::seams::TokenSource;
use std::net::SocketAddr;
use uuid::Uuid;

struct RetainedNodeAuth {
    hive_url: String,
    api_key: String,
    secret: String,
    expected_node_id: Uuid,
}

pub struct HiveHarness {
    #[allow(dead_code)]
    temp_dir: tempfile::TempDir,
    #[allow(dead_code)]
    mock_server: wiremock::MockServer,
    #[allow(dead_code)]
    deployment: DeploymentImpl,
    addr: SocketAddr,
    shutdown_sender: Option<tokio::sync::oneshot::Sender<()>>,
    serve_handle: Option<tokio::task::JoinHandle<()>>,
    current_server_generation: u64,
    last_completed_server_generation: Option<u64>,
    mounted_hive_mocks:
        std::sync::Arc<tokio::sync::Mutex<std::collections::BTreeSet<(String, String)>>>,
    redirect_mock_paths: std::sync::Arc<tokio::sync::Mutex<std::collections::BTreeSet<String>>>,
    retained_asset_dir: std::path::PathBuf,
    retained_database_path: std::path::PathBuf,
    retained_shared_api_base: Option<String>,
    retained_node_auth: Option<RetainedNodeAuth>,
}

pub struct Resp {
    pub status: u16,
    pub body: String,
    #[allow(dead_code)]
    pub content_type: Option<String>,
    /// Every response header, cloned BEFORE `.text()`/body consumption. HeaderMap preserves
    /// repeated values, including every Location/Set-Cookie surface task 018 must scan.
    #[allow(dead_code)]
    pub headers: reqwest::header::HeaderMap,
    /// RAW `Set-Cookie` lines, verbatim and unparsed, derived from `headers.get_all(SET_COOKIE)`.
    #[allow(dead_code)]
    pub set_cookie: Vec<String>,
}

impl Resp {
    /// True when this is the SPA `index.html` that the catch-all `/{*path}` route
    /// (crates/server/src/routes/mod.rs:76 -> frontend.rs:40-43) serves with 200 OK
    /// for ANY unmatched GET. A response that is the SPA fallback did NOT reach a
    /// registered API route, whatever its status code says.
    pub fn is_spa_fallback(&self) -> bool {
        self.content_type
            .as_deref()
            .is_some_and(|c| c.starts_with("text/html"))
            || self.body.trim_start().starts_with("<!DOCTYPE html")
    }

    /// Assert the request reached a REGISTERED API route rather than the SPA fallback.
    /// This — NOT a 404 check — is what proves route registration in this codebase.
    pub fn assert_registered(&self) {
        assert!(
            !self.is_spa_fallback(),
            "route is NOT registered: request fell through to the SPA catch-all \
             (status {}, content-type {:?}). A non-404 status proves nothing here.",
            self.status,
            self.content_type
        );
    }

    /// Extract the Location header value.
    #[allow(dead_code)]
    pub fn location(&self) -> Option<&str> {
        self.headers
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
    }
}

/// An independent browser cookie jar. Two jars in one test are two clean browsers.
#[allow(dead_code)]
#[derive(Default)]
pub struct CookieJar {
    cookies: std::collections::BTreeMap<String, String>,
}

#[allow(dead_code)]
impl CookieJar {
    pub fn new() -> Self {
        Self::default()
    }
    /// Set a cookie directly (used to forge a wrong-browser value).
    pub fn insert(&mut self, name: &str, value: &str) {
        self.cookies.insert(name.to_string(), value.to_string());
    }
    pub fn get(&self, name: &str) -> Option<&str> {
        self.cookies.get(name).map(|s| s.as_str())
    }
    /// The `Cookie:` request-header value, or None when the jar is empty.
    pub fn header_value(&self) -> Option<String> {
        if self.cookies.is_empty() {
            None
        } else {
            Some(
                self.cookies
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        }
    }
    /// Apply raw `Set-Cookie` lines: store `name=value`, and REMOVE the cookie when a
    /// semicolon-delimited, case-insensitive `Max-Age` attribute parses as integer zero. Complete
    /// numeric parsing is required: `Max-Age=01` is one second and must not be treated as zero.
    pub fn apply(&mut self, set_cookie: &[String]) {
        for line in set_cookie {
            let mut parts = line.split(';');
            let Some(name_value) = parts.next() else {
                continue;
            };
            let Some((name, value)) = name_value.split_once('=') else {
                continue;
            };
            let name = name.trim();
            let value = value.trim();
            let max_age_zero = parts.any(|attr| {
                let attr = attr.trim();
                let Some((attr_name, attr_value)) = attr.split_once('=') else {
                    return false;
                };
                attr_name.trim().eq_ignore_ascii_case("Max-Age")
                    && attr_value.trim().parse::<i64>() == Ok(0)
            });
            if max_age_zero {
                self.cookies.remove(name);
            } else {
                self.cookies.insert(name.to_string(), value.to_string());
            }
        }
    }
    /// A jar that shares nothing with `self` -- an explicitly clean second browser.
    pub fn fresh() -> Self {
        Self::default()
    }
}

/// Outcome of a REAL protocol probe (websocket handshake or SSE request).
#[allow(dead_code)]
pub struct ProtocolProbe {
    /// 101 on a completed upgrade; otherwise the HTTP status of the rejection.
    pub status: u16,
    /// True only when a websocket connection was actually established.
    pub upgraded: bool,
    pub content_type: Option<String>,
}

#[allow(dead_code)]
impl HiveHarness {
    /// VK_SHARED_API_BASE points at a live wiremock server -> hive IS configured.
    pub async fn configured() -> Self {
        Self::configured_inner(None).await
    }

    async fn configured_inner(node_auth: Option<(String, Uuid)>) -> Self {
        // 1. Create temp_dir
        let temp_dir = tempfile::TempDir::new().unwrap();

        // 2. Env hygiene
        unsafe {
            std::env::remove_var("VK_HIVE_URL");
            std::env::remove_var("VK_NODE_API_KEY");
            std::env::set_var("DISABLE_WORKTREE_ORPHAN_CLEANUP", "1");
            std::env::set_var("DISABLE_WORKTREE_EXPIRED_CLEANUP", "1");
        }

        // 3. Redirect all on-disk state into temp_dir
        unsafe {
            std::env::set_var("VK_ASSET_DIR", temp_dir.path());
            std::env::set_var("VK_DATABASE_PATH", temp_dir.path().join("db.sqlite"));
        }

        // 4. configured() only: seed credentials.json
        std::fs::write(
            temp_dir.path().join("credentials.json"),
            r#"{"refresh_token":"test-refresh-token"}"#,
        )
        .unwrap();

        // 5. configured() only: start MockServer and mount /v1/tokens/refresh
        let mock_server = wiremock::MockServer::start().await;

        // Mount /v1/tokens/refresh with real JWT token
        let access_token = test_access_token("configured-label");
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/v1/tokens/refresh"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "access_token": access_token,
                    "refresh_token": "test-refresh-token"
                })),
            )
            .mount(&mock_server)
            .await;

        if let Some((secret, _)) = &node_auth {
            let hive_url = mock_server.uri().replacen("http://", "ws://", 1);
            unsafe {
                std::env::set_var("VK_HIVE_URL", hive_url);
                std::env::set_var("VK_NODE_API_KEY", "test-api-key");
                std::env::set_var("VK_CONNECTION_TOKEN_SECRET", secret);
            }
        }

        // 6. configured(): set VK_SHARED_API_BASE
        unsafe {
            std::env::set_var("VK_SHARED_API_BASE", mock_server.uri());
        }

        // 7. Build deployment the same way main.rs does
        let deployment = LocalDeployment::new().await.unwrap();
        if let Some((_, expected_node_id)) = node_auth {
            let context = deployment
                .node_runner_context()
                .expect("node-auth harness must start the node runner");
            context.state.write().await.node_id = Some(expected_node_id);
        }

        // 9. Serve real router on ephemeral listener
        let (shutdown_sender, shutdown_receiver) = tokio::sync::oneshot::channel();
        let app = server::routes::router(deployment.clone()).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let serve_handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_receiver.await;
                })
                .await
                .unwrap();
        });

        HiveHarness {
            retained_asset_dir: temp_dir.path().to_path_buf(),
            retained_database_path: temp_dir.path().join("db.sqlite"),
            retained_shared_api_base: Some(mock_server.uri()),
            retained_node_auth: node_auth.map(|(secret, expected_node_id)| RetainedNodeAuth {
                hive_url: mock_server.uri().replacen("http://", "ws://", 1),
                api_key: "test-api-key".to_string(),
                secret,
                expected_node_id,
            }),
            temp_dir,
            mock_server,
            deployment,
            addr,
            shutdown_sender: Some(shutdown_sender),
            serve_handle: Some(serve_handle),
            current_server_generation: 1,
            last_completed_server_generation: None,
            mounted_hive_mocks: std::sync::Arc::new(tokio::sync::Mutex::new(
                std::collections::BTreeSet::from([(
                    "POST".to_string(),
                    "/v1/tokens/refresh".to_string(),
                )]),
            )),
            redirect_mock_paths: std::sync::Arc::new(tokio::sync::Mutex::new(
                std::collections::BTreeSet::new(),
            )),
        }
    }

    /// VK_SHARED_API_BASE unset -> deployment.remote_client() is Err(RemoteClientNotConfigured).
    pub async fn hive_absent() -> Self {
        // 1. Create temp_dir
        let temp_dir = tempfile::TempDir::new().unwrap();

        // 2. Env hygiene
        unsafe {
            std::env::remove_var("VK_HIVE_URL");
            std::env::remove_var("VK_NODE_API_KEY");
            std::env::set_var("DISABLE_WORKTREE_ORPHAN_CLEANUP", "1");
            std::env::set_var("DISABLE_WORKTREE_EXPIRED_CLEANUP", "1");
        }

        // 3. Redirect all on-disk state into temp_dir
        unsafe {
            std::env::set_var("VK_ASSET_DIR", temp_dir.path());
            std::env::set_var("VK_DATABASE_PATH", temp_dir.path().join("db.sqlite"));
        }

        // 5. hive_absent() also starts a MockServer (but skips step 4)
        let mock_server = wiremock::MockServer::start().await;

        // 6. hive_absent(): remove VK_SHARED_API_BASE (don't point it at mock_server)
        unsafe {
            std::env::remove_var("VK_SHARED_API_BASE");
        }

        // 7. Build deployment the same way main.rs does
        let deployment = LocalDeployment::new().await.unwrap();

        // 8. hive_absent() only: assert absence
        assert!(
            deployment.remote_client().is_err(),
            "hive_absent() built a CONFIGURED deployment — VK_SHARED_API_BASE was baked in at \
             compile time via build.rs/option_env!. Unset it (and check .env) and rebuild."
        );

        // 9. Serve real router on ephemeral listener
        let (shutdown_sender, shutdown_receiver) = tokio::sync::oneshot::channel();
        let app = server::routes::router(deployment.clone()).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let serve_handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_receiver.await;
                })
                .await
                .unwrap();
        });

        HiveHarness {
            retained_asset_dir: temp_dir.path().to_path_buf(),
            retained_database_path: temp_dir.path().join("db.sqlite"),
            retained_shared_api_base: None,
            retained_node_auth: None,
            temp_dir,
            mock_server,
            deployment,
            addr,
            shutdown_sender: Some(shutdown_sender),
            serve_handle: Some(serve_handle),
            current_server_generation: 1,
            last_completed_server_generation: None,
            mounted_hive_mocks: std::sync::Arc::new(tokio::sync::Mutex::new(
                std::collections::BTreeSet::new(),
            )),
            redirect_mock_paths: std::sync::Arc::new(tokio::sync::Mutex::new(
                std::collections::BTreeSet::new(),
            )),
        }
    }

    /// Queue a canned hive response. `path` is the hive-side path (e.g. "/v1/nodes").
    pub async fn mock_json(&self, method: &str, path: &str, status: u16, body: serde_json::Value) {
        wiremock::Mock::given(wiremock::matchers::method(method))
            .and(wiremock::matchers::path(path))
            .respond_with(wiremock::ResponseTemplate::new(status).set_body_json(body))
            .mount(&self.mock_server)
            .await;
        self.record_hive_mock(method, path).await;
    }

    /// Drive the REAL served router over HTTP
    pub async fn get(&self, path: &str) -> Resp {
        let client = reqwest::Client::new();
        let res = client
            .get(format!("http://{}{}", self.addr, path))
            .send()
            .await
            .unwrap();
        let status = res.status().as_u16();
        let headers = res.headers().clone();
        let content_type = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let set_cookie = headers
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect();
        let body = res.text().await.unwrap();
        Resp {
            status,
            body,
            content_type,
            headers,
            set_cookie,
        }
    }

    /// POST to the REAL served router over HTTP
    pub async fn post(&self, path: &str, body: serde_json::Value) -> Resp {
        let client = reqwest::Client::new();
        let res = client
            .post(format!("http://{}{}", self.addr, path))
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = res.status().as_u16();
        let headers = res.headers().clone();
        let content_type = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let set_cookie = headers
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect();
        let body = res.text().await.unwrap();
        Resp {
            status,
            body,
            content_type,
            headers,
            set_cookie,
        }
    }

    /// Seed one task carrying a `shared_task_id` under an existing project, inserted through
    /// the deployment's own pool (migrations already applied — never hand-written DDL).
    pub async fn seed_shared_task(&self, project_id: Uuid, shared_task_id: Uuid) -> Uuid {
        let pool = &self.deployment.db().pool;
        let task_id = Uuid::new_v4();
        let create_task = CreateTask {
            project_id,
            title: format!("shared-task-{task_id}"),
            description: None,
            status: Some(TaskStatus::Todo),
            parent_task_id: None,
            image_ids: None,
            shared_task_id: Some(shared_task_id),
        };
        Task::create(pool, &create_task, task_id)
            .await
            .expect("failed to seed shared task");
        task_id
    }

    /// True when the task row still exists in the node DB.
    pub async fn task_row_exists(&self, task_id: Uuid) -> bool {
        Task::find_by_id(&self.deployment.db().pool, task_id)
            .await
            .expect("task lookup failed")
            .is_some()
    }

    /// Seed a local project plus one task per entry in `task_statuses`, inserted through the
    /// deployment's own pool (never a hand-written `CREATE TABLE` — the harness DB already has
    /// migrations applied). If `task_statuses` is non-empty, also creates a single task attempt
    /// on the first task so `last_attempt_at` is non-null.
    pub async fn seed_project(&self, name: &str, task_statuses: &[TaskStatus]) -> Uuid {
        let pool = &self.deployment.db().pool;

        let project_id = Uuid::new_v4();
        let create_project = CreateProject {
            name: name.to_string(),
            git_repo_path: format!("/tmp/seed-project-{project_id}"),
            use_existing_repo: true,
            clone_url: None,
            setup_script: None,
            dev_script: None,
            cleanup_script: None,
            copy_files: None,
        };
        Project::create(pool, &create_project, project_id)
            .await
            .expect("failed to seed project");

        let mut first_task_id: Option<Uuid> = None;
        for (i, status) in task_statuses.iter().enumerate() {
            let task_id = Uuid::new_v4();
            let create_task = CreateTask {
                project_id,
                title: format!("{name}-task-{i}"),
                description: None,
                status: Some(status.clone()),
                parent_task_id: None,
                image_ids: None,
                shared_task_id: None,
            };
            Task::create(pool, &create_task, task_id)
                .await
                .expect("failed to seed task");
            if first_task_id.is_none() {
                first_task_id = Some(task_id);
            }
        }

        if let Some(task_id) = first_task_id {
            let attempt_id = Uuid::new_v4();
            let create_attempt = CreateTaskAttempt {
                executor: BaseCodingAgent::ClaudeCode,
                base_branch: "main".to_string(),
                branch: format!("seed/{project_id}"),
                origin_node_id: None,
            };
            TaskAttempt::create(pool, &create_attempt, attempt_id, task_id)
                .await
                .expect("failed to seed task attempt");
        }

        project_id
    }

    /// Get the HTTP listener address for making requests
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Get the deployment for direct access to services (e.g., event bus, database)
    pub fn deployment(&self) -> &DeploymentImpl {
        &self.deployment
    }

    /// GET through a jar: sends the jar's `Cookie:` header and applies the response's Set-Cookie.
    pub async fn get_with(&self, path: &str, jar: &mut CookieJar) -> Resp {
        let client = reqwest::Client::new();
        let builder = attach_jar_cookie(client.get(format!("http://{}{}", self.addr, path)), jar);
        let res = builder.send().await.unwrap();
        let status = res.status().as_u16();
        let headers = res.headers().clone();
        let content_type = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let set_cookie = headers
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect::<Vec<_>>();
        jar.apply(&set_cookie);
        let body = res.text().await.unwrap();
        Resp {
            status,
            body,
            content_type,
            headers,
            set_cookie,
        }
    }

    pub async fn post_with(
        &self,
        path: &str,
        body: serde_json::Value,
        jar: &mut CookieJar,
    ) -> Resp {
        let client = reqwest::Client::new();
        let builder = attach_jar_cookie(
            client
                .post(format!("http://{}{}", self.addr, path))
                .json(&body),
            jar,
        );
        let res = builder.send().await.unwrap();
        let status = res.status().as_u16();
        let headers = res.headers().clone();
        let content_type = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let set_cookie = headers
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect::<Vec<_>>();
        jar.apply(&set_cookie);
        let body = res.text().await.unwrap();
        Resp {
            status,
            body,
            content_type,
            headers,
            set_cookie,
        }
    }

    pub async fn delete_with(&self, path: &str, jar: &mut CookieJar) -> Resp {
        let client = reqwest::Client::new();
        let builder =
            attach_jar_cookie(client.delete(format!("http://{}{}", self.addr, path)), jar);
        let res = builder.send().await.unwrap();
        let status = res.status().as_u16();
        let headers = res.headers().clone();
        let content_type = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let set_cookie = headers
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect::<Vec<_>>();
        jar.apply(&set_cookie);
        let body = res.text().await.unwrap();
        Resp {
            status,
            body,
            content_type,
            headers,
            set_cookie,
        }
    }

    /// GET with arbitrary extra headers and NO jar (anonymous or hand-built requests).
    pub async fn get_with_headers(&self, path: &str, headers: &[(&str, &str)]) -> Resp {
        let client = reqwest::Client::new();
        let mut builder = client.get(format!("http://{}{}", self.addr, path));
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let res = builder.send().await.unwrap();
        let status = res.status().as_u16();
        let headers = res.headers().clone();
        let content_type = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let set_cookie = headers
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect();
        let body = res.text().await.unwrap();
        Resp {
            status,
            body,
            content_type,
            headers,
            set_cookie,
        }
    }

    /// GET that does NOT follow redirects, so a callback's `Location` can be inspected.
    pub async fn get_no_redirect(&self, path: &str, jar: &mut CookieJar) -> Resp {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let base = if self.redirect_mock_paths.lock().await.contains(path) {
            self.mock_server.uri()
        } else {
            format!("http://{}", self.addr)
        };
        let builder = attach_jar_cookie(client.get(format!("{base}{path}")), jar);
        let res = builder.send().await.unwrap();
        let status = res.status().as_u16();
        let headers = res.headers().clone();
        let content_type = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let set_cookie = headers
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect::<Vec<_>>();
        jar.apply(&set_cookie);
        let body = res.text().await.unwrap();
        Resp {
            status,
            body,
            content_type,
            headers,
            set_cookie,
        }
    }

    /// Exact complete generated JWT for a stable access-token label.
    pub fn access_token_for_label(&self, label: &str) -> String {
        test_access_token(label)
    }

    /// Observe the exact JWT returned by the redeem mock for this app code (harness self-test only).
    pub async fn redeemed_access_token(&self, app_code: &str) -> String {
        let response = reqwest::Client::new()
            .post(format!("{}/v1/oauth/web/redeem", self.mock_server.uri()))
            .json(&serde_json::json!({
                "handoff_id": Uuid::new_v4(),
                "app_code": app_code,
                "app_verifier": "harness-observer"
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200, "redeem mock did not match app_code");
        response.json::<serde_json::Value>().await.unwrap()["access_token"]
            .as_str()
            .unwrap()
            .to_string()
    }

    /// REAL websocket handshake via `tokio_tungstenite::connect_async`.
    pub async fn ws_probe(&self, path: &str, jar: Option<&CookieJar>) -> ProtocolProbe {
        self.ws_probe_with_headers(path, jar, &[]).await
    }

    /// The same real handshake with explicit request headers.
    pub async fn ws_probe_with_headers(
        &self,
        path: &str,
        jar: Option<&CookieJar>,
        headers: &[(&str, &str)],
    ) -> ProtocolProbe {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;

        let ws_url = format!("ws://{}{}", self.addr, path);
        let mut request = ws_url.into_client_request().unwrap();

        if let Some(jar) = jar
            && let Some(cookie_value) = jar.header_value()
        {
            request
                .headers_mut()
                .insert(reqwest::header::COOKIE, cookie_value.parse().unwrap());
        }

        // Add extra headers
        for (name, value) in headers {
            request.headers_mut().insert(
                reqwest::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                value.parse().unwrap(),
            );
        }

        match tokio_tungstenite::connect_async(request).await {
            Ok((mut socket, _response)) => {
                let _ = socket.close(None).await;
                ProtocolProbe {
                    status: 101,
                    upgraded: true,
                    content_type: None,
                }
            }
            Err(tokio_tungstenite::tungstenite::Error::Http(resp)) => {
                let status = resp.status().as_u16();
                let content_type = resp
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_owned);
                ProtocolProbe {
                    status,
                    upgraded: false,
                    content_type,
                }
            }
            Err(_) => ProtocolProbe {
                status: 0,
                upgraded: false,
                content_type: None,
            },
        }
    }

    /// REAL SSE request: GET with `Accept: text/event-stream`.
    pub async fn sse_probe(&self, path: &str, jar: Option<&CookieJar>) -> ProtocolProbe {
        let client = reqwest::Client::new();
        let mut builder = client
            .get(format!("http://{}{}", self.addr, path))
            .header(reqwest::header::ACCEPT, "text/event-stream");
        if let Some(jar) = jar {
            builder = attach_jar_cookie(builder, jar);
        }

        let res = builder.send().await.unwrap();
        let status = res.status().as_u16();
        let content_type = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Drop the response without reading the body (SSE streams are endless)
        drop(res);

        ProtocolProbe {
            status,
            upgraded: false,
            content_type,
        }
    }

    /// Mount the three hive endpoints a browser login needs.
    pub async fn mock_hive_oauth(
        &self,
        app_code: &str,
        access_token_label: &str,
        refresh_token: &str,
        subject: uuid::Uuid,
    ) -> uuid::Uuid {
        let handoff_id = uuid::Uuid::new_v4();
        let access_token = test_access_token(access_token_label);

        // Mount POST /v1/oauth/web/init — returns handoff_id
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/v1/oauth/web/init"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "handoff_id": handoff_id,
                    "authorize_url": "https://github.com/login/oauth/authorize"
                })),
            )
            .up_to_n_times(1)
            .mount(&self.mock_server)
            .await;

        // Mount POST /v1/oauth/web/redeem — returns tokens
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/v1/oauth/web/redeem"))
            .and(wiremock::matchers::body_partial_json(
                serde_json::json!({"app_code": app_code}),
            ))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "access_token": access_token,
                    "refresh_token": refresh_token
                })),
            )
            .mount(&self.mock_server)
            .await;

        // Mount GET /v1/profile with bearer token matching
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/v1/profile"))
            .and(wiremock::matchers::header(
                "authorization",
                format!("Bearer {}", access_token),
            ))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(test_profile(subject)))
            .mount(&self.mock_server)
            .await;

        for (method, path) in [
            ("POST", "/v1/oauth/web/init"),
            ("POST", "/v1/oauth/web/redeem"),
            ("GET", "/v1/profile"),
        ] {
            self.record_hive_mock(method, path).await;
        }

        handoff_id
    }

    /// Replace the profile mock for `access_token_label` with a priority-1 responder.
    pub async fn delay_hive_profile(
        &self,
        access_token_label: &str,
        subject: uuid::Uuid,
        delay: std::time::Duration,
    ) -> tokio::sync::oneshot::Receiver<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let access_token = test_access_token(access_token_label);
        let signal = std::sync::Mutex::new(Some(tx));

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/v1/profile"))
            .and(wiremock::matchers::header(
                "authorization",
                format!("Bearer {}", access_token),
            ))
            .respond_with(move |_: &wiremock::Request| {
                if let Some(tx) = signal.lock().unwrap().take() {
                    let _ = tx.send(());
                }
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(test_profile(subject))
                    .set_delay(delay)
            })
            .with_priority(1)
            .mount(&self.mock_server)
            .await;
        self.record_hive_mock("GET", "/v1/profile").await;

        rx
    }

    /// Test-only observation: derives the same JWT from `label`, requests `/v1/profile` with it, and
    /// returns the matched subject.
    pub async fn profile_subject_for(&self, label: &str) -> uuid::Uuid {
        let jwt = test_access_token(label);
        let client = reqwest::Client::new();
        let url = format!("{}/v1/profile", self.mock_server.uri());
        let res = client
            .get(&url)
            .header("authorization", format!("Bearer {}", jwt))
            .send()
            .await
            .unwrap();

        let profile: utils::api::oauth::ProfileResponse = res.json().await.unwrap();
        profile.user_id
    }

    /// Build a validator-enabled node harness.
    pub async fn configured_with_node_auth(secret: &str, expected_node_id: uuid::Uuid) -> Self {
        Self::configured_inner(Some((secret.to_string(), expected_node_id))).await
    }

    /// True when a mock is mounted for this method+path.
    pub async fn hive_mock_registered(&self, method: &str, path: &str) -> bool {
        self.mounted_hive_mocks
            .lock()
            .await
            .contains(&(method.to_ascii_uppercase(), path.to_string()))
    }

    /// Mount a priority-1 exact method+path override returning `status`.
    pub async fn mock_hive_failure(
        &self,
        method: &str,
        path: &str,
        status: u16,
    ) -> tokio::sync::oneshot::Receiver<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let signal = std::sync::Mutex::new(Some(tx));

        wiremock::Mock::given(wiremock::matchers::method(method))
            .and(wiremock::matchers::path(path))
            .respond_with(move |_: &wiremock::Request| {
                if let Some(tx) = signal.lock().unwrap().take() {
                    let _ = tx.send(());
                }
                wiremock::ResponseTemplate::new(status)
            })
            .with_priority(1)
            .mount(&self.mock_server)
            .await;
        self.record_hive_mock(method, path).await;

        rx
    }

    /// Priority-1 exact override whose `RespondErr` signals then returns `std::io::ErrorKind::ConnectionReset`.
    pub async fn mock_hive_connection_reset(
        &self,
        method: &str,
        path: &str,
    ) -> tokio::sync::oneshot::Receiver<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let signal = std::sync::Mutex::new(Some(tx));

        wiremock::Mock::given(wiremock::matchers::method(method))
            .and(wiremock::matchers::path(path))
            .respond_with_err(move |_: &wiremock::Request| {
                if let Some(tx) = signal.lock().unwrap().take() {
                    let _ = tx.send(());
                }
                std::io::Error::new(std::io::ErrorKind::ConnectionReset, "connection reset")
            })
            .with_priority(1)
            .mount(&self.mock_server)
            .await;
        self.record_hive_mock(method, path).await;

        rx
    }

    /// Priority-1 exact override whose `Respond` signals then returns a ResponseTemplate with a long delay.
    pub async fn mock_hive_delayed(
        &self,
        method: &str,
        path: &str,
    ) -> tokio::sync::oneshot::Receiver<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let signal = std::sync::Mutex::new(Some(tx));

        wiremock::Mock::given(wiremock::matchers::method(method))
            .and(wiremock::matchers::path(path))
            .respond_with(move |_: &wiremock::Request| {
                if let Some(tx) = signal.lock().unwrap().take() {
                    let _ = tx.send(());
                }
                wiremock::ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(60))
            })
            .with_priority(1)
            .mount(&self.mock_server)
            .await;
        self.record_hive_mock(method, path).await;

        rx
    }

    /// Priority-1 override that signals on arrival, then answers `body` after `delay_ms`.
    pub async fn mock_hive_delayed_json(
        &self,
        method: &str,
        path: &str,
        delay_ms: u64,
        body: serde_json::Value,
    ) -> tokio::sync::oneshot::Receiver<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let signal = std::sync::Mutex::new(Some(tx));

        wiremock::Mock::given(wiremock::matchers::method(method))
            .and(wiremock::matchers::path(path))
            .respond_with(move |_: &wiremock::Request| {
                if let Some(tx) = signal.lock().unwrap().take() {
                    let _ = tx.send(());
                }
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(body.clone())
                    .set_delay(std::time::Duration::from_millis(delay_ms))
            })
            .with_priority(1)
            .mount(&self.mock_server)
            .await;
        self.record_hive_mock(method, path).await;

        rx
    }

    /// Count recorded requests matching BOTH exact HTTP method and path.
    pub async fn hive_request_count(&self, method: &str, path: &str) -> usize {
        self.mock_server
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .filter(|r| r.method.as_str() == method && r.url.path() == path)
            .count()
    }

    /// Mount a mock_redirect for testing Location and Set-Cookie preservation.
    pub async fn mock_redirect(&self, from_path: &str, to_path: &str, set_cookie: &[&str]) {
        let mut template = wiremock::ResponseTemplate::new(302).append_header("Location", to_path);
        for cookie in set_cookie {
            template = template.append_header("Set-Cookie", *cookie);
        }

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(from_path))
            .respond_with(template)
            .mount(&self.mock_server)
            .await;
        self.redirect_mock_paths
            .lock()
            .await
            .insert(from_path.to_string());
    }

    /// Rebuild the deployment and served router on the SAME temp dir.
    pub async fn restart(mut self) -> Self {
        let sender = self
            .shutdown_sender
            .take()
            .expect("running harness must retain its shutdown sender");
        assert!(sender.send(()).is_ok(), "old server stopped before restart");

        let handle = self
            .serve_handle
            .take()
            .expect("running harness must retain its serve JoinHandle");
        let completed_generation = {
            handle.await.expect("old server task panicked");
            self.current_server_generation
        };
        self.last_completed_server_generation = Some(completed_generation);

        // The old serve task is complete. Quiesce the old generation's owned background
        // tasks through their owned handles BEFORE dropping the old deployment, so no
        // old-generation hive traffic can overlap the replacement's requests.
        if let Some(handle) = self.deployment.share_sync_handle().lock().await.take() {
            handle.shutdown().await;
        }
        self.deployment.shutdown_node_cache_sync().await;

        let HiveHarness {
            temp_dir,
            mock_server,
            deployment: old_deployment,
            current_server_generation,
            last_completed_server_generation,
            mounted_hive_mocks,
            redirect_mock_paths,
            retained_asset_dir,
            retained_database_path,
            retained_shared_api_base,
            retained_node_auth,
            ..
        } = self;
        drop(old_deployment);

        restore_harness_env(
            &retained_asset_dir,
            &retained_database_path,
            retained_shared_api_base.as_deref(),
            retained_node_auth.as_ref(),
        );

        let deployment = LocalDeployment::new().await.unwrap();
        if let Some(node_auth) = &retained_node_auth {
            let context = deployment
                .node_runner_context()
                .expect("node-auth harness must start the node runner");
            context.state.write().await.node_id = Some(node_auth.expected_node_id);
        }

        // Serve the new router
        let (shutdown_sender, shutdown_receiver) = tokio::sync::oneshot::channel();
        let app = server::routes::router(deployment.clone()).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let serve_handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_receiver.await;
                })
                .await
                .unwrap();
        });

        HiveHarness {
            temp_dir,
            mock_server,
            deployment,
            addr,
            shutdown_sender: Some(shutdown_sender),
            serve_handle: Some(serve_handle),
            current_server_generation: current_server_generation + 1,
            last_completed_server_generation,
            mounted_hive_mocks,
            redirect_mock_paths,
            retained_asset_dir,
            retained_database_path,
            retained_shared_api_base,
            retained_node_auth,
        }
    }

    /// Monotonic test-harness server generation; starts at 1.
    pub fn server_generation(&self) -> u64 {
        self.current_server_generation
    }

    /// Set to the old generation only AFTER its axum serve JoinHandle has completed.
    pub fn last_completed_server_generation(&self) -> Option<u64> {
        self.last_completed_server_generation
    }

    /// The raw sqlite pool.
    pub fn pool(&self) -> &sqlx::SqlitePool {
        &self.deployment.db().pool
    }

    /// Path of the credentials file inside the harness temp dir.
    pub fn credentials_path(&self) -> std::path::PathBuf {
        self.temp_dir.path().join("credentials.json")
    }

    /// Replace this harness's credential file with refresh-token-only credentials so the next
    /// `RemoteClient::access_token()` call must traverse the real `/v1/tokens/refresh` path.
    /// Takes and awaits BOTH owned background handles (share sync, then node-cache sync)
    /// before saving, so only an explicit caller can issue the next observed request.
    pub async fn write_refresh_only_credentials(&self, refresh_token: &str) {
        if let Some(handle) = self.deployment.share_sync_handle().lock().await.take() {
            handle.shutdown().await;
        }
        self.deployment.shutdown_node_cache_sync().await;
        let creds = services::services::oauth_credentials::Credentials {
            access_token: None,
            refresh_token: refresh_token.to_string(),
            expires_at: None,
        };
        self.deployment
            .auth_context()
            .save_credentials(&creds)
            .await
            .expect("refresh-only credentials must persist");
    }

    /// A jar holding a REAL live browser session.
    pub async fn authorized_jar(&self) -> CookieJar {
        let raw = server::auth::seams::OsTokenSource.generate_token();
        let hashed = server::auth::seams::hash_token(&raw);
        let now_millis = chrono::Utc::now().timestamp_millis();

        db::models::browser_auth::create_session(
            self.pool(),
            Uuid::new_v4(),
            &hashed,
            Uuid::new_v4(),
            now_millis,
        )
        .await
        .unwrap();

        let mut jar = CookieJar::new();
        jar.insert("vks_browser_session", &raw);
        jar
    }

    async fn record_hive_mock(&self, method: &str, path: &str) {
        self.mounted_hive_mocks
            .lock()
            .await
            .insert((method.to_ascii_uppercase(), path.to_string()));
    }
}

/// The access token MUST be a real JWT with a future `exp`: RemoteClient calls
/// utils::jwt::extract_expiration() on it, which uses jsonwebtoken::dangerous::insecure_decode.
/// A plain string yields RemoteClientError::Token -> HTTP 502, not 200. The signature is NOT
/// verified, so any secret works, but the JWT structure and the `exp` claim are mandatory.
/// Mirrors crates/utils/src/jwt.rs:34-48.
fn test_access_token(label: &str) -> String {
    #[derive(serde::Serialize)]
    struct Claims {
        exp: i64,
        test_label: String,
    }

    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &Claims {
            exp: 4_102_444_800i64,
            test_label: label.to_string(),
        },
        &jsonwebtoken::EncodingKey::from_secret(b"test-secret"),
    )
    .expect("failed to encode test JWT")
}

fn attach_jar_cookie(builder: reqwest::RequestBuilder, jar: &CookieJar) -> reqwest::RequestBuilder {
    match jar.header_value() {
        Some(cookie_header) => builder.header(reqwest::header::COOKIE, cookie_header),
        None => builder,
    }
}

fn restore_harness_env(
    asset_dir: &std::path::Path,
    database_path: &std::path::Path,
    shared_api_base: Option<&str>,
    node_auth: Option<&RetainedNodeAuth>,
) {
    unsafe {
        std::env::set_var("VK_ASSET_DIR", asset_dir);
        std::env::set_var("VK_DATABASE_PATH", database_path);
        std::env::set_var("DISABLE_WORKTREE_ORPHAN_CLEANUP", "1");
        std::env::set_var("DISABLE_WORKTREE_EXPIRED_CLEANUP", "1");
        match shared_api_base {
            Some(url) => std::env::set_var("VK_SHARED_API_BASE", url),
            None => std::env::remove_var("VK_SHARED_API_BASE"),
        }
        match node_auth {
            Some(auth) => {
                std::env::set_var("VK_HIVE_URL", &auth.hive_url);
                std::env::set_var("VK_NODE_API_KEY", &auth.api_key);
                std::env::set_var("VK_CONNECTION_TOKEN_SECRET", &auth.secret);
            }
            None => {
                std::env::remove_var("VK_HIVE_URL");
                std::env::remove_var("VK_NODE_API_KEY");
                std::env::remove_var("VK_CONNECTION_TOKEN_SECRET");
            }
        }
    }
}

fn test_profile(subject: Uuid) -> utils::api::oauth::ProfileResponse {
    utils::api::oauth::ProfileResponse {
        user_id: subject,
        username: Some("harness-user".to_string()),
        email: "harness@example.com".to_string(),
        providers: Vec::new(),
    }
}
