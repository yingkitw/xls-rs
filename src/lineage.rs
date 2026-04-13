//! Data lineage tracking
//!
//! Tracks data transformations and operations for audit and debugging.

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Lineage node representing a data operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub id: String,
    pub operation: String,
    pub input_files: Vec<String>,
    pub output_files: Vec<String>,
    pub timestamp: String,
    pub parameters: HashMap<String, String>,
    pub parent_nodes: Vec<String>,
}

/// Data lineage tracker
pub struct LineageTracker {
    nodes: Vec<LineageNode>,
    node_map: HashMap<String, usize>,
}

impl LineageTracker {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            node_map: HashMap::new(),
        }
    }

    /// Record an operation
    pub fn record_operation(
        &mut self,
        operation: &str,
        input_files: Vec<String>,
        output_files: Vec<String>,
        parameters: HashMap<String, String>,
    ) -> String {
        let id = format!("op_{}", self.nodes.len());
        let node = LineageNode {
            id: id.clone(),
            operation: operation.to_string(),
            input_files,
            output_files,
            timestamp: Utc::now().to_rfc3339(),
            parameters,
            parent_nodes: Vec::new(),
        };

        self.node_map.insert(id.clone(), self.nodes.len());
        self.nodes.push(node);

        id
    }

    /// Get lineage for a file
    pub fn get_lineage(&self, file: &str) -> Vec<&LineageNode> {
        self.nodes
            .iter()
            .filter(|node| {
                node.input_files.contains(&file.to_string())
                    || node.output_files.contains(&file.to_string())
            })
            .collect()
    }

    /// Export lineage as JSON
    pub fn export_json(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.nodes)
            .map_err(|e| anyhow::anyhow!("Failed to serialize lineage: {}", e))
    }

    /// Export lineage as graph (DOT format)
    pub fn export_dot(&self) -> String {
        let mut dot = String::from("digraph lineage {\n");

        for node in &self.nodes {
            dot.push_str(&format!(
                "  \"{}\" [label=\"{}\"];\n",
                node.id, node.operation
            ));
            for input in &node.input_files {
                dot.push_str(&format!("  \"{}\" -> \"{}\";\n", input, node.id));
            }
            for output in &node.output_files {
                dot.push_str(&format!("  \"{}\" -> \"{}\";\n", node.id, output));
            }
        }

        dot.push_str("}\n");
        dot
    }
}
