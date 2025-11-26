//! Health check endpoints.

use axum::Json;
use axum::Router;
use axum::routing::get;
use serde_json::{Value, json};

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/health", get(health))
        .route("/health/ready", get(ready))
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn ready() -> Json<Value> {
    // TODO: Check database connection
    Json(json!({ "status": "ready" }))
}
