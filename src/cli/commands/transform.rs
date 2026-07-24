//! Data transformation command handlers
//!
//! Implements data manipulation operations like sort, filter, replace, etc.

use xls_rs::{
    common::validation,
    converter::Converter,
    operations::{DataOperations, SortOrder},
};
use anyhow::Result;

/// Data transformation command handler
#[derive(Default)]
pub struct TransformCommandHandler;

impl TransformCommandHandler {
    /// Create a new transformation command handler
    pub fn new() -> Self {
        Self
    }

    /// Handle the sort command
    ///
    /// Sorts rows by a specific column in ascending or descending order.
    pub fn handle_sort(
        &self,
        input: String,
        output: String,
        column: String,
        ascending: bool,
    ) -> Result<()> {
        let converter = Converter::new();
        let mut data = converter.read_any_data(&input, None)?;

        // Find column index
        let col_idx = self.find_column_index(&data, &column)?;
        validation::validate_column_index(&data, col_idx)?;

        // Sort data (preserve header row at top)
        let ops = DataOperations::new();
        let order = if ascending {
            SortOrder::Ascending
        } else {
            SortOrder::Descending
        };
        if data.len() > 1 {
            let mut body = data.split_off(1);
            ops.sort_by_column(&mut body, col_idx, order)?;
            data.append(&mut body);
        }

        // Write output
        converter.write_any_data(&output, &data, None)?;
        if output != "-" {
            crate::cli::runtime::log(format!("Sorted by {column} ({order:?}); wrote {output}"));
        }

        Ok(())
    }

    /// Handle the filter command
    ///
    /// Filters rows based on a WHERE clause condition.
    pub fn handle_filter(&self, input: String, output: String, where_clause: String) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        // Parse WHERE clause (simple implementation)
        // Format: column operator value
        // Example: "age > 25" or "name == John"
        let parts: Vec<&str> = where_clause.split_whitespace().collect();
        if parts.len() < 3 {
            anyhow::bail!(
                "Invalid WHERE clause format. Expected: 'column operator value', got: '{where_clause}'"
            );
        }

        let column = parts[0];
        let operator = parts[1];
        let value = parts[2..].join(" ");

        let col_idx = self.find_column_index(&data, column)?;
        validation::validate_column_index(&data, col_idx)?;

        let ops = DataOperations::new();
        let filtered = ops.filter_rows(&data, col_idx, operator, &value)?;

        converter.write_any_data(&output, &filtered, None)?;
        if output != "-" {
            crate::cli::runtime::log(format!(
                "Filtered to {} rows; wrote {}",
                filtered.len(),
                output
            ));
        }

        Ok(())
    }

    /// Handle the replace command
    ///
    /// Finds and replaces values in the data.
    pub fn handle_replace(
        &self,
        input: String,
        output: String,
        find: String,
        replace: String,
        column: Option<String>,
    ) -> Result<()> {
        let converter = Converter::new();
        let mut data = converter.read_any_data(&input, None)?;

        if let Some(col_name) = column {
            // Replace in specific column
            let col_idx = self.find_column_index(&data, &col_name)?;
            validation::validate_column_index(&data, col_idx)?;

            let mut count = 0;
            for row in &mut data {
                if let Some(cell) = row.get_mut(col_idx)
                    && cell.contains(&find) {
                        *cell = cell.replace(&find, &replace);
                        count += 1;
                    }
            }
            crate::cli::runtime::log(format!(
                "Replaced {count} occurrences in column '{col_name}'"
            ));
        } else {
            // Replace in all cells
            let mut count = 0;
            for row in &mut data {
                for cell in row {
                    if cell.contains(&find) {
                        *cell = cell.replace(&find, &replace);
                        count += 1;
                    }
                }
            }
            crate::cli::runtime::log(format!("Replaced {count} occurrences in all cells"));
        }

        converter.write_any_data(&output, &data, None)?;
        crate::cli::runtime::log(format!("Wrote {output}"));

        Ok(())
    }

    /// Handle the dedupe command
    ///
    /// Removes duplicate rows from the data.
    pub fn handle_dedupe(
        &self,
        input: String,
        output: String,
        columns: Option<String>,
    ) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let ops = DataOperations::new();
        let deduped = if let Some(cols_str) = columns {
            // Dedupe based on specific columns - extract unique rows based on those columns
            let col_indices: Vec<usize> = cols_str
                .split(',')
                .map(|c| self.find_column_index(&data, c.trim()))
                .collect::<Result<Vec<_>>>()?;

            // Use a HashSet to track seen combinations
            use std::collections::HashSet;
            let mut seen = HashSet::new();
            let mut result = vec![data[0].clone()]; // Keep header

            for row in &data[1..] {
                let key: Vec<&String> = col_indices.iter().filter_map(|i| row.get(*i)).collect();
                if seen.insert(key.clone()) {
                    result.push(row.clone());
                }
            }
            result
        } else {
            // Dedupe based on all columns
            ops.deduplicate(&data)
        };

        converter.write_any_data(&output, &deduped, None)?;
        crate::cli::runtime::log(format!(
            "Removed {} duplicates; wrote {}",
            data.len() - deduped.len(),
            output
        ));

        Ok(())
    }

    /// Handle the transpose command
    ///
    /// Transposes data (rows become columns, columns become rows).
    pub fn handle_transpose(&self, input: String, output: String) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let ops = DataOperations::new();
        let transposed = ops.transpose(&data);

        converter.write_any_data(&output, &transposed, None)?;
        crate::cli::runtime::log(format!(
            "Transposed {}x{} to {}x{}; wrote {}",
            data.len(),
            data.first().map(|r| r.len()).unwrap_or(0),
            transposed.len(),
            transposed.first().map(|r| r.len()).unwrap_or(0),
            output
        ));

        Ok(())
    }

    /// Handle the select command
    ///
    /// Selects specific columns from the data.
    pub fn handle_select(&self, input: String, output: String, columns: String) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        // Parse column names
        let col_names: Vec<&str> = columns.split(',').map(|c| c.trim()).collect();

        let ops = DataOperations::new();
        let selected = ops.select_columns_by_name(&data, &col_names)?;

        converter.write_any_data(&output, &selected, None)?;
        crate::cli::runtime::log(format!(
            "Selected {} columns; wrote {}",
            col_names.len(),
            output
        ));

        Ok(())
    }

    /// Handle the rename command
    ///
    /// Renames columns in the data.
    pub fn handle_rename(
        &self,
        input: String,
        output: String,
        from: String,
        to: String,
    ) -> Result<()> {
        let converter = Converter::new();
        let mut data = converter.read_any_data(&input, None)?;

        let ops = DataOperations::new();
        ops.rename_columns(&mut data, &[(from.as_str(), to.as_str())])?;

        converter.write_any_data(&output, &data, None)?;
        crate::cli::runtime::log(format!("Renamed column '{from}' to '{to}'; wrote {output}"));

        Ok(())
    }

    /// Handle the drop command
    ///
    /// Drops specified columns from the data.
    pub fn handle_drop(&self, input: String, output: String, columns: String) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        // Parse column names and find indices
        let col_indices: Vec<usize> = columns
            .split(',')
            .map(|c| self.find_column_index(&data, c.trim()))
            .collect::<Result<Vec<_>>>()?;

        let ops = DataOperations::new();
        let dropped = ops.drop_columns(&data, &col_indices);

        converter.write_any_data(&output, &dropped, None)?;
        crate::cli::runtime::log(format!(
            "Dropped {} columns; wrote {}",
            col_indices.len(),
            output
        ));

        Ok(())
    }

    /// Handle the fillna command
    ///
    /// Fills missing/empty values with a specified value.
    pub fn handle_fillna(
        &self,
        input: String,
        output: String,
        value: String,
        columns: Option<String>,
    ) -> Result<()> {
        let converter = Converter::new();
        let mut data = converter.read_any_data(&input, None)?;

        if let Some(cols_str) = columns {
            // Fill specific columns
            let col_indices: Vec<usize> = cols_str
                .split(',')
                .map(|c| self.find_column_index(&data, c.trim()))
                .collect::<Result<Vec<_>>>()?;

            let mut count = 0;
            for row in &mut data.iter_mut().skip(1) {
                // Skip header
                for col_idx in &col_indices {
                    if let Some(cell) = row.get_mut(*col_idx)
                        && cell.is_empty() {
                            *cell = value.clone();
                            count += 1;
                        }
                }
            }
            crate::cli::runtime::log(format!("Filled {count} cells in specified columns"));
        } else {
            // Fill all columns
            let ops = DataOperations::new();
            ops.fillna(&mut data, &value);
            crate::cli::runtime::log(format!("Filled all empty cells with '{value}'"));
        }

        converter.write_any_data(&output, &data, None)?;
        crate::cli::runtime::log(format!("Wrote {output}"));

        Ok(())
    }

    /// Handle the dropna command
    ///
    /// Drops rows that contain any empty values.
    pub fn handle_dropna(&self, input: String, output: String) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let ops = DataOperations::new();
        let filtered = ops.dropna(&data);

        converter.write_any_data(&output, &filtered, None)?;
        crate::cli::runtime::log(format!(
            "Dropped {} rows with empty values; wrote {}",
            data.len() - filtered.len(),
            output
        ));

        Ok(())
    }

    /// Handle the mutate command
    ///
    /// Adds a computed column based on a formula.
    pub fn handle_mutate(
        &self,
        input: String,
        output: String,
        column: String,
        formula: String,
    ) -> Result<()> {
        let converter = Converter::new();
        let mut data = converter.read_any_data(&input, None)?;

        // Simple formula evaluation for common operations
        // Format: "column1 + column2" or "column * 2"
        let result_values = self.evaluate_formula(&data, &formula)?;

        // Add or update column
        if data.is_empty() {
            return Ok(()); // No data to modify
        }

        // Add header if new column
        if let Some(header) = data.first_mut()
            && !header.contains(&column) {
                header.push(column.clone());
            }

        // Add values to each row
        for (i, row) in data.iter_mut().enumerate().skip(1) {
            let value = result_values.get(i - 1).map(|s| s.as_str()).unwrap_or("");
            row.push(value.to_string());
        }

        converter.write_any_data(&output, &data, None)?;
        crate::cli::runtime::log(format!(
            "Added column '{column}' with formula '{formula}'; wrote {output}"
        ));

        Ok(())
    }

    /// Handle the query command
    ///
    /// Executes SQL-like query on the data.
    pub fn handle_query(&self, input: String, output: String, where_clause: String) -> Result<()> {
        // Query is similar to filter but with more advanced syntax
        // For now, delegate to filter
        self.handle_filter(input, output, where_clause)
    }

    /// Handle the astype command
    ///
    /// Casts a column to a different data type.
    pub fn handle_astype(
        &self,
        input: String,
        output: String,
        column: String,
        target_type: String,
    ) -> Result<()> {
        let converter = Converter::new();
        let mut data = converter.read_any_data(&input, None)?;

        let col_idx = self.find_column_index(&data, &column)?;
        validation::validate_column_index(&data, col_idx)?;

        let ops = DataOperations::new();
        let converted = ops.astype(&mut data, col_idx, &target_type)?;

        converter.write_any_data(&output, &data, None)?;
        crate::cli::runtime::log(format!(
            "Converted {converted} cells to type '{target_type}'; wrote {output}"
        ));

        Ok(())
    }

    /// Find column index by name
    fn find_column_index(&self, data: &[Vec<String>], column: &str) -> Result<usize> {
        if data.is_empty() {
            anyhow::bail!("Data is empty, cannot find column '{column}'");
        }

        let header = &data[0];
        header
            .iter()
            .position(|h| h == column)
            .ok_or_else(|| anyhow::anyhow!("Column '{column}' not found"))
    }

    /// Handle the clip command
    pub fn handle_clip(
        &self,
        input: String,
        output: String,
        column: String,
        min: f64,
        max: f64,
    ) -> Result<()> {
        let converter = Converter::new();
        let mut data = converter.read_any_data(&input, None)?;

        let col_idx = self.find_column_index(&data, &column)?;
        validation::validate_column_index(&data, col_idx)?;

        let ops = DataOperations::new();
        let clipped = ops.clip(&mut data, col_idx, Some(min), Some(max))?;

        converter.write_any_data(&output, &data, None)?;
        crate::cli::runtime::log(format!("Clipped {clipped} cells; wrote {output}"));
        Ok(())
    }

    /// Handle the normalize command
    pub fn handle_normalize(&self, input: String, output: String, column: String) -> Result<()> {
        let converter = Converter::new();
        let mut data = converter.read_any_data(&input, None)?;

        let col_idx = self.find_column_index(&data, &column)?;
        validation::validate_column_index(&data, col_idx)?;

        let ops = DataOperations::new();
        ops.normalize(&mut data, col_idx)?;

        converter.write_any_data(&output, &data, None)?;
        crate::cli::runtime::log(format!("Normalized column {column}; wrote {output}"));
        Ok(())
    }

    /// Handle the zscore command
    pub fn handle_zscore(&self, input: String, output: String, column: String) -> Result<()> {
        let converter = Converter::new();
        let mut data = converter.read_any_data(&input, None)?;

        let col_idx = self.find_column_index(&data, &column)?;
        validation::validate_column_index(&data, col_idx)?;

        let ops = DataOperations::new();
        ops.zscore(&mut data, col_idx)?;

        converter.write_any_data(&output, &data, None)?;
        crate::cli::runtime::log(format!("Z-score standardized column {column}; wrote {output}"));
        Ok(())
    }

    /// Handle the parse-date command
    pub fn handle_parse_date(
        &self,
        input: String,
        output: String,
        column: String,
        from_format: String,
        to_format: String,
    ) -> Result<()> {
        let converter = Converter::new();
        let mut data = converter.read_any_data(&input, None)?;

        let col_idx = self.find_column_index(&data, &column)?;
        validation::validate_column_index(&data, col_idx)?;

        let ops = DataOperations::new();
        let converted = ops.parse_date(&mut data, col_idx, &from_format, &to_format)?;

        converter.write_any_data(&output, &data, None)?;
        crate::cli::runtime::log(format!("Converted {converted} dates; wrote {output}"));
        Ok(())
    }

    /// Handle the regex-filter command
    pub fn handle_regex_filter(
        &self,
        input: String,
        output: String,
        column: String,
        pattern: String,
    ) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let col_idx = self.find_column_index(&data, &column)?;
        validation::validate_column_index(&data, col_idx)?;

        let ops = DataOperations::new();
        let filtered = ops.regex_filter(&data, col_idx, &pattern)?;

        converter.write_any_data(&output, &filtered, None)?;
        crate::cli::runtime::log(format!(
            "Filtered to {} rows; wrote {}",
            filtered.len().saturating_sub(1),
            output
        ));
        Ok(())
    }

    /// Handle the regex-replace command
    pub fn handle_regex_replace(
        &self,
        input: String,
        output: String,
        column: String,
        pattern: String,
        replacement: String,
    ) -> Result<()> {
        let converter = Converter::new();
        let mut data = converter.read_any_data(&input, None)?;

        let col_idx = self.find_column_index(&data, &column)?;
        validation::validate_column_index(&data, col_idx)?;

        let ops = DataOperations::new();
        let replaced = ops.regex_replace(&mut data, col_idx, &pattern, &replacement)?;

        converter.write_any_data(&output, &data, None)?;
        crate::cli::runtime::log(format!("Replaced {replaced} cells; wrote {output}"));
        Ok(())
    }

    /// Handle the diff command
    pub fn handle_diff(
        &self,
        left: String,
        right: String,
        key: Option<String>,
    ) -> Result<()> {
        let converter = Converter::new();
        let left_data = converter.read_any_data(&left, None)?;
        let right_data = converter.read_any_data(&right, None)?;

        let key_col = key.as_ref().and_then(|k| {
            if left_data.is_empty() {
                None
            } else {
                left_data[0].iter().position(|h| h == k)
            }
        });

        let result = xls_rs::operations::diff(&left_data, &right_data, key_col)?;

        println!("Diff: {} left, {} right", left_data.len(), right_data.len());
        println!("  Removed: {} rows", result.removed.len());
        println!("  Added:   {} rows", result.added.len());
        println!("  Changed: {} rows", result.changed.len());

        if !result.removed.is_empty() {
            println!("\n--- Removed (only in left) ---");
            for row in result.removed.iter().take(10) {
                println!("  {}", row.join(", "));
            }
            if result.removed.len() > 10 {
                println!("  ... and {} more", result.removed.len() - 10);
            }
        }
        if !result.added.is_empty() {
            println!("\n--- Added (only in right) ---");
            for row in result.added.iter().take(10) {
                println!("  {}", row.join(", "));
            }
            if result.added.len() > 10 {
                println!("  ... and {} more", result.added.len() - 10);
            }
        }
        if !result.changed.is_empty() {
            println!("\n--- Changed ---");
            for c in result.changed.iter().take(5) {
                println!("  Key {}: {:?} -> {:?}", c.key, c.left, c.right);
            }
            if result.changed.len() > 5 {
                println!("  ... and {} more", result.changed.len() - 5);
            }
        }
        Ok(())
    }

    /// Handle the histogram command
    pub fn handle_histogram(
        &self,
        input: String,
        column: String,
        bins: usize,
        width: usize,
    ) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let col_idx = self.find_column_index(&data, &column)?;
        validation::validate_column_index(&data, col_idx)?;

        let histogram_bins = xls_rs::operations::histogram(&data, col_idx, bins)?;
        let rendered = xls_rs::operations::render_histogram(&histogram_bins, width, true);
        println!("Histogram for column '{column}':");
        println!("{rendered}");
        Ok(())
    }

    /// Simple formula evaluator for mutate command
    fn evaluate_formula(&self, data: &[Vec<String>], formula: &str) -> Result<Vec<String>> {
        // This is a simplified implementation
        // A full implementation would parse arithmetic expressions
        let mut results = Vec::new();

        for (_i, _row) in data.iter().enumerate().skip(1) {
            // For now, just return the formula as-is (placeholder)
            // A real implementation would evaluate the formula against row data
            results.push(formula.to_string());
        }

        Ok(results)
    }
}
