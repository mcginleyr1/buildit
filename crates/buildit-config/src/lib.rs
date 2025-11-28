//! KDL configuration parsing for BuildIt CI/CD.
//!
//! This crate handles parsing of:
//! - Pipeline definitions (buildit.kdl)
//! - System configuration
//! - Variable interpolation

pub mod error;
pub mod pipeline;
pub mod system;
pub mod variables;

pub use error::{ConfigError, ConfigResult};
pub use variables::{
    GitContext, PipelineContext, RunContext, StageContext, VariableContext, VariableContextBuilder,
};
