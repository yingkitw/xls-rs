//! Main command handler implementation
//!
//! This module provides the default command handler that delegates
//! to specialized command handlers based on the command type.

use crate::cli::{
    commands::{
        io::IoCommandHandler, pandas::PandasCommandHandler, transform::TransformCommandHandler,
        AdvancedCommandHandler,
    },
    format::OutputFormat,
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
            } => {
                let format = OutputFormat::resolve_for_read(format)?;
                self.io.handle_read(input, sheet, range, format)
            }

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

            #[cfg(feature = "mcp")]
            Commands::Serve => self.io.handle_serve(),

            Commands::Sheets { input } => self.io.handle_sheets(input),

            Commands::ReadAll { input, format } => {
                let format = OutputFormat::resolve_for_read(format)?;
                self.io.handle_read_all(input, format)
            }

            Commands::WriteRange {
                input,
                output,
                start,
                mode,
            } => self.io.handle_write_range(input, output, start, mode),

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
                method,
                stratum_column,
            } => self.pandas.handle_sample(input, n, seed, format, method, stratum_column),

            Commands::Describe { input, format } => self.pandas.handle_describe(input, format),

            Commands::ValueCounts { input, column } => {
                self.pandas.handle_value_counts(input, column)
            }

            Commands::Corr { input, columns, method } => {
                self.pandas.handle_corr(input, columns, &method)
            }

            Commands::Regress {
                input,
                x_column,
                y_column,
            } => self.pandas.handle_regress(input, x_column, y_column),

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

            Commands::PivotLonger {
                input,
                output,
                cols,
                names_to,
                values_to,
            } => self
                .pandas
                .handle_pivot_longer(input, output, cols, names_to, values_to),

            Commands::PivotWider {
                input,
                output,
                names_from,
                values_from,
                id_cols,
            } => self.pandas.handle_pivot_wider(
                input,
                output,
                names_from,
                values_from,
                id_cols,
            ),

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

            Commands::ExamplesGenerate => self.advanced.handle_examples_generate(),

            Commands::ConfigInit => self.advanced.handle_config_init(),

            Commands::ExportStyled {
                input,
                output,
                style,
            } => self.advanced.handle_export_styled(input, output, style),

            Commands::AddChart {
                input,
                output,
                chart_type,
                title,
                category_column,
                value_columns,
            } => self
                .advanced
                .handle_add_chart(input, output, chart_type, title, category_column, value_columns),

            Commands::AddSparkline {
                output,
                data_range,
                sparkline_cell,
                sheet,
            } => self
                .advanced
                .handle_add_sparkline(output, data_range, sparkline_cell, sheet),

            Commands::ConditionalFormat {
                output,
                range,
                condition,
                bg_color,
                font_color,
                bold,
                sheet,
            } => self.advanced.handle_conditional_format(
                output,
                range,
                condition,
                bg_color,
                font_color,
                bold,
                sheet,
            ),

            Commands::ApplyFormulaRange {
                input,
                output,
                formula,
                range,
                sheet,
            } => self
                .advanced
                .handle_apply_formula_range(input, output, formula, range, sheet),

            Commands::Clip {
                input,
                output,
                column,
                min,
                max,
            } => {
                let min_val: f64 = min
                    .parse()
                    .with_context(|| format!("Invalid min value: {}", min))?;
                let max_val: f64 = max
                    .parse()
                    .with_context(|| format!("Invalid max value: {}", max))?;
                self.transform.handle_clip(input, output, column, min_val, max_val)
            }

            Commands::Normalize {
                input,
                output,
                column,
            } => self.transform.handle_normalize(input, output, column),

            Commands::Zscore {
                input,
                output,
                column,
            } => self.transform.handle_zscore(input, output, column),

            Commands::ParseDate {
                input,
                output,
                column,
                from_format,
                to_format,
            } => self.transform.handle_parse_date(input, output, column, from_format, to_format),

            Commands::RegexFilter {
                input,
                output,
                column,
                pattern,
            } => self.transform.handle_regex_filter(input, output, column, pattern),

            Commands::RegexReplace {
                input,
                output,
                column,
                pattern,
                replacement,
            } => self.transform.handle_regex_replace(input, output, column, pattern, replacement),

            Commands::Diff { left, right, key } => {
                self.transform.handle_diff(left, right, key)
            }

            Commands::StrDistance { a, b, method } => {
                use xls_rs::string_distance;
                match method.as_str() {
                    "levenshtein" => {
                        let d = string_distance::levenshtein(&a, &b);
                        println!("Levenshtein distance: {}", d);
                    }
                    "jaro" => {
                        let s = string_distance::jaro(&a, &b);
                        println!("Jaro similarity: {:.4}", s);
                    }
                    "jaro-winkler" | "jaro_winkler" => {
                        let s = string_distance::jaro_winkler(&a, &b);
                        println!("Jaro-Winkler similarity: {:.4}", s);
                    }
                    "hamming" => {
                        match string_distance::hamming(&a, &b) {
                            Some(d) => println!("Hamming distance: {}", d),
                            None => anyhow::bail!(
                                "Hamming distance requires strings of equal length (got {} and {})",
                                a.len(),
                                b.len()
                            ),
                        }
                    }
                    _ => anyhow::bail!(
                        "Unknown method '{}'. Use: levenshtein, jaro, jaro-winkler, hamming",
                        method
                    ),
                }
                Ok(())
            }

            Commands::Histogram {
                input,
                column,
                bins,
                width,
            } => self.transform.handle_histogram(input, column, bins, width),
        }
    }
}

