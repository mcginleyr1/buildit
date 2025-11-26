//! BuildIt API Server

use buildit_api::{AppState, routes};
use buildit_db::create_pool;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://buildit:buildit-dev-password@127.0.0.1:5432/buildit".to_string()
    });

    // Create database pool
    info!("Connecting to database...");
    let pool = create_pool(&database_url).await?;
    info!("Database connected");

    // Create app state
    let state = AppState::new(pool);

    // Build router
    let app = routes::router(state)
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    // Start server
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting server on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
