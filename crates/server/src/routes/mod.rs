use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{IntoMakeService, any, get},
};

use crate::DeploymentImpl;

pub mod all_tasks;
pub mod approvals;
pub mod backups;
pub mod breakdown;
pub mod browser_auth;
pub mod config;
pub mod containers;
pub mod dashboard;
pub mod database;
pub mod diagnostics;
pub mod filesystem;
// pub mod github;
pub mod drafts;
pub mod events;
pub mod execution_processes;
pub mod frontend;
pub mod health;
pub mod images;
pub mod labels;
pub mod logs;
pub mod message_queue;
pub mod nodes;
pub mod oauth;
pub mod organizations;
pub mod processes;
pub mod projects;
pub mod swarm_labels;
pub mod swarm_projects;
pub mod swarm_templates;
pub mod task_attempts;
pub mod task_variables;
pub mod tasks;
pub mod templates;
pub mod terminal;
pub mod webhooks;

pub async fn router(deployment: DeploymentImpl) -> IntoMakeService<Router> {
    // Create terminal router with its own state
    let terminal_router = terminal::router_with_state(&deployment).await;

    // Deny-by-default (D1): every route lives in exactly one of these two subtrees, and anything
    // added to `protected_routes` in future inherits authorization without opting in.
    let public_routes = Router::new()
        .route("/health", get(health::health_check))
        .merge(oauth::public_router())
        .merge(browser_auth::public_router());

    let protected_routes = Router::new()
        .merge(config::router())
        .merge(containers::router(&deployment))
        .merge(dashboard::router(&deployment))
        .merge(projects::router(&deployment))
        .merge(drafts::router(&deployment))
        .merge(tasks::router(&deployment))
        .merge(breakdown::router(&deployment))
        .merge(all_tasks::router(&deployment))
        .merge(task_attempts::router(&deployment))
        .merge(execution_processes::router(&deployment))
        .merge(processes::router(&deployment))
        .merge(templates::router(&deployment))
        .merge(labels::router(&deployment))
        .merge(task_variables::router(&deployment))
        .merge(oauth::protected_router())
        .merge(organizations::router())
        .merge(nodes::router())
        .merge(swarm_projects::router())
        .merge(swarm_labels::router())
        .merge(swarm_templates::router())
        .merge(filesystem::router())
        .merge(events::router(&deployment))
        .merge(approvals::router())
        .merge(backups::router())
        .merge(database::router())
        .merge(diagnostics::router(&deployment))
        .merge(logs::router(&deployment))
        .merge(message_queue::router(&deployment))
        .merge(webhooks::router(&deployment))
        .merge(terminal_router)
        .nest("/images", images::routes())
        .layer(from_fn_with_state(
            deployment.clone(),
            crate::auth::session::require_browser_session,
        ));

    // The three direct stream routes (live logs, raw logs, attempt-id diff) accept
    // EITHER a live browser session OR a strictly scoped Hive `connection` token.
    // The middleware sits OUTSIDE each direct router's resource loader so missing,
    // malformed, wrong-audience and wrong-resource credentials return 401 before
    // any lookup or protocol upgrade.
    let connection_stream_routes = Router::new()
        .merge(logs::direct_router())
        .merge(execution_processes::direct_router(&deployment))
        .merge(task_attempts::direct_router(&deployment))
        .layer(from_fn_with_state(
            deployment.clone(),
            crate::auth::node_token::require_session_or_connection_token,
        ));

    let base_routes = public_routes
        .merge(protected_routes)
        .merge(connection_stream_routes)
        // An unknown `/api/*` request must terminate INSIDE the API boundary and never reach the
        // outer `/{*path}` SPA catch-all. axum 0.8's `nest` files a nested custom `fallback`
        // under the PARENT's fallback router, which the outer `/{*path}` real route shadows, so
        // the JSON 404 is ALSO registered as a catch-all route inside the nest. See
        // Resp::is_spa_fallback in crates/server/tests/common/mod.rs.
        .route("/{*path}", any(api_not_found))
        .fallback(api_not_found)
        .with_state(deployment);

    Router::new()
        .nest("/api", base_routes)
        .route("/", get(frontend::serve_frontend_root))
        .route("/{*path}", get(frontend::serve_frontend))
        .into_make_service()
}

/// 404 for any unmatched path under `/api`, as JSON, so the SPA catch-all can never answer for
/// an API call.
async fn api_not_found() -> impl axum::response::IntoResponse {
    (
        axum::http::StatusCode::NOT_FOUND,
        axum::Json(serde_json::json!({"success": false, "message": "unknown api route"})),
    )
}
