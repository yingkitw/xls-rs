mod common;

use xls_rs::{Converter, CsvHandler, ExcelHandler};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_path(prefix: &str, ext: &str) -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("test_{prefix}_{id}.{ext}")
}

fn ensure_examples() {
    common::ensure_example_fixtures();
}

// ============ CSV to Excel Conversion ============

#[test]
fn test_convert_csv_to_xlsx() {
    ensure_examples();
    let converter = Converter::new();
    let csv_path = common::example_path("sales.csv");
    let xlsx_path = unique_path("conv_csv_xlsx", "xlsx");

    converter.convert(&csv_path, &xlsx_path, None).unwrap();

    assert!(Path::new(&xlsx_path).exists());

    // Verify content
    let handler = ExcelHandler::new();
    let content = handler.read_with_sheet(&xlsx_path, None).unwrap();
    assert!(content.contains("Product") || content.contains("Laptop"));

    fs::remove_file(&xlsx_path).ok();
}

#[test]
fn test_convert_csv_to_xlsx_with_sheet_name() {
    ensure_examples();
    let converter = Converter::new();
    let csv_path = common::example_path("employees.csv");
    let xlsx_path = unique_path("conv_csv_xlsx_sheet", "xlsx");

    converter
        .convert(&csv_path, &xlsx_path, Some("EmployeeData"))
        .unwrap();

    let handler = ExcelHandler::new();
    let sheets = handler.list_sheets(&xlsx_path).unwrap();
    assert!(sheets.contains(&"EmployeeData".to_string()));

    fs::remove_file(&xlsx_path).ok();
}

// ============ Excel to CSV Conversion ============

#[test]
fn test_convert_xlsx_to_csv() {
    ensure_examples();
    let converter = Converter::new();

    // First create an Excel file
    let csv_path = common::example_path("numbers.csv");
    let xlsx_path = unique_path("conv_xlsx_csv_src", "xlsx");
    let output_csv = unique_path("conv_xlsx_csv_out", "csv");

    converter.convert(&csv_path, &xlsx_path, None).unwrap();

    // Now convert back to CSV
    converter.convert(&xlsx_path, &output_csv, None).unwrap();

    assert!(Path::new(&output_csv).exists());

    let content = fs::read_to_string(&output_csv).unwrap();
    assert!(content.contains("1") || content.contains("2"));

    fs::remove_file(&xlsx_path).ok();
    fs::remove_file(&output_csv).ok();
}

// ============ CSV Handler Tests ============

#[test]
fn test_csv_handler_read() {
    ensure_examples();
    let handler = CsvHandler::new();
    let content = handler.read(&common::example_path("sales.csv")).unwrap();

    assert!(content.contains("Product"));
    assert!(content.contains("Laptop"));
}

#[test]
fn test_csv_handler_read_as_json() {
    ensure_examples();
    let handler = CsvHandler::new();
    let json = handler.read_as_json(&common::example_path("lookup.csv")).unwrap();

    assert!(json.starts_with("["));
    assert!(json.contains("Widget"));
    assert!(json.contains("Gadget"));
}

#[test]
fn test_csv_handler_read_range() {
    ensure_examples();
    let handler = CsvHandler::new();
    let range = xls_rs::CellRange::parse("A1:C3").unwrap();
    let data = handler.read_range(&common::example_path("sales.csv"), &range).unwrap();

    assert_eq!(data.len(), 3); // 3 rows
    assert_eq!(data[0].len(), 3); // 3 columns
    assert_eq!(data[0][0], "Product");
}

#[test]
fn test_csv_handler_write() {
    ensure_examples();
    let handler = CsvHandler::new();
    let input_path = common::example_path("duplicates.csv");
    let output_path = unique_path("csv_write", "csv");

    handler.write_from_csv(&input_path, &output_path).unwrap();

    assert!(Path::new(&output_path).exists());

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("Apple"));

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_csv_handler_append() {
    let handler = CsvHandler::new();

    // Create initial file
    let output_path = unique_path("csv_append", "csv");
    fs::write(&output_path, "A,B\n1,2\n").unwrap();

    // Append data
    let new_data = vec![
        vec!["3".to_string(), "4".to_string()],
        vec!["5".to_string(), "6".to_string()],
    ];
    handler.append_records(&output_path, &new_data).unwrap();

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("1,2"));
    assert!(content.contains("3,4"));
    assert!(content.contains("5,6"));

    fs::remove_file(&output_path).ok();
}

// ============ Cell Range Tests ============

#[test]
fn test_cell_range_parse() {
    let range = xls_rs::CellRange::parse("A1:C5").unwrap();

    assert_eq!(range.start_row, 0);
    assert_eq!(range.start_col, 0);
    assert_eq!(range.end_row, 4);
    assert_eq!(range.end_col, 2);
}

#[test]
fn test_cell_range_parse_single_cell() {
    let range = xls_rs::CellRange::parse("B3").unwrap();

    assert_eq!(range.start_row, 2);
    assert_eq!(range.start_col, 1);
}

#[test]
fn test_cell_range_parse_multi_letter_column() {
    let range = xls_rs::CellRange::parse("AA1:AB10").unwrap();

    assert_eq!(range.start_col, 26); // AA = 26
    assert_eq!(range.end_col, 27); // AB = 27
}

// ============ Multi-Format Conversion Tests ============

#[test]
fn test_convert_csv_to_parquet() {
    ensure_examples();
    let converter = Converter::new();
    let output_path = unique_path("conv_csv_parquet", "parquet");

    converter
        .convert(&common::example_path("sales.csv"), &output_path, None)
        .unwrap();

    assert!(Path::new(&output_path).exists());

    // Verify by reading back
    let handler = xls_rs::ParquetHandler::new();
    let data = handler.read_with_headers(&output_path).unwrap();
    assert!(data.len() > 1);

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_convert_csv_to_avro() {
    ensure_examples();
    let converter = Converter::new();
    let output_path = unique_path("conv_csv_avro", "avro");

    converter
        .convert(&common::example_path("employees.csv"), &output_path, None)
        .unwrap();

    assert!(Path::new(&output_path).exists());

    // Verify by reading back
    let handler = xls_rs::AvroHandler::new();
    let data = handler.read_with_headers(&output_path).unwrap();
    assert!(data.len() > 1);

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_convert_parquet_to_csv() {
    ensure_examples();
    let converter = Converter::new();
    let output_path = unique_path("conv_parquet_csv", "csv");

    converter
        .convert(&common::example_path("sales.parquet"), &output_path, None)
        .unwrap();

    assert!(Path::new(&output_path).exists());

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("Product") || content.contains("Laptop"));

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_convert_avro_to_csv() {
    ensure_examples();
    let converter = Converter::new();
    let output_path = unique_path("conv_avro_csv", "csv");

    converter
        .convert(&common::example_path("employees.avro"), &output_path, None)
        .unwrap();

    assert!(Path::new(&output_path).exists());

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("ID") || content.contains("Name"));

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_convert_xlsx_to_parquet() {
    ensure_examples();
    let converter = Converter::new();
    let output_path = unique_path("conv_xlsx_parquet", "parquet");

    converter
        .convert(&common::example_path("sales.xlsx"), &output_path, None)
        .unwrap();

    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_convert_parquet_to_xlsx() {
    ensure_examples();
    let converter = Converter::new();
    let output_path = unique_path("conv_parquet_xlsx", "xlsx");

    converter
        .convert(&common::example_path("numbers.parquet"), &output_path, None)
        .unwrap();

    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}
