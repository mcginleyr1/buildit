//! Kubernetes executor implementation.
//!
//! Runs CI jobs as Kubernetes Jobs with one-shot pods.

use async_trait::async_trait;
use buildit_core::executor::*;
use buildit_core::{Error, ResourceId, Result};
use chrono::Utc;
use futures::StreamExt;
use futures::stream::BoxStream;
use k8s_openapi::api::batch::v1::{Job, JobSpec as K8sJobSpec};
use k8s_openapi::api::core::v1::{
    Container, EnvVar, PodSpec, PodTemplateSpec, ResourceRequirements as K8sResourceRequirements,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::Client;
use kube::api::{Api, DeleteParams, LogParams, PostParams};
use kube::runtime::watcher::{Config as WatcherConfig, Event as WatcherEvent, watcher};
use std::collections::BTreeMap;
use tokio::time::{Duration, sleep};
use tracing::{debug, info, warn};

/// Kubernetes-based job executor.
///
/// Runs each job as a Kubernetes Job resource with a single pod.
/// Jobs are created in the configured namespace and labeled for easy tracking.
pub struct KubernetesExecutor {
    client: Client,
    namespace: String,
    /// Labels to apply to all jobs created by this executor
    labels: BTreeMap<String, String>,
}

impl KubernetesExecutor {
    /// Create a new KubernetesExecutor using the default kubeconfig.
    pub async fn new(namespace: impl Into<String>) -> Result<Self> {
        let client = Client::try_default()
            .await
            .map_err(|e| Error::Internal(format!("Failed to create K8s client: {}", e)))?;

        let mut labels = BTreeMap::new();
        labels.insert(
            "app.kubernetes.io/managed-by".to_string(),
            "buildit".to_string(),
        );
        labels.insert(
            "app.kubernetes.io/component".to_string(),
            "ci-job".to_string(),
        );

        Ok(Self {
            client,
            namespace: namespace.into(),
            labels,
        })
    }

    /// Create with a custom Kubernetes client.
    pub fn with_client(client: Client, namespace: impl Into<String>) -> Self {
        let mut labels = BTreeMap::new();
        labels.insert(
            "app.kubernetes.io/managed-by".to_string(),
            "buildit".to_string(),
        );
        labels.insert(
            "app.kubernetes.io/component".to_string(),
            "ci-job".to_string(),
        );

        Self {
            client,
            namespace: namespace.into(),
            labels,
        }
    }

    /// Generate a unique job name from the job ID.
    fn job_name(job_id: &ResourceId) -> String {
        // K8s names must be lowercase, alphanumeric, and max 63 chars
        format!("buildit-job-{}", job_id.to_string().to_lowercase())
    }

    /// Get the Jobs API for our namespace.
    fn jobs_api(&self) -> Api<Job> {
        Api::namespaced(self.client.clone(), &self.namespace)
    }

    /// Get the Pods API for our namespace.
    fn pods_api(&self) -> Api<k8s_openapi::api::core::v1::Pod> {
        Api::namespaced(self.client.clone(), &self.namespace)
    }

    /// Build a Kubernetes Job from our JobSpec.
    fn build_k8s_job(&self, spec: &JobSpec) -> Job {
        let job_name = Self::job_name(&spec.id);

        // Build environment variables
        let env_vars: Vec<EnvVar> = spec
            .env
            .iter()
            .map(|(k, v)| EnvVar {
                name: k.clone(),
                value: Some(v.clone()),
                value_from: None,
            })
            .collect();

        // Build resource requirements
        let mut requests = BTreeMap::new();
        let mut limits = BTreeMap::new();

        if let Some(cpu) = &spec.resources.cpu_request {
            requests.insert("cpu".to_string(), Quantity(cpu.clone()));
        }
        if let Some(mem) = &spec.resources.memory_request {
            requests.insert("memory".to_string(), Quantity(mem.clone()));
        }
        if let Some(cpu) = &spec.resources.cpu_limit {
            limits.insert("cpu".to_string(), Quantity(cpu.clone()));
        }
        if let Some(mem) = &spec.resources.memory_limit {
            limits.insert("memory".to_string(), Quantity(mem.clone()));
        }

        let resources = if requests.is_empty() && limits.is_empty() {
            None
        } else {
            Some(K8sResourceRequirements {
                requests: if requests.is_empty() {
                    None
                } else {
                    Some(requests)
                },
                limits: if limits.is_empty() {
                    None
                } else {
                    Some(limits)
                },
                ..Default::default()
            })
        };

        // Build the container
        let container = Container {
            name: "job".to_string(),
            image: Some(spec.image.clone()),
            command: if spec.command.is_empty() {
                None
            } else {
                Some(spec.command.clone())
            },
            working_dir: spec.working_dir.clone(),
            env: if env_vars.is_empty() {
                None
            } else {
                Some(env_vars)
            },
            resources,
            image_pull_policy: Some("IfNotPresent".to_string()),
            ..Default::default()
        };

        // Build labels for the job and pod
        let mut job_labels = self.labels.clone();
        job_labels.insert("buildit.io/job-id".to_string(), spec.id.to_string());

        // Build the Job
        Job {
            metadata: ObjectMeta {
                name: Some(job_name),
                namespace: Some(self.namespace.clone()),
                labels: Some(job_labels.clone()),
                ..Default::default()
            },
            spec: Some(K8sJobSpec {
                backoff_limit: Some(0),                 // Don't retry failed jobs
                ttl_seconds_after_finished: Some(3600), // Clean up after 1 hour
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(job_labels),
                        ..Default::default()
                    }),
                    spec: Some(PodSpec {
                        containers: vec![container],
                        restart_policy: Some("Never".to_string()),
                        ..Default::default()
                    }),
                },
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Find the pod created by a job.
    async fn find_job_pod(&self, job_id: &ResourceId) -> Result<Option<String>> {
        let pods_api = self.pods_api();
        let label_selector = format!("buildit.io/job-id={}", job_id);

        let pods = pods_api
            .list(&kube::api::ListParams::default().labels(&label_selector))
            .await
            .map_err(|e| Error::Internal(format!("Failed to list pods: {}", e)))?;

        Ok(pods.items.first().and_then(|p| p.metadata.name.clone()))
    }

    /// Wait for a pod to be created for the job.
    async fn wait_for_pod(&self, job_id: &ResourceId, timeout: Duration) -> Result<String> {
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err(Error::ExecutionFailed(
                    "Timeout waiting for pod to be created".to_string(),
                ));
            }

            if let Some(pod_name) = self.find_job_pod(job_id).await? {
                return Ok(pod_name);
            }

            sleep(Duration::from_millis(500)).await;
        }
    }
}

#[async_trait]
impl Executor for KubernetesExecutor {
    fn name(&self) -> &'static str {
        "kubernetes"
    }

    async fn can_execute(&self, _spec: &JobSpec) -> bool {
        // Try to connect to the cluster
        match self.client.apiserver_version().await {
            Ok(_) => true,
            Err(e) => {
                warn!(error = %e, "Cannot connect to Kubernetes cluster");
                false
            }
        }
    }

    async fn spawn(&self, spec: JobSpec) -> Result<JobHandle> {
        let jobs_api = self.jobs_api();
        let job_name = Self::job_name(&spec.id);

        info!(
            job_name = %job_name,
            image = %spec.image,
            namespace = %self.namespace,
            "Creating Kubernetes Job"
        );

        // Build the K8s Job resource
        let k8s_job = self.build_k8s_job(&spec);

        // Create the job
        let created = jobs_api
            .create(&PostParams::default(), &k8s_job)
            .await
            .map_err(|e| Error::ExecutionFailed(format!("Failed to create K8s Job: {}", e)))?;

        let uid = created
            .metadata
            .uid
            .ok_or_else(|| Error::Internal("Job created without UID".to_string()))?;

        info!(
            job_name = %job_name,
            uid = %uid,
            "Kubernetes Job created"
        );

        Ok(JobHandle {
            id: spec.id,
            executor_id: uid,
            executor_name: self.name().to_string(),
        })
    }

    async fn logs(&self, handle: &JobHandle) -> Result<BoxStream<'static, LogLine>> {
        // Wait for pod to be created (up to 60 seconds)
        let pod_name = self
            .wait_for_pod(&handle.id, Duration::from_secs(60))
            .await?;

        info!(pod = %pod_name, "Streaming logs from pod");

        // Use the non-streaming logs API in a polling loop via an async stream
        // This is simpler than dealing with AsyncBufRead conversion
        let (tx, rx) = tokio::sync::mpsc::channel::<LogLine>(100);

        let pods_api_clone = self.pods_api();
        let pod_name_clone = pod_name.clone();

        // Spawn a task to poll logs
        tokio::spawn(async move {
            let mut last_seen_lines = 0usize;

            loop {
                let log_params = LogParams {
                    follow: false,
                    timestamps: true,
                    container: Some("job".to_string()),
                    ..Default::default()
                };

                match pods_api_clone.logs(&pod_name_clone, &log_params).await {
                    Ok(logs) => {
                        let lines: Vec<&str> = logs.lines().collect();

                        // Send only new lines
                        for line in lines.iter().skip(last_seen_lines) {
                            let content = line.to_string();

                            // K8s log format: "2024-01-01T00:00:00.000000000Z message"
                            let (timestamp, message) =
                                if content.len() > 30 && content.chars().nth(4) == Some('-') {
                                    let ts_end = content.find(' ').unwrap_or(30).min(35);
                                    let ts_str = &content[..ts_end];
                                    match chrono::DateTime::parse_from_rfc3339(ts_str.trim()) {
                                        Ok(ts) => (
                                            ts.with_timezone(&Utc),
                                            content.get(ts_end + 1..).unwrap_or("").to_string(),
                                        ),
                                        Err(_) => (Utc::now(), content.clone()),
                                    }
                                } else {
                                    (Utc::now(), content.clone())
                                };

                            let log_line = LogLine {
                                timestamp,
                                stream: LogStream::Stdout,
                                content: message.trim_end().to_string(),
                            };

                            if tx.send(log_line).await.is_err() {
                                // Receiver dropped, stop polling
                                return;
                            }
                        }

                        last_seen_lines = lines.len();
                    }
                    Err(e) => {
                        // Pod might have completed or been deleted
                        debug!(error = %e, "Log polling ended");
                        break;
                    }
                }

                // Poll every 500ms
                sleep(Duration::from_millis(500)).await;
            }
        });

        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn status(&self, handle: &JobHandle) -> Result<JobStatus> {
        let jobs_api = self.jobs_api();
        let job_name = Self::job_name(&handle.id);

        let job = jobs_api
            .get(&job_name)
            .await
            .map_err(|e| Error::NotFound(format!("Job not found: {}", e)))?;

        let status = job.status.as_ref();

        // Check various conditions
        let active = status.and_then(|s| s.active).unwrap_or(0);
        let succeeded = status.and_then(|s| s.succeeded).unwrap_or(0);
        let failed = status.and_then(|s| s.failed).unwrap_or(0);

        let start_time = status.and_then(|s| s.start_time.as_ref()).map(|t| t.0);

        let completion_time = status.and_then(|s| s.completion_time.as_ref()).map(|t| t.0);

        if succeeded > 0 {
            Ok(JobStatus::Succeeded {
                started_at: start_time.unwrap_or_else(Utc::now),
                finished_at: completion_time.unwrap_or_else(Utc::now),
            })
        } else if failed > 0 {
            // Try to get failure reason from conditions
            let message = status
                .and_then(|s| s.conditions.as_ref())
                .and_then(|conditions| {
                    conditions
                        .iter()
                        .find(|c| c.type_ == "Failed")
                        .and_then(|c| c.message.clone())
                })
                .unwrap_or_else(|| "Job failed".to_string());

            Ok(JobStatus::Failed {
                started_at: start_time,
                finished_at: completion_time.unwrap_or_else(Utc::now),
                exit_code: None, // K8s Jobs don't expose exit codes directly
                message,
            })
        } else if active > 0 {
            Ok(JobStatus::Running {
                started_at: start_time.unwrap_or_else(Utc::now),
            })
        } else if start_time.is_none() {
            Ok(JobStatus::Pending)
        } else {
            // Job exists but no active/succeeded/failed - still pending/starting
            Ok(JobStatus::Running {
                started_at: start_time.unwrap_or_else(Utc::now),
            })
        }
    }

    async fn wait(&self, handle: &JobHandle) -> Result<JobResult> {
        let jobs_api = self.jobs_api();
        let job_name = Self::job_name(&handle.id);

        info!(job_name = %job_name, "Waiting for job completion");

        // Use a watcher to efficiently wait for job completion
        let config = WatcherConfig::default().fields(&format!("metadata.name={}", job_name));

        let mut stream = watcher(jobs_api, config).boxed();

        while let Some(event) = stream.next().await {
            match event {
                Ok(WatcherEvent::Apply(job)) => {
                    let status = job.status.as_ref();
                    let succeeded = status.and_then(|s| s.succeeded).unwrap_or(0);
                    let failed = status.and_then(|s| s.failed).unwrap_or(0);

                    if succeeded > 0 || failed > 0 {
                        debug!(
                            job_name = %job_name,
                            succeeded = succeeded,
                            failed = failed,
                            "Job completed"
                        );
                        break;
                    }
                }
                Ok(WatcherEvent::Delete(_)) => {
                    return Err(Error::ExecutionFailed("Job was deleted".to_string()));
                }
                Ok(WatcherEvent::Init | WatcherEvent::InitApply(_) | WatcherEvent::InitDone) => {
                    // Initial state events, ignore
                }
                Err(e) => {
                    warn!(error = %e, "Watcher error, retrying");
                    // Continue watching despite errors
                }
            }
        }

        // Get final status
        let final_status = self.status(handle).await?;

        // Try to get exit code from pod
        let exit_code = if let Some(pod_name) = self.find_job_pod(&handle.id).await? {
            let pods_api = self.pods_api();
            if let Ok(pod) = pods_api.get(&pod_name).await {
                pod.status
                    .and_then(|s| s.container_statuses)
                    .and_then(|cs| cs.first().cloned())
                    .and_then(|c| c.state)
                    .and_then(|s| s.terminated)
                    .map(|t| t.exit_code)
            } else {
                None
            }
        } else {
            None
        };

        Ok(JobResult {
            status: final_status,
            exit_code,
            artifacts: vec![], // TODO: Implement artifact collection
        })
    }

    async fn cancel(&self, handle: &JobHandle) -> Result<()> {
        let jobs_api = self.jobs_api();
        let job_name = Self::job_name(&handle.id);

        info!(job_name = %job_name, "Cancelling Kubernetes Job");

        // Delete the job with propagation policy to also delete pods
        let delete_params = DeleteParams {
            propagation_policy: Some(kube::api::PropagationPolicy::Background),
            ..Default::default()
        };

        jobs_api
            .delete(&job_name, &delete_params)
            .await
            .map_err(|e| Error::ExecutionFailed(format!("Failed to delete job: {}", e)))?;

        info!(job_name = %job_name, "Kubernetes Job cancelled");

        Ok(())
    }

    async fn exec_interactive(
        &self,
        handle: &JobHandle,
        _cmd: Vec<String>,
    ) -> Result<TerminalSession> {
        // Find the pod for this job
        let _pod_name = self
            .find_job_pod(&handle.id)
            .await?
            .ok_or_else(|| Error::NotFound("No pod found for job".to_string()))?;

        // TODO: Implement kubectl exec equivalent using kube::api::AttachParams
        // This requires more complex WebSocket handling
        Err(Error::Internal(
            "Interactive exec not yet implemented for Kubernetes".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;
    use std::collections::HashMap;

    fn make_test_spec() -> JobSpec {
        JobSpec {
            id: ResourceId::new(),
            image: "alpine:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            working_dir: Some("/workspace".to_string()),
            env: {
                let mut env = HashMap::new();
                env.insert("FOO".to_string(), "bar".to_string());
                env.insert("BAZ".to_string(), "qux".to_string());
                env
            },
            resources: ResourceRequirements {
                cpu_limit: Some("1".to_string()),
                memory_limit: Some("512Mi".to_string()),
                cpu_request: Some("100m".to_string()),
                memory_request: Some("128Mi".to_string()),
            },
            timeout: None,
            volumes: vec![],
        }
    }

    #[test]
    fn test_job_name_generation() {
        let id = ResourceId::new();
        let name = KubernetesExecutor::job_name(&id);

        assert!(name.starts_with("buildit-job-"));
        assert!(name.len() <= 63); // K8s name limit
        assert!(
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        );
    }

    #[test]
    fn test_job_name_is_deterministic() {
        let id = ResourceId::new();
        let name1 = KubernetesExecutor::job_name(&id);
        let name2 = KubernetesExecutor::job_name(&id);
        assert_eq!(name1, name2);
    }

    #[test]
    fn test_job_name_unique_per_id() {
        let id1 = ResourceId::new();
        let id2 = ResourceId::new();
        let name1 = KubernetesExecutor::job_name(&id1);
        let name2 = KubernetesExecutor::job_name(&id2);
        assert_ne!(name1, name2);
    }

    #[test]
    fn test_job_spec_validation() {
        let spec = make_test_spec();

        assert!(!spec.command.is_empty());
        assert_eq!(spec.image, "alpine:latest");
        assert_eq!(spec.working_dir, Some("/workspace".to_string()));
        assert_eq!(spec.env.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(spec.resources.cpu_limit, Some("1".to_string()));
        assert_eq!(spec.resources.memory_limit, Some("512Mi".to_string()));
    }

    #[test]
    fn test_empty_command_spec() {
        let spec = JobSpec {
            id: ResourceId::new(),
            image: "alpine:latest".to_string(),
            command: vec![],
            working_dir: None,
            env: HashMap::new(),
            resources: ResourceRequirements::default(),
            timeout: None,
            volumes: vec![],
        };

        assert!(spec.command.is_empty());
        assert!(spec.working_dir.is_none());
        assert!(spec.env.is_empty());
    }

    #[test]
    fn test_resource_requirements_default() {
        let resources = ResourceRequirements::default();

        assert!(resources.cpu_limit.is_none());
        assert!(resources.memory_limit.is_none());
        assert!(resources.cpu_request.is_none());
        assert!(resources.memory_request.is_none());
    }

    #[test]
    fn test_log_timestamp_parsing() {
        // Test the timestamp parsing logic used in logs()
        let content = "2024-01-15T10:30:45.123456789Z Hello, World!";

        let (timestamp, message) = if content.len() > 30 && content.chars().nth(4) == Some('-') {
            let ts_end = content.find(' ').unwrap_or(30).min(35);
            let ts_str = &content[..ts_end];
            match chrono::DateTime::parse_from_rfc3339(ts_str.trim()) {
                Ok(ts) => (
                    ts.with_timezone(&Utc),
                    content.get(ts_end + 1..).unwrap_or("").to_string(),
                ),
                Err(_) => (Utc::now(), content.to_string()),
            }
        } else {
            (Utc::now(), content.to_string())
        };

        assert_eq!(message, "Hello, World!");
        assert_eq!(timestamp.year(), 2024);
        assert_eq!(timestamp.month(), 1);
        assert_eq!(timestamp.day(), 15);
    }

    #[test]
    fn test_log_timestamp_parsing_no_timestamp() {
        let content = "Just a plain log line";

        let (_, message) = if content.len() > 30 && content.chars().nth(4) == Some('-') {
            let ts_end = content.find(' ').unwrap_or(30).min(35);
            let ts_str = &content[..ts_end];
            match chrono::DateTime::parse_from_rfc3339(ts_str.trim()) {
                Ok(ts) => (
                    ts.with_timezone(&Utc),
                    content.get(ts_end + 1..).unwrap_or("").to_string(),
                ),
                Err(_) => (Utc::now(), content.to_string()),
            }
        } else {
            (Utc::now(), content.to_string())
        };

        assert_eq!(message, "Just a plain log line");
    }

    #[test]
    fn test_job_handle_creation() {
        let id = ResourceId::new();
        let handle = JobHandle {
            id: id.clone(),
            executor_id: "test-uid-12345".to_string(),
            executor_name: "kubernetes".to_string(),
        };

        assert_eq!(handle.id, id);
        assert_eq!(handle.executor_id, "test-uid-12345");
        assert_eq!(handle.executor_name, "kubernetes");
    }

    #[test]
    fn test_job_status_is_terminal() {
        assert!(!JobStatus::Pending.is_terminal());
        assert!(
            !JobStatus::Running {
                started_at: Utc::now()
            }
            .is_terminal()
        );

        assert!(
            JobStatus::Succeeded {
                started_at: Utc::now(),
                finished_at: Utc::now(),
            }
            .is_terminal()
        );

        assert!(
            JobStatus::Failed {
                started_at: Some(Utc::now()),
                finished_at: Utc::now(),
                exit_code: Some(1),
                message: "test".to_string(),
            }
            .is_terminal()
        );

        assert!(
            JobStatus::Cancelled {
                started_at: Some(Utc::now()),
                cancelled_at: Utc::now(),
            }
            .is_terminal()
        );
    }
}

/// Integration tests that require a running Kubernetes cluster.
/// Run with: cargo test --features integration -- --ignored
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::collections::HashMap;

    /// Test that we can create an executor when K8s is available.
    /// This test is ignored by default as it requires a K8s cluster.
    #[tokio::test]
    #[ignore]
    async fn test_executor_creation() {
        let executor = KubernetesExecutor::new("default").await;
        assert!(executor.is_ok(), "Should connect to K8s cluster");

        let executor = executor.unwrap();
        assert_eq!(executor.name(), "kubernetes");
    }

    /// Test can_execute returns true when connected to cluster.
    #[tokio::test]
    #[ignore]
    async fn test_can_execute() {
        let executor = KubernetesExecutor::new("default").await.unwrap();

        let spec = JobSpec {
            id: ResourceId::new(),
            image: "alpine:latest".to_string(),
            command: vec!["echo".to_string(), "test".to_string()],
            working_dir: None,
            env: HashMap::new(),
            resources: ResourceRequirements::default(),
            timeout: None,
            volumes: vec![],
        };

        let can_execute = executor.can_execute(&spec).await;
        assert!(can_execute, "Should be able to execute when connected");
    }

    /// Test full job lifecycle: spawn, wait, check status.
    /// This test creates a real K8s Job.
    #[tokio::test]
    #[ignore]
    async fn test_job_lifecycle() {
        let executor = KubernetesExecutor::new("default").await.unwrap();

        let spec = JobSpec {
            id: ResourceId::new(),
            image: "alpine:latest".to_string(),
            command: vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "echo 'Hello from K8s!'".to_string(),
            ],
            working_dir: None,
            env: {
                let mut env = HashMap::new();
                env.insert("TEST_VAR".to_string(), "test_value".to_string());
                env
            },
            resources: ResourceRequirements {
                cpu_request: Some("50m".to_string()),
                memory_request: Some("32Mi".to_string()),
                cpu_limit: Some("100m".to_string()),
                memory_limit: Some("64Mi".to_string()),
            },
            timeout: None,
            volumes: vec![],
        };

        // Spawn the job
        let handle = executor.spawn(spec).await.expect("Should spawn job");
        assert_eq!(handle.executor_name, "kubernetes");

        // Wait for completion
        let result = executor.wait(&handle).await.expect("Should wait for job");

        // Check it succeeded
        match result.status {
            JobStatus::Succeeded { .. } => {}
            other => panic!("Expected Succeeded, got {:?}", other),
        }

        // Cleanup
        let _ = executor.cancel(&handle).await;
    }

    /// Test that a failing job reports failure correctly.
    #[tokio::test]
    #[ignore]
    async fn test_failing_job() {
        let executor = KubernetesExecutor::new("default").await.unwrap();

        let spec = JobSpec {
            id: ResourceId::new(),
            image: "alpine:latest".to_string(),
            command: vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "exit 1".to_string(),
            ],
            working_dir: None,
            env: HashMap::new(),
            resources: ResourceRequirements::default(),
            timeout: None,
            volumes: vec![],
        };

        let handle = executor.spawn(spec).await.expect("Should spawn job");
        let result = executor.wait(&handle).await.expect("Should wait for job");

        match result.status {
            JobStatus::Failed { .. } => {}
            other => panic!("Expected Failed, got {:?}", other),
        }

        // Cleanup
        let _ = executor.cancel(&handle).await;
    }

    /// Test job cancellation.
    #[tokio::test]
    #[ignore]
    async fn test_job_cancellation() {
        let executor = KubernetesExecutor::new("default").await.unwrap();

        let spec = JobSpec {
            id: ResourceId::new(),
            image: "alpine:latest".to_string(),
            command: vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "sleep 300".to_string(), // Long running job
            ],
            working_dir: None,
            env: HashMap::new(),
            resources: ResourceRequirements::default(),
            timeout: None,
            volumes: vec![],
        };

        let handle = executor.spawn(spec).await.expect("Should spawn job");

        // Give it a moment to start
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Cancel it
        executor.cancel(&handle).await.expect("Should cancel job");

        // Verify it's gone (status should error)
        let status_result = executor.status(&handle).await;
        assert!(status_result.is_err(), "Job should be deleted");
    }

    /// Test log streaming from a job.
    #[tokio::test]
    #[ignore]
    async fn test_log_streaming() {
        let executor = KubernetesExecutor::new("default").await.unwrap();

        let spec = JobSpec {
            id: ResourceId::new(),
            image: "alpine:latest".to_string(),
            command: vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "echo 'line1'; echo 'line2'; echo 'line3'".to_string(),
            ],
            working_dir: None,
            env: HashMap::new(),
            resources: ResourceRequirements::default(),
            timeout: None,
            volumes: vec![],
        };

        let handle = executor.spawn(spec).await.expect("Should spawn job");

        // Get logs
        let mut log_stream = executor.logs(&handle).await.expect("Should get logs");

        // Collect some logs
        let mut logs = Vec::new();
        while let Some(log_line) = log_stream.next().await {
            logs.push(log_line.content);
            if logs.len() >= 3 {
                break;
            }
        }

        // Wait for job to complete
        let _ = executor.wait(&handle).await;

        // Verify we got expected output
        assert!(logs.iter().any(|l| l.contains("line1")));
        assert!(logs.iter().any(|l| l.contains("line2")));
        assert!(logs.iter().any(|l| l.contains("line3")));

        // Cleanup
        let _ = executor.cancel(&handle).await;
    }
}
