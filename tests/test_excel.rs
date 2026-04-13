mod common;

use xls_rs::{CellStyle, ChartConfig, DataChartType, ExcelHandler, WriteOptions};
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

fn read_example_csv(name: &str) -> Vec<Vec<String>> {
    ensure_examples();
    let path = common::example_path(&format!("{name}.csv"));
    let content = fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {path}"));
    content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.split(',').map(|s| s.to_string()).collect())
        .collect()
}

// ============ Excel Example File Tests ============

#[test]
fn test_read_excel_sales_example() {
    ensure_examples();
    let handler = ExcelHandler::new();
    let content = handler
        .read_with_sheet(&common::example_path("sales.xlsx"), None)
        .unwrap();

    // Verify content contains expected data
    assert!(content.contains("Product"));
    assert!(content.contains("Laptop") || content.contains("Electronics"));
}

#[test]
fn test_read_missing_sheet_name_lists_available() {
    ensure_examples();
    let handler = ExcelHandler::new();
    let path = common::example_path("sales.xlsx");
    let err = handler
        .read_with_sheet(&path, Some("NoSuchSheet"))
        .unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("NoSuchSheet") && msg.contains("Available sheets"),
        "{msg}"
    );
}

#[test]
fn test_read_excel_employees_example() {
    ensure_examples();
    let handler = ExcelHandler::new();
    let content = handler
        .read_with_sheet(&common::example_path("employees.xlsx"), None)
        .unwrap();

    // Verify content contains expected data
    assert!(content.contains("Name") || content.contains("ID"));
    assert!(content.contains("Alice") || content.contains("Engineering"));
}

#[test]
fn test_excel_example_list_sheets() {
    ensure_examples();
    let handler = ExcelHandler::new();
    let sheets = handler
        .list_sheets(&common::example_path("sales.xlsx"))
        .unwrap();

    // Should have at least one sheet
    assert!(!sheets.is_empty());
}

#[test]
fn test_excel_example_read_as_json() {
    ensure_examples();
    let handler = ExcelHandler::new();
    let json = handler
        .read_as_json(&common::example_path("employees.xlsx"), None)
        .unwrap();

    // Should be valid JSON array
    assert!(json.starts_with("["));
    assert!(json.ends_with("]"));
}

// ============ Excel Read/Write Tests ============

#[test]
fn test_excel_write_and_read() {
    let handler = ExcelHandler::new();
    let data = read_example_csv("numbers");

    let output_path = unique_path("excel_rw", "xlsx");

    // Write to Excel
    let options = WriteOptions::default();
    handler.write_styled(&output_path, &data, &options).unwrap();

    assert!(Path::new(&output_path).exists());

    // Read back
    let content = handler.read_with_sheet(&output_path, None).unwrap();
    assert!(!content.is_empty());
    assert!(content.contains("A") || content.contains("10"));

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_excel_write_from_csv() {
    let handler = ExcelHandler::new();
    let csv_path = common::example_path("sales.csv");
    let output_path = unique_path("excel_from_csv", "xlsx");

    handler
        .write_from_csv(&csv_path, &output_path, Some("Sales"))
        .unwrap();

    assert!(Path::new(&output_path).exists());

    // Verify sheet name
    let sheets = handler.list_sheets(&output_path).unwrap();
    assert!(sheets.contains(&"Sales".to_string()));

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_excel_read_range() {
    let handler = ExcelHandler::new();
    let csv_path = common::example_path("numbers.csv");
    let excel_path = unique_path("excel_range", "xlsx");

    // First create an Excel file
    handler.write_from_csv(&csv_path, &excel_path, None).unwrap();

    // Read a specific range
    let range = xls_rs::CellRange::parse("A1:B3").unwrap();
    let data = handler.read_range(&excel_path, &range, None).unwrap();

    assert_eq!(data.len(), 3); // 3 rows (header + 2 data rows)
    assert_eq!(data[0].len(), 2); // 2 columns (A and B)

    fs::remove_file(&excel_path).ok();
}

#[test]
fn test_excel_list_sheets() {
    let handler = ExcelHandler::new();
    let csv_path = common::example_path("employees.csv");
    let excel_path = unique_path("excel_sheets", "xlsx");

    handler
        .write_from_csv(&csv_path, &excel_path, Some("Employees"))
        .unwrap();

    let sheets = handler.list_sheets(&excel_path).unwrap();

    assert!(!sheets.is_empty());
    assert!(sheets.contains(&"Employees".to_string()));

    fs::remove_file(&excel_path).ok();
}

#[test]
fn test_excel_read_as_json() {
    let handler = ExcelHandler::new();
    let csv_path = common::example_path("lookup.csv");
    let excel_path = unique_path("excel_json", "xlsx");

    handler.write_from_csv(&csv_path, &excel_path, None).unwrap();

    let json = handler.read_as_json(&excel_path, None).unwrap();

    assert!(json.starts_with("["));
    assert!(json.contains("Widget") || json.contains("Gadget"));

    fs::remove_file(&excel_path).ok();
}

// ============ Styled Write Tests ============

#[test]
fn test_excel_write_styled_with_header() {
    let handler = ExcelHandler::new();
    let data = read_example_csv("sales");
    let output_path = unique_path("excel_styled", "xlsx");

    let options = WriteOptions {
        sheet_name: Some("StyledSheet".to_string()),
        style_header: true,
        header_style: CellStyle::header(),
        column_styles: None,
        freeze_header: true,
        auto_filter: true,
        auto_fit: true,
    };

    handler.write_styled(&output_path, &data, &options).unwrap();

    assert!(Path::new(&output_path).exists());

    // Verify content
    let content = handler
        .read_with_sheet(&output_path, Some("StyledSheet"))
        .unwrap();
    assert!(content.contains("Product"));

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_cell_style_header() {
    let style = CellStyle::header();

    assert!(style.bold);
    assert!(style.border);
    assert_eq!(style.bg_color, Some("4472C4".to_string()));
    assert_eq!(style.font_color, Some("FFFFFF".to_string()));
}

#[test]
fn test_cell_style_custom() {
    let _style = CellStyle {
        bold: true,
        italic: true,
        bg_color: Some("FF0000".to_string()),
        font_color: Some("000000".to_string()),
        font_size: Some(14.0),
        border: true,
        align: Some("center".to_string()),
        number_format: Some("#,##0.00".to_string()),
    };

    // Note: to_format() was removed with rust_xlsxwriter dependency
    // CellStyle is now just a data structure for style configuration
    // The custom XLSX writer handles styling internally
}

// ============ Chart Tests ============

#[test]
fn test_chart_type_from_str() {
    assert_eq!(DataChartType::from_str("bar").unwrap(), DataChartType::Bar);
    assert_eq!(
        DataChartType::from_str("column").unwrap(),
        DataChartType::Column
    );
    assert_eq!(
        DataChartType::from_str("line").unwrap(),
        DataChartType::Line
    );
    assert_eq!(
        DataChartType::from_str("area").unwrap(),
        DataChartType::Area
    );
    assert_eq!(DataChartType::from_str("pie").unwrap(), DataChartType::Pie);
    assert_eq!(
        DataChartType::from_str("scatter").unwrap(),
        DataChartType::Scatter
    );
    assert_eq!(
        DataChartType::from_str("doughnut").unwrap(),
        DataChartType::Doughnut
    );
    assert_eq!(
        DataChartType::from_str("donut").unwrap(),
        DataChartType::Doughnut
    );
}

#[test]
fn test_chart_type_invalid() {
    assert!(DataChartType::from_str("invalid").is_err());
}

#[test]
fn test_chart_config_default() {
    let config = ChartConfig::default();

    assert_eq!(config.chart_type, DataChartType::Column);
    assert_eq!(config.category_column, 0);
    assert_eq!(config.value_columns, vec![1]);
    assert_eq!(config.width, 600);
    assert_eq!(config.height, 400);
    assert!(config.show_legend);
}

#[test]
fn test_write_with_chart_column() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["Category".to_string(), "Value".to_string()],
        vec!["A".to_string(), "10".to_string()],
        vec!["B".to_string(), "20".to_string()],
        vec!["C".to_string(), "30".to_string()],
    ];
    let output_path = unique_path("chart_column", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Column,
        title: Some("Test Chart".to_string()),
        x_axis_title: Some("Categories".to_string()),
        y_axis_title: Some("Values".to_string()),
        category_column: 0,
        value_columns: vec![1],
        width: 600,
        height: 400,
        show_legend: true,
        colors: None,
    };

    handler.write_with_chart(&output_path, &data, &config).unwrap();
    assert!(Path::new(&output_path).exists());

    // Verify data can be read back
    let content = handler.read_with_sheet(&output_path, None).unwrap();
    assert!(content.contains("Category"));

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_write_with_chart_bar() {
    let handler = ExcelHandler::new();
    let data = read_example_csv("numbers");
    let output_path = unique_path("chart_bar", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Bar,
        title: Some("Bar Chart".to_string()),
        category_column: 0,
        value_columns: vec![1, 2],
        ..Default::default()
    };

    handler.write_with_chart(&output_path, &data, &config).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_write_with_chart_line() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec![
            "Month".to_string(),
            "Sales".to_string(),
            "Expenses".to_string(),
        ],
        vec!["Jan".to_string(), "100".to_string(), "80".to_string()],
        vec!["Feb".to_string(), "120".to_string(), "90".to_string()],
        vec!["Mar".to_string(), "140".to_string(), "100".to_string()],
    ];
    let output_path = unique_path("chart_line", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Line,
        title: Some("Monthly Trend".to_string()),
        category_column: 0,
        value_columns: vec![1, 2],
        show_legend: true,
        ..Default::default()
    };

    handler.write_with_chart(&output_path, &data, &config).unwrap();
    assert!(Path::new(&output_path).exists());

    // Verify data readable
    let content = handler.read_with_sheet(&output_path, None).unwrap();
    assert!(content.contains("Month"));

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_write_with_chart_pie() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["Category".to_string(), "Share".to_string()],
        vec!["Electronics".to_string(), "45".to_string()],
        vec!["Furniture".to_string(), "30".to_string()],
        vec!["Office".to_string(), "25".to_string()],
    ];
    let output_path = unique_path("chart_pie", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Pie,
        title: Some("Market Share".to_string()),
        category_column: 0,
        value_columns: vec![1],
        ..Default::default()
    };

    handler.write_with_chart(&output_path, &data, &config).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_write_with_chart_custom_colors() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["X".to_string(), "Y".to_string()],
        vec!["1".to_string(), "10".to_string()],
        vec!["2".to_string(), "20".to_string()],
        vec!["3".to_string(), "15".to_string()],
    ];
    let output_path = unique_path("chart_colors", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Column,
        title: Some("Custom Colors".to_string()),
        category_column: 0,
        value_columns: vec![1],
        colors: Some(vec!["FF5733".to_string()]),
        ..Default::default()
    };

    handler.write_with_chart(&output_path, &data, &config).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_write_with_chart_no_legend() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["X".to_string(), "Y".to_string()],
        vec!["A".to_string(), "50".to_string()],
        vec!["B".to_string(), "75".to_string()],
    ];
    let output_path = unique_path("chart_no_legend", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Column,
        show_legend: false,
        ..Default::default()
    };

    handler.write_with_chart(&output_path, &data, &config).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

// ============ Write Range Tests ============

#[test]
fn test_write_range() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["X".to_string(), "Y".to_string()],
        vec!["1".to_string(), "2".to_string()],
    ];
    let output_path = unique_path("excel_write_range", "xlsx");

    // Write starting at B2 (row 1, col 1)
    handler
        .write_range(&output_path, &data, 1, 1, None)
        .unwrap();

    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

// ============ Parse Cell Reference Tests ============

#[test]
fn test_parse_cell_reference() {
    let handler = ExcelHandler::new();

    let (row, col) = handler.parse_cell_reference("A1").unwrap();
    assert_eq!(row, 0);
    assert_eq!(col, 0);

    let (row, col) = handler.parse_cell_reference("B5").unwrap();
    assert_eq!(row, 4);
    assert_eq!(col, 1);

    let (row, col) = handler.parse_cell_reference("Z10").unwrap();
    assert_eq!(row, 9);
    assert_eq!(col, 25);
}
