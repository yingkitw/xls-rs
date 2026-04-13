//! Schema export command handler
//!
//! Outputs column names and inferred types as JSON.

use xls_rs::converter::Converter;
use anyhow::Result;
use serde_json::json;

/// Handle the schema command
///
/// Outputs column names and inferred types as JSON.
pub fn handle_schema(input: String, output: Option<String>) -> Result<()> {
    let converter = Converter::new();
    let data = converter.read_any_data(&input, None)?;

    if data.is_empty() {
        let schema = json!({ "columns": [], "row_count": 0 });
        let report = serde_json::to_string_pretty(&schema)?;
        if let Some(output_path) = &output {
            std::fs::write(output_path, report)?;
            println!("Schema saved to {}", output_path);
        } else {
            println!("{}", report);
        }
        return Ok(());
    }

    let header = &data[0];
    let mut columns = Vec::new();

    for (i, col_name) in header.iter().enumerate() {
        let mut has_numbers = false;
        let mut has_strings = false;
        let mut non_null_count = 0usize;

        for row in data.iter().skip(1) {
            if let Some(cell) = row.get(i) {
                if cell.is_empty() {
                    continue;
                }
                non_null_count += 1;
                if cell.parse::<f64>().is_ok() {
                    has_numbers = true;
                } else {
                    has_strings = true;
                }
            }
        }

        let dtype = if has_strings {
            "string"
        } else if has_numbers {
            "number"
        } else {
            "empty"
        };

        columns.push(json!({
            "name": col_name,
            "type": dtype,
            "non_null_count": non_null_count
        }));
    }

    let schema = json!({
        "columns": columns,
        "row_count": data.len().saturating_sub(1)
    });

    let report = serde_json::to_string_pretty(&schema)?;

    if let Some(output_path) = &output {
        std::fs::write(output_path, report)?;
        println!("Schema saved to {}", output_path);
    } else {
        println!("{}", report);
    }

    Ok(())
}
