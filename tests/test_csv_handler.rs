//! Tests for CSV handler and streaming

mod common;

use xls_rs::{CellRange, CsvHandler, StreamingCsvReader, StreamingCsvWriter};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_path(prefix: &str) -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("test_{prefix}_{id}.csv")
}

// ============ CellRange Tests ============

#[test]
fn test_cell_range_single_cell() {
    let range = CellRange::parse("A1").unwrap();
    assert_eq!(range.start_row, 0);
    assert_eq!(range.start_col, 0);
    assert_eq!(range.end_row, 0);
    assert_eq!(range.end_col, 0);
}

#[test]
fn test_cell_range_simple_range() {
    let range = CellRange::parse("A1:C3").unwrap();
    assert_eq!(range.start_row, 0);
    assert_eq!(range.start_col, 0);
    assert_eq!(range.end_row, 2);
    assert_eq!(range.end_col, 2);
}

#[test]
fn test_cell_range_lowercase() {
    let range = CellRange::parse("b2:d5").unwrap();
    assert_eq!(range.start_row, 1);
    assert_eq!(range.start_col, 1);
    assert_eq!(range.end_row, 4);
    assert_eq!(range.end_col, 3);
}

#[test]
fn test_cell_range_double_letter_column() {
    let range = CellRange::parse("AA1:AB10").unwrap();
    assert_eq!(range.start_col, 26); // AA = 26
    assert_eq!(range.end_col, 27); // AB = 27
}

#[test]
fn test_cell_range_triple_letter_column() {
    let range = CellRange::parse("AAA1").unwrap();
    // AAA = 26*26 + 26 + 1 - 1 = 702
    assert_eq!(range.start_col, 702);
}

#[test]
fn test_cell_range_with_whitespace() {
    let range = CellRange::parse("  A1:B2  ").unwrap();
    assert_eq!(range.start_row, 0);
    assert_eq!(range.end_row, 1);
}

// ============ CsvHandler Read Tests ============

#[test]
fn test_csv_handler_read_file() {
    let handler = CsvHandler::new();
    let content = handler.read(&common::example_path("sales.csv")).unwrap();

    assert!(content.contains("Product"));
    assert!(content.contains("Laptop"));
}

#[test]
fn test_csv_handler_read_as_json() {
    let handler = CsvHandler::new();
    let json = handler.read_as_json(&common::example_path("numbers.csv")).unwrap();

    assert!(json.starts_with("["));
    assert!(json.ends_with("]"));
    assert!(json.contains("1"));
}

#[test]
fn test_csv_handler_read_range() {
    let handler = CsvHandler::new();
    let range = CellRange::parse("A1:B2").unwrap();
    let data = handler.read_range(&common::example_path("numbers.csv"), &range).unwrap();

    assert_eq!(data.len(), 2);
    assert_eq!(data[0].len(), 2);
    assert_eq!(data[0][0], "A");
    assert_eq!(data[0][1], "B");
}

#[test]
fn test_csv_handler_read_range_middle() {
    let handler = CsvHandler::new();
    let range = CellRange::parse("B2:C3").unwrap();
    let path = unique_path("csv_range_middle");
    fs::write(&path, "A,B,C\n1,2,3\n4,5,6\n").unwrap();

    let data = handler.read_range(&path, &range).unwrap();

    assert_eq!(data.len(), 2);
    // Row 2 (index 1) columns B-C
    assert_eq!(data[0][0], "2");
    assert_eq!(data[0][1], "3");

    fs::remove_file(&path).ok();
}

// ============ CsvHandler Write Tests ============

#[test]
fn test_csv_handler_write_records() {
    let handler = CsvHandler::new();
    let path = unique_path("csv_write");

    let data = vec![
        vec!["A".to_string(), "B".to_string()],
        vec!["1".to_string(), "2".to_string()],
        vec!["3".to_string(), "4".to_string()],
    ];

    handler.write_records(&path, data).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("A,B"));
    assert!(content.contains("1,2"));
    assert!(content.contains("3,4"));

    fs::remove_file(&path).ok();
}

#[test]
fn test_csv_handler_append_records() {
    let handler = CsvHandler::new();
    let path = unique_path("csv_append");

    // Create initial file
    fs::write(&path, "A,B\n1,2\n").unwrap();

    // Append records
    let new_data = vec![
        vec!["3".to_string(), "4".to_string()],
        vec!["5".to_string(), "6".to_string()],
    ];
    handler.append_records(&path, &new_data).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("1,2"));
    assert!(content.contains("3,4"));
    assert!(content.contains("5,6"));

    fs::remove_file(&path).ok();
}

#[test]
fn test_csv_handler_write_range() {
    let handler = CsvHandler::new();
    let path = unique_path("csv_write_range");

    // Create initial file
    let initial = vec![
        vec!["A".to_string(), "B".to_string(), "C".to_string()],
        vec!["1".to_string(), "2".to_string(), "3".to_string()],
        vec!["4".to_string(), "5".to_string(), "6".to_string()],
    ];
    handler.write_records(&path, initial).unwrap();

    // Write to range starting at B2 (row 1, col 1)
    let new_data = vec![vec!["X".to_string(), "Y".to_string()]];
    handler.write_range(&path, &new_data, 1, 1).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("1,X,Y"));

    fs::remove_file(&path).ok();
}

#[test]
fn test_csv_handler_write_range_expand() {
    let handler = CsvHandler::new();
    let path = unique_path("csv_write_expand");

    // Create initial file first
    let initial = vec![
        vec!["A".to_string(), "B".to_string(), "C".to_string()],
        vec!["1".to_string(), "2".to_string(), "3".to_string()],
    ];
    handler.write_records(&path, initial).unwrap();

    // Write to expand the file at row 3, col 2
    let data = vec![vec!["X".to_string()]];
    handler.write_range(&path, &data, 2, 2).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert!(lines.len() >= 3);

    fs::remove_file(&path).ok();
}

// ============ Streaming Tests ============

#[test]
fn test_streaming_csv_reader() {
    let path = unique_path("streaming_read");
    fs::write(&path, "A,B\n1,2\n3,4\n5,6\n").unwrap();

    let reader = StreamingCsvReader::open(&path).unwrap();
    let rows: Vec<_> = reader.filter_map(|r| r.ok()).collect();

    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0], vec!["A", "B"]);
    assert_eq!(rows[1], vec!["1", "2"]);

    fs::remove_file(&path).ok();
}

#[test]
fn test_streaming_csv_writer() {
    let path = unique_path("streaming_write");

    {
        let mut writer = StreamingCsvWriter::create(&path).unwrap();
        writer
            .write_row(&["A".to_string(), "B".to_string()])
            .unwrap();
        writer
            .write_row(&["1".to_string(), "2".to_string()])
            .unwrap();
        writer
            .write_row(&["3".to_string(), "4".to_string()])
            .unwrap();

        assert_eq!(writer.rows_written(), 3);
        writer.flush().unwrap();
    }

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("A,B"));
    assert!(content.contains("3,4"));

    fs::remove_file(&path).ok();
}

#[test]
fn test_streaming_large_file() {
    let path = unique_path("streaming_large");

    // Write a larger file
    {
        let mut writer = StreamingCsvWriter::create(&path).unwrap();
        writer
            .write_row(&["ID".to_string(), "Value".to_string()])
            .unwrap();
        for i in 0..1000 {
            writer
                .write_row(&[i.to_string(), (i * 2).to_string()])
                .unwrap();
        }
        writer.flush().unwrap();
    }

    // Read it back streaming
    let reader = StreamingCsvReader::open(&path).unwrap();
    let count = reader.count();

    assert_eq!(count, 1001); // Header + 1000 rows

    fs::remove_file(&path).ok();
}

#[test]
fn test_csv_write_from_csv_sanitizes_formula_like_cells() {
    let handler = CsvHandler::new();
    let input_path = unique_path("inj_in");
    let output_path = unique_path("inj_out");
    fs::write(&input_path, "=cmd|\"\" /c calc\n").unwrap();
    handler
        .write_from_csv(&input_path, &output_path)
        .unwrap();
    let content = fs::read_to_string(&output_path).unwrap();
    assert!(
        content.contains("'=cmd"),
        "expected leading apostrophe neutralization, got: {content:?}"
    );
    fs::remove_file(&input_path).ok();
    fs::remove_file(&output_path).ok();
}

// ============ Edge Cases ============

#[test]
fn test_csv_handler_empty_file() {
    let handler = CsvHandler::new();
    let path = unique_path("csv_empty");

    fs::write(&path, "").unwrap();

    let content = handler.read(&path).unwrap();
    assert!(content.is_empty());

    fs::remove_file(&path).ok();
}

#[test]
fn test_csv_handler_single_column() {
    let handler = CsvHandler::new();
    let path = unique_path("csv_single_col");

    let data = vec![
        vec!["Name".to_string()],
        vec!["Alice".to_string()],
        vec!["Bob".to_string()],
    ];
    handler.write_records(&path, data).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("Name"));
    assert!(content.contains("Alice"));

    fs::remove_file(&path).ok();
}

#[test]
fn test_csv_handler_special_characters() {
    let handler = CsvHandler::new();
    let path = unique_path("csv_special");

    let data = vec![
        vec!["Name".to_string(), "Description".to_string()],
        vec!["Test".to_string(), "Hello, World".to_string()],
        vec!["Quote".to_string(), "Say \"Hi\"".to_string()],
    ];
    handler.write_records(&path, data).unwrap();

    assert!(std::path::Path::new(&path).exists());

    fs::remove_file(&path).ok();
}

#[test]
fn test_csv_handler_unicode() {
    let handler = CsvHandler::new();
    let path = unique_path("csv_unicode");

    let data = vec![
        vec!["Name".to_string(), "City".to_string()],
        vec!["日本語".to_string(), "東京".to_string()],
        vec!["中文".to_string(), "北京".to_string()],
    ];
    handler.write_records(&path, data).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("日本語"));
    assert!(content.contains("東京"));

    fs::remove_file(&path).ok();
}
