//! Worker that processes jobs from the queue.

use crate::queue::JobQueue;
use buildit_executor::Executor;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

/// A worker that claims and executes jobs.
pub struct Worker {
    id: String,
    queue: Arc<JobQueue>,
    executor: Arc<dyn Executor>,
}

impl Worker {
    pub fn new(id: impl Into<String>, queue: Arc<JobQueue>, executor: Arc<dyn Executor>) -> Self {
        Self {
            id: id.into(),
            queue,
            executor,
        }
    }

    /// Run the worker loop.
    pub async fn run(&self) {
        info!(worker_id = %self.id, "Starting worker");

        loop {
            match self.queue.claim(&self.id).await {
                Ok(Some(job)) => {
                    info!(job_id = %job.id, stage = %job.stage_name, "Claimed job");

                    // TODO: Convert QueuedJob to JobSpec and execute
                    // For now, just mark as completed
                    if let Err(e) = self.queue.complete(job.id).await {
                        warn!(job_id = %job.id, error = %e, "Failed to mark job complete");
                    }
                }
                Ok(None) => {
                    // No jobs available, wait before polling again
                    sleep(Duration::from_secs(1)).await;
                }
                Err(e) => {
                    warn!(error = %e, "Failed to claim job");
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
}
