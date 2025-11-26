//! Core domain types and traits for BuildIt CI/CD platform.
//!
//! This crate contains:
//! - Resource identifiers and common types
//! - Executor trait and job types
//! - Deployer trait and deployment types
//! - Pipeline and stage definitions
//! - Storage abstractions (artifacts, secrets)

pub mod artifact;
pub mod deployer;
pub mod error;
pub mod executor;
pub mod id;
pub mod pipeline;
pub mod secret;

pub use error::{Error, Result};
pub use id::ResourceId;
