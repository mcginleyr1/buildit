//! API server for BuildIt CI/CD.
//!
//! Provides HTTP REST API and WebSocket endpoints.

pub mod error;
pub mod routes;
pub mod state;
pub mod ws;

pub use state::AppState;
