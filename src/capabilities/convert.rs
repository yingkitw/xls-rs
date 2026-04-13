//! Convert capability (library parity with CLI `convert`)

use crate::capabilities::{Capability, CapabilityMetadata};
use crate::converter::Converter;
use anyhow::{Context, Result};
use serde_json::{json, Value};

pub struct ConvertCapability;

impl Capability for ConvertCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "convert".to_string(),
            description: "Convert between supported spreadsheet formats (e.g. csv, xlsx, parquet)"
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "Input file path" },
                    "output": { "type": "string", "description": "Output file path" },
                    "sheet": { "type": "string", "description": "Optional sheet name for Excel/ODS" }
                },
                "required": ["input", "output"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let input = args["input"].as_str().context("Missing input")?;
        let output = args["output"].as_str().context("Missing output")?;
        let sheet = args["sheet"].as_str();

        let converter = Converter::new();
        converter.convert(input, output, sheet)?;

        Ok(json!({
            "status": "success",
            "message": format!("Converted '{}' to '{}'", input, output)
        }))
    }
}
