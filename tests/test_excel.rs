mod common;

use std::fs;
use std::path::Path;
use tempfile::TempDir;
use xls_rs::{
    CellStyle, ChartConfig, DataChartType, DataWriter, ExcelHandler, WriteMode, WriteOptions,
    XlsxWriter,
};

/// Returns a path inside `dir` for a test artifact. Per-test `TempDir`
/// isolation makes name collisions impossible, so no global counter is
/// needed — and the dir is cleaned up automatically on drop (no manual
/// `remove_file`, no artifacts leaking into the repo root on panic).
fn unique_path(dir: &TempDir, name: &str, ext: &str) -> String {
    dir.path()
        .join(format!("{name}.{ext}"))
        .to_string_lossy()
        .to_string()
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

    // Verify content contains expected data.
    // Guards fixture integrity: values with internal spaces must survive
    // the CSV→XLSX generation path intact (regression for truncated
    // "Alice Johnson" → "Alice" fixture).
    assert!(content.contains("Name") && content.contains("ID"));
    assert!(content.contains("Alice Johnson") && content.contains("Bob Smith"));
    assert!(content.contains("Department") && content.contains("Salary"));
    assert!(content.contains("85000") && content.contains("65000"));
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
    let dir = tempfile::tempdir().unwrap();

    let output_path = unique_path(&dir, "excel_rw", "xlsx");

    // Write to Excel
    let options = WriteOptions::default();
    handler.write_styled(&output_path, &data, &options).unwrap();

    assert!(Path::new(&output_path).exists());

    // Read back
    let content = handler.read_with_sheet(&output_path, None).unwrap();
    assert!(!content.is_empty());
    assert!(content.contains("A") || content.contains("10"));
}

#[test]
fn test_excel_write_from_csv() {
    let handler = ExcelHandler::new();
    let csv_path = common::example_path("sales.csv");
    let dir = tempfile::tempdir().unwrap();
    let output_path = unique_path(&dir, "excel_from_csv", "xlsx");

    handler
        .write_from_csv(&csv_path, &output_path, Some("Sales"))
        .unwrap();

    assert!(Path::new(&output_path).exists());

    // Verify sheet name
    let sheets = handler.list_sheets(&output_path).unwrap();
    assert!(sheets.contains(&"Sales".to_string()));
}

#[test]
fn test_excel_write_from_csv_preserves_spaces_and_columns() {
    // Regression: unquoted CSV fields with internal spaces (e.g. "Alice Johnson")
    // and trailing columns must survive the CSV → XLSX ingest path intact.
    let handler = ExcelHandler::new();
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("spaces.csv");
    std::fs::write(
        &csv_path,
        "ID,Name,Department,Salary\n1,Alice Johnson,Engineering,85000\n2,Bob Smith,Sales,65000\n",
    )
    .unwrap();
    let output_path = unique_path(&dir, "excel_csv_spaces", "xlsx");

    handler
        .write_from_csv(csv_path.to_str().unwrap(), &output_path, None)
        .unwrap();

    let data = handler.read_sheet_data(&output_path, None).unwrap();

    assert_eq!(data[0], vec!["ID", "Name", "Department", "Salary"]);
    assert_eq!(data[1], vec!["1", "Alice Johnson", "Engineering", "85000"]);
    assert_eq!(data[2], vec!["2", "Bob Smith", "Sales", "65000"]);
}

#[test]
fn test_excel_read_range() {
    let handler = ExcelHandler::new();
    let csv_path = common::example_path("numbers.csv");
    let dir = tempfile::tempdir().unwrap();
    let excel_path = unique_path(&dir, "excel_range", "xlsx");

    // First create an Excel file
    handler
        .write_from_csv(&csv_path, &excel_path, None)
        .unwrap();

    // Read a specific range
    let range = xls_rs::excel::reader::CellRange::parse("A1:B3").unwrap();
    let data = handler.read_range(&excel_path, &range, None).unwrap();

    assert_eq!(data.len(), 3); // 3 rows (header + 2 data rows)
    assert_eq!(data[0].len(), 2); // 2 columns (A and B)
}

#[test]
fn test_excel_list_sheets() {
    let handler = ExcelHandler::new();
    let csv_path = common::example_path("employees.csv");
    let dir = tempfile::tempdir().unwrap();
    let excel_path = unique_path(&dir, "excel_sheets", "xlsx");

    handler
        .write_from_csv(&csv_path, &excel_path, Some("Employees"))
        .unwrap();

    let sheets = handler.list_sheets(&excel_path).unwrap();

    assert!(!sheets.is_empty());
    assert!(sheets.contains(&"Employees".to_string()));
}

#[test]
fn test_excel_read_as_json() {
    let handler = ExcelHandler::new();
    let csv_path = common::example_path("lookup.csv");
    let dir = tempfile::tempdir().unwrap();
    let excel_path = unique_path(&dir, "excel_json", "xlsx");

    handler
        .write_from_csv(&csv_path, &excel_path, None)
        .unwrap();

    let json = handler.read_as_json(&excel_path, None).unwrap();

    assert!(json.starts_with("["));
    assert!(json.contains("Widget") || json.contains("Gadget"));
}

// ============ Styled Write Tests ============

#[test]
fn test_excel_write_styled_with_header() {
    let handler = ExcelHandler::new();
    let data = read_example_csv("sales");
    let dir = tempfile::tempdir().unwrap();
    let output_path = unique_path(&dir, "excel_styled", "xlsx");

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
    assert_eq!(DataChartType::parse("bar").unwrap(), DataChartType::Bar);
    assert_eq!(
        DataChartType::parse("column").unwrap(),
        DataChartType::Column
    );
    assert_eq!(DataChartType::parse("line").unwrap(), DataChartType::Line);
    assert_eq!(DataChartType::parse("area").unwrap(), DataChartType::Area);
    assert_eq!(DataChartType::parse("pie").unwrap(), DataChartType::Pie);
    assert_eq!(
        DataChartType::parse("scatter").unwrap(),
        DataChartType::Scatter
    );
    assert_eq!(
        DataChartType::parse("doughnut").unwrap(),
        DataChartType::Doughnut
    );
    assert_eq!(
        DataChartType::parse("donut").unwrap(),
        DataChartType::Doughnut
    );
}

#[test]
fn test_chart_type_invalid() {
    assert!(DataChartType::parse("invalid").is_err());
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
    let dir = tempfile::tempdir().unwrap();
    let output_path = unique_path(&dir, "chart_column", "xlsx");

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

    handler
        .write_with_chart(&output_path, &data, &config)
        .unwrap();
    assert!(Path::new(&output_path).exists());

    // Verify data can be read back
    let content = handler.read_with_sheet(&output_path, None).unwrap();
    assert!(content.contains("Category"));
}

#[test]
fn test_write_with_chart_bar() {
    let handler = ExcelHandler::new();
    let data = read_example_csv("numbers");
    let dir = tempfile::tempdir().unwrap();
    let output_path = unique_path(&dir, "chart_bar", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Bar,
        title: Some("Bar Chart".to_string()),
        category_column: 0,
        value_columns: vec![1, 2],
        ..Default::default()
    };

    handler
        .write_with_chart(&output_path, &data, &config)
        .unwrap();
    assert!(Path::new(&output_path).exists());
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
    let dir = tempfile::tempdir().unwrap();
    let output_path = unique_path(&dir, "chart_line", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Line,
        title: Some("Monthly Trend".to_string()),
        category_column: 0,
        value_columns: vec![1, 2],
        show_legend: true,
        ..Default::default()
    };

    handler
        .write_with_chart(&output_path, &data, &config)
        .unwrap();
    assert!(Path::new(&output_path).exists());

    // Verify data readable
    let content = handler.read_with_sheet(&output_path, None).unwrap();
    assert!(content.contains("Month"));
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
    let dir = tempfile::tempdir().unwrap();
    let output_path = unique_path(&dir, "chart_pie", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Pie,
        title: Some("Market Share".to_string()),
        category_column: 0,
        value_columns: vec![1],
        ..Default::default()
    };

    handler
        .write_with_chart(&output_path, &data, &config)
        .unwrap();
    assert!(Path::new(&output_path).exists());
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
    let dir = tempfile::tempdir().unwrap();
    let output_path = unique_path(&dir, "chart_colors", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Column,
        title: Some("Custom Colors".to_string()),
        category_column: 0,
        value_columns: vec![1],
        colors: Some(vec!["FF5733".to_string()]),
        ..Default::default()
    };

    handler
        .write_with_chart(&output_path, &data, &config)
        .unwrap();
    assert!(Path::new(&output_path).exists());
}

#[test]
fn test_write_with_chart_no_legend() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["X".to_string(), "Y".to_string()],
        vec!["A".to_string(), "50".to_string()],
        vec!["B".to_string(), "75".to_string()],
    ];
    let dir = tempfile::tempdir().unwrap();
    let output_path = unique_path(&dir, "chart_no_legend", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Column,
        show_legend: false,
        ..Default::default()
    };

    handler
        .write_with_chart(&output_path, &data, &config)
        .unwrap();
    assert!(Path::new(&output_path).exists());
}

// ============ Write Range Tests ============

#[test]
fn test_write_range() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["X".to_string(), "Y".to_string()],
        vec!["1".to_string(), "2".to_string()],
    ];
    let dir = tempfile::tempdir().unwrap();
    let output_path = unique_path(&dir, "excel_write_range", "xlsx");

    // Write starting at B2 (row 1, col 1)
    handler
        .write_range(&output_path, &data, 1, 1, None)
        .unwrap();

    assert!(Path::new(&output_path).exists());
}

#[test]
fn test_write_range_expand() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["X".to_string(), "Y".to_string()],
        vec!["1".to_string(), "2".to_string()],
    ];
    let dir = tempfile::tempdir().unwrap();
    let output_path = unique_path(&dir, "excel_write_expand", "xlsx");

    handler
        .write_range_with_mode(&output_path, &data, 1, 1, None, WriteMode::Expand)
        .unwrap();

    assert!(Path::new(&output_path).exists());
}

#[test]
fn test_write_range_preserve() {
    let handler = ExcelHandler::new();
    let dir = tempfile::tempdir().unwrap();
    let output_path = unique_path(&dir, "excel_write_preserve", "xlsx");

    // First write baseline data
    let baseline = vec![
        vec!["A".to_string(), "B".to_string(), "C".to_string()],
        vec!["1".to_string(), "2".to_string(), "3".to_string()],
        vec!["4".to_string(), "5".to_string(), "6".to_string()],
    ];
    handler
        .write(&output_path, &baseline, Default::default())
        .unwrap();

    // Overwrite a sub-range starting at B2 (row 1, col 1)
    let patch = vec![vec!["X".to_string()], vec!["Y".to_string()]];
    handler
        .write_range_with_mode(&output_path, &patch, 1, 1, None, WriteMode::Preserve)
        .unwrap();

    assert!(Path::new(&output_path).exists());
}

#[test]
fn test_write_range_overwrite() {
    let handler = ExcelHandler::new();
    let dir = tempfile::tempdir().unwrap();
    let output_path = unique_path(&dir, "excel_write_overwrite", "xlsx");

    // First write baseline data
    let baseline = vec![
        vec!["A".to_string(), "B".to_string()],
        vec!["1".to_string(), "2".to_string()],
    ];
    handler
        .write(&output_path, &baseline, Default::default())
        .unwrap();

    // Overwrite starting at B2 (row 1, col 1)
    let patch = vec![vec!["X".to_string(), "Y".to_string()]];
    handler
        .write_range_with_mode(&output_path, &patch, 1, 1, None, WriteMode::Overwrite)
        .unwrap();

    assert!(Path::new(&output_path).exists());
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

// ============ Cell Typing Consistency Tests ============

#[test]
fn test_cell_typing_consistency_across_writers() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["Num".to_string(), "Str".to_string(), "Empty".to_string()],
        vec!["42.5".to_string(), "hello".to_string(), "".to_string()],
        vec!["0".to_string(), "=SUM(A1)".to_string(), "   ".to_string()],
    ];

    let dir = tempfile::tempdir().unwrap();

    // Test DataWriter::write (uses XlsxWriter::add_data)
    let path1 = unique_path(&dir, "typing_datawriter", "xlsx");
    handler.write(&path1, &data, Default::default()).unwrap();
    let content1 = handler.read_with_sheet(&path1, None).unwrap();
    assert!(content1.contains("42.5"));
    assert!(content1.contains("hello"));

    // Test write_styled (uses add_cell_to_row directly)
    let path2 = unique_path(&dir, "typing_styled", "xlsx");
    handler
        .write_styled(&path2, &data, &WriteOptions::default())
        .unwrap();
    let content2 = handler.read_with_sheet(&path2, None).unwrap();
    assert!(content2.contains("42.5"));
    assert!(content2.contains("hello"));

    // Test write_range_with_mode Expand (uses add_cell_to_row)
    let path3 = unique_path(&dir, "typing_range", "xlsx");
    handler
        .write_range_with_mode(&path3, &data, 0, 0, None, WriteMode::Expand)
        .unwrap();
    let content3 = handler.read_with_sheet(&path3, None).unwrap();
    assert!(content3.contains("42.5"));
    assert!(content3.contains("hello"));
}

#[test]
fn test_read_sheet_data_preserves_commas() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["Name".to_string(), "Description".to_string()],
        vec!["Alice".to_string(), "Loves, commas".to_string()],
        vec!["Bob".to_string(), "Also, loves, them".to_string()],
    ];

    let dir = tempfile::tempdir().unwrap();
    let path = unique_path(&dir, "commas", "xlsx");
    handler.write(&path, &data, Default::default()).unwrap();

    // read_sheet_data preserves commas correctly
    let read = handler.read_sheet_data(&path, None).unwrap();
    assert_eq!(read[1][1], "Loves, commas");
    assert_eq!(read[2][1], "Also, loves, them");

    // read_with_sheet CSV path splits on commas (documented behavior for string output)
    let csv = handler.read_with_sheet(&path, None).unwrap();
    assert!(csv.contains("Loves, commas"));
}

// ============ Parallel Sheet Parsing Tests ============

#[test]
fn test_parallel_read_multi_sheet_order_and_content() {
    // Workbooks with >= 2 sheets take the rayon-parallel parse path in
    // XlsxReader::from_archive. Sheet order and per-sheet content must be
    // identical to the sequential path (indexed collect preserves order).
    let mut writer = XlsxWriter::new();
    let sheet_data: Vec<(&str, Vec<Vec<String>>)> = vec![
        (
            "Alpha",
            vec![
                vec!["Item".to_string(), "Notes".to_string()],
                vec!["1".to_string(), "Red Apple".to_string()],
            ],
        ),
        (
            "Beta",
            vec![
                vec!["Code".to_string(), "Label".to_string()],
                vec!["2".to_string(), "Blue Whale".to_string()],
            ],
        ),
        (
            "Gamma",
            vec![vec!["Name".to_string()], vec!["Carol Davis".to_string()]],
        ),
        (
            "Delta",
            vec![
                vec!["Price".to_string(), "Qty".to_string()],
                vec!["19.99".to_string(), "3".to_string()],
            ],
        ),
    ];
    for (name, data) in &sheet_data {
        writer.add_sheet(name).unwrap();
        writer.add_data(data);
    }
    let dir = tempfile::tempdir().unwrap();
    let path = unique_path(&dir, "parallel_sheets", "xlsx");
    let file = fs::File::create(&path).unwrap();
    writer.save(std::io::BufWriter::new(file)).unwrap();

    let wb = xls_rs::excel::xlsx_reader::XlsxReader::from_path(&path).unwrap();

    // Sheet order preserved
    assert_eq!(wb.sheet_names(), vec!["Alpha", "Beta", "Gamma", "Delta"]);

    // Per-sheet content intact (values with internal spaces, numbers)
    let alpha = wb.get_sheet_by_name("Alpha").unwrap();
    let alpha_rows = alpha.to_string_vec();
    assert_eq!(alpha_rows[1][1], "Red Apple");

    let gamma = wb.get_sheet_by_name("Gamma").unwrap();
    assert_eq!(gamma.to_string_vec()[1][0], "Carol Davis");

    let delta = wb.get_sheet_by_name("Delta").unwrap();
    let delta_rows = delta.to_string_vec();
    assert_eq!(delta_rows[1][0], "19.99");
    assert_eq!(delta_rows[1][1], "3");

    // read_all_to_string_vec returns every sheet with correct data
    let all = wb.read_all_to_string_vec();
    assert_eq!(all.len(), 4);
    assert_eq!(all["Beta"][1][1], "Blue Whale");
}

#[test]
fn test_formula_cell_without_cached_value_does_not_leak_next_value() {
    // Regression: a formula cell written without a cached <v> (what
    // `xls-rs formula` produces) must read as Empty. The value-tag search
    // is now bounded to the cell body; previously it leaked past </c> and
    // stole the next cell's value (A2 displayed B2's "40").
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["10".to_string(), "20".to_string()],
        vec!["30".to_string(), "40".to_string()],
    ];
    let dir = tempfile::tempdir().unwrap();
    let input = unique_path(&dir, "formula_leak_in", "xlsx");
    handler.write(&input, &data, Default::default()).unwrap();

    let output = unique_path(&dir, "formula_leak_out", "xlsx");
    let evaluator = xls_rs::FormulaEvaluator::new();
    evaluator
        .apply_to_excel(&input, &output, "A1+B1", "A2", None)
        .unwrap();

    let wb = xls_rs::excel::xlsx_reader::XlsxReader::from_path(&output).unwrap();
    let rows = wb.get_sheet_by_name("Sheet1").unwrap().to_string_vec();

    assert_eq!(rows[0][0], "10");
    assert_eq!(rows[0][1], "20");
    assert_eq!(
        rows[1][0], "",
        "formula cell without cached value reads empty"
    );
    assert_eq!(
        rows[1][1], "40",
        "value after formula cell must not be stolen"
    );
}

// ============ Shared Strings Compatibility Tests (real-Excel files) ============

#[test]
fn test_read_shared_strings_all_entries_in_order() {
    // Regression: real Excel files store strings in a shared-strings table.
    // The reader previously advanced into the next <si> while skipping the
    // current one, silently dropping every second string.
    let handler = ExcelHandler::new();
    let path = common::fixture_path("shared_strings_basic.xlsx");
    let rows = handler.read_sheet_data(&path, None).unwrap();

    let expected = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot"];
    assert_eq!(rows.len(), expected.len());
    for (i, e) in expected.iter().enumerate() {
        assert_eq!(rows[i][0], *e, "shared string {} must match", i);
    }
}

#[test]
fn test_read_shared_strings_rich_text_concatenates_runs() {
    // Rich-text entries (<si><r><t>..</t></r><r><t>..</t></r></si>) must
    // concatenate all runs; previously only the first run was kept.
    let handler = ExcelHandler::new();
    let path = common::fixture_path("shared_strings_richtext.xlsx");
    let rows = handler.read_sheet_data(&path, None).unwrap();

    assert_eq!(rows[0][0], "Hello World");
    assert_eq!(rows[0][1], "plain entry");
}

#[test]
fn test_apply_formula_beyond_existing_grid() {
    // Regression: `formula --cell F1` on a 4-column sheet used to silently
    // write nothing. The grid must extend to the target cell.
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["10".to_string(), "20".to_string()],
        vec!["30".to_string(), "40".to_string()],
    ];
    let dir = tempfile::tempdir().unwrap();
    let input = unique_path(&dir, "beyond_grid_in", "xlsx");
    handler.write(&input, &data, Default::default()).unwrap();

    let output = unique_path(&dir, "beyond_grid_out", "xlsx");
    let evaluator = xls_rs::FormulaEvaluator::new();
    evaluator
        .apply_to_excel(&input, &output, "A1*3", "E2", None)
        .unwrap();

    let wb = xls_rs::excel::xlsx_reader::XlsxReader::from_path(&output).unwrap();
    let rows = wb.get_sheet_by_name("Sheet1").unwrap().to_string_vec();

    // Existing cells preserved; row 2 extended to column E with formula
    assert_eq!(rows[0][0], "10");
    assert_eq!(rows[1][0], "30", "existing cells preserved");
    assert_eq!(rows[1][1], "40", "existing cells preserved");
    assert_eq!(rows[1].len(), 5, "grid extends to target column E");
}

#[test]
fn test_read_formula_cells_with_cached_values() {
    // Real Excel files cache the last computed result next to the formula:
    // numeric `<f>..</f><v>42</v>`, string-result `t="str"`, or no cache at
    // all. The reader must return the cached value and never leak values
    // from neighboring cells (see formula-leak regression above).
    let handler = ExcelHandler::new();
    let path = common::fixture_path("formula_cached.xlsx");
    let rows = handler.read_sheet_data(&path, None).unwrap();

    assert_eq!(rows[0][0], "21"); // plain input A1
    assert_eq!(rows[0][1], "hello"); // inline string
    assert_eq!(rows[1][1], "42", "numeric cached formula result");
    assert_eq!(rows[2][1], "HELLO", "string cached formula result (t=str)");
    assert_eq!(rows[3][1], "", "uncached formula cell reads empty");
}

#[test]
fn test_read_array_formula_with_cached_value() {
    // Array formulas (<f t="array" ref="...">) written by Excel carry a
    // cached result; the reader must return it and ignore the <f> attributes.
    let handler = ExcelHandler::new();
    let path = common::fixture_path("array_formula.xlsx");
    let rows = handler.read_sheet_data(&path, None).unwrap();

    assert_eq!(rows[0], vec!["1", "2", "3"]);
    assert_eq!(rows[1][0], "array sum");
    assert_eq!(rows[1][1], "6", "cached array formula result");
}
