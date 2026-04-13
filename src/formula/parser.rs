//! Formula parsing utilities

use super::types::CellRange;
use anyhow::{Context, Result};

/// Parse cell reference like "A1" to (row, col)
pub fn parse_cell_reference(cell: &str) -> Result<(u32, u16)> {
    let mut col_str = String::new();
    let mut row_str = String::new();

    for ch in cell.chars() {
        if ch.is_alphabetic() {
            col_str.push(ch);
        } else if ch.is_ascii_digit() {
            row_str.push(ch);
        }
    }

    let col = column_to_index(&col_str)?;
    let row = row_str
        .parse::<u32>()
        .with_context(|| format!("Invalid row number in cell reference: {}", cell))?;

    // CSV rows are 1-indexed, but we use 0-indexed internally
    Ok((row - 1, col))
}

/// Convert column letter to index (A=0, B=1, ..., Z=25, AA=26, ...)
pub fn column_to_index(col: &str) -> Result<u16> {
    let mut index = 0u16;
    for ch in col.chars() {
        index = index * 26 + (ch.to_ascii_uppercase() as u16 - b'A' as u16 + 1);
    }
    Ok(index - 1)
}

/// Parse a cell range like "A1:C10"
pub fn parse_range(range_str: &str) -> Result<CellRange> {
    let parts: Vec<&str> = range_str.split(':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid range format: {}", range_str);
    }

    let (start_row, start_col) = parse_cell_reference(parts[0])?;
    let (end_row, end_col) = parse_cell_reference(parts[1])?;

    Ok(CellRange {
        start_row,
        start_col,
        end_row,
        end_col,
    })
}

/// Extract function arguments from formula like "SUM(A1:A10)"
pub fn extract_function_args(formula: &str) -> Result<String> {
    let start = formula
        .find('(')
        .ok_or_else(|| anyhow::anyhow!("Missing opening parenthesis in formula"))?;
    let end = formula
        .rfind(')')
        .ok_or_else(|| anyhow::anyhow!("Missing closing parenthesis in formula"))?;

    if end <= start {
        anyhow::bail!("Invalid parentheses in formula");
    }

    Ok(formula[start + 1..end].to_string())
}

/// Split function arguments by comma, respecting nested parentheses
pub fn split_args(args: &str) -> Result<Vec<String>> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in args.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                result.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        result.push(current.trim().to_string());
    }

    Ok(result)
}

/// Get cell value from data
pub fn get_cell_value(cell_ref: &str, data: &[Vec<String>]) -> Result<f64> {
    let (row, col) = parse_cell_reference(cell_ref)?;

    let value = data
        .get(row as usize)
        .and_then(|r| r.get(col as usize))
        .map(|s| s.as_str())
        .unwrap_or("0");

    value
        .parse::<f64>()
        .with_context(|| format!("Cannot parse '{}' as number at {}", value, cell_ref))
}

/// Get cell value as string
pub fn get_cell_value_str(cell_ref: &str, data: &[Vec<String>]) -> Result<String> {
    let (row, col) = parse_cell_reference(cell_ref)?;

    Ok(data
        .get(row as usize)
        .and_then(|r| r.get(col as usize))
        .cloned()
        .unwrap_or_default())
}

/// Get values from a range
pub fn get_range_values(range: &CellRange, data: &[Vec<String>]) -> Vec<f64> {
    let mut values = Vec::new();

    for row in range.start_row..=range.end_row {
        for col in range.start_col..=range.end_col {
            if let Some(row_data) = data.get(row as usize) {
                if let Some(cell) = row_data.get(col as usize) {
                    if let Ok(num) = cell.parse::<f64>() {
                        values.push(num);
                    }
                }
            }
        }
    }

    values
}
