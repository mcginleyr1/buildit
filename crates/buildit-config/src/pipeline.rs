//! Pipeline configuration parsing.

use crate::{ConfigError, ConfigResult};

/// Parse a pipeline configuration from KDL text.
pub fn parse_pipeline(_kdl: &str) -> ConfigResult<buildit_core::pipeline::Pipeline> {
    // TODO: Implement KDL parsing
    Err(ConfigError::MissingField("not yet implemented".to_string()))
}
