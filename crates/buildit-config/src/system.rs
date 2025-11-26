//! System configuration parsing.

use crate::{ConfigError, ConfigResult};
use serde::{Deserialize, Serialize};

/// System-wide configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// Enable multi-tenant mode.
    pub multi_tenant: bool,
    /// Artifact store configuration.
    pub artifact_store: Option<ArtifactStoreConfig>,
    /// Secret store configuration.
    pub secret_store: Option<SecretStoreConfig>,
    /// Executor configurations.
    pub executors: Vec<ExecutorConfig>,
    /// Deployer configurations.
    pub deployers: Vec<DeployerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactStoreConfig {
    pub backend: String,
    pub bucket: Option<String>,
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretStoreConfig {
    pub backend: String,
    pub project: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    pub name: String,
    pub executor_type: String,
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployerConfig {
    pub name: String,
    pub deployer_type: String,
    pub context: Option<String>,
    pub allowed_namespaces: Vec<String>,
}

/// Parse system configuration from KDL text.
pub fn parse_system_config(_kdl: &str) -> ConfigResult<SystemConfig> {
    // TODO: Implement KDL parsing
    Err(ConfigError::MissingField("not yet implemented".to_string()))
}
