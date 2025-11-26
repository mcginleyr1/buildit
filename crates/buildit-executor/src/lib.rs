//! Job execution backends for BuildIt CI/CD.
//!
//! Provides executor implementations for running CI jobs:
//! - Kubernetes (production)
//! - Local Docker (development)

pub mod kubernetes;

pub use buildit_core::executor::{
    Executor, JobHandle, JobResult, JobSpec, JobStatus, LogLine, LogStream, TerminalSession,
};
