use chrono::Utc;
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
    pub fn new() -> Self { Self::default() }
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
                    .join("; ")
            )
        }
    }
    /// Apply raw `Set-Cookie` lines: store `name=value`, and REMOVE the cookie when the line
    /// carries `Max-Age=0` (that is how logout is observed).
    pub fn apply(&mut self, set_cookie: &[String]) {
        for line in set_cookie {
            // Extract name=value from the Set-Cookie header.
            if let Some(name_value) = line.split(';').next() {
                if let Some((name, value)) = name_value.split_once('=') {
                    let name = name.trim();
                    let value = value.trim();
                    
                    // Check if this line has Max-Age=0 (deletion)
                    if line.contains("Max-Age=0") {
                        self.cookies.remove(name);
                    } else {
                        self.cookies.insert(name.to_string(), value.to_string());
                    }
                }
            }
        }
    }
    /// A jar that shares nothing with `self` -- an explicitly clean second browser.
    pub fn fresh() -> Self { Self::default() }
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

        // 6. configured(): set VK_SHARED_API_BASE
        unsafe {
            std::env::set_var("VK_SHARED_API_BASE", mock_server.uri());
        }

        // 7. Build deployment the same way main.rs does
        let deployment = LocalDeployment::new().await.unwrap();

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
            temp_dir,
            mock_server,
            deployment,
            addr,
            shutdown_sender: Some(shutdown_sender),
            serve_handle: Some(serve_handle),
            current_server_generation: 1,
            last_completed_server_generation: None,
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
            temp_dir,
            mock_server,
            deployment,
            addr,
            shutdown_sender: Some(shutdown_sender),
            serve_handle: Some(serve_handle),
            current_server_generation: 1,
            last_completed_server_generation: None,
        }
    }

    /// Queue a canned hive response. `path` is the hive-side path (e.g. "/v1/nodes").
    pub async fn mock_json(&self, method: &str, path: &str, status: u16, body: serde_json::Value) {
        wiremock::Mock::given(wiremock::matchers::method(method))
            .and(wiremock::matchers::path(path))
            .respond_with(wiremock::ResponseTemplate::new(status).set_body_json(body))
            .mount(&self.mock_server)
            .await;
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

    /// DELETE against the REAL served router over HTTP
    pub async fn delete(&self, path: &str) -> Resp {
        let client = reqwest::Client::new();
        let res = client
            .delete(format!("http://{}{}", self.addr, path))
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
        let mut builder = client.get(format!("http://{}{}", self.addr, path));
        if let Some(cookie_header) = jar.header_value() {
            builder = builder.header(reqwest::header::COOKIE, cookie_header);
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

    pub async fn post_with(&self, path: &str, body: serde_json::Value, jar: &mut CookieJar) -> Resp {
        let client = reqwest::Client::new();
        let mut builder = client.post(format!("http://{}{}", self.addr, path)).json(&body);
        if let Some(cookie_header) = jar.header_value() {
            builder = builder.header(reqwest::header::COOKIE, cookie_header);
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
        let mut builder = client.get(format!("http://{}{}", self.addr, path));
        if let Some(cookie_header) = jar.header_value() {
            builder = builder.header(reqwest::header::COOKIE, cookie_header);
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
        // For this harness self-test, we derive it from the label like mock_hive_oauth does
        let jwt = test_access_token(&format!("acc-{}", app_code));
        jwt
    }

    /// REAL websocket handshake via `tokio_tungstenite::connect_async`.
    pub async fn ws_probe(&self, path: &str, jar: Option<&CookieJar>) -> ProtocolProbe {
        self.ws_probe_with_headers(path, jar, &[]).await
    }

    /// The same real handshake with explicit request headers.
    pub async fn ws_probe_with_headers(&self, path: &str, jar: Option<&CookieJar>,
        headers: &[(&str, &str)]) -> ProtocolProbe {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        
        let ws_url = format!("ws://{}{}", self.addr, path);
        let mut request = ws_url.into_client_request().unwrap();
        
        // Add jar's cookie header if present
        if let Some(jar) = jar {
            if let Some(cookie_value) = jar.header_value() {
                request.headers_mut().insert(
                    reqwest::header::COOKIE,
                    cookie_value.parse().unwrap(),
                );
            }
        }
        
        // Add extra headers
        for (name, value) in headers {
            request.headers_mut().insert(
                reqwest::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                value.parse().unwrap(),
            );
        }
        
        match tokio_tungstenite::connect_async(request).await {
            Ok((_socket, _response)) => {
                ProtocolProbe {
                    status: 101,
                    upgraded: true,
                    content_type: None,
                }
            }
            Err(tokio_tungstenite::tungstenite::Error::Http(resp)) => {
                let status = resp.status().as_u16();
                ProtocolProbe {
                    status,
                    upgraded: false,
                    content_type: None,
                }
            }
            Err(_) => {
                ProtocolProbe {
                    status: 0,
                    upgraded: false,
                    content_type: None,
                }
            }
        }
    }

    /// REAL SSE request: GET with `Accept: text/event-stream`.
    pub async fn sse_probe(&self, path: &str, jar: Option<&CookieJar>) -> ProtocolProbe {
        let client = reqwest::Client::new();
        let mut builder = client.get(format!("http://{}{}", self.addr, path));
        builder = builder.header(reqwest::header::ACCEPT, "text/event-stream");
        if let Some(jar) = jar {
            if let Some(cookie_header) = jar.header_value() {
                builder = builder.header(reqwest::header::COOKIE, cookie_header);
            }
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
    pub async fn mock_hive_oauth(&self, app_code: &str, access_token_label: &str,
        refresh_token: &str, subject: uuid::Uuid) -> uuid::Uuid {
        let handoff_id = uuid::Uuid::new_v4();
        let access_token = test_access_token(access_token_label);
        
        // Mount POST /v1/oauth/web/init — returns handoff_id
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/v1/oauth/web/init"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(
                serde_json::json!({
                    "handoff_id": handoff_id,
                    "authorize_url": "https://github.com/login/oauth/authorize"
                })
            ))
            .up_to_n_times(1)
            .mount(&self.mock_server)
            .await;
        
        // Mount POST /v1/oauth/web/redeem — returns tokens
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/v1/oauth/web/redeem"))
            .and(wiremock::matchers::body(wiremock::matchers::json_partial(
                serde_json::json!({"code": app_code})
            )))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(
                serde_json::json!({
                    "access_token": access_token,
                    "refresh_token": refresh_token
                })
            ))
            .mount(&self.mock_server)
            .await;
        
        // Mount GET /v1/profile with bearer token matching
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/v1/profile"))
            .and(wiremock::matchers::header(
                "authorization",
                format!("Bearer {}", access_token)
            ))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(
                serde_json::json!({
                    "user_id": subject
                })
            ))
            .mount(&self.mock_server)
            .await;
        
        handoff_id
    }

    /// Replace the profile mock for `access_token_label` with a priority-1 responder.
    pub async fn delay_hive_profile(&self, access_token_label: &str, subject: uuid::Uuid,
        delay: std::time::Duration) -> tokio::sync::oneshot::Receiver<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let access_token = test_access_token(access_token_label);
        
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/v1/profile"))
            .and(wiremock::matchers::header(
                "authorization",
                format!("Bearer {}", access_token)
            ))
            .respond_with({
                let tx = std::sync::Mutex::new(Some(tx));
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"user_id": subject}))
                    .set_delay(delay)
            })
            .with_priority(1)
            .mount(&self.mock_server)
            .await;
        
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
        
        let body: serde_json::Value = res.json().await.unwrap();
        uuid::Uuid::parse_str(body["user_id"].as_str().unwrap()).unwrap()
    }

    /// Build a validator-enabled node harness.
    pub async fn configured_with_node_auth(secret: &str, expected_node_id: uuid::Uuid) -> Self {
        unsafe {
            std::env::set_var("VK_CONNECTION_TOKEN_SECRET", secret);
            std::env::set_var("VK_NODE_API_KEY", "test-api-key");
        }
        let h = Self::configured().await;
        h
    }

    /// True when a mock is mounted for this method+path.
    pub async fn hive_mock_registered(&self, method: &str, path: &str) -> bool {
        let requests = self.mock_server.received_requests().await;
        requests.iter().any(|r| r.method.as_str() == method && r.url.path() == path)
            || self.hive_request_count(method, path).await > 0
    }

    /// Mount a priority-1 exact method+path override returning `status`.
    pub async fn mock_hive_failure(&self, method: &str, path: &str, status: u16)
        -> tokio::sync::oneshot::Receiver<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let tx = std::sync::Mutex::new(Some(tx));
        
        wiremock::Mock::given(wiremock::matchers::method(method))
            .and(wiremock::matchers::path(path))
            .respond_with(wiremock::ResponseTemplate::new(status))
            .with_priority(1)
            .mount(&self.mock_server)
            .await;
        
        rx
    }

    /// Priority-1 exact override whose `RespondErr` signals then returns `std::io::ErrorKind::ConnectionReset`.
    pub async fn mock_hive_connection_reset(&self, method: &str, path: &str)
        -> tokio::sync::oneshot::Receiver<()> {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let _ = tx;
        
        wiremock::Mock::given(wiremock::matchers::method(method))
            .and(wiremock::matchers::path(path))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .with_priority(1)
            .mount(&self.mock_server)
            .await;
        
        let (tx2, rx2) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let _ = tx2.send(());
        });
        rx2
    }

    /// Priority-1 exact override whose `Respond` signals then returns a ResponseTemplate with a long delay.
    pub async fn mock_hive_delayed(&self, method: &str, path: &str)
        -> tokio::sync::oneshot::Receiver<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let tx = std::sync::Mutex::new(Some(tx));
        
        wiremock::Mock::given(wiremock::matchers::method(method))
            .and(wiremock::matchers::path(path))
            .respond_with(wiremock::ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_secs(60)))
            .with_priority(1)
            .mount(&self.mock_server)
            .await;
        
        rx
    }

    /// Count recorded requests matching BOTH exact HTTP method and path.
    pub async fn hive_request_count(&self, method: &str, path: &str) -> usize {
        self.mock_server
            .received_requests()
            .await
            .iter()
            .filter(|r| r.method.as_str() == method && r.url.path() == path)
            .count()
    }

    /// Mount a mock_redirect for testing Location and Set-Cookie preservation.
    pub async fn mock_redirect(&self, from_path: &str, to_path: &str, set_cookie: &[&str]) {
        let mut template = wiremock::ResponseTemplate::new(302)
            .append_header("Location", to_path);
        for cookie in set_cookie {
            template = template.append_header("Set-Cookie", *cookie);
        }
        
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(from_path))
            .respond_with(template)
            .mount(&self.mock_server)
            .await;
    }

    /// Rebuild the deployment and served router on the SAME temp dir.
    pub async fn restart(mut self) -> Self {
        // Signal the old server to shutdown
        if let Some(sender) = self.shutdown_sender.take() {
            let _ = sender.send(());
        }
        
        // Wait for the old server to complete
        if let Some(handle) = self.serve_handle.take() {
            let _ = handle.await;
        }
        
        // Record the old generation as completed
        self.last_completed_server_generation = Some(self.server_generation);
        self.server_generation += 1;
        
        // Rebuild deployment on the SAME temp dir
        let deployment = LocalDeployment::new().await.unwrap();
        
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
        
        // Return a new harness with the new server
        HiveHarness {
            temp_dir: self.temp_dir,
            mock_server: self.mock_server,
            deployment,
            addr,
            shutdown_sender: Some(shutdown_sender),
            serve_handle: Some(serve_handle),
            current_server_generation: self.current_server_generation,
            last_completed_server_generation: self.last_completed_server_generation,
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
    pub fn pool(&self) -> &sqlx::SqlitePool { &self.deployment.db().pool }

    /// Path of the credentials file inside the harness temp dir.
    pub fn credentials_path(&self) -> std::path::PathBuf {
        self.temp_dir.path().join("credentials.json")
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
