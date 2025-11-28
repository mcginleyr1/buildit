//! Core domain types and traits for BuildIt CI/CD platform.
//!
//! This crate contains:
//! - Resource identifiers and common types
//! - Executor trait and job types
//! - Deployer trait and deployment types
//! - Pipeline and stage definitions
//! - Repository and stack types
//! - Application types (GitOps)
//! - Storage abstractions (artifacts, secrets)

pub mod application;
pub mod artifact;
pub mod deployer;
pub mod error;
pub mod executor;
pub mod id;
pub mod pipeline;
pub mod repository;
pub mod secret;
pub mod stack;

pub use error::{Error, Result};
pub use id::ResourceId;
