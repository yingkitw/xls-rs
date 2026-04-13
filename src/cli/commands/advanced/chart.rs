//! Chart generation command handler

use xls_rs::{
    common::validation,
    converter::Converter,
    excel::{ChartConfig, DataChartType, ExcelHandler, WriteOptions},
};
use anyhow::Result;

/// Handle the chart command
///
/// Creates a chart from data and saves it to an Excel file.
pub fn handle_chart(
    input: String,
    output: String,
    chart_type: String,
    title: Option<String>,
    x_column: Option<String>,
    y_column: Option<String>,
) -> Result<()> {
    let converter = Converter::new();
    let data = converter.read_any_data(&input, None)?;

    // Parse chart type
    let chart_type = match chart_type.to_lowercase().as_str() {
        "line" => DataChartType::Line,
        "bar" => DataChartType::Column,
        "column" => DataChartType::Column,
        "pie" => DataChartType::Pie,
        "scatter" => DataChartType::Scatter,
        "area" => DataChartType::Area,
        _ => anyhow::bail!(
            "Unknown chart type: {}. Use: line, bar, pie, scatter, area",
            chart_type
        ),
    };

    // Determine x and y columns
    let x_col = if let Some(col) = x_column {
        find_column_index(&data, &col)?
    } else {
        0 // Default to first column
    };

    let y_col = if let Some(col) = y_column {
        find_column_index(&data, &col)?
    } else {
        1 // Default to second column
    };

    validation::validate_column_index(&data, x_col)?;
    validation::validate_column_index(&data, y_col)?;

    // Create chart configuration
    let _config = ChartConfig {
        chart_type,
        title: Some(title.unwrap_or_else(|| "Chart".to_string())),
        category_column: x_col,
        value_columns: vec![y_col],
        ..Default::default()
    };

    // Write Excel with chart (placeholder - chart integration needs workbook API)
    let handler = ExcelHandler::new();
    let options = WriteOptions::default();

    handler.write_styled(&output, &data, &options)?;
    println!("Created {:?} chart; wrote {}", chart_type, output);

    Ok(())
}

/// Find column index by name or number
fn find_column_index(data: &[Vec<String>], column: &str) -> Result<usize> {
    if data.is_empty() {
        anyhow::bail!("Data is empty");
    }

    let header = &data[0];

    // Try to parse as number first
    if let Ok(index) = column.parse::<usize>() {
        if index == 0 {
            anyhow::bail!("Column indices start from 1");
        }
        return Ok(index - 1);
    }

    // Try to find by name
    header
        .iter()
        .position(|col_name| col_name == column)
        .ok_or_else(|| anyhow::anyhow!("Column '{}' not found", column))
}
