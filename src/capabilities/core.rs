//! Core capability definitions

use anyhow::Result;
use serde_json::Value;

/// Metadata for a capability
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CapabilityMetadata {
    pub name: String,
    pub description: String,
    pub parameters: Value, // JSON Schema
}

/// A capability that can be executed
pub trait Capability: Send + Sync {
    /// Get capability metadata
    fn metadata(&self) -> CapabilityMetadata;

    /// Execute the capability with arguments
    fn execute(&self, args: Value) -> Result<Value>;
}
