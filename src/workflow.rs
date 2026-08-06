//! Workflow orchestration
//!
//! Provides pipeline execution capabilities for chaining multiple operations.

use crate::handler_registry::HandlerRegistry;
use crate::operations::{DataOperations, SortOrder};
use crate::traits::DataWriteOptions;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

/// Workflow operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowOperation {
    Read,
    Filter {
        column: usize,
        where_clause: String,
    },
    Sort {
        column: usize,
        ascending: bool,
    },
    Transform {
        operation: TransformOp,
    },
    Mutate {
        column: String,
        formula: String,
    },
    Select {
        columns: Vec<String>,
    },
    Describe,
}

/// Transform operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransformOp {
    Replace {
        find: String,
        replace: String,
        column: usize,
    },
    Dedupe,
    Transpose,
    Fillna {
        value: String,
    },
    Dropna,
}

impl WorkflowOperation {
    /// Parse from string (for backward compatibility with TOML/JSON configs)
    pub fn from_str_with_args(operation: &str, args: Option<&serde_json::Value>) -> Result<Self> {
        match operation {
            "read" => Ok(WorkflowOperation::Read),
            "filter" => {
                let args = args.context("Filter requires args")?;
                let column = args.get("column").and_then(|v| v.as_u64()).context("Missing column")? as usize;
                let where_clause = args.get("where").and_then(|v| v.as_str()).context("Missing where clause")?.to_string();
                Ok(WorkflowOperation::Filter { column, where_clause })
            }
            "sort" => {
                let args = args.context("Sort requires args")?;
                let column = args.get("column").and_then(|v| v.as_u64()).context("Missing column")? as usize;
                let ascending = args.get("ascending").and_then(|v| v.as_bool()).unwrap_or(true);
                Ok(WorkflowOperation::Sort { column, ascending })
            }
            "transform" => {
                let args = args.context("Transform requires args")?;
                let op_type = args.get("operation").and_then(|v| v.as_str()).context("Missing operation type")?;
                let transform_op = match op_type {
                    "replace" => {
                        let find = args.get("find").and_then(|v| v.as_str()).context("Missing find")?.to_string();
                        let replace = args.get("replace").and_then(|v| v.as_str()).context("Missing replace")?.to_string();
                        let column = args.get("column").and_then(|v| v.as_u64()).context("Missing column")? as usize;
                        TransformOp::Replace { find, replace, column }
                    }
                    "dedupe" => TransformOp::Dedupe,
                    "transpose" => TransformOp::Transpose,
                    "fillna" => {
                        let value = args.get("value").and_then(|v| v.as_str()).context("Missing value")?.to_string();
                        TransformOp::Fillna { value }
                    }
                    "dropna" => TransformOp::Dropna,
                    _ => anyhow::bail!("Unknown transform operation: {}", op_type),
                };
                Ok(WorkflowOperation::Transform { operation: transform_op })
            }
            "mutate" => {
                let args = args.context("Mutate requires args")?;
                let column = args.get("column").and_then(|v| v.as_str()).context("Missing column")?.to_string();
                let formula = args.get("formula").and_then(|v| v.as_str()).context("Missing formula")?.to_string();
                Ok(WorkflowOperation::Mutate { column, formula })
            }
            "select" => {
                let args = args.context("Select requires args")?;
                let columns = args.get("columns").and_then(|v| v.as_array())
                    .context("Missing columns array")?
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
                Ok(WorkflowOperation::Select { columns })
            }
            "describe" => Ok(WorkflowOperation::Describe),
            _ => anyhow::bail!("Unknown operation: {}", operation),
        }
    }
}

/// Workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub operation: String,
    pub input: Option<String>,
    pub output: Option<String>,
    pub args: Option<serde_json::Value>,
}

impl WorkflowStep {
    /// Convert to typed operation (for internal use)
    pub fn to_operation(&self) -> Result<WorkflowOperation> {
        WorkflowOperation::from_str_with_args(&self.operation, self.args.as_ref())
    }
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

impl Default for WorkflowExecutor {
    fn default() -> Self {
        Self::new()
    }
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

        self.execute_config(&config)
    }

    /// Execute workflow from an in-memory configuration (same semantics as [`Self::execute`]).
    pub fn execute_config(&self, config: &WorkflowConfig) -> Result<()> {
        println!("Executing workflow: {}", config.name);

        let mut current_data: Option<Vec<Vec<String>>> = None;

        for (step_idx, step) in config.steps.iter().enumerate() {
            println!("Step {}: {}", step_idx + 1, step.operation);

            // Get input data — take ownership of prior step output when possible
            // to avoid retaining two full grid copies across the step boundary.
            let input_data = if let Some(ref input) = step.input {
                self.registry.read(input)?
            } else if let Some(data) = current_data.take() {
                data
            } else {
                anyhow::bail!("No input data available for step {}", step_idx + 1);
            };

            // Execute operation
            let output_data =
                self.execute_step(&step.operation, &input_data, step.args.as_ref())?;

            // Save output if specified
            if let Some(ref output) = step.output {
                let mut options = DataWriteOptions::default();
                let out = output.to_lowercase();
                if out.ends_with(".parquet") || out.ends_with(".avro") {
                    options.include_headers = true;
                }
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

        // Parse operation to enum for type-safe pattern matching
        let op = WorkflowOperation::from_str_with_args(operation, args)?;

        match op {
            WorkflowOperation::Read => Ok(data.to_vec()),

            WorkflowOperation::Filter { column, where_clause } => {
                result = ops.filter_rows(&result, column, &where_clause, "")?;
                Ok(result)
            }

            WorkflowOperation::Sort { column, ascending } => {
                let order = if ascending { SortOrder::Ascending } else { SortOrder::Descending };
                ops.sort_by_column(&mut result, column, order)?;
                Ok(result)
            }

            WorkflowOperation::Transform { operation: transform_op } => {
                match transform_op {
                    TransformOp::Replace { find, replace, column } => {
                        let _count = ops.replace(&mut result, column, &find, &replace);
                        println!("  Replaced '{}' with '{}' in column {}", find, replace, column);
                    }
                    TransformOp::Dedupe => {
                        let count = ops.deduplicate_mut(&mut result);
                        println!("  Removed {} duplicate rows", count);
                    }
                    TransformOp::Transpose => {
                        result = ops.transpose(&result);
                    }
                    TransformOp::Fillna { value } => {
                        ops.fillna(&mut result, &value);
                    }
                    TransformOp::Dropna => {
                        result = ops.dropna(&result);
                    }
                }
                Ok(result)
            }

            WorkflowOperation::Mutate { column, formula } => {
                ops.mutate(&mut result, &column, &formula)?;
                Ok(result)
            }

            WorkflowOperation::Select { columns } => {
                let column_names: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();
                result = ops.select_columns_by_name(&result, &column_names)?;
                Ok(result)
            }

            WorkflowOperation::Describe => {
                let desc = ops.describe(&result)?;
                println!("  Statistics: {:?}", desc);
                Ok(desc)
            }
        }
    }
}
