//! Pandas-style command handlers
//!
//! Implements pandas-inspired operations like head, tail, join, groupby, concat, etc.

use crate::cli::OutputFormat;
use xls_rs::{
    common::validation,
    converter::Converter,
    operations::{AggFunc, DataOperations, JoinType},
};
use anyhow::Result;

/// Pandas-style operation command handler
#[derive(Default)]
pub struct PandasCommandHandler;

impl PandasCommandHandler {
    /// Create a new pandas command handler
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle the head command
    ///
    /// Displays the first N rows of data.
    pub fn handle_head(&self, input: String, n: usize, format: OutputFormat) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let ops = DataOperations::new();
        let head_data = ops.head(&data, n);

        // Output in requested format
        self.print_data(&head_data, format)?;

        Ok(())
    }

    /// Handle the tail command
    ///
    /// Displays the last N rows of data.
    pub fn handle_tail(&self, input: String, n: usize, format: OutputFormat) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let ops = DataOperations::new();
        let tail_data = ops.tail(&data, n);

        // Output in requested format
        self.print_data(&tail_data, format)?;

        Ok(())
    }

    /// Handle the sample command
    ///
    /// Displays a random sample of N rows.
    pub fn handle_sample(
        &self,
        input: String,
        n: usize,
        seed: Option<u64>,
        format: OutputFormat,
    ) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let ops = DataOperations::new();
        let sample_data = ops.sample(&data, n, seed);

        // Output in requested format
        self.print_data(&sample_data, format)?;

        Ok(())
    }

    /// Handle the describe command
    ///
    /// Displays descriptive statistics for the data.
    pub fn handle_describe(&self, input: String, format: OutputFormat) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let ops = DataOperations::new();
        let description = ops.describe(&data)?;

        // Output in requested format
        self.print_data(&description, format)?;

        Ok(())
    }

    /// Handle the value_counts command
    ///
    /// Counts unique values in a column.
    pub fn handle_value_counts(&self, input: String, column: String) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let col_idx = self.find_column_index(&data, &column)?;
        validation::validate_column_index(&data, col_idx)?;

        let ops = DataOperations::new();
        let counts = ops.value_counts(&data, col_idx);

        println!("Value counts for column '{column}':");
        for row in &counts[1..] {
            if row.len() >= 2 {
                println!("  {}: {}", row[0], row[1]);
            }
        }

        Ok(())
    }

    /// Handle the corr command
    ///
    /// Calculates the correlation matrix for numeric columns.
    pub fn handle_corr(&self, input: String, columns: Option<String>) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let col_indices = if let Some(cols_str) = columns {
            cols_str
                .split(',')
                .map(|c| self.find_column_index(&data, c.trim()))
                .collect::<Result<Vec<_>>>()?
        } else {
            // Use all numeric columns
            self.find_numeric_columns(&data)?
        };

        let ops = DataOperations::new();
        let corr_matrix = ops.correlation(&data, &col_indices)?;

        println!("Correlation Matrix:");
        for row in &corr_matrix {
            for val in row {
                print!("{val} ");
            }
            println!();
        }

        Ok(())
    }

    /// Handle the groupby command
    ///
    /// Groups data by a column and applies an aggregation function.
    pub fn handle_groupby(
        &self,
        input: String,
        output: String,
        by: String,
        agg: String,
    ) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let by_idx = self.find_column_index(&data, &by)?;
        validation::validate_column_index(&data, by_idx)?;

        // Parse aggregation function
        let agg_func = AggFunc::from_str(&agg)?;

        // For simple groupby, aggregate the first value column (column 1 if exists)
        let value_col = if data[0].len() > 1 { 1 } else { 0 };
        let aggregations = vec![(value_col, agg_func)];

        let ops = DataOperations::new();
        let grouped = ops.groupby(&data, by_idx, &aggregations)?;

        converter.write_any_data(&output, &grouped, None)?;
        println!("Grouped by '{by}' with '{agg}' aggregation; wrote {output}");

        Ok(())
    }

    /// Handle the join command
    ///
    /// Joins two files on a common column.
    pub fn handle_join(
        &self,
        left: String,
        right: String,
        output: String,
        on: String,
        how: String,
    ) -> Result<()> {
        let converter = Converter::new();

        // Read both files
        let left_data = converter.read_any_data(&left, None)?;
        let right_data = converter.read_any_data(&right, None)?;

        // Find column indices
        let left_col = self.find_column_index(&left_data, &on)?;
        let right_col = self.find_column_index(&right_data, &on)?;

        validation::validate_column_index(&left_data, left_col)?;
        validation::validate_column_index(&right_data, right_col)?;

        // Parse join type
        let join_type = JoinType::from_str(&how)?;

        let ops = DataOperations::new();
        let joined = ops.join(&left_data, &right_data, left_col, right_col, join_type)?;

        converter.write_any_data(&output, &joined, None)?;
        println!("Joined {left} and {right} on '{on}' ({how} join); wrote {output}");

        Ok(())
    }

    /// Handle the concat command
    ///
    /// Concatenates multiple files vertically.
    pub fn handle_concat(&self, inputs: String, output: String) -> Result<()> {
        let converter = Converter::new();

        // Parse input files (glob pattern or comma-separated)
        let input_files: Vec<String> = if inputs.contains('*') {
            // Use glob
            glob::glob(&inputs)?
                .filter_map(|entry| match entry {
                    Ok(path) => Some(path),
                    Err(e) => {
                        eprintln!("Warning: glob error for pattern '{}': {}", inputs, e);
                        None
                    }
                })
                .filter(|entry| entry.is_file())
                .map(|entry| entry.to_string_lossy().to_string())
                .collect()
        } else {
            inputs.split(',').map(|s| s.trim().to_string()).collect()
        };

        if input_files.is_empty() {
            anyhow::bail!("No input files found for pattern: {inputs}");
        }

        // Read all datasets
        let datasets: Result<Vec<Vec<Vec<String>>>> = input_files
            .iter()
            .map(|path| converter.read_any_data(path, None))
            .collect();
        let datasets = datasets?;

        let ops = DataOperations::new();
        let concatenated = ops.concat(&datasets);

        converter.write_any_data(&output, &concatenated, None)?;
        println!("Concatenated {} files; wrote {}", input_files.len(), output);

        Ok(())
    }

    /// Handle the unique command
    ///
    /// Returns unique values from a column.
    pub fn handle_unique(&self, input: String, column: String) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let col_idx = self.find_column_index(&data, &column)?;
        validation::validate_column_index(&data, col_idx)?;

        let ops = DataOperations::new();
        let unique = ops.unique(&data, col_idx);

        println!("Unique values in column '{column}':");
        for row in &unique[1..] {
            if let Some(val) = row.first() {
                println!("  {val}");
            }
        }

        Ok(())
    }

    /// Handle the info command
    ///
    /// Displays information about the dataset.
    pub fn handle_info(&self, input: String) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        println!("Dataset Info:");
        println!("  Rows: {}", data.len().saturating_sub(1));
        println!("  Columns: {}", data.first().map(|h| h.len()).unwrap_or(0));

        if let Some(header) = data.first() {
            println!("\nColumns:");
            for (i, col) in header.iter().enumerate() {
                // Detect type
                let mut type_count: std::collections::HashMap<&str, usize> =
                    std::collections::HashMap::new();
                for row in data.iter().skip(1) {
                    if let Some(cell) = row.get(i) {
                        let dtype = if cell.parse::<f64>().is_ok() {
                            "number"
                        } else if cell.is_empty() {
                            "empty"
                        } else {
                            "string"
                        };
                        *type_count.entry(dtype).or_insert(0) += 1;
                    }
                }

                let most_common = type_count
                    .iter()
                    .max_by_key(|&(_, count)| count)
                    .map(|(dtype, _)| *dtype)
                    .unwrap_or("unknown");

                let non_empty = data.len() - 1 - type_count.get("empty").unwrap_or(&0);
                println!("  {}: {} ({} non-null)", col, most_common, non_empty);
            }
        }

        Ok(())
    }

    /// Handle the dtypes command
    ///
    /// Shows the data types of each column.
    pub fn handle_dtypes(&self, input: String) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        if let Some(header) = data.first() {
            println!("Column Types:");
            for (i, col) in header.iter().enumerate() {
                // Detect type
                let mut has_numbers = false;
                let mut has_strings = false;

                for row in data.iter().skip(1) {
                    if let Some(cell) = row.get(i) {
                        if cell.is_empty() {
                            continue;
                        }
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

                println!("  {}: {}", col, dtype);
            }
        }

        Ok(())
    }

    /// Handle the pivot command
    ///
    /// Creates a pivot table.
    pub fn handle_pivot(
        &self,
        input: String,
        output: String,
        index: String,
        columns: String,
        values: String,
        agg: String,
    ) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let index_idx = self.find_column_index(&data, &index)?;
        let cols_idx = self.find_column_index(&data, &columns)?;
        let vals_idx = self.find_column_index(&data, &values)?;

        let agg_func = AggFunc::from_str(&agg)?;

        let ops = DataOperations::new();
        let pivoted = ops.pivot(&data, index_idx, cols_idx, vals_idx, agg_func)?;

        converter.write_any_data(&output, &pivoted, None)?;
        println!("Created pivot table; wrote {}", output);

        Ok(())
    }

    /// Rolling mean or sum with a fixed window over ordered rows.
    pub fn handle_rolling(
        &self,
        input: String,
        output: String,
        column: String,
        window: usize,
        agg: String,
        name: Option<String>,
    ) -> Result<()> {
        let converter = Converter::new();
        let mut data = converter.read_any_data(&input, None)?;

        let col_idx = self.find_column_index(&data, &column)?;
        validation::validate_column_index(&data, col_idx)?;

        let agg_lower = agg.to_lowercase();
        let ops = DataOperations::new();
        let new_name = match agg_lower.as_str() {
            "sum" => {
                let new_name = name.unwrap_or_else(|| format!("{column}_roll{window}_sum"));
                ops.rolling_sum_column(&mut data, col_idx, window, &new_name)?;
                new_name
            }
            "mean" | "avg" => {
                let new_name = name.unwrap_or_else(|| format!("{column}_roll{window}_mean"));
                ops.rolling_mean_column(&mut data, col_idx, window, &new_name)?;
                new_name
            }
            other => anyhow::bail!("unknown rolling aggregation '{other}' (use mean or sum)"),
        };

        converter.write_any_data(&output, &data, None)?;
        println!(
            "Rolling {agg_lower} with window={window}; added column '{new_name}'; wrote {output}"
        );

        Ok(())
    }

    /// Crosstab frequency table for two columns.
    pub fn handle_crosstab(
        &self,
        input: String,
        output: String,
        rows: String,
        cols: String,
    ) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let row_idx = self.find_column_index(&data, &rows)?;
        let col_idx = self.find_column_index(&data, &cols)?;
        validation::validate_column_index(&data, row_idx)?;
        validation::validate_column_index(&data, col_idx)?;

        let ops = DataOperations::new();
        let out = ops.crosstab(&data, row_idx, col_idx)?;

        converter.write_any_data(&output, &out, None)?;
        println!("Crosstab '{rows}' × '{cols}'; wrote {output}");

        Ok(())
    }

    /// Melt wide data to long tidy form.
    pub fn handle_melt(
        &self,
        input: String,
        output: String,
        id_vars: String,
        value_vars: Option<String>,
    ) -> Result<()> {
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        let id_indices = self.parse_column_names(&data, &id_vars)?;
        let value_indices = if let Some(vs) = value_vars {
            self.parse_column_names(&data, &vs)?
        } else {
            Vec::new()
        };

        let ops = DataOperations::new();
        let melted = ops.melt(&data, &id_indices, &value_indices)?;

        converter.write_any_data(&output, &melted, None)?;
        println!("Melt; wrote {output}");

        Ok(())
    }

    fn parse_column_names(&self, data: &[Vec<String>], columns: &str) -> Result<Vec<usize>> {
        columns
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|name| self.find_column_index(data, name))
            .collect()
    }

    /// Find column index by name
    fn find_column_index(&self, data: &[Vec<String>], column: &str) -> Result<usize> {
        if data.is_empty() {
            anyhow::bail!("Data is empty, cannot find column '{}'", column);
        }

        let header = &data[0];
        header
            .iter()
            .position(|h| h == column)
            .ok_or_else(|| anyhow::anyhow!("Column '{}' not found", column))
    }

    /// Find all numeric columns
    fn find_numeric_columns(&self, data: &[Vec<String>]) -> Result<Vec<usize>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut numeric_cols = Vec::new();
        let num_cols = data[0].len();

        for i in 0..num_cols {
            let mut is_numeric = true;
            for row in data.iter().skip(1) {
                if let Some(cell) = row.get(i) {
                    if !cell.is_empty() && cell.parse::<f64>().is_err() {
                        is_numeric = false;
                        break;
                    }
                }
            }
            if is_numeric {
                numeric_cols.push(i);
            }
        }

        Ok(numeric_cols)
    }

    /// Print data in the specified format
    fn print_data(&self, data: &[Vec<String>], format: OutputFormat) -> Result<()> {
        match format {
            OutputFormat::Csv => {
                for row in data {
                    println!("{}", row.join(","));
                }
            }
            OutputFormat::Jsonl => {
                if data.is_empty() {
                    return Ok(());
                }
                let headers = &data[0];
                for row in &data[1..] {
                    let mut obj = serde_json::Map::new();
                    for (i, header) in headers.iter().enumerate() {
                        let value = row.get(i).map(|s| s.as_str()).unwrap_or("");
                        obj.insert(header.clone(), serde_json::json!(value));
                    }
                    println!("{}", serde_json::to_string(&serde_json::Value::Object(obj))?);
                }
            }
            OutputFormat::Json => {
                if data.is_empty() {
                    println!("[]");
                    return Ok(());
                }

                let headers = &data[0];
                let rows: Vec<serde_json::Value> = data[1..]
                    .iter()
                    .map(|row| {
                        let mut obj = serde_json::Map::new();
                        for (i, header) in headers.iter().enumerate() {
                            let value = row.get(i).map(|s| s.as_str()).unwrap_or("");
                            obj.insert(header.clone(), serde_json::json!(value));
                        }
                        serde_json::Value::Object(obj)
                    })
                    .collect();

                println!("{}", serde_json::to_string_pretty(&rows)?);
            }
            OutputFormat::Markdown => {
                if data.is_empty() {
                    return Ok(());
                }

                // Calculate column widths
                let num_cols = data.iter().map(|r| r.len()).max().unwrap_or(0);
                let mut col_widths = vec![0; num_cols];

                for row in data {
                    for (i, cell) in row.iter().enumerate() {
                        if i < col_widths.len() {
                            col_widths[i] = col_widths[i].max(cell.len());
                        }
                    }
                }

                // Print header
                if let Some(header) = data.first() {
                    for (i, cell) in header.iter().enumerate() {
                        print!("| {:<width$} ", cell, width = col_widths[i]);
                    }
                    println!("|");

                    // Print separator
                    for width in &col_widths {
                        print!("|-{:<width$}-", "", width = width);
                    }
                    println!("|");
                }

                // Print data rows
                for row in &data[1..] {
                    for (i, cell) in row.iter().enumerate() {
                        if i < col_widths.len() {
                            print!("| {:<width$} ", cell, width = col_widths[i]);
                        }
                    }
                    println!("|");
                }
            }
        }

        Ok(())
    }
}
