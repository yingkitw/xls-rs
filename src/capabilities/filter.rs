//! Filter capability

use crate::capabilities::{Capability, CapabilityMetadata};
use crate::converter::Converter;
use crate::operations::DataOperations;
// use crate::traits::FilterCondition;
use anyhow::{Context, Result};
use serde_json::{json, Value};

pub struct FilterCapability;

impl Capability for FilterCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "filter".to_string(),
            description: "Filter rows based on a condition".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "Input file path" },
                    "output": { "type": "string", "description": "Output file path" },
                    "column": { "type": "string", "description": "Column to filter on" },
                    "operator": { 
                        "type": "string", 
                        "description": "Operator: =, !=, >, >=, <, <=, contains, starts_with, ends_with, regex" 
                    },
                    "value": { "type": "string", "description": "Value to compare against" }
                },
                "required": ["input", "output", "column", "operator", "value"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let input = args["input"].as_str().context("Missing input")?;
        let output = args["output"].as_str().context("Missing output")?;
        let column = args["column"].as_str().context("Missing column")?;
        let operator = args["operator"].as_str().context("Missing operator")?;
        let value = args["value"].as_str().context("Missing value")?;

        let converter = Converter::new();
        let data = converter.read_any_data(input, None)?;

        // Find column index
        let header = &data[0];
        let col_idx = header.iter().position(|h| h == column)
            .or_else(|| column.parse::<usize>().ok())
            .context(format!("Column '{}' not found", column))?;

        let ops = DataOperations::new();

        if data.len() <= 1 {
            converter.write_any_data(output, &data, None)?;
            return Ok(json!({
                "status": "success",
                "message": format!("Filtered data where {} {} '{}' and wrote to '{}'. Rows: {}", column, operator, value, output, data.len())
            }));
        }

        let header = data[0].clone();
        let filtered_body = ops.filter_rows(&data[1..], col_idx, operator, value)?;
        let mut filtered = Vec::with_capacity(1 + filtered_body.len());
        filtered.push(header);
        filtered.extend(filtered_body);

        converter.write_any_data(output, &filtered, None)?;

        Ok(json!({
            "status": "success",
            "message": format!("Filtered data where {} {} '{}' and wrote to '{}'. Rows: {}", column, operator, value, output, filtered.len())
        }))
    }
}
