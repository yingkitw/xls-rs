//! Common utilities and shared functionality for xls-rs
//!
//! This module contains reusable components to reduce code duplication
//! and promote DRY principles across the codebase.

/// File format detection utilities
pub mod format {
    use std::path::Path;

    /// Get file format from extension
    pub fn from_extension(path: &str) -> &'static str {
        let ext = Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            "csv" => "csv",
            "xlsx" | "xls" => "excel",
            "ods" => "ods",
            "parquet" => "parquet",
            "avro" => "avro",
            "json" => "json",
            _ => "unknown",
        }
    }

    /// Check if format is supported
    pub fn is_supported(format: &str) -> bool {
        matches!(
            format,
            "csv" | "excel" | "ods" | "parquet" | "avro" | "json"
        )
    }
}

/// Data validation utilities
pub mod validation {
    use anyhow::{Result, anyhow};

    /// Validate column index exists in data
    pub fn validate_column_index(data: &[Vec<String>], col_idx: usize) -> Result<()> {
        if data.is_empty() {
            return Err(anyhow!("Data is empty"));
        }

        let num_cols = data[0].len();
        if col_idx >= num_cols {
            return Err(anyhow!(
                "Column index {} out of range (max: {})",
                col_idx,
                num_cols - 1
            ));
        }

        Ok(())
    }

    /// Validate data has consistent column counts
    pub fn validate_consistent_columns(data: &[Vec<String>]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        let expected_cols = data[0].len();
        for (i, row) in data.iter().enumerate() {
            if row.len() != expected_cols {
                return Err(anyhow!(
                    "Row {} has {} columns, expected {}",
                    i,
                    row.len(),
                    expected_cols
                ));
            }
        }

        Ok(())
    }

    /// Validate cell range string format
    pub fn validate_cell_range(range: &str) -> Result<()> {
        let re = regex::Regex::new(r"^[A-Z]+[0-9]+(:[A-Z]+[0-9]+)?$")?;
        if !re.is_match(range) {
            return Err(anyhow!("Invalid cell range format: {}", range));
        }
        Ok(())
    }
}

/// Common data transformation utilities
pub mod transform {
    use super::validation::validate_column_index;
    use anyhow::Result;
    use rayon::prelude::*;

    /// Apply a transformation function to a column
    pub fn apply_to_column<F>(
        data: &mut [Vec<String>],
        col_idx: usize,
        mut transform_fn: F,
    ) -> Result<()>
    where
        F: FnMut(&str) -> String,
    {
        validate_column_index(data, col_idx)?;

        for row in data.iter_mut().skip(1) {
            // Skip header
            if let Some(cell) = row.get_mut(col_idx) {
                *cell = transform_fn(cell);
            }
        }

        Ok(())
    }

    /// Apply a transformation function to a column in parallel
    pub fn apply_to_column_parallel<F>(
        data: &mut [Vec<String>],
        col_idx: usize,
        transform_fn: F,
    ) -> Result<()>
    where
        F: Fn(&str) -> String + Sync + Send,
    {
        validate_column_index(data, col_idx)?;

        data.par_iter_mut().skip(1).for_each(|row| {
            if let Some(cell) = row.get_mut(col_idx) {
                *cell = transform_fn(cell);
            }
        });

        Ok(())
    }

    /// Filter data based on a predicate function
    pub fn filter_data<F>(data: &[Vec<String>], predicate: F) -> Vec<Vec<String>>
    where
        F: Fn(&[String]) -> bool,
    {
        if data.is_empty() {
            return Vec::new();
        }

        let mut result = vec![data[0].clone()]; // Keep header
        result.extend(data.iter().skip(1).filter(|row| predicate(row)).cloned());
        result
    }

    /// Filter data based on a predicate function in parallel
    pub fn filter_data_parallel<F>(data: &[Vec<String>], predicate: F) -> Vec<Vec<String>>
    where
        F: Fn(&[String]) -> bool + Sync + Send,
    {
        if data.is_empty() {
            return Vec::new();
        }

        let mut result = vec![data[0].clone()]; // Keep header
        let filtered: Vec<Vec<String>> = data
            .par_iter()
            .skip(1)
            .filter(|row| predicate(row))
            .cloned()
            .collect();
        result.extend(filtered);
        result
    }

    /// Sort data by column with custom comparison
    pub fn sort_by_column<F>(
        data: &mut [Vec<String>],
        col_idx: usize,
        mut compare_fn: F,
    ) -> Result<()>
    where
        F: FnMut(&str, &str) -> std::cmp::Ordering,
    {
        validate_column_index(data, col_idx)?;

        if data.len() <= 1 {
            return Ok(());
        }

        let _header = data[0].clone();
        let mut data_rows: Vec<&mut Vec<String>> = data.iter_mut().skip(1).collect();

        data_rows.sort_by(|a, b| {
            let a_val = a.get(col_idx).map(|s| s.as_str()).unwrap_or("");
            let b_val = b.get(col_idx).map(|s| s.as_str()).unwrap_or("");
            compare_fn(a_val, b_val)
        });

        Ok(())
    }

    /// Sort data by column with custom comparison in parallel
    pub fn sort_by_column_parallel<F>(
        data: &mut [Vec<String>],
        col_idx: usize,
        compare_fn: F,
    ) -> Result<()>
    where
        F: Fn(&str, &str) -> std::cmp::Ordering + Sync + Send,
    {
        validate_column_index(data, col_idx)?;

        if data.len() <= 1 {
            return Ok(());
        }

        let _header = data[0].clone();
        let mut data_rows: Vec<&mut Vec<String>> = data.iter_mut().skip(1).collect();

        data_rows.par_sort_by(|a, b| {
            let a_val = a.get(col_idx).map(|s| s.as_str()).unwrap_or("");
            let b_val = b.get(col_idx).map(|s| s.as_str()).unwrap_or("");
            compare_fn(a_val, b_val)
        });

        Ok(())
    }
}

/// Error handling utilities
pub mod error {
    use anyhow::anyhow;
    use std::fmt;

    /// Create a contextual error with file information
    pub fn with_file_context(error: impl fmt::Display, file: &str) -> anyhow::Error {
        anyhow!("Error processing file '{}': {}", file, error)
    }

    /// Create a contextual error with row and column information
    pub fn with_cell_context(
        error: impl fmt::Display,
        file: &str,
        row: usize,
        col: usize,
    ) -> anyhow::Error {
        anyhow!("Error at {}:{}:{}: {}", file, row + 1, col + 1, error)
    }

    /// Create a contextual error with column information
    pub fn with_column_context(
        error: impl fmt::Display,
        file: &str,
        column: &str,
    ) -> anyhow::Error {
        anyhow!("Error in column '{}' of file '{}': {}", column, file, error)
    }
}

/// String manipulation utilities
pub mod string {
    /// Trim and normalize whitespace
    pub fn normalize_whitespace(s: &str) -> String {
        s.trim().split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Check if string represents a number
    pub fn is_numeric(s: &str) -> bool {
        s.parse::<f64>().is_ok()
    }

    /// Check if string is empty or whitespace
    pub fn is_empty_or_whitespace(s: &str) -> bool {
        s.trim().is_empty()
    }

    /// Safe string to number conversion
    pub fn to_number(s: &str) -> Option<f64> {
        s.trim().parse::<f64>().ok()
    }
}

/// Collection utilities
pub mod collection {
    /// Get unique values from a vector while preserving order
    pub fn unique_preserve_order<T: Clone + Eq + std::hash::Hash>(vec: &[T]) -> Vec<T> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();

        for item in vec {
            if seen.insert(item) {
                result.push(item.clone());
            }
        }

        result
    }

    /// Chunk a vector into smaller pieces
    pub fn chunk<T: std::clone::Clone>(data: Vec<T>, chunk_size: usize) -> Vec<Vec<T>> {
        if chunk_size == 0 {
            return vec![data];
        }

        data.chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    /// Flatten a nested vector
    pub fn flatten<T>(nested: Vec<Vec<T>>) -> Vec<T> {
        nested.into_iter().flatten().collect()
    }
}
