//! Deployment backends for BuildIt CI/CD.
//!
//! Provides deployer implementations:
//! - Kubernetes (production)
//! - Fly.io
//! - Cloud Run (future)
//! - Lambda (future)

pub mod kubernetes;

pub use buildit_core::deployer::{
    Deployer, DeploymentHandle, DeploymentSpec, DeploymentState, DeploymentStatus,
    DeploymentStrategy, LogOptions, RollbackTarget, ValidationWarning,
};
