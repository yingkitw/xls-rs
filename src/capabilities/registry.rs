//! Registry for managing capabilities

use crate::capabilities::core::{Capability, CapabilityMetadata};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Registry for capabilities
#[derive(Clone)]
pub struct CapabilityRegistry {
    capabilities: Arc<RwLock<HashMap<String, Arc<dyn Capability>>>>,
}

impl CapabilityRegistry {
    /// Create a new capability registry
    pub fn new() -> Self {
        Self {
            capabilities: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a capability
    pub fn register(&self, capability: Arc<dyn Capability>) {
        let metadata = capability.metadata();
        let mut caps = self.capabilities.write().unwrap();
        caps.insert(metadata.name.clone(), capability);
    }

    /// Get a capability by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Capability>> {
        let caps = self.capabilities.read().unwrap();
        caps.get(name).cloned()
    }

    /// List all capabilities
    pub fn list(&self) -> Vec<CapabilityMetadata> {
        let caps = self.capabilities.read().unwrap();
        let mut list: Vec<_> = caps.values().map(|c| c.metadata()).collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }
    
    /// Execute a capability by name
    pub fn execute(&self, name: &str, args: serde_json::Value) -> Result<serde_json::Value> {
        let cap = self.get(name).ok_or_else(|| anyhow::anyhow!("Capability not found: {}", name))?;
        cap.execute(args)
    }
}
