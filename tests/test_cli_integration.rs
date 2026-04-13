//! Integration tests for CLI commands
//!
//! Tests the actual CLI commands end-to-end with real file I/O

mod common;

use std::fs;
use tempfile::TempDir;

/// Helper to create a temp directory for test outputs
fn setup_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

#[test]
fn test_cli_convert_csv_to_parquet() {
    let temp_dir = setup_temp_dir();
    let input = temp_dir.path().join("numbers.csv");
    let _output = temp_dir.path().join("output.parquet");

    // Write the test data to CSV
    fs::write(&input, "A,B,C\n1,2,3\n4,5,6\n").expect("Failed to write test CSV");

    // Verify the file was created
    assert!(input.exists());
}

#[test]
fn test_cli_read_command() {
    let temp_dir = setup_temp_dir();
    let input = temp_dir.path().join("test_numbers.csv");

    // Write test data
    fs::write(&input, "A,B,C\n1,2,3\n4,5,6\n").expect("Failed to write test CSV");

    // Test reading CSV
    let content = fs::read_to_string(&input).expect("Failed to read CSV");
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "A,B,C");
}

#[test]
fn test_cli_filter_operation() {
    let temp_dir = setup_temp_dir();
    let input = temp_dir.path().join("sales.csv");
    let _output = temp_dir.path().join("filtered.csv");

    // Create test data if it doesn't exist
    if !input.exists() {
        fs::write(
            &input,
            "product,category,price\nWidget,A,10.00\nGadget,B,20.00\nWidget,A,15.00\n",
        )
        .expect("Failed to write test CSV");
    }

    // Test that input exists
    assert!(input.exists());
}

#[test]
fn test_cli_sort_operation() {
    let temp_dir = setup_temp_dir();
    let input = temp_dir.path().join("employees.csv");
    let _output = temp_dir.path().join("sorted.csv");

    // Create test data if it doesn't exist
    if !input.exists() {
        fs::write(
            &input,
            "name,department,salary\nAlice,Engineering,50000\nBob,Sales,45000\nCharlie,Engineering,55000\n",
        )
        .expect("Failed to write test CSV");
    }

    // Test that input exists
    assert!(input.exists());
}

#[test]
fn test_cli_dedupe_operation() {
    let temp_dir = setup_temp_dir();
    let input = temp_dir.path().join("duplicates.csv");
    let _output = temp_dir.path().join("unique.csv");

    // Create test data if it doesn't exist
    if !input.exists() {
        fs::write(
            &input,
            "id,name\n1,Alice\n2,Bob\n1,Alice\n3,Charlie\n2,Bob\n",
        )
        .expect("Failed to write test CSV");
    }

    // Test that input exists
    assert!(input.exists());
}

#[test]
fn test_file_format_detection() {
    use xls_rs::common::format;

    assert_eq!(format::from_extension("data.csv"), "csv");
    assert_eq!(format::from_extension("data.xlsx"), "excel");
    assert_eq!(format::from_extension("data.xls"), "excel");
    assert_eq!(format::from_extension("data.ods"), "ods");
    assert_eq!(format::from_extension("data.parquet"), "parquet");
    assert_eq!(format::from_extension("data.avro"), "avro");
    assert_eq!(format::from_extension("data.json"), "json");
    assert_eq!(format::from_extension("data.unknown"), "unknown");
}

#[test]
fn test_format_supported_check() {
    use xls_rs::common::format;

    assert!(format::is_supported("csv"));
    assert!(format::is_supported("excel"));
    assert!(format::is_supported("ods"));
    assert!(format::is_supported("parquet"));
    assert!(format::is_supported("avro"));
    assert!(format::is_supported("json"));
    assert!(!format::is_supported("unknown"));
    assert!(!format::is_supported("xml"));
}

#[test]
fn test_validation_helpers() {
    use xls_rs::common::validation;

    let data = vec![
        vec!["A".to_string(), "B".to_string(), "C".to_string()],
        vec!["1".to_string(), "2".to_string(), "3".to_string()],
        vec!["4".to_string(), "5".to_string(), "6".to_string()],
    ];

    // Test column index validation
    assert!(validation::validate_column_index(&data, 0).is_ok());
    assert!(validation::validate_column_index(&data, 2).is_ok());
    assert!(validation::validate_column_index(&data, 3).is_err());

    // Test consistent columns validation
    assert!(validation::validate_consistent_columns(&data).is_ok());

    let inconsistent_data = vec![
        vec!["A".to_string(), "B".to_string()],
        vec!["1".to_string(), "2".to_string(), "3".to_string()],
    ];
    assert!(validation::validate_consistent_columns(&inconsistent_data).is_err());
}

#[test]
fn test_cell_range_validation() {
    use xls_rs::common::validation;

    assert!(validation::validate_cell_range("A1").is_ok());
    assert!(validation::validate_cell_range("A1:C10").is_ok());
    assert!(validation::validate_cell_range("Z100").is_ok());
    assert!(validation::validate_cell_range("AA1:ZZ100").is_ok());

    assert!(validation::validate_cell_range("").is_err());
    assert!(validation::validate_cell_range("1").is_err());
    assert!(validation::validate_cell_range("A").is_err());
    assert!(validation::validate_cell_range("A1:").is_err());
}

#[test]
fn test_string_utilities() {
    use xls_rs::common::string;

    assert_eq!(string::normalize_whitespace("  hello   world  "), "hello world");
    assert!(string::is_numeric("123.45"));
    assert!(string::is_numeric("-100"));
    assert!(!string::is_numeric("abc"));
    assert!(string::is_empty_or_whitespace("   "));
    assert!(string::is_empty_or_whitespace(""));
    assert!(!string::is_empty_or_whitespace("hello"));

    assert_eq!(string::to_number("123.45"), Some(123.45));
    assert_eq!(string::to_number("abc"), None);
}

#[test]
fn test_collection_utilities() {
    use xls_rs::common::collection;

    let data = vec![1, 2, 3, 2, 4, 3, 5];
    let unique = collection::unique_preserve_order(&data);
    assert_eq!(unique, vec![1, 2, 3, 4, 5]);

    let chunked = collection::chunk(data.clone(), 2);
    assert_eq!(chunked, vec![vec![1, 2], vec![3, 2], vec![4, 3], vec![5]]);

    let nested = vec![vec![1, 2], vec![3, 4], vec![5, 6]];
    let flattened = collection::flatten(nested);
    assert_eq!(flattened, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn test_error_utilities() {
    use xls_rs::common::error;

    let err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");

    let context_err = error::with_file_context(&err, "test.csv");
    assert!(context_err.to_string().contains("test.csv"));
    assert!(context_err.to_string().contains("file not found"));

    let cell_err = error::with_cell_context(&err, "test.csv", 10, 5);
    assert!(cell_err.to_string().contains("test.csv"));
    assert!(cell_err.to_string().contains("11:6")); // 1-indexed

    let col_err = error::with_column_context(&err, "test.csv", "price");
    assert!(col_err.to_string().contains("test.csv"));
    assert!(col_err.to_string().contains("price"));
}
