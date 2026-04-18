//! Cell typing utilities for consistent cell type detection across all writers

use crate::excel::xlsx_writer::CellData;
use crate::excel::xlsx_writer::RowData;

/// Consistently determine cell type from string value
///
/// This function implements the unified cell typing strategy used across all writers:
/// 1. Try to parse as f64 (number)
/// 2. If parsing fails and value is not empty, treat as string
/// 3. Otherwise, treat as empty
///
/// # Examples
/// ```
/// use xls_rs::excel::{classify_cell, CellData};
///
/// let cell = classify_cell("123.45");
/// assert!(matches!(cell, CellData::Number(123.45)));
///
/// let cell = classify_cell("hello");
/// assert!(matches!(cell, CellData::String(_)));
///
/// let cell = classify_cell("");
/// assert!(matches!(cell, CellData::Empty));
/// ```
pub fn classify_cell(value: &str) -> CellData {
    if let Ok(num) = value.parse::<f64>() {
        CellData::Number(num)
    } else if !value.is_empty() {
        CellData::String(value.to_string())
    } else {
        CellData::Empty
    }
}

/// Add a cell to a row with consistent typing
///
/// This is a convenience method that combines cell type detection with row data addition.
pub fn add_cell_to_row(row: &mut RowData, value: &str) {
    let cell = classify_cell(value);
    match cell {
        CellData::Number(n) => row.add_number(n),
        CellData::String(s) => row.add_string(&s),
        CellData::Empty => row.add_empty(),
        CellData::Formula(f) => row.add_formula(&f),
    }
}

/// Add multiple cells to a row with consistent typing
///
/// This is a convenience method for adding multiple cells at once.
pub fn add_cells_to_row(row: &mut RowData, values: &[String]) {
    for value in values {
        add_cell_to_row(row, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_number() {
        let cell = classify_cell("123.45");
        assert!(matches!(cell, CellData::Number(n) if (n - 123.45).abs() < f64::EPSILON));
    }

    #[test]
    fn test_classify_integer() {
        let cell = classify_cell("42");
        assert!(matches!(cell, CellData::Number(42.0)));
    }

    #[test]
    fn test_classify_negative_number() {
        let cell = classify_cell("-123.45");
        assert!(matches!(cell, CellData::Number(n) if (n + 123.45).abs() < f64::EPSILON));
    }

    #[test]
    fn test_classify_string() {
        let cell = classify_cell("hello");
        assert!(matches!(cell, CellData::String(s) if s == "hello"));
    }

    #[test]
    fn test_classify_empty() {
        let cell = classify_cell("");
        assert!(matches!(cell, CellData::Empty));
    }

    #[test]
    fn test_classify_whitespace() {
        let cell = classify_cell("   ");
        assert!(matches!(cell, CellData::String(_)));
    }

    #[test]
    fn test_classify_invalid_number() {
        let cell = classify_cell("not a number");
        assert!(matches!(cell, CellData::String(_)));
    }

    #[test]
    fn test_add_cell_to_row() {
        let mut row = RowData::new();
        add_cell_to_row(&mut row, "123");
        add_cell_to_row(&mut row, "hello");
        add_cell_to_row(&mut row, "");
        
        assert_eq!(row.cells.len(), 3);
        assert!(matches!(row.cells[0], CellData::Number(123.0)));
        assert!(matches!(&row.cells[1], CellData::String(s) if s == "hello"));
        assert!(matches!(row.cells[2], CellData::Empty));
    }

    #[test]
    fn test_add_cells_to_row() {
        let mut row = RowData::new();
        let values = vec!["123".to_string(), "hello".to_string(), "".to_string()];
        add_cells_to_row(&mut row, &values);
        
        assert_eq!(row.cells.len(), 3);
    }
}
