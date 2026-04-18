//! Excel parity tests between library, CLI, and MCP.
//!
//! This file tests that Excel-specific operations behave consistently
//! across the library API, CLI commands, and MCP tools.

use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_excel_write_styled_parity() {
    let dir = tempfile::tempdir().unwrap();
    let csv_input = dir.path().join("input.csv");
    let xlsx_output = dir.path().join("output.xlsx");

    // Create test CSV
    std::fs::write(
        &csv_input,
        "Name,Value\nAlice,100\nBob,200\nCharlie,300\n",
    )
    .unwrap();

    // Library write styled
    let handler = xls_rs::ExcelHandler::new();
    let converter = xls_rs::Converter::new();
    let data = converter.read_any_data(csv_input.to_string_lossy().as_ref(), None).unwrap();
    handler
        .write_styled(
            xlsx_output.to_string_lossy().as_ref(),
            &data,
            &xls_rs::WriteOptions::default(),
        )
        .unwrap();

    // CLI export styled
    let cli_output = dir.path().join("cli_output.xlsx");
    let exe = env!("CARGO_BIN_EXE_xls-rs");
    let out = Command::new(exe)
        .args([
            "--quiet",
            "export-styled",
            "--input",
            csv_input.to_string_lossy().as_ref(),
            "--output",
            cli_output.to_string_lossy().as_ref(),
            "--style",
            "default",
        ])
        .output()
        .unwrap();

    assert!(out.status.success(), "CLI export-styled failed: {:?}", out);

    // Both should produce valid Excel files
    assert!(xlsx_output.exists());
    assert!(cli_output.exists());
}

#[test]
fn test_excel_list_sheets_parity() {
    let dir = tempfile::tempdir().unwrap();
    let xlsx_input = dir.path().join("test.xlsx");

    // Create a simple Excel file with data
    let handler = xls_rs::ExcelHandler::new();
    let data = vec![
        vec!["Name".to_string(), "Value".to_string()],
        vec!["Alice".to_string(), "100".to_string()],
    ];
    let converter = xls_rs::Converter::new();
    converter
        .write_any_data(xlsx_input.to_string_lossy().as_ref(), &data, None)
        .unwrap();

    // Library list sheets
    let library_sheets = handler.list_sheets(xlsx_input.to_string_lossy().as_ref()).unwrap();
    assert_eq!(library_sheets.len(), 1);
    assert_eq!(library_sheets[0], "Sheet1");

    // CLI list sheets
    let exe = env!("CARGO_BIN_EXE_xls-rs");
    let out = Command::new(exe)
        .args([
            "--quiet",
            "sheets",
            "--input",
            xlsx_input.to_string_lossy().as_ref(),
        ])
        .output()
        .unwrap();

    assert!(out.status.success(), "CLI sheets failed: {:?}", out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Sheet1"));
}

#[test]
fn test_excel_read_range_parity() {
    let dir = tempfile::tempdir().unwrap();
    let xlsx_input = dir.path().join("test.xlsx");

    // Create Excel file with multiple rows
    let data = vec![
        vec!["A1".to_string(), "B1".to_string(), "C1".to_string()],
        vec!["A2".to_string(), "B2".to_string(), "C2".to_string()],
        vec!["A3".to_string(), "B3".to_string(), "C3".to_string()],
    ];
    let converter = xls_rs::Converter::new();
    converter
        .write_any_data(xlsx_input.to_string_lossy().as_ref(), &data, None)
        .unwrap();

    // Library read range
    let handler = xls_rs::ExcelHandler::new();
    let range = xls_rs::CellRange::parse("A1:B2").unwrap();
    let library_data = handler
        .read_range(xlsx_input.to_string_lossy().as_ref(), &range, None)
        .unwrap();
    assert_eq!(library_data.len(), 2);
    assert_eq!(library_data[0].len(), 2);
    assert_eq!(library_data[0][0], "A1");
    assert_eq!(library_data[0][1], "B1");

    // CLI read range
    let exe = env!("CARGO_BIN_EXE_xls-rs");
    let out = Command::new(exe)
        .args([
            "--quiet",
            "read",
            "--input",
            xlsx_input.to_string_lossy().as_ref(),
            "--range",
            "A1:B2",
            "--format",
            "csv",
        ])
        .output()
        .unwrap();

    assert!(out.status.success(), "CLI read with range failed: {:?}", out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("A1,B1"));
    assert!(stdout.contains("A2,B2"));
}

#[test]
fn test_excel_cell_typing_consistency() {
    // Test that cell typing is consistent across all writers
    let dir = tempfile::tempdir().unwrap();

    // Test data with mixed types
    let data = vec![
        vec!["Number".to_string(), "Text".to_string(), "Empty".to_string()],
        vec!["123.45".to_string(), "hello".to_string(), "".to_string()],
        vec!["42".to_string(), "world".to_string(), "".to_string()],
    ];

    // Write to Excel
    let xlsx_output = dir.path().join("test.xlsx");
    let handler = xls_rs::ExcelHandler::new();
    handler
        .write_styled(
            xlsx_output.to_string_lossy().as_ref(),
            &data,
            &xls_rs::WriteOptions::default(),
        )
        .unwrap();

    // Read back
    let converter = xls_rs::Converter::new();
    let read_data = converter
        .read_any_data(xlsx_output.to_string_lossy().as_ref(), None)
        .unwrap();

    // Verify numbers are preserved as numbers
    assert_eq!(read_data[1][0], "123.45");
    assert_eq!(read_data[2][0], "42");

    // Verify text is preserved
    assert_eq!(read_data[1][1], "hello");
    assert_eq!(read_data[2][1], "world");

    // Verify empty cells
    assert!(read_data[1][2].is_empty());
    assert!(read_data[2][2].is_empty());
}

#[test]
fn test_excel_write_range_expand_mode() {
    let dir = tempfile::tempdir().unwrap();
    let xlsx_output = dir.path().join("test.xlsx");

    let handler = xls_rs::ExcelHandler::new();
    let data = vec![vec!["A1".to_string(), "B1".to_string()]];

    // Write with expand mode (default) - start at row 0, column 0 for simplicity
    handler
        .write_range(
            xlsx_output.to_string_lossy().as_ref(),
            &data,
            0,
            0,
            None,
        )
        .unwrap();

    // Read back and verify
    let converter = xls_rs::Converter::new();
    let read_data = converter
        .read_any_data(xlsx_output.to_string_lossy().as_ref(), None)
        .unwrap();

    // Verify data is written
    assert!(!read_data.is_empty());
    assert_eq!(read_data[0][0], "A1");
    assert_eq!(read_data[0][1], "B1");
}

#[test]
fn test_excel_write_range_preserve_mode() {
    let dir = tempfile::tempdir().unwrap();
    let xlsx_file = dir.path().join("test.xlsx");

    // Create initial file with data
    let initial_data = vec![
        vec!["A1".to_string(), "B1".to_string(), "C1".to_string()],
        vec!["A2".to_string(), "B2".to_string(), "C2".to_string()],
    ];
    let converter = xls_rs::Converter::new();
    converter
        .write_any_data(xlsx_file.to_string_lossy().as_ref(), &initial_data, None)
        .unwrap();

    // Write to range with preserve mode
    let handler = xls_rs::ExcelHandler::new();
    let new_data = vec![vec!["X1".to_string(), "Y1".to_string()]];
    handler
        .write_range_with_mode(
            xlsx_file.to_string_lossy().as_ref(),
            &new_data,
            1,
            1,
            None,
            xls_rs::WriteMode::Preserve,
        )
        .unwrap();

    // Read back and verify preservation
    let read_data = converter
        .read_any_data(xlsx_file.to_string_lossy().as_ref(), None)
        .unwrap();

    // Original data outside the range should be preserved
    assert_eq!(read_data[0][0], "A1");
    assert_eq!(read_data[0][2], "C1");

    // Data within the range should be updated - row 1 (0-indexed), col 1-2
    assert_eq!(read_data[1][1], "X1");
}

#[test]
fn test_formula_apply_to_cell_parity() {
    let dir = tempfile::tempdir().unwrap();
    let xlsx_input = dir.path().join("input.xlsx");
    let xlsx_output = dir.path().join("output.xlsx");

    // Create test data as Excel file
    let converter = xls_rs::Converter::new();
    let data = vec![
        vec!["Value".to_string()],
        vec!["10".to_string()],
        vec!["20".to_string()],
    ];
    converter
        .write_any_data(xlsx_input.to_string_lossy().as_ref(), &data, None)
        .unwrap();

    // Library apply formula
    let evaluator = xls_rs::FormulaEvaluator::new();
    evaluator
        .apply_to_excel(
            xlsx_input.to_string_lossy().as_ref(),
            xlsx_output.to_string_lossy().as_ref(),
            "=B2*2",
            "B3",
            None,
        )
        .unwrap();

    // File should exist
    assert!(xlsx_output.exists());

    // CLI apply formula
    let cli_output = dir.path().join("cli_output.xlsx");
    let exe = env!("CARGO_BIN_EXE_xls-rs");
    let out = Command::new(exe)
        .args([
            "--quiet",
            "formula",
            "--input",
            xlsx_input.to_string_lossy().as_ref(),
            "--output",
            cli_output.to_string_lossy().as_ref(),
            "--formula",
            "=B2*2",
            "--cell",
            "B3",
        ])
        .output()
        .unwrap();

    assert!(out.status.success(), "CLI formula failed: {:?}", out);
    assert!(cli_output.exists());
}

#[test]
fn test_excel_read_all_sheets_parity() {
    let dir = tempfile::tempdir().unwrap();
    let xlsx_input = dir.path().join("test.xlsx");

    // Create Excel file
    let data = vec![vec!["A1".to_string(), "B1".to_string()]];
    let converter = xls_rs::Converter::new();
    converter
        .write_any_data(xlsx_input.to_string_lossy().as_ref(), &data, None)
        .unwrap();

    // Library read all sheets
    let handler = xls_rs::ExcelHandler::new();
    let library_sheets = handler
        .read_all_sheets(xlsx_input.to_string_lossy().as_ref())
        .unwrap();
    assert_eq!(library_sheets.len(), 1);
    assert!(library_sheets.contains_key("Sheet1"));

    // CLI read all sheets
    let exe = env!("CARGO_BIN_EXE_xls-rs");
    let out = Command::new(exe)
        .args([
            "--quiet",
            "read-all",
            "--input",
            xlsx_input.to_string_lossy().as_ref(),
            "--format",
            "csv",
        ])
        .output()
        .unwrap();

    assert!(out.status.success(), "CLI read-all failed: {:?}", out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Sheet: Sheet1"));
}

#[test]
fn test_cell_typer_classify_cell() {
    use xls_rs::classify_cell;
    use xls_rs::excel::xlsx_writer::CellData;

    // Number
    let cell = classify_cell("123.45");
    assert!(matches!(cell, CellData::Number(n) if (n - 123.45).abs() < f64::EPSILON));

    // Integer
    let cell = classify_cell("42");
    assert!(matches!(cell, CellData::Number(42.0)));

    // Negative number
    let cell = classify_cell("-123.45");
    assert!(matches!(cell, CellData::Number(n) if (n + 123.45).abs() < f64::EPSILON));

    // String
    let cell = classify_cell("hello");
    assert!(matches!(cell, CellData::String(s) if s == "hello"));

    // Empty
    let cell = classify_cell("");
    assert!(matches!(cell, CellData::Empty));

    // Invalid number (treated as string)
    let cell = classify_cell("not a number");
    assert!(matches!(cell, CellData::String(_)));
}
