//! Sort capability

use crate::capabilities::{Capability, CapabilityMetadata};
use crate::converter::Converter;
use crate::operations::{DataOperations, SortOrder};
use anyhow::{Context, Result};
use serde_json::{json, Value};

pub struct SortCapability;

impl Capability for SortCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "sort".to_string(),
            description: "Sort data by a specific column".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "Input file path" },
                    "output": { "type": "string", "description": "Output file path" },
                    "column": { "type": "string", "description": "Column name or index to sort by" },
                    "ascending": { "type": "boolean", "description": "Sort in ascending order (default: true)" }
                },
                "required": ["input", "output", "column"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let input = args["input"].as_str().context("Missing input")?;
        let output = args["output"].as_str().context("Missing output")?;
        let column = args["column"].as_str().context("Missing column")?;
        let ascending = args["ascending"].as_bool().unwrap_or(true);

        let converter = Converter::new();
        let data = converter.read_any_data(input, None)?;
        if data.is_empty() {
            anyhow::bail!("Input has no rows");
        }

        // Find column index from header; sort data rows only (keep header first).
        let header = &data[0];
        let col_idx = header.iter().position(|h| h == column)
            .or_else(|| column.parse::<usize>().ok())
            .context(format!("Column '{}' not found", column))?;

        let ops = DataOperations::new();
        let order = if ascending { SortOrder::Ascending } else { SortOrder::Descending };

        let mut body: Vec<Vec<String>> = data[1..].to_vec();
        ops.sort_by_column(&mut body, col_idx, order)?;
        let mut out = Vec::with_capacity(1 + body.len());
        out.push(data[0].clone());
        out.extend(body);

        converter.write_any_data(output, &out, None)?;

        Ok(json!({
            "status": "success",
            "message": format!("Sorted data by '{}' and wrote to '{}'", column, output)
        }))
    }
}
