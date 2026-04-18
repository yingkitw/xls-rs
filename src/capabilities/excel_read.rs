//! Excel read capabilities

use crate::capabilities::{Capability, CapabilityMetadata};
use crate::csv_handler::CellRange;
use crate::excel::ExcelHandler;
use anyhow::{Context, Result};
use serde_json::{json, Value};

pub struct ListSheetsCapability;

impl Capability for ListSheetsCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "list_sheets".to_string(),
            description: "List all sheet names in an Excel workbook".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "Input file path (.xlsx, .xls, .ods)" }
                },
                "required": ["input"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let input = args["input"].as_str().context("Missing input")?;

        let handler = ExcelHandler::new();
        let sheets = handler.list_sheets(input)?;

        Ok(json!({
            "status": "success",
            "sheets": sheets
        }))
    }
}

pub struct ReadExcelCapability;

impl Capability for ReadExcelCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "read_excel".to_string(),
            description: "Read data from an Excel file with optional sheet and range selection".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "Input file path (.xlsx, .xls, .ods)" },
                    "sheet": { "type": "string", "description": "Sheet name (default: first sheet)" },
                    "range": { "type": "string", "description": "Cell range in A1 notation (e.g., A1:B10)" }
                },
                "required": ["input"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let input = args["input"].as_str().context("Missing input")?;
        let sheet = args["sheet"].as_str();
        let range = args["range"].as_str();

        let handler = ExcelHandler::new();

        let data = if let Some(range_str) = range {
            let cell_range = CellRange::parse(range_str)?;
            handler.read_range(input, &cell_range, sheet)?
        } else {
            let csv_str = handler.read_with_sheet(input, sheet)?;
            csv_str
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.split(',').map(|s| s.to_string()).collect())
                .collect()
        };

        Ok(json!({
            "status": "success",
            "data": data,
            "rows": data.len(),
            "columns": data.first().map(|r| r.len()).unwrap_or(0)
        }))
    }
}

pub struct ReadAllSheetsCapability;

impl Capability for ReadAllSheetsCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "read_all_sheets".to_string(),
            description: "Read data from all sheets in an Excel workbook".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "Input file path (.xlsx, .xls, .ods)" }
                },
                "required": ["input"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let input = args["input"].as_str().context("Missing input")?;

        let handler = ExcelHandler::new();
        let all_sheets = handler.read_all_sheets(input)?;

        Ok(json!({
            "status": "success",
            "sheets": all_sheets
        }))
    }
}
