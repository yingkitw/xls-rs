//! Helper functions for common operations (DRY principle)

use crate::csv_handler::CellRange;
use anyhow::{Context, Result};

/// Filter data by cell range (used by multiple handlers)
pub fn filter_by_range(data: &[Vec<String>], range: &CellRange) -> Vec<Vec<String>> {
    let mut result = Vec::new();

    for (row_idx, row) in data.iter().enumerate() {
        if row_idx < range.start_row {
            continue;
        }
        if row_idx > range.end_row {
            break;
        }

        let filtered_row: Vec<String> = row
            .iter()
            .enumerate()
            .filter(|(col_idx, _)| *col_idx >= range.start_col && *col_idx <= range.end_col)
            .map(|(_, val)| val.clone())
            .collect();
        result.push(filtered_row);
    }

    result
}

/// Get default column names if not provided
pub fn default_column_names(num_cols: usize, prefix: &str) -> Vec<String> {
    (0..num_cols).map(|i| format!("{}_{}", prefix, i)).collect()
}

/// Get maximum column count from data
pub fn max_column_count(data: &[Vec<String>]) -> usize {
    data.iter().map(|r| r.len()).max().unwrap_or(0)
}

/// Check if a path matches any of the given extensions
pub fn matches_extension(path: &str, extensions: &[&str]) -> bool {
    let path_lower = path.to_lowercase();
    extensions
        .iter()
        .any(|ext| path_lower.ends_with(&format!(".{}", ext)))
}

/// Safe numeric parsing with bounds checking
///
/// Prevents overflow/underflow and validates numeric ranges
pub fn parse_safe_f64(value: &str, min: Option<f64>, max: Option<f64>) -> Result<f64> {
    let num = value
        .trim()
        .parse::<f64>()
        .with_context(|| format!("Invalid numeric value: '{}'", value))?;

    // Check for NaN and Infinity
    if !num.is_finite() {
        anyhow::bail!("Numeric value must be finite: '{}'", value);
    }

    // Check bounds
    if let Some(min_val) = min {
        if num < min_val {
            anyhow::bail!("Value {} is below minimum {}", num, min_val);
        }
    }
    if let Some(max_val) = max {
        if num > max_val {
            anyhow::bail!("Value {} exceeds maximum {}", num, max_val);
        }
    }

    Ok(num)
}

/// Safe integer parsing with bounds checking
///
/// Prevents overflow/underflow and validates integer ranges
pub fn parse_safe_i64(value: &str, min: Option<i64>, max: Option<i64>) -> Result<i64> {
    let num = value
        .trim()
        .parse::<i64>()
        .with_context(|| format!("Invalid integer value: '{}'", value))?;

    // Check bounds
    if let Some(min_val) = min {
        if num < min_val {
            anyhow::bail!("Value {} is below minimum {}", num, min_val);
        }
    }
    if let Some(max_val) = max {
        if num > max_val {
            anyhow::bail!("Value {} exceeds maximum {}", num, max_val);
        }
    }

    Ok(num)
}

/// Safe usize parsing for indices with bounds checking
///
/// Prevents negative values and validates within max value
pub fn parse_safe_usize(value: &str, max: Option<usize>) -> Result<usize> {
    let trimmed = value.trim();

    // Check for negative sign
    if trimmed.starts_with('-') {
        anyhow::bail!("Index cannot be negative: '{}'", value);
    }

    let num = trimmed
        .parse::<usize>()
        .with_context(|| format!("Invalid index value: '{}'", value))?;

    // Check bounds
    if let Some(max_val) = max {
        if num > max_val {
            anyhow::bail!("Index {} exceeds maximum {}", num, max_val);
        }
    }

    Ok(num)
}

/// Add file context to an error
///
/// Wraps an error with file path information for better debugging
pub fn with_file_context<T>(result: Result<T>, file_path: &str) -> Result<T> {
    result.with_context(|| format!("Error processing file: '{}'", file_path))
}

/// Add row and column context to an error
///
/// Wraps an error with row and column information for better debugging
pub fn with_cell_context<T>(result: Result<T>, row: usize, col: usize) -> Result<T> {
    result.with_context(|| format!("Error at row {}, column {}", row, col))
}

/// Add file, row, and column context to an error
///
/// Wraps an error with complete location information for better debugging
pub fn with_full_context<T>(result: Result<T>, file_path: &str, row: usize, col: usize) -> Result<T> {
    result.with_context(|| format!("Error in '{}' at row {}, column {}", file_path, row, col))
}

/// Validate row index is within bounds
pub fn validate_row_index(data: &[Vec<String>], row: usize) -> Result<()> {
    if row >= data.len() {
        anyhow::bail!("Row index {} out of bounds (data has {} rows)", row, data.len());
    }
    Ok(())
}

/// Validate column index is within bounds
pub fn validate_column_index(data: &[Vec<String>], col: usize) -> Result<()> {
    if data.is_empty() {
        anyhow::bail!("Cannot validate column index: data is empty");
    }
    if col >= data[0].len() {
        anyhow::bail!("Column index {} out of bounds (row has {} columns)", col, data[0].len());
    }
    Ok(())
}

