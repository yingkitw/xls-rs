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
        Self::default()
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

        // Sort data
        let ops = DataOperations::new();
        let order = if ascending {
            SortOrder::Ascending
        } else {
            SortOrder::Descending
        };
        ops.sort_by_column(&mut data, col_idx, order)?;

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
                if let Some(cell) = row.get_mut(col_idx) {
                    if cell.contains(&find) {
                        *cell = cell.replace(&find, &replace);
                        count += 1;
                    }
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
                    if let Some(cell) = row.get_mut(*col_idx) {
                        if cell.is_empty() {
                            *cell = value.clone();
                            count += 1;
                        }
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
        if let Some(header) = data.first_mut() {
            if !header.contains(&column) {
                header.push(column.clone());
            }
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
