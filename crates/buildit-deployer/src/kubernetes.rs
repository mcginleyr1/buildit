//! Kubernetes deployer implementation.

use async_trait::async_trait;
use buildit_core::Result;
use buildit_core::deployer::*;
use buildit_core::executor::{LogLine, TerminalSession};
use futures::stream::BoxStream;
use kube::Client;

/// Kubernetes-based deployer.
pub struct KubernetesDeployer {
    #[allow(dead_code)]
    client: Client,
    #[allow(dead_code)]
    namespace: String,
}

impl KubernetesDeployer {
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
impl Deployer for KubernetesDeployer {
    fn name(&self) -> &'static str {
        "kubernetes"
    }

    fn supported_strategies(&self) -> Vec<DeploymentStrategy> {
        vec![
            DeploymentStrategy::RollingUpdate {
                max_surge: 1,
                max_unavailable: 0,
            },
            DeploymentStrategy::Recreate,
        ]
    }

    async fn validate(&self, _spec: &DeploymentSpec) -> Result<Vec<ValidationWarning>> {
        // TODO: Validate the deployment spec
        Ok(vec![])
    }

    async fn deploy(&self, _spec: DeploymentSpec) -> Result<DeploymentHandle> {
        // TODO: Create/update Kubernetes Deployment
        todo!("implement kubernetes deployment")
    }

    async fn state(&self, _handle: &DeploymentHandle) -> Result<DeploymentState> {
        // TODO: Get deployment state
        todo!("implement state retrieval")
    }

    async fn events(
        &self,
        _handle: &DeploymentHandle,
    ) -> Result<BoxStream<'static, DeploymentEvent>> {
        // TODO: Watch deployment events
        todo!("implement event streaming")
    }

    async fn rollback(
        &self,
        _handle: &DeploymentHandle,
        _target: RollbackTarget,
    ) -> Result<DeploymentHandle> {
        // TODO: Rollback deployment
        todo!("implement rollback")
    }

    async fn scale(&self, _handle: &DeploymentHandle, _replicas: u32) -> Result<()> {
        // TODO: Scale deployment
        todo!("implement scale")
    }

    async fn pause(&self, _handle: &DeploymentHandle) -> Result<()> {
        // TODO: Pause rollout
        todo!("implement pause")
    }

    async fn resume(&self, _handle: &DeploymentHandle) -> Result<()> {
        // TODO: Resume rollout
        todo!("implement resume")
    }

    async fn destroy(&self, _handle: &DeploymentHandle) -> Result<()> {
        // TODO: Delete deployment
        todo!("implement destroy")
    }

    async fn logs(
        &self,
        _handle: &DeploymentHandle,
        _opts: LogOptions,
    ) -> Result<BoxStream<'static, LogLine>> {
        // TODO: Stream pod logs
        todo!("implement log streaming")
    }

    async fn exec(
        &self,
        _handle: &DeploymentHandle,
        _instance: Option<String>,
        _cmd: Vec<String>,
    ) -> Result<TerminalSession> {
        // TODO: Exec into pod
        todo!("implement exec")
    }
}
