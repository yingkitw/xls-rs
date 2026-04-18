//! CLI command definitions and handlers
//!
//! This module is organized into submodules for better maintainability:
//! - `commands/io`: File I/O commands (read, write, convert)
//! - `commands/transform`: Data transformation commands (sort, filter, etc.)
//! - `commands/pandas`: Pandas-style operations (head, tail, join, etc.)
//! - `commands/advanced`: Advanced features (validate, chart, batch, etc.)

pub mod commands;
pub mod format;
pub mod handler;
pub mod runtime;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub use commands::CommandHandler;
pub use format::OutputFormat;
pub use handler::DefaultCommandHandler;

/// CLI structure
#[derive(Parser)]
#[command(name = "xls-rs")]
#[command(
    about = "A CLI tool for reading, writing, converting spreadsheet files with formula support",
    version = env!("CARGO_PKG_VERSION")
)]
pub struct Cli {
    /// Path to a config file (overrides discovery)
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Suppress non-data output (logs/progress)
    #[arg(long, default_value_t = false)]
    pub quiet: bool,

    /// Print additional debug output (logs/progress)
    #[arg(long, default_value_t = false)]
    pub verbose: bool,

    /// Allow overwriting output files
    #[arg(long, default_value_t = false)]
    pub overwrite: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// CLI commands
///
/// This enum represents all available commands in the xls-rs CLI.
/// Each command variant includes its specific parameters.
#[derive(Subcommand)]
pub enum Commands {
    /// Read data from a file and display it
    Read {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        sheet: Option<String>,
        #[arg(short, long)]
        range: Option<String>,
        /// If omitted, uses `default_format` from config (or csv).
        #[arg(short = 'f', long)]
        format: Option<OutputFormat>,
    },

    /// Write data to a file
    Write {
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        csv: Option<String>,
        #[arg(short, long)]
        sheet: Option<String>,
    },

    /// Convert between file formats
    Convert {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        sheet: Option<String>,
    },

    /// Apply formulas to a file
    Formula {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        formula: String,
        #[arg(short, long)]
        cell: String,
        #[arg(short, long)]
        sheet: Option<String>,
    },

    /// Start MCP server
    Serve,

    /// Sort data by column
    Sort {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        column: String,
        #[arg(short, long)]
        ascending: bool,
    },

    /// Filter rows by condition
    Filter {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short = 'w', long)]
        where_clause: String,
    },

    /// Find and replace values
    Replace {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        find: String,
        #[arg(short, long)]
        replace: String,
        #[arg(short, long)]
        column: Option<String>,
    },

    /// Remove duplicate rows
    Dedupe {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        columns: Option<String>,
    },

    /// Transpose data (rows to columns)
    Transpose {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
    },

    /// Append data to existing file
    Append {
        #[arg(short, long)]
        source: String,
        #[arg(short, long)]
        target: String,
    },

    /// List sheets in Excel file
    Sheets {
        #[arg(short, long)]
        input: String,
    },

    /// Read all sheets from Excel file
    ReadAll {
        #[arg(short, long)]
        input: String,
        /// If omitted, uses `default_format` from config (or csv).
        #[arg(short = 'f', long)]
        format: Option<OutputFormat>,
    },

    /// Write data to specific cell range
    WriteRange {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        start: String,
    },

    /// Select specific columns
    Select {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        columns: String,
    },

    /// Show first N rows
    Head {
        #[arg(short, long)]
        input: String,
        #[arg(short = 'n', long, default_value = "10")]
        n: usize,
        #[arg(short = 'f', long, default_value = "csv")]
        format: OutputFormat,
    },

    /// Show last N rows
    Tail {
        #[arg(short, long)]
        input: String,
        #[arg(short = 'n', long, default_value = "10")]
        n: usize,
        #[arg(short = 'f', long, default_value = "csv")]
        format: OutputFormat,
    },

    /// Sample random rows
    Sample {
        #[arg(short, long)]
        input: String,
        #[arg(short = 'n', long, default_value = "10")]
        n: usize,
        #[arg(short, long)]
        seed: Option<u64>,
        #[arg(short = 'f', long, default_value = "csv")]
        format: OutputFormat,
    },

    /// Show descriptive statistics
    Describe {
        #[arg(short, long)]
        input: String,
        #[arg(short = 'f', long, default_value = "csv")]
        format: OutputFormat,
    },

    /// Count unique values in column
    ValueCounts {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        column: String,
    },

    /// Calculate correlation matrix
    Corr {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        columns: Option<String>,
    },

    /// Group by column with aggregation
    Groupby {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        by: String,
        #[arg(short, long)]
        agg: String,
    },

    /// Join/merge two files
    Join {
        #[arg(short, long)]
        left: String,
        #[arg(short, long)]
        right: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        on: String,
        #[arg(short, long)]
        how: String,
    },

    /// Concatenate multiple files
    Concat {
        #[arg(short, long)]
        inputs: String,
        #[arg(short, long)]
        output: String,
    },

    /// Add computed column
    Mutate {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        column: String,
        #[arg(short, long)]
        formula: String,
    },

    /// Rename columns
    Rename {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        from: String,
        #[arg(short, long)]
        to: String,
    },

    /// Drop columns
    Drop {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        columns: String,
    },

    /// Fill missing values
    Fillna {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        value: String,
        #[arg(short, long)]
        columns: Option<String>,
    },

    /// Drop rows with missing values
    Dropna {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
    },

    /// Show column data types
    Dtypes {
        #[arg(short, long)]
        input: String,
    },

    /// Cast column types
    Astype {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        column: String,
        #[arg(short = 't', long)]
        target_type: String,
    },

    /// Get unique values
    Unique {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        column: String,
    },

    /// Show dataset info
    Info {
        #[arg(short, long)]
        input: String,
    },

    /// Clip values to range
    Clip {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        column: String,
        #[arg(short, long)]
        min: String,
        #[arg(short, long)]
        max: String,
    },

    /// Normalize column (0-1)
    Normalize {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        column: String,
    },

    /// Standardize column to z-scores (mean 0, std 1) for ML / statistics
    Zscore {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        column: String,
    },

    /// Query with SQL-like syntax
    Query {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short = 'w', long)]
        where_clause: String,
    },

    /// Create pivot table
    Pivot {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        index: String,
        #[arg(short, long)]
        columns: String,
        #[arg(short, long)]
        values: String,
        #[arg(short, long)]
        agg: String,
    },

    /// Rolling window mean or sum on a column (data rows ordered top to bottom)
    Rolling {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        column: String,
        #[arg(short, long)]
        window: usize,
        /// Aggregation: `mean`, `sum`, `avg` (same as mean)
        #[arg(long, default_value = "mean")]
        agg: String,
        /// Name for the new column (default: `{column}_roll{window}_{mean|sum}`)
        #[arg(long)]
        name: Option<String>,
    },

    /// Crosstab counts for two categorical columns
    Crosstab {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        rows: String,
        #[arg(short, long)]
        cols: String,
    },

    /// Melt / unpivot to long form (id columns + variable + value)
    Melt {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        /// Comma-separated id column names
        #[arg(long)]
        id_vars: String,
        /// Comma-separated value column names (omit to use all other columns)
        #[arg(long)]
        value_vars: Option<String>,
    },

    /// Parse and convert dates
    ParseDate {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        column: String,
        #[arg(short, long)]
        from_format: String,
        #[arg(short, long)]
        to_format: String,
    },

    /// Filter by regex pattern
    RegexFilter {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        column: String,
        #[arg(short, long)]
        pattern: String,
    },

    /// Replace by regex pattern
    RegexReplace {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        column: String,
        #[arg(short, long)]
        pattern: String,
        #[arg(short, long)]
        replacement: String,
    },

    /// Compare two datasets (diff)
    Diff {
        #[arg(short, long)]
        left: String,
        #[arg(short, long)]
        right: String,
        #[arg(short, long)]
        key: Option<String>,
    },

    /// Display ASCII histogram for numeric column
    Histogram {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        column: String,
        #[arg(short = 'n', long, default_value = "10")]
        bins: usize,
        #[arg(short, long, default_value = "40")]
        width: usize,
    },

    /// Export schema (column names and types) as JSON
    #[command(name = "schema")]
    Schema {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Generate SQL INSERT statements from data
    #[command(name = "to-sql")]
    ToSql {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        table: String,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long)]
        batch_size: Option<usize>,
    },

    /// Profile data quality
    Profile {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Validate data with rules
    Validate {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        rules: String,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(short, long)]
        report: Option<String>,
    },

    /// Create chart from data
    Chart {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        chart_type: String,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        x_column: Option<String>,
        #[arg(short, long)]
        y_column: Option<String>,
    },

    /// Encrypt file
    Encrypt {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        algorithm: String,
        #[arg(short, long)]
        key_file: Option<String>,
    },

    /// Decrypt file
    Decrypt {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        key_file: Option<String>,
    },

    /// Batch process multiple files
    Batch {
        #[arg(short, long)]
        inputs: String,
        #[arg(short, long)]
        output_dir: String,
        #[arg(short, long)]
        operation: String,
        #[arg(short, long)]
        args: Vec<String>,
    },

    /// Run plugin function
    Plugin {
        #[arg(short, long)]
        function: String,
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        args: Vec<String>,
    },

    /// Stream process large file
    Stream {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(long, default_value_t = 1000)]
        chunk_size: usize,
    },

    /// Generate shell completions
    Completions {
        #[arg(short, long)]
        shell: String,
    },

    /// Generate deterministic example files under ./examples
    ExamplesGenerate,

    /// Watch file and re-run command on change
    #[cfg(feature = "watch")]
    Watch {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        command: String,
    },

    /// Initialize config file
    ConfigInit,

    /// Export styled Excel
    ExportStyled {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        style: Option<String>,
    },

    /// Add chart to Excel file
    AddChart {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        chart_type: String,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        category_column: Option<usize>,
        #[arg(short, long)]
        value_columns: Option<Vec<usize>>,
    },

    /// Add sparkline to Excel file
    AddSparkline {
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        data_range: String,
        #[arg(short, long)]
        sparkline_cell: String,
        #[arg(short, long)]
        sheet: Option<String>,
    },

    /// Add conditional formatting to Excel range
    ConditionalFormat {
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        range: String,
        #[arg(short, long)]
        condition: String,
        #[arg(short, long)]
        bg_color: Option<String>,
        #[arg(short, long)]
        font_color: Option<String>,
        #[arg(short = 'b', long)]
        bold: Option<bool>,
        #[arg(short, long)]
        sheet: Option<String>,
    },

    /// Apply formula to range in Excel file
    ApplyFormulaRange {
        #[arg(short, long)]
        input: String,
        #[arg(short, long)]
        output: String,
        #[arg(short, long)]
        formula: String,
        #[arg(short, long)]
        range: String,
        #[arg(short, long)]
        sheet: Option<String>,
    },

    /// List sheets in Google Sheets
    GSheetsList {
        #[arg(short, long)]
        spreadsheet: String,
    },

    /// Authorize Google Sheets access
    GSheetsAuth,

    /// Set default Google Sheets spreadsheet
    GSheetsSetDefault {
        #[arg(short, long)]
        spreadsheet: String,
    },
}

/// Execute a CLI command
///
/// This is the main entry point for command execution.
pub fn run(command: Commands) -> anyhow::Result<()> {
    let handler = DefaultCommandHandler::new();
    handler.handle(command)
}
