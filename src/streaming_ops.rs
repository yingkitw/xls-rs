//! Streaming operations for large files
//!
//! This module provides memory-efficient operations that work on streams
//! rather than loading entire datasets into memory.

use crate::csv_handler::StreamingCsvReader;
use anyhow::{Context, Result};
use std::collections::HashMap;

/// Schema information for a dataset
#[derive(Debug, Clone)]
pub struct Schema {
    /// Column names
    pub columns: Vec<String>,
    /// Column types inferred from sample data
    pub types: Vec<ColumnType>,
    /// Total row count (if available)
    pub row_count: Option<usize>,
}

/// Inferred column type
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum ColumnType {
    String,
    Integer,
    Float,
    Boolean,
    Empty,
    Unknown,
}

impl ColumnType {
    /// Infer column type from a sample of values
    pub fn infer_from_samples(samples: &[String]) -> Self {
        if samples.is_empty() {
            return ColumnType::Unknown;
        }

        let mut has_integers = false;
        let mut has_floats = false;
        let mut has_booleans = false;
        let mut has_strings = false;
        let mut has_empty = false;

        for sample in samples {
            if sample.is_empty() {
                has_empty = true;
            } else if sample.parse::<i64>().is_ok() {
                has_integers = true;
            } else if sample.parse::<f64>().is_ok() {
                has_floats = true;
            } else if matches!(sample.to_lowercase().as_str(), "true" | "false") {
                has_booleans = true;
            } else {
                has_strings = true;
            }
        }

        // Determine the most specific type that fits all samples
        if has_strings {
            ColumnType::String
        } else if has_floats {
            ColumnType::Float
        } else if has_integers {
            ColumnType::Integer
        } else if has_booleans {
            ColumnType::Boolean
        } else if has_empty && samples.len() == 1 {
            ColumnType::Empty
        } else {
            ColumnType::Unknown
        }
    }
}

/// Read first N rows from a CSV file without loading the entire file
///
/// This is memory-efficient for large files where you only need the first few rows.
///
/// # Arguments
/// * `path` - Path to the CSV file
/// * `n` - Number of rows to read (excluding headers if present)
///
/// # Returns
/// Vector of rows (as Vec<String>)
pub fn head(path: &str, n: usize) -> Result<Vec<Vec<String>>> {
    let mut reader = StreamingCsvReader::open(path)?;
    let mut result = Vec::with_capacity(n);

    for row_result in reader.by_ref().take(n) {
        result.push(row_result?);
    }

    Ok(result)
}

/// Read last N rows from a CSV file
///
/// Note: This currently requires reading the entire file to find the end,
/// but only keeps the last N rows in memory.
///
/// # Arguments
/// * `path` - Path to the CSV file
/// * `n` - Number of rows to read
///
/// # Returns
/// Vector of rows (as Vec<String>)
pub fn tail(path: &str, n: usize) -> Result<Vec<Vec<String>>> {
    let mut reader = StreamingCsvReader::open(path)?;
    let mut buffer: Vec<Vec<String>> = Vec::with_capacity(n);

    for row_result in reader {
        let row = row_result?;
        buffer.push(row);

        // Keep only last N rows
        if buffer.len() > n {
            buffer.remove(0);
        }
    }

    Ok(buffer)
}

/// Infer schema from a CSV file by sampling the first N rows
///
/// This is memory-efficient for large files as it only reads a sample.
///
/// # Arguments
/// * `path` - Path to the CSV file
/// * `sample_size` - Number of rows to sample (default: 1000)
/// * `has_headers` - Whether the first row contains headers (default: true)
///
/// # Returns
/// Schema information including column names and inferred types
pub fn infer_schema(path: &str, sample_size: usize, has_headers: bool) -> Result<Schema> {
    let mut reader = StreamingCsvReader::open(path)?;

    // Read headers if present
    let headers = if has_headers {
        match reader.next() {
            Some(Ok(row)) => row,
            _ => return Ok(Schema {
                columns: Vec::new(),
                types: Vec::new(),
                row_count: Some(0),
            }),
        }
    } else {
        // No headers, need to infer column count from first data row
        match reader.next() {
            Some(Ok(row)) => {
                (0..row.len())
                    .map(|i| format!("column_{}", i))
                    .collect()
            }
            _ => return Ok(Schema {
                columns: Vec::new(),
                types: Vec::new(),
                row_count: Some(0),
            }),
        }
    };

    // Sample rows for type inference
    let mut sample_rows: Vec<Vec<String>> = Vec::with_capacity(sample_size);
    for row_result in reader.by_ref().take(sample_size) {
        if let Ok(row) = row_result {
            sample_rows.push(row);
        }
    }

    infer_types(&headers, &sample_rows)
}

fn infer_types(headers: &[String], sample_rows: &[Vec<String>]) -> Result<Schema> {
    let num_cols = headers.len();
    let mut column_samples: Vec<Vec<String>> = vec![Vec::new(); num_cols];

    // Collect samples for each column
    for row in sample_rows {
        for (col_idx, value) in row.iter().enumerate().take(num_cols) {
            column_samples[col_idx].push(value.clone());
        }
    }

    // Infer types for each column
    let types: Vec<ColumnType> = column_samples
        .iter()
        .map(|samples| ColumnType::infer_from_samples(samples))
        .collect();

    Ok(Schema {
        columns: headers.to_vec(),
        types,
        row_count: None, // Row count not available without reading entire file
    })
}

/// Get basic info about a CSV file without loading all data
///
/// Returns file size, row count (estimated), column count, and schema.
///
/// # Arguments
/// * `path` - Path to the CSV file
/// * `max_sample_rows` - Maximum rows to sample for schema inference
///
/// # Returns
/// Map containing file information
pub fn get_info(path: &str, max_sample_rows: usize) -> Result<HashMap<String, serde_json::Value>> {
    let metadata = std::fs::metadata(path)?;
    let file_size = metadata.len();

    let mut reader = StreamingCsvReader::open(path)?;

    // Read first row to determine column count
    let first_row = reader.next();
    let (num_cols, has_headers) = match first_row {
        Some(Ok(row)) => (row.len(), true),
        _ => return Ok(HashMap::new()),
    };

    // Sample rows for schema
    let schema = infer_schema(path, max_sample_rows.saturating_sub(1), true)?;

    // Count total rows (optional - can be expensive for large files)
    let row_count = count_rows(path)?;

    let mut info = HashMap::new();
    info.insert(
        "file_size".to_string(),
        serde_json::json!(file_size),
    );
    info.insert(
        "row_count".to_string(),
        serde_json::json!(row_count),
    );
    info.insert(
        "column_count".to_string(),
        serde_json::json!(num_cols),
    );
    info.insert(
        "has_headers".to_string(),
        serde_json::json!(has_headers),
    );
    info.insert(
        "columns".to_string(),
        serde_json::json!(schema.columns),
    );
    info.insert(
        "column_types".to_string(),
        serde_json::json!(schema.types),
    );

    Ok(info)
}

/// Count total rows in a CSV file (requires reading the entire file)
///
/// This operation is O(n) but uses streaming to minimize memory usage.
pub fn count_rows(path: &str) -> Result<usize> {
    let reader = StreamingCsvReader::open(path)?;
    Ok(reader.count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_head() {
        let dir = TempDir::new().unwrap();
        let csv_path = dir.path().join("test.csv");

        let data = vec![
            vec!["A".to_string(), "B".to_string()],
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string(), "4".to_string()],
            vec!["5".to_string(), "6".to_string()],
            vec!["7".to_string(), "8".to_string()],
        ];

        write_csv(&csv_path, &data).unwrap();

        let result = head(csv_path.to_str().unwrap(), 2).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec!["A".to_string(), "B".to_string()]);
        assert_eq!(result[1], vec!["1".to_string(), "2".to_string()]);
    }

    #[test]
    fn test_tail() {
        let dir = TempDir::new().unwrap();
        let csv_path = dir.path().join("test.csv");

        let data = vec![
            vec!["A".to_string(), "B".to_string()],
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string(), "4".to_string()],
            vec!["5".to_string(), "6".to_string()],
            vec!["7".to_string(), "8".to_string()],
        ];

        write_csv(&csv_path, &data).unwrap();

        let result = tail(csv_path.to_str().unwrap(), 2).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec!["5".to_string(), "6".to_string()]);
        assert_eq!(result[1], vec!["7".to_string(), "8".to_string()]);
    }

    #[test]
    fn test_infer_schema() {
        let dir = TempDir::new().unwrap();
        let csv_path = dir.path().join("test.csv");

        let data = vec![
            vec!["Name".to_string(), "Age".to_string(), "Active".to_string()],
            vec!["Alice".to_string(), "25".to_string(), "true".to_string()],
            vec!["Bob".to_string(), "30".to_string(), "false".to_string()],
        ];

        write_csv(&csv_path, &data).unwrap();

        let schema = infer_schema(csv_path.to_str().unwrap(), 10, true).unwrap();
        assert_eq!(schema.columns, vec!["Name", "Age", "Active"]);
        assert_eq!(schema.types.len(), 3);
        assert_eq!(schema.types[0], ColumnType::String);
        assert_eq!(schema.types[1], ColumnType::Integer);
        assert_eq!(schema.types[2], ColumnType::Boolean);
    }

    #[test]
    fn test_column_type_inference() {
        assert_eq!(
            ColumnType::infer_from_samples(&["1".to_string(), "2".to_string(), "3".to_string()]),
            ColumnType::Integer
        );

        assert_eq!(
            ColumnType::infer_from_samples(&["1.5".to_string(), "2.5".to_string()]),
            ColumnType::Float
        );

        assert_eq!(
            ColumnType::infer_from_samples(&["hello".to_string(), "world".to_string()]),
            ColumnType::String
        );

        assert_eq!(
            ColumnType::infer_from_samples(&["true".to_string(), "false".to_string()]),
            ColumnType::Boolean
        );
    }

    fn write_csv(path: &std::path::Path, data: &[Vec<String>]) -> Result<()> {
        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_path(path)?;
        for row in data {
            writer.write_record(row)?;
        }
        writer.flush()?;
        Ok(())
    }
}
