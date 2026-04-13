mod common;

use xls_rs::{AvroHandler, ParquetHandler};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_path(prefix: &str, ext: &str) -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("test_{prefix}_{id}.{ext}")
}

fn ensure_example_artifacts() {
    common::ensure_example_fixtures();
}

// ============ Parquet Example File Tests ============

#[test]
fn test_read_parquet_sales_example() {
    ensure_example_artifacts();
    let handler = ParquetHandler::new();
    let data = handler.read_with_headers(&common::example_path("sales.parquet")).unwrap();

    // Verify header
    assert_eq!(data[0][0], "Product");
    assert_eq!(data[0][1], "Category");
    assert_eq!(data[0][2], "Price");

    // Verify data rows exist
    assert!(data.len() > 1);

    // Verify some data content
    let has_laptop = data.iter().any(|r| r[0] == "Laptop");
    assert!(has_laptop, "Should contain Laptop product");
}

#[test]
fn test_read_parquet_employees_example() {
    ensure_example_artifacts();
    let handler = ParquetHandler::new();
    let data = handler
        .read_with_headers(&common::example_path("employees.parquet"))
        .unwrap();

    // Verify header
    assert_eq!(data[0][0], "ID");
    assert_eq!(data[0][1], "Name");
    assert_eq!(data[0][2], "Department");

    // Verify data rows
    assert!(data.len() > 1);

    // Verify some employee exists
    let has_alice = data.iter().any(|r| r.len() > 1 && r[1].contains("Alice"));
    assert!(has_alice, "Should contain Alice");
}

#[test]
fn test_read_parquet_numbers_example() {
    ensure_example_artifacts();
    let handler = ParquetHandler::new();
    let data = handler
        .read_with_headers(&common::example_path("numbers.parquet"))
        .unwrap();

    // Verify header
    assert_eq!(data[0][0], "A");
    assert_eq!(data[0][1], "B");

    // Verify numeric data exists
    assert!(data.len() > 1);
}

// ============ Avro Example File Tests ============

#[test]
fn test_read_avro_sales_example() {
    ensure_example_artifacts();
    let handler = AvroHandler::new();
    let data = handler.read_with_headers(&common::example_path("sales.avro")).unwrap();

    // Verify header
    assert_eq!(data[0][0], "Product");
    assert_eq!(data[0][1], "Category");

    // Verify data rows exist
    assert!(data.len() > 1);
}

#[test]
fn test_read_avro_employees_example() {
    ensure_example_artifacts();
    let handler = AvroHandler::new();
    let data = handler
        .read_with_headers(&common::example_path("employees.avro"))
        .unwrap();

    // Verify header
    assert_eq!(data[0][0], "ID");
    assert_eq!(data[0][1], "Name");

    // Verify data rows
    assert!(data.len() > 1);
}

#[test]
fn test_read_avro_lookup_example() {
    ensure_example_artifacts();
    let handler = AvroHandler::new();
    let data = handler.read_with_headers(&common::example_path("lookup.avro")).unwrap();

    // Verify header
    assert_eq!(data[0][0], "Code");
    assert_eq!(data[0][1], "Name");

    // Verify data rows
    assert!(data.len() > 1);
}

// ============ Parquet Write/Read Tests ============

#[test]
fn test_parquet_write_read_basic() {
    let handler = ParquetHandler::new();
    let header = vec!["Name".to_string(), "Value".to_string()];
    let data = vec![
        vec!["A".to_string(), "100".to_string()],
        vec!["B".to_string(), "200".to_string()],
    ];
    let path = unique_path("parquet_basic", "parquet");

    // Write with explicit column names
    handler.write(&path, &data, Some(&header)).unwrap();
    assert!(Path::new(&path).exists());

    let read_data = handler.read_with_headers(&path).unwrap();
    // Header + data rows
    assert!(read_data.len() >= 2);
    assert_eq!(read_data[0][0], "Name");

    fs::remove_file(&path).ok();
}

#[test]
fn test_parquet_write_and_read() {
    let handler = ParquetHandler::new();
    let header = vec!["ID".to_string(), "Score".to_string()];
    let data = vec![
        vec!["1".to_string(), "95".to_string()],
        vec!["2".to_string(), "87".to_string()],
    ];
    let path = unique_path("parquet_wr", "parquet");

    handler.write(&path, &data, Some(&header)).unwrap();

    let read_data = handler.read_with_headers(&path).unwrap();

    // Verify header
    assert_eq!(read_data[0][0], "ID");
    assert_eq!(read_data[0][1], "Score");

    // Verify data exists
    assert!(read_data.len() > 1);

    fs::remove_file(&path).ok();
}

#[test]
fn test_parquet_file_exists() {
    let handler = ParquetHandler::new();
    let header = vec!["Col1".to_string()];
    let data = vec![vec!["test".to_string()]];
    let path = unique_path("parquet_exists", "parquet");

    handler.write(&path, &data, Some(&header)).unwrap();

    assert!(Path::new(&path).exists());

    fs::remove_file(&path).ok();
}

// ============ Avro Tests ============

#[test]
fn test_avro_write_read_basic() {
    let handler = AvroHandler::new();
    let header = vec!["Name".to_string(), "Value".to_string()];
    let data = vec![
        vec!["X".to_string(), "10".to_string()],
        vec!["Y".to_string(), "20".to_string()],
    ];
    let path = unique_path("avro_basic", "avro");

    handler.write(&path, &data, Some(&header)).unwrap();
    assert!(Path::new(&path).exists());

    let read_data = handler.read_with_headers(&path).unwrap();
    // Verify we got data back
    assert!(read_data.len() >= 2);
    assert_eq!(read_data[0][0], "Name");

    fs::remove_file(&path).ok();
}

#[test]
fn test_avro_write_and_read() {
    let handler = AvroHandler::new();
    let header = vec!["ID".to_string(), "Label".to_string()];
    let data = vec![
        vec!["1".to_string(), "Alice".to_string()],
        vec!["2".to_string(), "Bob".to_string()],
    ];
    let path = unique_path("avro_wr", "avro");

    handler.write(&path, &data, Some(&header)).unwrap();

    let read_data = handler.read_with_headers(&path).unwrap();

    // Verify header
    assert_eq!(read_data[0][0], "ID");
    assert_eq!(read_data[0][1], "Label");

    // Verify data exists
    assert!(read_data.len() > 1);

    fs::remove_file(&path).ok();
}

#[test]
fn test_avro_file_exists() {
    let handler = AvroHandler::new();
    let header = vec!["Col1".to_string()];
    let data = vec![vec!["test".to_string()]];
    let path = unique_path("avro_exists", "avro");

    handler.write(&path, &data, Some(&header)).unwrap();

    assert!(Path::new(&path).exists());

    fs::remove_file(&path).ok();
}

// ============ Schema Tests ============

#[test]
fn test_parquet_get_schema() {
    let handler = ParquetHandler::new();
    let header = vec!["Name".to_string(), "Age".to_string(), "Active".to_string()];
    let data = vec![vec![
        "Alice".to_string(),
        "30".to_string(),
        "true".to_string(),
    ]];
    let path = unique_path("parquet_schema", "parquet");

    handler.write(&path, &data, Some(&header)).unwrap();

    let schema = handler.get_schema(&path).unwrap();

    assert_eq!(schema.len(), 3);
    assert_eq!(schema[0].0, "Name");
    assert_eq!(schema[1].0, "Age");
    assert_eq!(schema[2].0, "Active");

    fs::remove_file(&path).ok();
}
