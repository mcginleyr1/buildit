//! KDL configuration parsing for BuildIt CI/CD.
//!
//! This crate handles parsing of:
//! - Pipeline definitions (buildit.kdl)
//! - System configuration

pub mod error;
pub mod pipeline;
pub mod system;

pub use error::{ConfigError, ConfigResult};
