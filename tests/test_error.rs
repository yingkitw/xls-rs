//! Tests for error types and context

use xls_rs::{ErrorContext, ErrorKind, XlsRsError};

#[test]
fn test_error_column_not_found() {
    let error = XlsRsError::column_not_found("missing_col");

    let msg = format!("{error}");
    assert!(msg.contains("Column 'missing_col' not found"));
}

#[test]
fn test_error_invalid_value() {
    let error = XlsRsError::invalid_value("abc", "number");

    let msg = format!("{error}");
    assert!(msg.contains("Invalid value 'abc'"));
    assert!(msg.contains("number"));
}

#[test]
fn test_error_type_conversion() {
    let error = XlsRsError::type_conversion("hello", "integer");

    let msg = format!("{error}");
    assert!(msg.contains("Type conversion failed"));
    assert!(msg.contains("hello"));
    assert!(msg.contains("integer"));
}

#[test]
fn test_error_with_file_context() {
    let error = XlsRsError::column_not_found("col")
        .with_context(ErrorContext::new().with_file("data.csv"));

    let msg = format!("{error}");
    assert!(msg.contains("data.csv"));
}

#[test]
fn test_error_with_row_context() {
    let error = XlsRsError::invalid_value("x", "number")
        .with_context(ErrorContext::new().with_file("test.csv").with_row(5));

    let msg = format!("{error}");
    assert!(msg.contains("test.csv"));
    assert!(msg.contains("row 6")); // 1-indexed for display
}

#[test]
fn test_error_with_cell_context() {
    let error = XlsRsError::type_conversion("bad", "float").with_context(
        ErrorContext::new()
            .with_file("input.csv")
            .with_row(2)
            .with_column(3),
    );

    let msg = format!("{error}");
    assert!(msg.contains("input.csv"));
    assert!(msg.contains("row 3"));
    assert!(msg.contains("column 4"));
}

#[test]
fn test_error_with_cell_ref() {
    let error = XlsRsError::invalid_value("err", "date")
        .with_context(ErrorContext::new().with_cell_ref("B5"));

    let msg = format!("{error}");
    assert!(msg.contains("cell B5"));
}

#[test]
fn test_error_context_builder() {
    let ctx = ErrorContext::new()
        .with_file("test.xlsx")
        .with_row(10)
        .with_column(2)
        .with_cell_ref("C11")
        .with_column_name("Price");

    assert_eq!(ctx.file, Some("test.xlsx".to_string()));
    assert_eq!(ctx.row, Some(10));
    assert_eq!(ctx.column, Some(2));
    assert_eq!(ctx.cell_ref, Some("C11".to_string()));
    assert_eq!(ctx.column_name, Some("Price".to_string()));
}

#[test]
fn test_error_kind_division_by_zero() {
    let error = XlsRsError {
        kind: ErrorKind::DivisionByZero,
        context: ErrorContext::new(),
    };

    let msg = format!("{error}");
    assert!(msg.contains("Division by zero"));
}

#[test]
fn test_error_kind_invalid_formula() {
    let error = XlsRsError {
        kind: ErrorKind::InvalidFormula("SUM(A1:".to_string()),
        context: ErrorContext::new(),
    };

    let msg = format!("{error}");
    assert!(msg.contains("Invalid formula"));
    assert!(msg.contains("SUM(A1:"));
}

#[test]
fn test_error_kind_file_not_found() {
    let error = XlsRsError {
        kind: ErrorKind::FileNotFound("missing.csv".to_string()),
        context: ErrorContext::new(),
    };

    let msg = format!("{error}");
    assert!(msg.contains("File not found"));
    assert!(msg.contains("missing.csv"));
}

#[test]
fn test_error_kind_unsupported_format() {
    let error = XlsRsError {
        kind: ErrorKind::UnsupportedFormat("xyz".to_string()),
        context: ErrorContext::new(),
    };

    let msg = format!("{error}");
    assert!(msg.contains("Unsupported file format"));
    assert!(msg.contains("xyz"));
}

#[test]
fn test_error_kind_parse_error() {
    let error = XlsRsError {
        kind: ErrorKind::ParseError("unexpected token".to_string()),
        context: ErrorContext::new(),
    };

    let msg = format!("{error}");
    assert!(msg.contains("Parse error"));
}

#[test]
fn test_error_kind_invalid_date_format() {
    let error = XlsRsError {
        kind: ErrorKind::InvalidDateFormat("not-a-date".to_string()),
        context: ErrorContext::new(),
    };

    let msg = format!("{error}");
    assert!(msg.contains("Invalid date format"));
}

#[test]
fn test_error_kind_invalid_regex() {
    let error = XlsRsError {
        kind: ErrorKind::InvalidRegex("[invalid".to_string()),
        context: ErrorContext::new(),
    };

    let msg = format!("{error}");
    assert!(msg.contains("Invalid regex"));
}

#[test]
fn test_error_kind_io_error() {
    let error = XlsRsError {
        kind: ErrorKind::IoError("permission denied".to_string()),
        context: ErrorContext::new(),
    };

    let msg = format!("{error}");
    assert!(msg.contains("IO error"));
}

#[test]
fn test_error_kind_other() {
    let error = XlsRsError {
        kind: ErrorKind::Other("custom error message".to_string()),
        context: ErrorContext::new(),
    };

    let msg = format!("{error}");
    assert!(msg.contains("custom error message"));
}
