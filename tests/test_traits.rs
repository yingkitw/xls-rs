//! Tests for trait implementations

use xls_rs::{
    AvroHandler, CsvHandler, DataWriteOptions, DataWriter, DefaultFormatDetector, FormatDetector,
    ParquetHandler, SchemaProvider,
};
use std::fs;

#[test]
fn test_csv_handler_traits() {
    use xls_rs::{DataReader, FileHandler};
    let handler = CsvHandler::new();

    // Test FileHandler trait
    assert_eq!(handler.format_name(), "csv");
    assert_eq!(handler.supported_extensions(), &["csv"]);
    assert!(DataReader::supports_format(&handler, "test.csv"));
    assert!(!DataReader::supports_format(&handler, "test.xlsx"));
}

#[test]
fn test_parquet_handler_traits() {
    use xls_rs::{DataReader, FileHandler};
    let handler = ParquetHandler::new();

    // Test FileHandler trait
    assert_eq!(handler.format_name(), "parquet");
    assert_eq!(handler.supported_extensions(), &["parquet"]);
    assert!(DataReader::supports_format(&handler, "test.parquet"));
    assert!(!DataReader::supports_format(&handler, "test.csv"));
}

#[test]
fn test_avro_handler_traits() {
    use xls_rs::{DataReader, FileHandler};
    let handler = AvroHandler::new();

    // Test FileHandler trait
    assert_eq!(handler.format_name(), "avro");
    assert_eq!(handler.supported_extensions(), &["avro"]);
    assert!(DataReader::supports_format(&handler, "test.avro"));
    assert!(!DataReader::supports_format(&handler, "test.csv"));
}

#[test]
fn test_format_detector() {
    let detector = DefaultFormatDetector::new();

    assert_eq!(detector.detect_format("test.csv").unwrap(), "csv");
    assert_eq!(detector.detect_format("test.xlsx").unwrap(), "xlsx");
    assert_eq!(detector.detect_format("test.parquet").unwrap(), "parquet");

    assert!(detector.is_supported("csv"));
    assert!(detector.is_supported("xlsx"));
    assert!(!detector.is_supported("txt"));

    let formats = detector.supported_formats();
    assert!(formats.contains(&"csv".to_string()));
    assert!(formats.contains(&"parquet".to_string()));
}

#[test]
fn test_csv_read_write_traits() {
    let handler = CsvHandler::new();
    let test_file = "/tmp/test_traits.csv";

    // Clean up if exists
    fs::remove_file(test_file).ok();

    // Write data using trait
    let data = vec![
        vec!["name".to_string(), "age".to_string()],
        vec!["Alice".to_string(), "30".to_string()],
        vec!["Bob".to_string(), "25".to_string()],
    ];

    let options = DataWriteOptions {
        include_headers: false,
        ..Default::default()
    };

    handler.write(test_file, &data, options).unwrap();

    // Read data using trait
    use xls_rs::DataReader;
    let read_data = DataReader::read(&handler, test_file).unwrap();
    assert_eq!(read_data.len(), 3);
    assert_eq!(read_data[0][0], "name");
    assert_eq!(read_data[1][0], "Alice");

    // Test schema provider
    let schema = handler.get_schema(test_file).unwrap();
    assert_eq!(schema.len(), 2);

    let column_names = handler.get_column_names(test_file).unwrap();
    assert_eq!(column_names[0], "name");

    // Clean up
    fs::remove_file(test_file).ok();
}

#[test]
fn test_cell_range_provider() {
    use xls_rs::{CellRangeHelper, CellRangeProvider};

    let provider = CellRangeHelper;

    // Test parsing
    let range = provider.parse_range("A1:C3").unwrap();
    assert_eq!(range.start_row, 0);
    assert_eq!(range.start_col, 0);
    assert_eq!(range.end_row, 2);
    assert_eq!(range.end_col, 2);

    // Test cell reference conversion
    let ref_str = provider.to_cell_reference(0, 0);
    assert_eq!(ref_str, "A1");

    let ref_str = provider.to_cell_reference(2, 2);
    assert_eq!(ref_str, "C3");

    // Test parsing cell reference
    let (row, col) = provider.from_cell_reference("B2").unwrap();
    assert_eq!(row, 1);
    assert_eq!(col, 1);
}
