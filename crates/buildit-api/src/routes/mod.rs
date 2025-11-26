//! API routes.

pub mod health;
pub mod pipelines;
pub mod tenants;
pub mod ui;

use crate::AppState;
use crate::ws::ws_handler;
use axum::Router;
use axum::routing::get;

/// Build the main API router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(ui::router())
        .nest("/api/v1", api_router(state.clone()))
        .route("/ws", get(ws_handler))
        .merge(health::router())
        .with_state(state)
}

fn api_router(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/tenants", tenants::router())
        .nest("/pipelines", pipelines::router())
        .with_state(state)
}
