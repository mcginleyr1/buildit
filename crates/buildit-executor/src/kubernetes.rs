//! Kubernetes executor implementation.

use async_trait::async_trait;
use buildit_core::Result;
use buildit_core::executor::*;
use futures::stream::BoxStream;
use kube::Client;

/// Kubernetes-based job executor.
pub struct KubernetesExecutor {
    client: Client,
    namespace: String,
}

impl KubernetesExecutor {
    pub async fn new(namespace: impl Into<String>) -> Result<Self> {
        let client = Client::try_default()
            .await
            .map_err(|e| buildit_core::Error::Internal(e.to_string()))?;
        Ok(Self {
            client,
            namespace: namespace.into(),
        })
    }

    pub fn with_client(client: Client, namespace: impl Into<String>) -> Self {
        Self {
            client,
            namespace: namespace.into(),
        }
    }
}

#[async_trait]
impl Executor for KubernetesExecutor {
    fn name(&self) -> &'static str {
        "kubernetes"
    }

    async fn can_execute(&self, _spec: &JobSpec) -> bool {
        // TODO: Check if we can schedule to the cluster
        true
    }

    async fn spawn(&self, _spec: JobSpec) -> Result<JobHandle> {
        // TODO: Create a Kubernetes Job
        todo!("implement kubernetes job creation")
    }

    async fn logs(&self, _handle: &JobHandle) -> Result<BoxStream<'static, LogLine>> {
        // TODO: Stream pod logs
        todo!("implement log streaming")
    }

    async fn status(&self, _handle: &JobHandle) -> Result<JobStatus> {
        // TODO: Get job/pod status
        todo!("implement status check")
    }

    async fn wait(&self, _handle: &JobHandle) -> Result<JobResult> {
        // TODO: Wait for job completion
        todo!("implement wait")
    }

    async fn cancel(&self, _handle: &JobHandle) -> Result<()> {
        // TODO: Delete the job
        todo!("implement cancel")
    }

    async fn exec_interactive(
        &self,
        _handle: &JobHandle,
        _cmd: Vec<String>,
    ) -> Result<TerminalSession> {
        // TODO: kubectl exec equivalent
        todo!("implement interactive exec")
    }
}
