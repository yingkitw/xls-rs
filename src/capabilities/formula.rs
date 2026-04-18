//! Formula capabilities

use crate::capabilities::{Capability, CapabilityMetadata};
use crate::csv_handler::CellRange;
use crate::formula::FormulaEvaluator;
use anyhow::{Context, Result};
use serde_json::{json, Value};

pub struct ApplyFormulaCapability;

impl Capability for ApplyFormulaCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "apply_formula".to_string(),
            description: "Apply a formula to a cell or range in a spreadsheet".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "Input file path" },
                    "output": { "type": "string", "description": "Output file path" },
                    "formula": { "type": "string", "description": "Formula to apply (e.g., '=A1*2' or '=SUM(A1:A10)')" },
                    "cell": { "type": "string", "description": "Target cell (e.g., B1)" },
                    "range": { "type": "string", "description": "Target range (e.g., B1:B10) - if specified, applies formula to each cell in range" },
                    "sheet": { "type": "string", "description": "Sheet name for Excel files" }
                },
                "required": ["input", "output", "formula"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let input = args["input"].as_str().context("Missing input")?;
        let output = args["output"].as_str().context("Missing output")?;
        let formula = args["formula"].as_str().context("Missing formula")?;
        let cell = args["cell"].as_str();
        let range = args["range"].as_str();
        let sheet = args["sheet"].as_str();

        if cell.is_none() && range.is_none() {
            return Err(anyhow::anyhow!("Either 'cell' or 'range' must be specified"));
        }

        let evaluator = FormulaEvaluator::new();

        let cells_affected = if let Some(range_str) = range {
            let cell_range = CellRange::parse(range_str).context("Invalid range format")?;
            evaluator.apply_to_range(input, output, formula, &cell_range, sheet)?
        } else {
            let cell_ref = cell.context("Missing cell reference")?;
            evaluator.apply_to_excel(input, output, formula, cell_ref, sheet)?;
            1
        };

        Ok(json!({
            "status": "success",
            "message": format!("Applied formula to {} cell(s)", cells_affected),
            "cells_affected": cells_affected
        }))
    }
}
