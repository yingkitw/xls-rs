//! Excel write capabilities (styles, charts, sparklines, conditional formatting)

use crate::capabilities::{Capability, CapabilityMetadata};
use crate::excel::{ChartConfig, DataChartType, ExcelHandler};
use crate::traits::DataReader;
use anyhow::{Context, Result};
use serde_json::{json, Value};

pub struct WriteStyledCapability;

impl Capability for WriteStyledCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "write_styled".to_string(),
            description: "Write data to Excel with styling options (header style, column styles, freeze, auto-filter)"
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "output": { "type": "string", "description": "Output file path (.xlsx)" },
                    "data": {
                        "type": "array",
                        "description": "2D array of string values",
                        "items": { "type": "array", "items": { "type": "string" } }
                    },
                    "sheet_name": { "type": "string", "description": "Sheet name (default: Sheet1)" },
                    "style_header": { "type": "boolean", "description": "Apply header styling to first row (default: true)" },
                    "freeze_header": { "type": "boolean", "description": "Freeze first row (default: false)" },
                    "auto_filter": { "type": "boolean", "description": "Enable auto-filter (default: false)" },
                    "auto_fit": { "type": "boolean", "description": "Auto-fit column widths (default: true)" }
                },
                "required": ["output", "data"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let output = args["output"].as_str().context("Missing output")?;
        let data: Vec<Vec<String>> = serde_json::from_value(args["data"].clone())
            .context("Invalid data format")?;

        let sheet_name = args["sheet_name"].as_str().map(|s| s.to_string());
        let style_header = args["style_header"].as_bool().unwrap_or(true);
        let freeze_header = args["freeze_header"].as_bool().unwrap_or(false);
        let auto_filter = args["auto_filter"].as_bool().unwrap_or(false);
        let auto_fit = args["auto_fit"].as_bool().unwrap_or(true);

        use crate::excel::types::WriteOptions;
        let options = WriteOptions {
            sheet_name,
            style_header,
            freeze_header,
            auto_filter,
            auto_fit,
            ..Default::default()
        };

        let handler = ExcelHandler::new();
        handler.write_styled(output, &data, &options)?;

        Ok(json!({
            "status": "success",
            "message": format!("Wrote styled data to '{}'", output)
        }))
    }
}

pub struct AddChartCapability;

impl Capability for AddChartCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "add_chart".to_string(),
            description: "Write data to Excel with an embedded chart".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "output": { "type": "string", "description": "Output file path (.xlsx)" },
                    "data": {
                        "type": "array",
                        "description": "2D array of string values",
                        "items": { "type": "array", "items": { "type": "string" } }
                    },
                    "chart_type": { "type": "string", "description": "Chart type: bar, column, line, area, pie, scatter, doughnut" },
                    "title": { "type": "string", "description": "Chart title" },
                    "category_column": { "type": "integer", "description": "Column index for category labels (default: 0)" },
                    "value_columns": { "type": "array", "items": { "type": "integer" }, "description": "Column indices for values (default: [1])" }
                },
                "required": ["output", "data"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let output = args["output"].as_str().context("Missing output")?;
        let data: Vec<Vec<String>> = serde_json::from_value(args["data"].clone())
            .context("Invalid data format")?;

        let chart_type = args["chart_type"]
            .as_str()
            .and_then(|s| DataChartType::from_str(s).ok())
            .unwrap_or(DataChartType::Column);

        let mut chart_config = ChartConfig {
            chart_type,
            ..Default::default()
        };

        if let Some(title) = args["title"].as_str() {
            chart_config.title = Some(title.to_string());
        }
        if let Some(cat_col) = args["category_column"].as_i64() {
            chart_config.category_column = cat_col as usize;
        }
        if let Some(value_cols) = args["value_columns"].as_array() {
            chart_config.value_columns = value_cols
                .iter()
                .filter_map(|v| v.as_i64())
                .map(|i| i as usize)
                .collect();
        }

        let handler = ExcelHandler::new();
        handler.write_with_chart(output, &data, &chart_config)?;

        Ok(json!({
            "status": "success",
            "message": format!("Wrote data with chart to '{}'", output)
        }))
    }
}

pub struct AddSparklineCapability;

impl Capability for AddSparklineCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "add_sparkline".to_string(),
            description: "Add a sparkline to an Excel file".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "output": { "type": "string", "description": "Output file path (.xlsx)" },
                    "data_range": { "type": "string", "description": "Data range for sparkline (e.g., A1:A10)" },
                    "sparkline_cell": { "type": "string", "description": "Cell to place sparkline (e.g., B1)" },
                    "sheet_name": { "type": "string", "description": "Sheet name (default: Sheet1)" }
                },
                "required": ["output", "data_range", "sparkline_cell"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let output = args["output"].as_str().context("Missing output")?;
        let data_range = args["data_range"].as_str().context("Missing data_range")?;
        let sparkline_cell = args["sparkline_cell"].as_str().context("Missing sparkline_cell")?;
        let sheet_name = args["sheet_name"].as_str();

        let handler = ExcelHandler::new();
        handler.add_sparkline_formula(output, data_range, sparkline_cell, sheet_name)?;

        Ok(json!({
            "status": "success",
            "message": format!("Added sparkline to '{}'", output)
        }))
    }
}

pub struct ConditionalFormatCapability;

impl Capability for ConditionalFormatCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "conditional_format".to_string(),
            description: "Apply conditional formatting to an Excel range".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "output": { "type": "string", "description": "Output file path (.xlsx)" },
                    "range": { "type": "string", "description": "Range to format (e.g., A1:B10)" },
                    "condition": { "type": "string", "description": "Formula condition (e.g., '=A1>100')" },
                    "bg_color": { "type": "string", "description": "Background color hex (e.g., 'FF0000')" },
                    "font_color": { "type": "string", "description": "Font color hex (e.g., 'FFFFFF')" },
                    "bold": { "type": "boolean", "description": "Bold text (default: true)" },
                    "sheet_name": { "type": "string", "description": "Sheet name (default: Sheet1)" }
                },
                "required": ["output", "range", "condition"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let output = args["output"].as_str().context("Missing output")?;
        let range = args["range"].as_str().context("Missing range")?;
        let condition = args["condition"].as_str().context("Missing condition")?;
        let sheet_name = args["sheet_name"].as_str();

        let bg_color = args["bg_color"].as_str().map(|s| s.to_string());
        let font_color = args["font_color"].as_str().map(|s| s.to_string());
        let bold = args["bold"].as_bool().unwrap_or(true);

        use crate::excel::types::CellStyle;
        let cell_style = CellStyle {
            bg_color,
            font_color,
            bold,
            ..Default::default()
        };

        let handler = ExcelHandler::new();
        handler.apply_conditional_format_formula(output, range, condition, &cell_style, None, sheet_name)?;

        Ok(json!({
            "status": "success",
            "message": format!("Applied conditional formatting to '{}'", output)
        }))
    }
}
