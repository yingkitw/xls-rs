//! Workflow capability

use crate::capabilities::{Capability, CapabilityMetadata};
use crate::workflow::{WorkflowExecutor, WorkflowConfig};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::sync::Arc;
use tempfile::Builder;

pub struct WorkflowCapability {
    executor: Arc<WorkflowExecutor>,
}

impl WorkflowCapability {
    pub fn new() -> Self {
        Self {
            executor: Arc::new(WorkflowExecutor::new()),
        }
    }
}

impl Capability for WorkflowCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "execute_workflow".to_string(),
            description: "Execute a data processing workflow from a JSON plan".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "workflow": { 
                        "type": "object", 
                        "description": "Workflow configuration object (JSON)",
                        "properties": {
                            "name": { "type": "string" },
                            "description": { "type": "string" },
                            "steps": { 
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "operation": { "type": "string" },
                                        "input": { "type": "string" },
                                        "output": { "type": "string" },
                                        "args": { "type": "object" }
                                    },
                                    "required": ["operation"]
                                }
                            }
                        },
                        "required": ["name", "steps"]
                    }
                },
                "required": ["workflow"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let workflow_json = args.get("workflow").context("Missing workflow object")?;
        
        // Validate workflow structure
        let config: WorkflowConfig = serde_json::from_value(workflow_json.clone())
            .context("Invalid workflow configuration format")?;
            
        // Write to temp file because WorkflowExecutor currently expects a file path
        // TODO: Refactor WorkflowExecutor to accept config object directly
        let temp_file = Builder::new()
            .suffix(".json")
            .tempfile()?;
        let temp_path = temp_file.path().to_str().unwrap();
        
        std::fs::write(temp_path, serde_json::to_string(&config)?)?;
        
        self.executor.execute(temp_path)?;
        
        Ok(json!({
            "status": "success",
            "message": format!("Executed workflow '{}' with {} steps", config.name, config.steps.len())
        }))
    }
}
