//! API routes.

pub mod applications;
pub mod auth;
pub mod deployment;
pub mod health;
pub mod pipelines;
pub mod repositories;
pub mod stacks;
pub mod tenants;
pub mod ui;
pub mod webhooks;

use crate::AppState;
use crate::ws::ws_handler;
use axum::Router;
use axum::routing::get;

/// Build the main API router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(ui::router())
        .nest("/api/v1", api_router())
        .nest("/auth", auth::router())
        .nest("/webhooks", webhooks::router())
        .route("/ws", get(ws_handler))
        .merge(health::router())
        .with_state(state)
}

fn api_router() -> Router<AppState> {
    Router::new()
        .nest("/tenants", tenants::router())
        .nest("/pipelines", pipelines::router())
        .nest("/repositories", repositories::router())
        .nest("/stacks", stacks::router())
        .nest("/applications", applications::router())
        .nest("/deployment", deployment::router())
}
