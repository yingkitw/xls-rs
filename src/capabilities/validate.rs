//! Validate capability

use crate::capabilities::{Capability, CapabilityMetadata};
use crate::converter::Converter;
use crate::validation::{DataValidator, ValidationConfig};
use anyhow::{Context, Result};
use serde_json::{json, Value};

pub struct ValidateCapability;

impl Capability for ValidateCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "validate".to_string(),
            description: "Validate data against a set of rules".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "Input file path" },
                    "rules": { "type": "string", "description": "Path to JSON rules file, or 'auto' for default rules" },
                    "output": { "type": "string", "description": "Optional path to save validation result JSON" },
                    "report": { "type": "string", "description": "Optional path to save validation report" }
                },
                "required": ["input"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let input = args["input"].as_str().context("Missing input")?;
        let rules = args["rules"].as_str().unwrap_or("auto");
        let output = args["output"].as_str();
        let report = args["report"].as_str();

        let converter = Converter::new();
        let data = converter.read_any_data(input, None)?;

        let validator = if rules.ends_with(".json") {
            DataValidator::from_config_file(rules)?
        } else {
            DataValidator::new(ValidationConfig::default())
        };

        let result = validator.validate(&data)?;

        if let Some(output_path) = output {
            validator.save_result(&result, output_path)?;
        }

        if let Some(report_path) = report {
            let report_text = validator.generate_report(&result);
            std::fs::write(report_path, report_text)
                .context(format!("Failed to write report to {report_path}"))?;
        }

        Ok(json!({
            "status": if result.is_valid { "passed" } else { "failed" },
            "total_rows": result.stats.total_rows,
            "valid_rows": result.stats.valid_rows,
            "invalid_rows": result.stats.invalid_rows,
            "total_errors": result.stats.total_errors,
            "total_warnings": result.stats.total_warnings,
            "errors": result.errors.iter().take(20).map(|e| json!({
                "row": e.row,
                "column": e.column,
                "value": e.value,
                "message": e.message
            })).collect::<Vec<_>>(),
        }))
    }
}
