//! Integration tests for the streaming XLSX reader.
//!
//! Verifies that `XlsxStreamingReader` produces the same row data as the
//! full-materialization `XlsxReader`, and handles edge cases like empty
//! sheets, sparse cells, and various cell types.

use xls_rs::excel::xlsx_reader::{XlsxCellValue, XlsxReader};
use xls_rs::excel::xlsx_streaming_reader::XlsxStreamingReader;
use std::fs::File;
use xls_rs::{RowData, XlsxWriter};

fn make_test_xlsx(path: &str) {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    // Row 0: header
    let mut row = RowData::new();
    row.add_string("Name");
    row.add_string("Age");
    row.add_string("Score");
    writer.add_row(row);

    // Row 1: data
    let mut row = RowData::new();
    row.add_string("Alice");
    row.add_number(30.0);
    row.add_number(95.5);
    writer.add_row(row);

    // Row 2: data with empty cell
    let mut row = RowData::new();
    row.add_string("Bob");
    row.add_empty();
    row.add_number(87.0);
    writer.add_row(row);

    // Row 3: boolean
    let mut row = RowData::new();
    row.add_string("Carol");
    row.add_number(25.0);
    row.add_bool(true);
    writer.add_row(row);

    writer.save(File::create(path).unwrap()).unwrap();
}

#[test]
fn test_streaming_matches_full_reader() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.xlsx");
    let path_str = path.to_string_lossy().to_string();
    make_test_xlsx(&path_str);

    // Read with full-materialization reader
    let full_reader = XlsxReader::from_path(&path_str).unwrap();
    let full_sheet = full_reader.get_sheet_by_name("Sheet1").unwrap();
    let full_rows: Vec<Vec<String>> = full_sheet.to_string_vec();

    // Read with streaming reader
    let mut stream_reader = XlsxStreamingReader::from_path(&path_str).unwrap();
    assert_eq!(stream_reader.sheet_names(), &["Sheet1".to_string()]);

    let iter = stream_reader.row_iter("Sheet1").unwrap();
    let streamed_rows: Vec<Vec<String>> = iter
        .map(|row| row.iter().map(|c| c.to_string()).collect())
        .collect();

    // Compare
    assert_eq!(full_rows.len(), streamed_rows.len(), "Row count mismatch");
    for (i, (full, streamed)) in full_rows.iter().zip(streamed_rows.iter()).enumerate() {
        assert_eq!(full, streamed, "Row {i} mismatch: full={full:?} streamed={streamed:?}");
    }
}

#[test]
fn test_streaming_cell_types() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("types.xlsx");
    let path_str = path.to_string_lossy().to_string();
    make_test_xlsx(&path_str);

    let mut reader = XlsxStreamingReader::from_path(&path_str).unwrap();
    let iter = reader.row_iter("Sheet1").unwrap();
    let rows: Vec<Vec<XlsxCellValue>> = iter.collect();

    // Row 0: header (all strings)
    assert_eq!(rows.len(), 4);
    assert!(matches!(rows[0][0], XlsxCellValue::String(ref s) if s == "Name"));
    assert!(matches!(rows[0][1], XlsxCellValue::String(ref s) if s == "Age"));

    // Row 1: mixed string + number
    assert!(matches!(rows[1][0], XlsxCellValue::String(ref s) if s == "Alice"));
    assert!(matches!(rows[1][1], XlsxCellValue::Number(n) if n == 30.0));
    assert!(matches!(rows[1][2], XlsxCellValue::Number(n) if n == 95.5));

    // Row 2: empty cell in middle
    assert!(matches!(rows[2][0], XlsxCellValue::String(ref s) if s == "Bob"));
    assert!(matches!(rows[2][1], XlsxCellValue::Empty));
    assert!(matches!(rows[2][2], XlsxCellValue::Number(n) if n == 87.0));

    // Row 3: boolean
    assert!(matches!(rows[3][2], XlsxCellValue::Bool(true)));
}

#[test]
fn test_streaming_empty_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.xlsx");
    let path_str = path.to_string_lossy().to_string();

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Empty").unwrap();
    writer.save(File::create(&path).unwrap()).unwrap();

    let mut reader = XlsxStreamingReader::from_path(&path_str).unwrap();
    let iter = reader.row_iter("Empty").unwrap();
    let rows: Vec<_> = iter.collect();
    assert!(rows.is_empty(), "Empty sheet should yield no rows");
}

#[test]
fn test_streaming_sheet_names() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("multisheet.xlsx");
    let path_str = path.to_string_lossy().to_string();

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Alpha").unwrap();
    writer.add_sheet("Beta").unwrap();
    writer.save(File::create(&path).unwrap()).unwrap();

    let reader = XlsxStreamingReader::from_path(&path_str).unwrap();
    let names = reader.sheet_names();
    assert_eq!(names, &["Alpha".to_string(), "Beta".to_string()]);
}
