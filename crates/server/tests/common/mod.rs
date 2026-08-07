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
}

pub struct Resp {
    pub status: u16,
    pub body: String,
    #[allow(dead_code)]
    pub content_type: Option<String>,
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
        let access_token = test_access_token();
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
        let app = server::routes::router(deployment.clone()).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        HiveHarness {
            temp_dir,
            mock_server,
            deployment,
            addr,
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
        let app = server::routes::router(deployment.clone()).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        HiveHarness {
            temp_dir,
            mock_server,
            deployment,
            addr,
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
        let content_type = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = res.text().await.unwrap();
        Resp {
            status,
            body,
            content_type,
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
        let content_type = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = res.text().await.unwrap();
        Resp {
            status,
            body,
            content_type,
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
        let content_type = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = res.text().await.unwrap();
        Resp {
            status,
            body,
            content_type,
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
}

/// The access token MUST be a real JWT with a future `exp`: RemoteClient calls
/// utils::jwt::extract_expiration() on it, which uses jsonwebtoken::dangerous::insecure_decode.
/// A plain string yields RemoteClientError::Token -> HTTP 502, not 200. The signature is NOT
/// verified, so any secret works, but the JWT structure and the `exp` claim are mandatory.
/// Mirrors crates/utils/src/jwt.rs:34-48.
fn test_access_token() -> String {
    #[derive(serde::Serialize)]
    struct Claims {
        exp: usize,
    }

    let exp = (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize;
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &Claims { exp },
        &jsonwebtoken::EncodingKey::from_secret(b"test-secret"),
    )
    .expect("failed to encode test JWT")
}
