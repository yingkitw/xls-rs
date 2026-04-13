//! Workflow orchestration
//!
//! Provides pipeline execution capabilities for chaining multiple operations.

use crate::handler_registry::HandlerRegistry;
use crate::operations::DataOperations;
use crate::traits::DataWriteOptions;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

/// Workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub operation: String,
    pub input: Option<String>,
    pub output: Option<String>,
    pub args: Option<serde_json::Value>,
}

/// Workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<WorkflowStep>,
}

/// Workflow executor
pub struct WorkflowExecutor {
    registry: HandlerRegistry,
}

impl WorkflowExecutor {
    pub fn new() -> Self {
        Self {
            registry: HandlerRegistry::new(),
        }
    }

    /// Execute workflow from config file
    pub fn execute(&self, config_path: &str) -> Result<()> {
        let config_str = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read workflow config: {}", config_path))?;

        let config: WorkflowConfig = toml::from_str(&config_str)
            .or_else(|_| serde_json::from_str(&config_str))
            .with_context(|| "Failed to parse workflow config. Expected TOML or JSON")?;

        println!("Executing workflow: {}", config.name);

        let mut current_data: Option<Vec<Vec<String>>> = None;

        for (step_idx, step) in config.steps.iter().enumerate() {
            println!("Step {}: {}", step_idx + 1, step.operation);

            // Get input data
            let input_data = if let Some(ref input) = step.input {
                self.registry.read(input)?
            } else if let Some(ref data) = current_data {
                data.clone()
            } else {
                anyhow::bail!("No input data available for step {}", step_idx + 1);
            };

            // Execute operation
            let output_data =
                self.execute_step(&step.operation, &input_data, step.args.as_ref())?;

            // Save output if specified
            if let Some(ref output) = step.output {
                let options = DataWriteOptions::default();
                self.registry.write(output, &output_data, options)?;
                println!("  Output saved to: {}", output);
            }

            current_data = Some(output_data);
        }

        Ok(())
    }

    fn execute_step(
        &self,
        operation: &str,
        data: &[Vec<String>],
        args: Option<&serde_json::Value>,
    ) -> Result<Vec<Vec<String>>> {
        let mut result = data.to_vec();
        let ops = DataOperations::new();

        match operation {
            "read" => Ok(data.to_vec()),

            "filter" => {
                if let Some(args) = args {
                    if let Some(column_idx) = args.get("column").and_then(|v| v.as_u64()) {
                        if let Some(where_clause) = args.get("where").and_then(|v| v.as_str()) {
                            result = ops.filter_rows(&result, column_idx as usize, where_clause, "")?;
                        }
                    }
                }
                Ok(result)
            }

            "sort" => {
                if let Some(args) = args {
                    if let Some(column_idx) = args.get("column").and_then(|v| v.as_u64()) {
                        let ascending = args.get("ascending")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);

                        use crate::operations::types::SortOrder;
                        let order = if ascending { SortOrder::Ascending } else { SortOrder::Descending };
                        ops.sort_by_column(&mut result, column_idx as usize, order)?;
                    }
                }
                Ok(result)
            }

            "transform" => {
                if let Some(args) = args {
                    if let Some(op_type) = args.get("operation").and_then(|v| v.as_str()) {
                        match op_type {
                            "replace" => {
                                if let Some(find) = args.get("find").and_then(|v| v.as_str()) {
                                    if let Some(replace) = args.get("replace").and_then(|v| v.as_str()) {
                                        if let Some(column_idx) = args.get("column").and_then(|v| v.as_u64()) {
                                            let _count = ops.replace(&mut result, column_idx as usize, find, replace);
                                            println!("  Replaced '{}' with '{}' in column {}", find, replace, column_idx);
                                        }
                                    }
                                }
                            }
                            "dedupe" => {
                                let count = ops.deduplicate_mut(&mut result);
                                println!("  Removed {} duplicate rows", count);
                            }
                            "transpose" => {
                                result = ops.transpose(&result);
                            }
                            "fillna" => {
                                if let Some(value) = args.get("value").and_then(|v| v.as_str()) {
                                    ops.fillna(&mut result, value);
                                }
                            }
                            "dropna" => {
                                result = ops.dropna(&result);
                            }
                            _ => anyhow::bail!("Unknown transform operation: {}", op_type),
                        }
                    }
                }
                Ok(result)
            }

            "mutate" => {
                if let Some(args) = args {
                    if let Some(_column) = args.get("column").and_then(|v| v.as_str()) {
                        if let Some(_formula) = args.get("formula").and_then(|v| v.as_str()) {
                            // For now, just add a placeholder column
                            // Full formula evaluation with mutate is complex
                            for row in &mut result {
                                row.push("MUTATED".to_string());
                            }
                        }
                    }
                }
                Ok(result)
            }

            "select" => {
                if let Some(args) = args {
                    if let Some(columns) = args.get("columns").and_then(|v| v.as_array()) {
                        let column_names: Vec<&str> = columns
                            .iter()
                            .filter_map(|v| v.as_str())
                            .collect();

                        result = ops.select_columns_by_name(&result, &column_names)?;
                    }
                }
                Ok(result)
            }

            "describe" => {
                let desc = ops.describe(&result)?;
                println!("  Statistics: {:?}", desc);
                Ok(desc)
            }

            _ => anyhow::bail!("Unknown operation: {}", operation),
        }
    }
}

use anyhow::Context;
