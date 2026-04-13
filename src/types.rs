//! Type-safe data structures for xls-rs
//!
//! This module provides strongly-typed representations of cell data
//! to improve type safety and performance over string-only representations.

use std::fmt;

/// A strongly-typed cell value that can represent different data types
///
/// This enum provides type safety for cell values, allowing the codebase
/// to distinguish between strings, numbers, booleans, dates, and empty values.
/// This eliminates the need for repeated string parsing and improves performance.
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    /// String data
    String(String),
    /// Numeric data (float)
    Number(f64),
    /// Integer data (for exact precision when needed)
    Integer(i64),
    /// Boolean data
    Boolean(bool),
    /// Date/time data (stored as timestamp)
    DateTime(i64),
    /// Empty/null value
    Empty,
}

impl CellValue {
    /// Create a String cell value
    pub fn string(s: impl Into<String>) -> Self {
        CellValue::String(s.into())
    }

    /// Create a Number cell value
    pub fn number(n: f64) -> Self {
        CellValue::Number(n)
    }

    /// Create an Integer cell value
    pub fn integer(i: i64) -> Self {
        CellValue::Integer(i)
    }

    /// Create a Boolean cell value
    pub fn boolean(b: bool) -> Self {
        CellValue::Boolean(b)
    }

    /// Create a DateTime cell value from timestamp
    pub fn datetime(timestamp: i64) -> Self {
        CellValue::DateTime(timestamp)
    }

    /// Create an Empty cell value
    pub fn empty() -> Self {
        CellValue::Empty
    }

    /// Check if the value is empty
    pub fn is_empty(&self) -> bool {
        matches!(self, CellValue::Empty)
    }

    /// Check if the value is numeric (Number or Integer)
    pub fn is_numeric(&self) -> bool {
        matches!(self, CellValue::Number(_) | CellValue::Integer(_))
    }

    /// Get the value as a string reference
    pub fn as_str(&self) -> Option<&str> {
        match self {
            CellValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get the value as a number (f64)
    ///
    /// Returns Some(f64) for Number and Integer values, None otherwise
    pub fn as_number(&self) -> Option<f64> {
        match self {
            CellValue::Number(n) => Some(*n),
            CellValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Get the value as a boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            CellValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Convert to display string
    pub fn to_display_string(&self) -> String {
        match self {
            CellValue::String(s) => s.clone(),
            CellValue::Number(n) => {
                // Format without unnecessary decimal places
                if n.fract() == 0.0 && n.abs() < (i64::MAX as f64) {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            CellValue::Integer(i) => format!("{}", i),
            CellValue::Boolean(b) => format!("{}", b),
            CellValue::DateTime(ts) => format!("{}", ts),
            CellValue::Empty => String::new(),
        }
    }

    /// Parse a string into the most appropriate CellValue type
    ///
    /// Attempts to parse the string as:
    /// 1. Empty -> Empty
    /// 2. Boolean ("true"/"false") -> Boolean
    /// 3. Integer -> Integer
    /// 4. Float -> Number
    /// 5. Otherwise -> String
    pub fn parse(s: &str) -> Self {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return CellValue::Empty;
        }

        // Try boolean
        match trimmed.to_lowercase().as_str() {
            "true" | "yes" | "1" => return CellValue::Boolean(true),
            "false" | "no" | "0" => return CellValue::Boolean(false),
            _ => {}
        }

        // Try integer first (more precise)
        if let Ok(i) = trimmed.parse::<i64>() {
            return CellValue::Integer(i);
        }

        // Try float
        if let Ok(n) = trimmed.parse::<f64>() {
            return CellValue::Number(n);
        }

        // Default to string
        CellValue::String(trimmed.to_string())
    }

    /// Convert from string representation with type hint
    pub fn from_string_with_type(s: &str, type_hint: Option<&DataType>) -> Self {
        match type_hint {
            Some(DataType::Integer) => s.parse::<i64>()
                .map(CellValue::Integer)
                .unwrap_or_else(|_| CellValue::String(s.to_string())),
            Some(DataType::Number) => s.parse::<f64>()
                .map(CellValue::Number)
                .unwrap_or_else(|_| CellValue::String(s.to_string())),
            Some(DataType::Boolean) => match s.to_lowercase().as_str() {
                "true" | "yes" | "1" => CellValue::Boolean(true),
                "false" | "no" | "0" => CellValue::Boolean(false),
                _ => CellValue::String(s.to_string()),
            },
            Some(DataType::String) | None => CellValue::parse(s),
            Some(DataType::DateTime) => s.parse::<i64>()
                .map(CellValue::DateTime)
                .unwrap_or_else(|_| CellValue::String(s.to_string())),
        }
    }
}

impl Default for CellValue {
    fn default() -> Self {
        CellValue::Empty
    }
}

impl fmt::Display for CellValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellValue::String(s) => write!(f, "{}", s),
            CellValue::Number(n) => write!(f, "{}", n),
            CellValue::Integer(i) => write!(f, "{}", i),
            CellValue::Boolean(b) => write!(f, "{}", b),
            CellValue::DateTime(ts) => write!(f, "{}", ts),
            CellValue::Empty => Ok(()),
        }
    }
}

impl From<String> for CellValue {
    fn from(s: String) -> Self {
        CellValue::parse(&s)
    }
}

impl From<&str> for CellValue {
    fn from(s: &str) -> Self {
        CellValue::parse(s)
    }
}

impl From<f64> for CellValue {
    fn from(n: f64) -> Self {
        CellValue::Number(n)
    }
}

impl From<i64> for CellValue {
    fn from(i: i64) -> Self {
        CellValue::Integer(i)
    }
}

impl From<bool> for CellValue {
    fn from(b: bool) -> Self {
        CellValue::Boolean(b)
    }
}

/// Data type metadata for columns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    String,
    Number,
    Integer,
    Boolean,
    DateTime,
}

impl DataType {
    /// Detect the data type from a cell value
    pub fn from_value(value: &CellValue) -> Self {
        match value {
            CellValue::String(_) => DataType::String,
            CellValue::Number(_) => DataType::Number,
            CellValue::Integer(_) => DataType::Integer,
            CellValue::Boolean(_) => DataType::Boolean,
            CellValue::DateTime(_) => DataType::DateTime,
            CellValue::Empty => DataType::String,
        }
    }

    /// Get the string name of the data type
    pub fn name(&self) -> &'static str {
        match self {
            DataType::String => "string",
            DataType::Number => "float",
            DataType::Integer => "integer",
            DataType::Boolean => "boolean",
            DataType::DateTime => "datetime",
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// A row of type-safe cell values
pub type DataRow = Vec<CellValue>;

/// A dataset of type-safe data
#[derive(Debug, Clone, PartialEq)]
pub struct DataSet {
    /// Column names (header row)
    pub columns: Vec<String>,
    /// Data rows
    pub rows: Vec<DataRow>,
    /// Column type metadata
    pub column_types: Vec<DataType>,
}

impl DataSet {
    /// Create a new empty dataset
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            column_types: Vec::new(),
        }
    }

    /// Create a dataset with columns but no data
    pub fn with_columns(columns: Vec<String>) -> Self {
        let column_types = vec![DataType::String; columns.len()];
        Self {
            columns,
            rows: Vec::new(),
            column_types,
        }
    }

    /// Add a row to the dataset
    pub fn push_row(&mut self, row: DataRow) {
        // Update column types based on new data
        for (i, cell) in row.iter().enumerate() {
            if i < self.column_types.len() {
                let detected = DataType::from_value(cell);
                // Prefer more specific types
                if std::mem::discriminant(&self.column_types[i])
                    != std::mem::discriminant(&detected)
                {
                    self.column_types[i] = detected;
                }
            }
        }
        self.rows.push(row);
    }

    /// Get the number of rows
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Get the number of columns
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// Check if the dataset is empty
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Infer column types from existing data
    pub fn infer_types(&mut self) {
        for col_idx in 0..self.columns.len() {
            let mut type_count: std::collections::HashMap<DataType, usize> =
                std::collections::HashMap::new();

            for row in &self.rows {
                if let Some(cell) = row.get(col_idx) {
                    let dt = DataType::from_value(cell);
                    *type_count.entry(dt).or_insert(0) += 1;
                }
            }

            // Choose the most common non-empty type
            let most_common = type_count
                .iter()
                .filter(|(dt, _)| *dt != &DataType::String)
                .max_by_key(|(_, count)| *count)
                .map(|(dt, _)| *dt)
                .unwrap_or(DataType::String);

            if col_idx < self.column_types.len() {
                self.column_types[col_idx] = most_common;
            }
        }
    }
}

impl Default for DataSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Conversion from legacy `Vec<Vec<String>>` format
impl From<Vec<Vec<String>>> for DataSet {
    fn from(data: Vec<Vec<String>>) -> Self {
        if data.is_empty() {
            return DataSet::new();
        }

        let columns = data[0].clone();
        let mut dataset = DataSet::with_columns(columns);

        for row in &data[1..] {
            let typed_row: DataRow = row.iter().map(|s| CellValue::parse(s)).collect();
            dataset.push_row(typed_row);
        }

        dataset.infer_types();
        dataset
    }
}

/// Conversion to legacy `Vec<Vec<String>>` format
impl From<DataSet> for Vec<Vec<String>> {
    fn from(dataset: DataSet) -> Vec<Vec<String>> {
        let mut result = vec![dataset.columns];

        for row in dataset.rows {
            let string_row: Vec<String> =
                row.iter().map(|v| v.to_display_string()).collect();
            result.push(string_row);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_value_parse() {
        assert_eq!(CellValue::parse(""), CellValue::Empty);
        assert_eq!(CellValue::parse("true"), CellValue::Boolean(true));
        assert_eq!(CellValue::parse("false"), CellValue::Boolean(false));
        assert_eq!(CellValue::parse("42"), CellValue::Integer(42));
        assert_eq!(CellValue::parse("3.14"), CellValue::Number(3.14));
        assert_eq!(CellValue::parse("hello"), CellValue::String("hello".to_string()));
    }

    #[test]
    fn test_cell_value_numeric() {
        assert!(CellValue::Integer(42).is_numeric());
        assert!(CellValue::Number(3.14).is_numeric());
        assert!(!CellValue::String("42".to_string()).is_numeric());
        assert!(!CellValue::Boolean(true).is_numeric());
    }

    #[test]
    fn test_cell_value_as_number() {
        assert_eq!(CellValue::Integer(42).as_number(), Some(42.0));
        assert_eq!(CellValue::Number(3.14).as_number(), Some(3.14));
        assert_eq!(CellValue::String("42".to_string()).as_number(), None);
    }

    #[test]
    fn test_dataset_conversion() {
        let legacy = vec![
            vec!["name".to_string(), "age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ];

        let dataset: DataSet = legacy.clone().into();
        assert_eq!(dataset.columns, vec!["name", "age"]);
        assert_eq!(dataset.row_count(), 2);

        let back: Vec<Vec<String>> = dataset.into();
        assert_eq!(back, legacy);
    }
}
