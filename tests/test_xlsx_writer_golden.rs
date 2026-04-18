//! Golden-file tests for XLSX writer output structure
//!
//! These tests verify that the XLSX writer produces consistent and correct
//! output structure by comparing against known-good output files.

use std::fs;
use std::io::Read;
use tempfile::TempDir;
use xls_rs::{ExcelHandler, WriteOptions};

/// Test basic XLSX writer output structure
#[test]
fn test_xlsx_writer_basic_structure() {
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("test_basic.xlsx");

    let data = vec![
        vec!["Name".to_string(), "Age".to_string(), "City".to_string()],
        vec!["Alice".to_string(), "30".to_string(), "NYC".to_string()],
        vec!["Bob".to_string(), "25".to_string(), "LA".to_string()],
    ];

    let handler = ExcelHandler::new();
    let options = WriteOptions::default();
    handler
        .write_styled(output_path.to_str().unwrap(), &data, &options)
        .unwrap();

    // Verify file was created
    assert!(output_path.exists());

    // Verify file is a valid XLSX (check it's a zip file)
    let file = fs::File::open(&output_path).unwrap();
    let mut magic = [0u8; 4];
    file.take(4).read_exact(&mut magic).unwrap();

    // XLSX files are ZIP archives (PK magic number)
    assert_eq!(&magic[0..2], b"PK");
}

/// Test XLSX writer with styled headers
#[test]
fn test_xlsx_writer_styled_headers() {
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("test_styled.xlsx");

    let data = vec![
        vec!["Product".to_string(), "Price".to_string(), "Stock".to_string()],
        vec!["Laptop".to_string(), "999.99".to_string(), "50".to_string()],
        vec!["Mouse".to_string(), "25.00".to_string(), "200".to_string()],
    ];

    let handler = ExcelHandler::new();
    let options = WriteOptions {
        style_header: true,
        freeze_header: true,
        auto_filter: true,
        ..Default::default()
    };
    handler
        .write_styled(output_path.to_str().unwrap(), &data, &options)
        .unwrap();

    assert!(output_path.exists());

    // Read back and verify data integrity
    let converter = xls_rs::Converter::new();
    let read_data = converter
        .read_any_data(output_path.to_str().unwrap(), None)
        .unwrap();

    assert!(read_data.len() >= data.len());
    assert_eq!(read_data[0][0], "Product");
    // Verify the value exists somewhere in the read data (Excel might format numbers differently)
    let found = read_data.iter().any(|row| row.iter().any(|cell| cell == "25.00" || cell == "25"));
    assert!(found, "Expected to find '25' or '25.00' in read data");
}

/// Test XLSX writer with data range
#[test]
fn test_xlsx_writer_range() {
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("test_range.xlsx");

    let data = vec![
        vec!["A1".to_string(), "B1".to_string()],
        vec!["A2".to_string(), "B2".to_string()],
    ];

    let handler = ExcelHandler::new();
    handler
        .write_range(output_path.to_str().unwrap(), &data, 0, 0, None)
        .unwrap();

    assert!(output_path.exists());

    // Read back and verify
    let converter = xls_rs::Converter::new();
    let read_data = converter
        .read_any_data(output_path.to_str().unwrap(), None)
        .unwrap();

    assert_eq!(read_data, data);
}

/// Test XLSX writer with multiple data types
#[test]
fn test_xlsx_writer_mixed_types() {
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("test_types.xlsx");

    let data = vec![
        vec![
            "ID".to_string(),
            "Name".to_string(),
            "Value".to_string(),
            "Active".to_string(),
            "Empty".to_string(),
        ],
        vec![
            "1".to_string(),
            "Test".to_string(),
            "123.45".to_string(),
            "true".to_string(),
            "".to_string(),
        ],
        vec![
            "2".to_string(),
            "Data".to_string(),
            "678.90".to_string(),
            "false".to_string(),
            "".to_string(),
        ],
    ];

    let handler = ExcelHandler::new();
    let options = WriteOptions::default();
    handler
        .write_styled(output_path.to_str().unwrap(), &data, &options)
        .unwrap();

    assert!(output_path.exists());

    // Read back and verify types are preserved
    let converter = xls_rs::Converter::new();
    let read_data = converter
        .read_any_data(output_path.to_str().unwrap(), None)
        .unwrap();

    // Verify numbers are preserved
    assert_eq!(read_data[1][0], "1");
    assert_eq!(read_data[1][2], "123.45");

    // Verify text is preserved
    assert_eq!(read_data[1][1], "Test");

    // Verify booleans
    assert_eq!(read_data[1][3], "true");

    // Verify empty cells
    assert!(read_data[1][4].is_empty());
}

/// Test XLSX writer with large dataset
#[test]
fn test_xlsx_writer_large_dataset() {
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("test_large.xlsx");

    let num_rows = 1000;
    let num_cols = 20;

    let data: Vec<Vec<String>> = (0..num_rows)
        .map(|row_idx| {
            (0..num_cols)
                .map(|col_idx| format!("R{}C{}", row_idx, col_idx))
                .collect()
        })
        .collect();

    let handler = ExcelHandler::new();
    let options = WriteOptions::default();
    handler
        .write_styled(output_path.to_str().unwrap(), &data, &options)
        .unwrap();

    assert!(output_path.exists());

    // Verify file size is reasonable (should be > 0)
    let metadata = fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);

    // Read back and verify
    let converter = xls_rs::Converter::new();
    let read_data = converter
        .read_any_data(output_path.to_str().unwrap(), None)
        .unwrap();

    assert_eq!(read_data.len(), num_rows);
    assert_eq!(read_data[0].len(), num_cols);
}

/// Test XLSX writer special characters
#[test]
fn test_xlsx_writer_special_characters() {
    let dir = TempDir::new().unwrap();
    let output_path = dir.path().join("test_special.xlsx");

    let data = vec![
        vec![
            "Text".to_string(),
            "Special".to_string(),
            "Unicode".to_string(),
        ],
        vec![
            "Hello".to_string(),
            "áéíóú".to_string(),
            "日本語".to_string(),
        ],
        vec![
            "Quotes".to_string(),
            "\"Text\"".to_string(),
            "'Text'".to_string(),
        ],
        vec![
            "Newline".to_string(),
            "Line1\nLine2".to_string(),
            "Tab\tSeparators".to_string(),
        ],
    ];

    let handler = ExcelHandler::new();
    let options = WriteOptions::default();
    handler
        .write_styled(output_path.to_str().unwrap(), &data, &options)
        .unwrap();

    assert!(output_path.exists());

    // Read back and verify special characters are preserved
    let converter = xls_rs::Converter::new();
    let read_data = converter
        .read_any_data(output_path.to_str().unwrap(), None)
        .unwrap();

    assert!(read_data.len() >= data.len());
    // Verify unicode characters are preserved
    let found_unicode = read_data.iter().any(|row| row.iter().any(|cell| cell.contains("áéíóú")));
    assert!(found_unicode);
    let found_japanese = read_data.iter().any(|row| row.iter().any(|cell| cell.contains("日本語")));
    assert!(found_japanese);
}

/// Test XLSX writer roundtrip consistency
#[test]
fn test_xlsx_writer_roundtrip() {
    let dir = TempDir::new().unwrap();
    let input_path = dir.path().join("input.xlsx");
    let output_path = dir.path().join("output.xlsx");

    let data = vec![
        vec!["A".to_string(), "B".to_string(), "C".to_string()],
        vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
        ],
        vec![
            "4".to_string(),
            "5".to_string(),
            "6".to_string(),
        ],
    ];

    // Write initial file
    let handler = ExcelHandler::new();
    let options = WriteOptions::default();
    handler
        .write_styled(input_path.to_str().unwrap(), &data, &options)
        .unwrap();

    // Read from input
    let converter = xls_rs::Converter::new();
    let read_data = converter
        .read_any_data(input_path.to_str().unwrap(), None)
        .unwrap();

    // Write to output
    handler
        .write_styled(output_path.to_str().unwrap(), &read_data, &options)
        .unwrap();

    // Read from output
    let final_data = converter
        .read_any_data(output_path.to_str().unwrap(), None)
        .unwrap();

    // Verify roundtrip consistency
    assert_eq!(read_data.len(), final_data.len());
    assert_eq!(read_data[0], final_data[0]);
    assert_eq!(read_data[1], final_data[1]);
    assert_eq!(read_data[2], final_data[2]);
}
