//! Job scheduling for BuildIt CI/CD.
//!
//! Manages the job queue and dispatches work to executors.
//! Uses PostgreSQL with SKIP LOCKED for distributed job claiming.

pub mod queue;
pub mod worker;

pub use queue::JobQueue;
pub use worker::Worker;
