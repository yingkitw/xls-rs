//! Enhanced error types with context information

use std::fmt;

/// Error with file and location context
#[derive(Debug)]
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

impl std::error::Error for XlsRsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
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
#[derive(Debug)]
pub enum ErrorKind {
    ColumnNotFound(String),
    InvalidCellRef(String),
    InvalidValue(String, String),
    TypeConversion(String, String),
    DivisionByZero,
    InvalidFormula(String),
    FileNotFound(String),
    UnsupportedFormat(String),
    ParseError(String),
    InvalidDateFormat(String),
    InvalidRegex(String),
    IoError(String),
    Other(String),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::ColumnNotFound(s) => write!(f, "Column '{s}' not found"),
            ErrorKind::InvalidCellRef(s) => write!(f, "Invalid cell reference '{s}'"),
            ErrorKind::InvalidValue(a, b) => write!(f, "Invalid value '{a}' - expected {b}"),
            ErrorKind::TypeConversion(a, b) => {
                write!(f, "Type conversion failed: cannot convert '{a}' to {b}")
            }
            ErrorKind::DivisionByZero => write!(f, "Division by zero"),
            ErrorKind::InvalidFormula(s) => write!(f, "Invalid formula: {s}"),
            ErrorKind::FileNotFound(s) => write!(f, "File not found: {s}"),
            ErrorKind::UnsupportedFormat(s) => write!(f, "Unsupported file format: {s}"),
            ErrorKind::ParseError(s) => write!(f, "Parse error: {s}"),
            ErrorKind::InvalidDateFormat(s) => write!(f, "Invalid date format: {s}"),
            ErrorKind::InvalidRegex(s) => write!(f, "Invalid regex pattern: {s}"),
            ErrorKind::IoError(s) => write!(f, "IO error: {s}"),
            ErrorKind::Other(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for ErrorKind {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl ErrorKind {
    /// Stable error code for programmatic matching across CLI and MCP.
    pub fn code(&self) -> &'static str {
        match self {
            ErrorKind::ColumnNotFound(_) => "column_not_found",
            ErrorKind::InvalidCellRef(_) => "invalid_cell_ref",
            ErrorKind::InvalidValue(_, _) => "invalid_value",
            ErrorKind::TypeConversion(_, _) => "type_conversion",
            ErrorKind::DivisionByZero => "division_by_zero",
            ErrorKind::InvalidFormula(_) => "invalid_formula",
            ErrorKind::FileNotFound(_) => "file_not_found",
            ErrorKind::UnsupportedFormat(_) => "unsupported_format",
            ErrorKind::ParseError(_) => "parse_error",
            ErrorKind::InvalidDateFormat(_) => "invalid_date_format",
            ErrorKind::InvalidRegex(_) => "invalid_regex",
            ErrorKind::IoError(_) => "io_error",
            ErrorKind::Other(_) => "other",
        }
    }
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

    /// Stable error code for programmatic matching across CLI and MCP.
    pub fn code(&self) -> &'static str {
        self.kind.code()
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
