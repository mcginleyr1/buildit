//! Resource identifiers.

use derive_more::Display;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A unique identifier for any resource in the system.
/// Uses UUIDv7 for time-ordered, sortable IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
#[display("{_0}")]
pub struct ResourceId(Uuid);

impl ResourceId {
    /// Create a new unique ResourceId using UUIDv7.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Create a ResourceId from an existing UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ResourceId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for ResourceId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<ResourceId> for Uuid {
    fn from(id: ResourceId) -> Self {
        id.0
    }
}

impl std::str::FromStr for ResourceId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}
