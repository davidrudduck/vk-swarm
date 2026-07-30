use chrono::Utc;
use deployment::Deployment;
use local_deployment::LocalDeployment;
use server::DeploymentImpl;
use std::net::SocketAddr;

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
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
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
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
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
        let body = res.text().await.unwrap();
        Resp { status, body }
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
        let body = res.text().await.unwrap();
        Resp { status, body }
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
