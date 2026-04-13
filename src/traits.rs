//! Trait definitions for xls-rs operations
//!
//! This module provides trait-based interfaces for better testability,
//! maintainability, and separation of concerns.

use crate::csv_handler::CellRange;
use anyhow::Result;

/// Trait for reading data from files
pub trait DataReader: Send + Sync {
    /// Read all data from a file
    fn read(&self, path: &str) -> Result<Vec<Vec<String>>>;

    /// Read data with headers (first row contains column names)
    fn read_with_headers(&self, path: &str) -> Result<Vec<Vec<String>>>;

    /// Read a specific cell range from a file
    fn read_range(&self, path: &str, range: &CellRange) -> Result<Vec<Vec<String>>>;

    /// Read data as JSON string
    fn read_as_json(&self, path: &str) -> Result<String>;

    /// Check if the file format is supported
    fn supports_format(&self, path: &str) -> bool;
}

/// Trait for writing data to files
pub trait DataWriter: Send + Sync {
    /// Write data to a file
    fn write(&self, path: &str, data: &[Vec<String>], options: DataWriteOptions) -> Result<()>;

    /// Write data to a specific cell range
    fn write_range(
        &self,
        path: &str,
        data: &[Vec<String>],
        start_row: usize,
        start_col: usize,
    ) -> Result<()>;

    /// Append data to an existing file
    fn append(&self, path: &str, data: &[Vec<String>]) -> Result<()>;

    /// Check if the file format is supported
    fn supports_format(&self, path: &str) -> bool;
}

/// Options for writing data
#[derive(Debug, Clone, Default)]
pub struct DataWriteOptions {
    /// Sheet name (for Excel/ODS files)
    pub sheet_name: Option<String>,
    /// Column names (for Parquet/Avro files)
    pub column_names: Option<Vec<String>>,
    /// Whether to include headers
    pub include_headers: bool,
}

/// Unified trait for file handlers that can both read and write
pub trait FileHandler: DataReader + DataWriter {
    /// Get the format name (e.g., "csv", "xlsx", "parquet")
    fn format_name(&self) -> &'static str;

    /// Get supported file extensions
    fn supported_extensions(&self) -> &'static [&'static str];
}

/// Trait for format detection
pub trait FormatDetector: Send + Sync {
    /// Detect the format of a file based on its path/extension
    fn detect_format(&self, path: &str) -> Result<String>;

    /// Check if a format is supported
    fn is_supported(&self, format: &str) -> bool;

    /// Get all supported formats
    fn supported_formats(&self) -> Vec<String>;
}

/// Trait for schema/metadata operations
pub trait SchemaProvider: Send + Sync {
    /// Get schema information from a file
    fn get_schema(&self, path: &str) -> Result<Vec<(String, String)>>;

    /// Get column names from a file
    fn get_column_names(&self, path: &str) -> Result<Vec<String>>;

    /// Get number of rows in a file
    fn get_row_count(&self, path: &str) -> Result<usize>;

    /// Get number of columns in a file
    fn get_column_count(&self, path: &str) -> Result<usize>;
}

/// Trait for streaming data reading
pub trait StreamingReader: Send + Sync {
    /// Open a file for streaming read
    fn open(&self, path: &str) -> Result<Box<dyn StreamingReadIterator>>;
}

/// Iterator for streaming reads
pub trait StreamingReadIterator: Iterator<Item = Result<Vec<String>>> {
    /// Get the current row number
    fn current_row(&self) -> usize;
}

/// Trait for streaming data writing
pub trait StreamingWriter: Send + Sync {
    /// Create a file for streaming write
    fn create(&self, path: &str) -> Result<Box<dyn StreamingWriteHandle>>;
}

/// Handle for streaming writes
pub trait StreamingWriteHandle: Send + Sync {
    /// Write a single row
    fn write_row(&mut self, row: &[String]) -> Result<()>;

    /// Get number of rows written
    fn rows_written(&self) -> usize;

    /// Flush buffered data
    fn flush(&mut self) -> Result<()>;
}

/// Trait for cell range operations
pub trait CellRangeProvider: Send + Sync {
    /// Parse a cell range string (e.g., "A1:C10")
    fn parse_range(&self, range_str: &str) -> Result<CellRange>;

    /// Convert row/column indices to cell reference (e.g., (0, 0) -> "A1")
    fn to_cell_reference(&self, row: usize, col: usize) -> String;

    /// Convert cell reference to row/column indices (e.g., "A1" -> (0, 0))
    fn from_cell_reference(&self, cell: &str) -> Result<(usize, usize)>;
}

/// Trait for sorting operations
pub trait SortOperator: Send + Sync {
    fn sort(&self, data: &mut Vec<Vec<String>>, column: usize, ascending: bool) -> Result<()>;
}

/// Trait for filtering operations
pub trait FilterOperator: Send + Sync {
    fn filter(
        &self,
        data: &[Vec<String>],
        column: usize,
        condition: FilterCondition,
    ) -> Result<Vec<Vec<String>>>;
}

/// Trait for transformation operations
pub trait TransformOperator: Send + Sync {
    fn transform(&self, data: &mut Vec<Vec<String>>, operation: TransformOperation) -> Result<()>;
}

/// Combined trait for all data operations
pub trait DataOperator: SortOperator + FilterOperator + TransformOperator {}

/// Filter condition for data operations
#[derive(Debug, Clone)]
pub enum FilterCondition {
    Equals(String),
    NotEquals(String),
    GreaterThan(String),
    GreaterThanOrEqual(String),
    LessThan(String),
    LessThanOrEqual(String),
    Contains(String),
    StartsWith(String),
    EndsWith(String),
    Regex(String),
}

/// Transform operation for data operations
#[derive(Debug, Clone)]
pub enum TransformOperation {
    RenameColumn {
        from: usize,
        to: String,
    },
    DropColumn(usize),
    AddColumn {
        name: String,
        formula: Option<String>,
    },
    FillNa {
        column: usize,
        value: String,
    },
}
