//! API routes.

pub mod health;
pub mod pipelines;
pub mod tenants;

use crate::AppState;
use axum::Router;

/// Build the main API router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/api/v1", api_router(state.clone()))
        .merge(health::router())
        .with_state(state)
}

fn api_router(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/tenants", tenants::router())
        .nest("/pipelines", pipelines::router())
        .with_state(state)
}
