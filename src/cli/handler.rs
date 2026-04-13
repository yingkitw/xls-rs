//! Main command handler implementation
//!
//! This module provides the default command handler that delegates
//! to specialized command handlers based on the command type.

use crate::cli::{
    commands::{
        io::IoCommandHandler, pandas::PandasCommandHandler, transform::TransformCommandHandler,
        AdvancedCommandHandler,
    },
    Commands,
};
use anyhow::{Context, Result};

/// Default command handler
///
/// This handler delegates to specialized command handlers based on the command type.
pub struct DefaultCommandHandler {
    io: IoCommandHandler,
    transform: TransformCommandHandler,
    pandas: PandasCommandHandler,
    advanced: AdvancedCommandHandler,
}

impl DefaultCommandHandler {
    /// Create a new default command handler
    pub fn new() -> Self {
        Self {
            io: IoCommandHandler::new(),
            transform: TransformCommandHandler::new(),
            pandas: PandasCommandHandler::new(),
            advanced: AdvancedCommandHandler::new(),
        }
    }
}

impl Default for DefaultCommandHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl super::commands::CommandHandler for DefaultCommandHandler {
    /// Handle a command by delegating to the appropriate specialized handler
    fn handle(&self, command: Commands) -> Result<()> {
        match command {
            // I/O commands
            Commands::Read {
                input,
                sheet,
                range,
                format,
            } => self.io.handle_read(input, sheet, range, format),

            Commands::Write { output, csv, sheet } => self.io.handle_write(output, csv, sheet),

            Commands::Convert {
                input,
                output,
                sheet,
            } => self.io.handle_convert(input, output, sheet),

            Commands::Formula {
                input,
                output,
                formula,
                cell,
                sheet,
            } => self.io.handle_formula(input, output, formula, cell, sheet),

            Commands::Serve => self.io.handle_serve(),

            Commands::Sheets { input } => self.io.handle_sheets(input),

            Commands::ReadAll { input, format } => self.io.handle_read_all(input, format),

            Commands::WriteRange {
                input,
                output,
                start,
            } => self.io.handle_write_range(input, output, start),

            Commands::Append { source, target } => self.io.handle_append(source, target),

            // Transform commands
            Commands::Sort {
                input,
                output,
                column,
                ascending,
            } => self.transform.handle_sort(input, output, column, ascending),

            Commands::Filter {
                input,
                output,
                where_clause,
            } => self.transform.handle_filter(input, output, where_clause),

            Commands::Replace {
                input,
                output,
                find,
                replace,
                column,
            } => self
                .transform
                .handle_replace(input, output, find, replace, column),

            Commands::Dedupe {
                input,
                output,
                columns,
            } => self.transform.handle_dedupe(input, output, columns),

            Commands::Transpose { input, output } => self.transform.handle_transpose(input, output),

            Commands::Select {
                input,
                output,
                columns,
            } => self.transform.handle_select(input, output, columns),

            Commands::Rename {
                input,
                output,
                from,
                to,
            } => self.transform.handle_rename(input, output, from, to),

            Commands::Drop {
                input,
                output,
                columns,
            } => self.transform.handle_drop(input, output, columns),

            Commands::Fillna {
                input,
                output,
                value,
                columns,
            } => self.transform.handle_fillna(input, output, value, columns),

            Commands::Dropna { input, output } => self.transform.handle_dropna(input, output),

            Commands::Mutate {
                input,
                output,
                column,
                formula,
            } => self.transform.handle_mutate(input, output, column, formula),

            Commands::Query {
                input,
                output,
                where_clause,
            } => self.transform.handle_query(input, output, where_clause),

            Commands::Astype {
                input,
                output,
                column,
                target_type,
            } => self
                .transform
                .handle_astype(input, output, column, target_type),

            // Pandas-style commands
            Commands::Head { input, n, format } => self.pandas.handle_head(input, n, format),

            Commands::Tail { input, n, format } => self.pandas.handle_tail(input, n, format),

            Commands::Sample {
                input,
                n,
                seed,
                format,
            } => self.pandas.handle_sample(input, n, seed, format),

            Commands::Describe { input, format } => self.pandas.handle_describe(input, format),

            Commands::ValueCounts { input, column } => {
                self.pandas.handle_value_counts(input, column)
            }

            Commands::Corr { input, columns } => self.pandas.handle_corr(input, columns),

            Commands::Groupby {
                input,
                output,
                by,
                agg,
            } => self.pandas.handle_groupby(input, output, by, agg),

            Commands::Join {
                left,
                right,
                output,
                on,
                how,
            } => self.pandas.handle_join(left, right, output, on, how),

            Commands::Concat { inputs, output } => self.pandas.handle_concat(inputs, output),

            Commands::Unique { input, column } => self.pandas.handle_unique(input, column),

            Commands::Info { input } => self.pandas.handle_info(input),

            Commands::Dtypes { input } => self.pandas.handle_dtypes(input),

            Commands::Pivot {
                input,
                output,
                index,
                columns,
                values,
                agg,
            } => self
                .pandas
                .handle_pivot(input, output, index, columns, values, agg),

            Commands::Rolling {
                input,
                output,
                column,
                window,
                agg,
                name,
            } => self
                .pandas
                .handle_rolling(input, output, column, window, agg, name),

            Commands::Crosstab {
                input,
                output,
                rows,
                cols,
            } => self.pandas.handle_crosstab(input, output, rows, cols),

            Commands::Melt {
                input,
                output,
                id_vars,
                value_vars,
            } => self.pandas.handle_melt(input, output, id_vars, value_vars),

            // Advanced commands
            Commands::Schema { input, output } => self.advanced.handle_schema(input, output),

            Commands::ToSql {
                input,
                table,
                output,
                batch_size,
            } => self.advanced.handle_to_sql(input, table, output, batch_size),

            Commands::Profile { input, output } => self.advanced.handle_profile(input, output),

            Commands::Validate {
                input,
                rules,
                output,
                report,
            } => self.advanced.handle_validate(input, rules, output, report),

            Commands::Chart {
                input,
                output,
                chart_type,
                title,
                x_column,
                y_column,
            } => self
                .advanced
                .handle_chart(input, output, chart_type, title, x_column, y_column),

            Commands::Encrypt {
                input,
                output,
                algorithm,
                key_file,
            } => self
                .advanced
                .handle_encrypt(input, output, algorithm, key_file),

            Commands::Decrypt {
                input,
                output,
                key_file,
            } => self.advanced.handle_decrypt(input, output, key_file),

            Commands::Batch {
                inputs,
                output_dir,
                operation,
                args,
            } => self
                .advanced
                .handle_batch(inputs, output_dir, operation, args),

            Commands::Plugin {
                function,
                input,
                output,
                args,
            } => self.advanced.handle_plugin(function, input, output, args),

            Commands::Stream {
                input,
                output,
                chunk_size,
            } => self.advanced.handle_stream(input, output, chunk_size),

            Commands::Completions { shell } => self.advanced.handle_completions(shell),

            Commands::ExamplesGenerate => self.advanced.handle_examples_generate(),

            #[cfg(feature = "watch")]
            Commands::Watch { input, command } => self.advanced.handle_watch(input, command),

            Commands::ConfigInit => self.advanced.handle_config_init(),

            Commands::ExportStyled {
                input,
                output,
                style,
            } => self.advanced.handle_export_styled(input, output, style),

            // Google Sheets commands
            Commands::GSheetsList { spreadsheet } => self.io.handle_gsheets_list(spreadsheet),
            Commands::GSheetsAuth => self.io.handle_gsheets_auth(),
            Commands::GSheetsSetDefault { spreadsheet } => {
                self.io.handle_gsheets_set_default(spreadsheet)
            }

            Commands::Clip {
                input,
                output,
                column,
                min,
                max,
            } => {
                let converter = xls_rs::converter::Converter::new();
                let mut data = converter.read_any_data(&input, None)?;

                let col_idx = Self::find_column_index(&data, &column)?;
                validation::validate_column_index(&data, col_idx)?;

                let min_val: f64 = min
                    .parse()
                    .with_context(|| format!("Invalid min value: {}", min))?;
                let max_val: f64 = max
                    .parse()
                    .with_context(|| format!("Invalid max value: {}", max))?;

                let ops = xls_rs::operations::DataOperations::new();
                let clipped = ops.clip(&mut data, col_idx, Some(min_val), Some(max_val))?;

                converter.write_any_data(&output, &data, None)?;
                println!("Clipped {} cells; wrote {}", clipped, output);
                Ok(())
            }

            Commands::Normalize {
                input,
                output,
                column,
            } => {
                let converter = xls_rs::converter::Converter::new();
                let mut data = converter.read_any_data(&input, None)?;

                let col_idx = Self::find_column_index(&data, &column)?;
                validation::validate_column_index(&data, col_idx)?;

                let ops = xls_rs::operations::DataOperations::new();
                ops.normalize(&mut data, col_idx)?;

                converter.write_any_data(&output, &data, None)?;
                println!("Normalized column {}; wrote {}", column, output);
                Ok(())
            }

            Commands::Zscore {
                input,
                output,
                column,
            } => {
                let converter = xls_rs::converter::Converter::new();
                let mut data = converter.read_any_data(&input, None)?;

                let col_idx = Self::find_column_index(&data, &column)?;
                validation::validate_column_index(&data, col_idx)?;

                let ops = xls_rs::operations::DataOperations::new();
                ops.zscore(&mut data, col_idx)?;

                converter.write_any_data(&output, &data, None)?;
                println!("Z-score standardized column {}; wrote {}", column, output);
                Ok(())
            }

            Commands::ParseDate {
                input,
                output,
                column,
                from_format,
                to_format,
            } => {
                let converter = xls_rs::converter::Converter::new();
                let mut data = converter.read_any_data(&input, None)?;

                let col_idx = Self::find_column_index(&data, &column)?;
                validation::validate_column_index(&data, col_idx)?;

                let ops = xls_rs::operations::DataOperations::new();
                let converted = ops.parse_date(&mut data, col_idx, &from_format, &to_format)?;

                converter.write_any_data(&output, &data, None)?;
                println!("Converted {} dates; wrote {}", converted, output);
                Ok(())
            }

            Commands::RegexFilter {
                input,
                output,
                column,
                pattern,
            } => {
                let converter = xls_rs::converter::Converter::new();
                let data = converter.read_any_data(&input, None)?;

                let col_idx = Self::find_column_index(&data, &column)?;
                validation::validate_column_index(&data, col_idx)?;

                let ops = xls_rs::operations::DataOperations::new();
                let filtered = ops.regex_filter(&data, col_idx, &pattern)?;

                converter.write_any_data(&output, &filtered, None)?;
                println!(
                    "Filtered to {} rows; wrote {}",
                    filtered.len().saturating_sub(1),
                    output
                );
                Ok(())
            }

            Commands::RegexReplace {
                input,
                output,
                column,
                pattern,
                replacement,
            } => {
                let converter = xls_rs::converter::Converter::new();
                let mut data = converter.read_any_data(&input, None)?;

                let col_idx = Self::find_column_index(&data, &column)?;
                validation::validate_column_index(&data, col_idx)?;

                let ops = xls_rs::operations::DataOperations::new();
                let replaced = ops.regex_replace(&mut data, col_idx, &pattern, &replacement)?;

                converter.write_any_data(&output, &data, None)?;
                println!("Replaced {} cells; wrote {}", replaced, output);
                Ok(())
            }

            Commands::Diff { left, right, key } => {
                let converter = xls_rs::converter::Converter::new();
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

            Commands::Histogram {
                input,
                column,
                bins,
                width,
            } => {
                let converter = xls_rs::converter::Converter::new();
                let data = converter.read_any_data(&input, None)?;

                let col_idx = Self::find_column_index(&data, &column)?;
                validation::validate_column_index(&data, col_idx)?;

                let histogram_bins = xls_rs::operations::histogram(&data, col_idx, bins)?;
                let rendered =
                    xls_rs::operations::render_histogram(&histogram_bins, width, true);
                println!("Histogram for column '{}':", column);
                println!("{}", rendered);
                Ok(())
            }
        }
    }
}

// Import validation utility
use xls_rs::common::validation;

impl DefaultCommandHandler {
    /// Find column index by name (helper method)
    fn find_column_index(data: &[Vec<String>], column: &str) -> Result<usize> {
        if data.is_empty() {
            anyhow::bail!("Data is empty, cannot find column '{}'", column);
        }

        let header = &data[0];
        header
            .iter()
            .position(|h| h == column)
            .ok_or_else(|| anyhow::anyhow!("Column '{}' not found", column))
    }
}
