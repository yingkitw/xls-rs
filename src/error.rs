//! Enhanced error types with context information

use std::fmt;
use thiserror::Error;

/// Error with file and location context
#[derive(Error, Debug)]
pub struct XlsRsError {
    pub kind: ErrorKind,
    pub context: ErrorContext,
}

impl fmt::Display for XlsRsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;
        if let Some(file) = &self.context.file {
            write!(f, " in file '{}'", file)?;
        }
        if let Some(row) = self.context.row {
            write!(f, " at row {}", row + 1)?; // 1-indexed for users
        }
        if let Some(col) = self.context.column {
            write!(f, ", column {}", col + 1)?;
        }
        if let Some(cell) = &self.context.cell_ref {
            write!(f, " (cell {})", cell)?;
        }
        Ok(())
    }
}


/// Error context with location information
#[derive(Debug, Default, Clone)]
pub struct ErrorContext {
    /// File path
    pub file: Option<String>,
    /// Row number (0-indexed)
    pub row: Option<usize>,
    /// Column number (0-indexed)
    pub column: Option<usize>,
    /// Cell reference (e.g., "A1")
    pub cell_ref: Option<String>,
    /// Column name
    pub column_name: Option<String>,
}

impl ErrorContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_file(mut self, file: &str) -> Self {
        self.file = Some(file.to_string());
        self
    }

    pub fn with_row(mut self, row: usize) -> Self {
        self.row = Some(row);
        self
    }

    pub fn with_column(mut self, col: usize) -> Self {
        self.column = Some(col);
        self
    }

    pub fn with_cell_ref(mut self, cell: &str) -> Self {
        self.cell_ref = Some(cell.to_string());
        self
    }

    pub fn with_column_name(mut self, name: &str) -> Self {
        self.column_name = Some(name.to_string());
        self
    }
}

/// Types of errors
#[derive(Error, Debug)]
pub enum ErrorKind {
    #[error("Column '{0}' not found")]
    ColumnNotFound(String),

    #[error("Invalid cell reference '{0}'")]
    InvalidCellRef(String),

    #[error("Invalid value '{0}' - expected {1}")]
    InvalidValue(String, String),

    #[error("Type conversion failed: cannot convert '{0}' to {1}")]
    TypeConversion(String, String),

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Invalid formula: {0}")]
    InvalidFormula(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid date format: {0}")]
    InvalidDateFormat(String),

    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("{0}")]
    Other(String),
}

impl XlsRsError {
    pub fn column_not_found(name: &str) -> Self {
        Self {
            kind: ErrorKind::ColumnNotFound(name.to_string()),
            context: ErrorContext::new(),
        }
    }

    pub fn invalid_value(value: &str, expected: &str) -> Self {
        Self {
            kind: ErrorKind::InvalidValue(value.to_string(), expected.to_string()),
            context: ErrorContext::new(),
        }
    }

    pub fn type_conversion(value: &str, target_type: &str) -> Self {
        Self {
            kind: ErrorKind::TypeConversion(value.to_string(), target_type.to_string()),
            context: ErrorContext::new(),
        }
    }

    pub fn with_context(mut self, context: ErrorContext) -> Self {
        self.context = context;
        self
    }
}

/// Result type alias for xls-rs operations
pub type XlsRsResult<T> = Result<T, XlsRsError>;

/// Extension trait for adding context to anyhow errors
pub trait ResultExt<T> {
    fn with_file_context(self, file: &str) -> anyhow::Result<T>;
    fn with_row_context(self, file: &str, row: usize) -> anyhow::Result<T>;
    fn with_cell_context(self, file: &str, row: usize, col: usize) -> anyhow::Result<T>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> ResultExt<T> for Result<T, E> {
    fn with_file_context(self, file: &str) -> anyhow::Result<T> {
        self.map_err(|e| anyhow::anyhow!("{} in file '{}'", e, file))
    }

    fn with_row_context(self, file: &str, row: usize) -> anyhow::Result<T> {
        self.map_err(|e| anyhow::anyhow!("{} in file '{}' at row {}", e, file, row + 1))
    }

    fn with_cell_context(self, file: &str, row: usize, col: usize) -> anyhow::Result<T> {
        self.map_err(|e| {
            anyhow::anyhow!(
                "{} in file '{}' at row {}, column {}",
                e,
                file,
                row + 1,
                col + 1
            )
        })
    }
}
