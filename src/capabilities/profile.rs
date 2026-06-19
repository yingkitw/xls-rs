//! Profile capability

use crate::capabilities::{Capability, CapabilityMetadata};
use crate::converter::Converter;
use crate::profiling::DataProfiler;
use anyhow::{Context, Result};
use serde_json::{json, Value};

pub struct ProfileCapability;

impl Capability for ProfileCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "profile".to_string(),
            description: "Generate a data quality profile for a dataset".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "Input file path" },
                    "output": { "type": "string", "description": "Optional path to save profile JSON" }
                },
                "required": ["input"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let input = args["input"].as_str().context("Missing input")?;
        let output = args["output"].as_str();

        let converter = Converter::new();
        let data = converter.read_any_data(input, None)?;

        let profiler = DataProfiler::new();
        let profile = profiler.profile(&data, input)?;

        if let Some(output_path) = output {
            let json = serde_json::to_string_pretty(&profile)?;
            std::fs::write(output_path, json)
                .context(format!("Failed to write profile to {output_path}"))?;
        }

        Ok(json!({
            "status": "success",
            "file_path": profile.file_path,
            "total_rows": profile.total_rows,
            "total_columns": profile.total_columns,
            "total_cells": profile.total_cells,
            "null_cells": profile.null_cells,
            "null_percentage": profile.null_percentage,
            "duplicate_rows": profile.duplicate_rows,
            "duplicate_percentage": profile.duplicate_percentage,
            "data_quality_score": profile.data_quality_score,
            "columns": profile.columns.iter().map(|c| json!({
                "name": c.name,
                "data_type": format!("{:?}", c.data_type),
                "null_count": c.null_count,
                "null_percentage": c.null_percentage,
                "unique_count": c.unique_count,
                "unique_percentage": c.unique_percentage,
                "quality_score": c.quality_score,
            })).collect::<Vec<_>>(),
            "recommendations": profile.recommendations,
        }))
    }
}
