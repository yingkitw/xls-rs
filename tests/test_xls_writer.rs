//! Integration tests for the from-scratch XLS (BIFF8) writer.
//!
//! These tests use the native `XlsReader` to read back files produced by
//! `XlsWriter`, exercising the BIFF8 / OLE2 byte stream we generate in
//! `src/excel/xls_writer/` without any external spreadsheet library.

use xls_rs::excel::xls_reader::XlsReader;
use xls_rs::{XlsRowData, XlsWriter};

fn make_writer() -> XlsWriter {
    let mut w = XlsWriter::new();
    w.add_sheet("Sheet1").unwrap();
    w
}

#[test]
fn magic_and_size() {
    let mut w = make_writer();
    let mut r = XlsRowData::new();
    r.add_string("a");
    w.add_row(r);
    let bytes = w.to_bytes().unwrap();
    assert_eq!(&bytes[0..8], &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
    assert_eq!(u16::from_le_bytes([bytes[26], bytes[27]]), 0x0003);
}

#[test]
fn round_trip_strings() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("strings.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("Greetings").unwrap();
    let mut r = XlsRowData::new();
    r.add_string("hello");
    r.add_string("world");
    r.add_string("héllo, wörld");
    w.add_row(r);
    w.save(path.to_str().unwrap()).unwrap();

    let wb = XlsReader::from_path(path.to_str().unwrap()).expect("native reader must open our xls");
    let names = wb.sheet_names();
    assert_eq!(names, vec!["Greetings".to_string()]);
    let sheet = wb.get_sheet_by_name("Greetings").expect("sheet present");
    let rows: Vec<Vec<String>> = sheet.to_string_vec();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], vec!["hello", "world", "héllo, wörld"]);
}

#[test]
fn round_trip_numbers_and_bools() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("nums.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("Data").unwrap();
    let mut r1 = XlsRowData::new();
    r1.add_number(1.0);
    r1.add_number(2.5);
    r1.add_number(-3.5);
    r1.add_bool(true);
    r1.add_bool(false);
    w.add_row(r1);
    let mut r2 = XlsRowData::new();
    r2.add_number(0.0);
    w.add_row(r2);
    w.save(path.to_str().unwrap()).unwrap();

    let wb = XlsReader::from_path(path.to_str().unwrap()).expect("native reader must open our xls");
    let sheet = wb.get_sheet_by_name("Data").expect("sheet present");
    let rows: Vec<Vec<String>> = sheet.to_string_vec();
    assert_eq!(rows[0][0], "1");
    assert_eq!(rows[0][1], "2.5");
    assert_eq!(rows[0][2], "-3.5");
    assert!(rows[0][3] == "TRUE" || rows[0][3] == "1" || rows[0][3] == "true");
    assert!(rows[0][4] == "FALSE" || rows[0][4] == "0" || rows[0][4] == "false");
}

#[test]
fn round_trip_multiple_sheets() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("multi.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("First").unwrap();
    let mut r1 = XlsRowData::new();
    r1.add_string("first-row");
    w.add_row(r1);
    w.add_sheet("Second").unwrap();
    let mut r2 = XlsRowData::new();
    r2.add_string("second-row");
    w.add_row(r2);
    w.add_sheet("Third").unwrap();
    let mut r3 = XlsRowData::new();
    r3.add_string("third-row");
    w.add_row(r3);
    w.save(path.to_str().unwrap()).unwrap();

    let wb = XlsReader::from_path(path.to_str().unwrap()).expect("native reader must open our xls");
    let names = wb.sheet_names();
    assert_eq!(names.len(), 3);
    assert_eq!(names[0], "First");
    assert_eq!(names[1], "Second");
    assert_eq!(names[2], "Third");
}

#[test]
fn round_trip_grid() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("grid.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("Grid").unwrap();
    let mut r1 = XlsRowData::new();
    r1.add_string("A");
    r1.add_string("B");
    r1.add_string("C");
    w.add_row(r1);
    let mut r2 = XlsRowData::new();
    r2.add_number(1.0);
    r2.add_number(2.0);
    r2.add_number(3.0);
    w.add_row(r2);
    let mut r3 = XlsRowData::new();
    r3.add_string("x");
    r3.add_empty();
    r3.add_string("y");
    w.add_row(r3);
    w.save(path.to_str().unwrap()).unwrap();

    let wb = XlsReader::from_path(path.to_str().unwrap()).expect("native reader must open our xls");
    let sheet = wb.get_sheet_by_name("Grid").expect("sheet present");
    let rows: Vec<Vec<String>> = sheet.to_string_vec();
    assert_eq!(rows[0], vec!["A", "B", "C"]);
    assert_eq!(rows[1], vec!["1", "2", "3"]);
    assert_eq!(rows[2][0], "x");
    assert_eq!(rows[2][2], "y");
}

#[test]
fn round_trip_formula_cached_value() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("formula.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("Calc").unwrap();
    let mut r = XlsRowData::new();
    r.add_number(2.0);
    r.add_number(3.0);
    w.add_row(r);
    let mut r2 = XlsRowData::new();
    r2.add_formula("A1+B1");
    w.add_row(r2);
    w.save(path.to_str().unwrap()).unwrap();

    let wb = XlsReader::from_path(path.to_str().unwrap()).expect("native reader must open our xls");
    let sheet = wb.get_sheet_by_name("Calc").expect("sheet present");
    let rows: Vec<Vec<String>> = sheet.to_string_vec();
    assert!(rows.len() >= 2);
}

#[test]
fn sheet_name_validation() {
    let mut w = XlsWriter::new();
    assert!(w.add_sheet("").is_err());
    assert!(w.add_sheet(&"a".repeat(32)).is_err());
    assert!(w.add_sheet("with/slash").is_err());
    assert!(w.add_sheet("with\\backslash").is_err());
    assert!(w.add_sheet("with?qmark").is_err());
    assert!(w.add_sheet("with*star").is_err());
    assert!(w.add_sheet("with[bracket").is_err());
    assert!(w.add_sheet("with]bracket").is_err());
    assert!(w.add_sheet("with:colon").is_err());
    assert!(w.add_sheet("'leading-apos").is_err());
    assert!(w.add_sheet("OK Name").is_ok());
    assert!(w.add_sheet("日本語").is_ok());
}

#[test]
fn add_data_classifies_cells() {
    let mut w = XlsWriter::new();
    w.add_sheet("D").unwrap();
    w.add_data(&[
        vec!["Name".into(), "Age".into(), "Active".into()],
        vec!["Alice".into(), "30".into(), "TRUE".into()],
        vec!["Bob".into(), "25".into(), "FALSE".into()],
    ]);
    let bytes = w.to_bytes().unwrap();
    assert_eq!(&bytes[0..8], &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
    let wb = XlsReader::from_bytes(&bytes).expect("read back");
    let sheet = wb.get_sheet_by_name("D").expect("sheet present");
    let rows: Vec<Vec<String>> = sheet.to_string_vec();
    assert_eq!(rows[0], vec!["Name", "Age", "Active"]);
    assert_eq!(rows[1][0], "Alice");
    assert_eq!(rows[1][1], "30");
    assert!(rows[1][2] == "TRUE" || rows[1][2] == "1" || rows[1][2] == "true");
    assert!(rows[2][2] == "FALSE" || rows[2][2] == "0" || rows[2][2] == "false");
}

#[test]
fn empty_sheet_still_readable() {
    let mut w = XlsWriter::new();
    w.add_sheet("Empty").unwrap();
    let bytes = w.to_bytes().unwrap();
    let wb = XlsReader::from_bytes(&bytes).expect("read back");
    let names = wb.sheet_names();
    assert_eq!(names, vec!["Empty".to_string()]);
}

#[test]
fn column_widths_round_trip() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("widths.xls");
    let mut w = XlsWriter::new();
    w.add_sheet("W").unwrap();
    w.set_column_width(0, 25.0);
    w.set_column_width(1, 12.5);
    let mut r = XlsRowData::new();
    r.add_string("wide");
    r.add_string("narrow");
    w.add_row(r);
    w.save(path.to_str().unwrap()).unwrap();

    let wb = XlsReader::from_path(path.to_str().unwrap()).expect("native reader opens our xls");
    let sheet = wb.get_sheet_by_name("W").expect("sheet present");
    let rows: Vec<Vec<String>> = sheet.to_string_vec();
    assert_eq!(rows[0], vec!["wide", "narrow"]);
}

#[test]
fn utf16_strings_with_astral_codepoints() {
    let mut w = XlsWriter::new();
    w.add_sheet("U").unwrap();
    let mut r = XlsRowData::new();
    r.add_string("ascii");
    r.add_string("café");
    r.add_string("🦀 rust");
    w.add_row(r);
    let bytes = w.to_bytes().unwrap();
    let wb = XlsReader::from_bytes(&bytes).expect("read back");
    let sheet = wb.get_sheet_by_name("U").expect("sheet present");
    let rows: Vec<Vec<String>> = sheet.to_string_vec();
    assert_eq!(rows[0][0], "ascii");
    assert_eq!(rows[0][1], "café");
    assert_eq!(rows[0][2], "🦀 rust");
}

#[test]
fn formula_with_sum_function_round_trip() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("sum.xls");
    let mut w = XlsWriter::new();
    w.add_sheet("S").unwrap();
    let mut r1 = XlsRowData::new();
    r1.add_number(10.0);
    r1.add_number(20.0);
    r1.add_number(30.0);
    w.add_row(r1);
    let mut r2 = XlsRowData::new();
    r2.add_formula("SUM(A1:C1)");
    w.add_row(r2);
    w.save(path.to_str().unwrap()).unwrap();

    let wb = XlsReader::from_path(path.to_str().unwrap()).expect("native reader opens our xls");
    let names = wb.sheet_names();
    assert_eq!(names, vec!["S".to_string()]);
}

#[test]
fn converter_writes_xls_via_csv() {
    // End-to-end test of the converter path used by the CLI: `xls-rs convert
    // --input foo.csv --output foo.xls`.
    let dir = tempfile::TempDir::new().unwrap();
    let csv_path = dir.path().join("in.csv");
    let xls_path = dir.path().join("out.xls");
    std::fs::write(&csv_path, "Name,Age\nAlice,30\nBob,25\n").unwrap();

    let converter = xls_rs::Converter::new();
    converter
        .convert(csv_path.to_str().unwrap(), xls_path.to_str().unwrap(), None)
        .expect("CSV → XLS conversion succeeds");

    let wb = XlsReader::from_path(xls_path.to_str().unwrap()).expect("native reader reads converted xls");
    let sheet = wb.get_sheet_by_name("Sheet1").expect("Sheet1 present");
    let rows: Vec<Vec<String>> = sheet.to_string_vec();
    assert_eq!(rows[0], vec!["Name", "Age"]);
    assert_eq!(rows[1][0], "Alice");
    assert_eq!(rows[1][1], "30");
    assert_eq!(rows[2][0], "Bob");
    assert_eq!(rows[2][1], "25");
}

#[test]
fn excel_handler_write_xls_round_trip() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("from_handler.xls");
    let handler = xls_rs::ExcelHandler::new();
    let data = vec![
        vec!["col1".to_string(), "col2".to_string()],
        vec!["1".to_string(), "2".to_string()],
        vec!["3".to_string(), "4".to_string()],
    ];
    handler
        .write_xls(path.to_str().unwrap(), &data, Some("Data"))
        .expect("write_xls succeeds");

    let wb = XlsReader::from_path(path.to_str().unwrap()).expect("native reader opens our xls");
    let names = wb.sheet_names();
    assert_eq!(names, vec!["Data".to_string()]);
    let sheet = wb.get_sheet_by_name("Data").expect("Data sheet present");
    let rows: Vec<Vec<String>> = sheet.to_string_vec();
    assert_eq!(rows[0], vec!["col1", "col2"]);
    assert_eq!(rows[1], vec!["1", "2"]);
    assert_eq!(rows[2], vec!["3", "4"]);
}

#[test]
fn formula_cached_value_round_trip() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("formula_cached.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("Calc").unwrap();
    let mut r = XlsRowData::new();
    r.add_number(10.0);
    r.add_number(20.0);
    w.add_row(r);
    let mut r2 = XlsRowData::new();
    r2.add_formula("A1+B1");
    w.add_row(r2);
    w.save(path.to_str().unwrap()).unwrap();

    let wb = XlsReader::from_path(path.to_str().unwrap()).expect("reader opens xls");
    let sheet = wb.get_sheet_by_name("Calc").expect("sheet present");
    let rows: Vec<Vec<String>> = sheet.to_string_vec();
    assert!(rows.len() >= 2);
    // The formula's cached value (0.0 placeholder) should be read back as "0",
    // not lost as empty
    assert_eq!(rows[1][0], "0");
}

#[test]
fn merged_cells_round_trip() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("merged.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("S").unwrap();
    let mut r = XlsRowData::new();
    r.add_string("Title");
    r.add_string("B");
    r.add_string("C");
    w.add_row(r);
    w.merge_cells(0, 0, 0, 2); // merge A1:C1
    w.save(path.to_str().unwrap()).unwrap();

    // File should be readable
    let wb = XlsReader::from_path(path.to_str().unwrap()).expect("reader opens merged xls");
    let sheet = wb.get_sheet_by_name("S").expect("sheet present");
    let rows: Vec<Vec<String>> = sheet.to_string_vec();
    assert_eq!(rows[0][0], "Title");
}

#[test]
fn freeze_panes_round_trip() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("freeze.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("S").unwrap();
    let mut r = XlsRowData::new();
    r.add_string("Header");
    w.add_row(r);
    let mut r2 = XlsRowData::new();
    r2.add_string("Data");
    w.add_row(r2);
    w.freeze_panes(1, 0); // freeze first row
    w.save(path.to_str().unwrap()).unwrap();

    let wb = XlsReader::from_path(path.to_str().unwrap()).expect("reader opens freeze xls");
    assert_eq!(wb.sheet_names(), vec!["S".to_string()]);
}

#[test]
fn auto_filter_round_trip() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("filter.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("S").unwrap();
    let mut r = XlsRowData::new();
    r.add_string("Name");
    r.add_string("Age");
    w.add_row(r);
    let mut r2 = XlsRowData::new();
    r2.add_string("Alice");
    r2.add_string("30");
    w.add_row(r2);
    w.set_auto_filter(0, 0, 1, 1);
    w.save(path.to_str().unwrap()).unwrap();

    let wb = XlsReader::from_path(path.to_str().unwrap()).expect("reader opens filter xls");
    let sheet = wb.get_sheet_by_name("S").expect("sheet present");
    let rows: Vec<Vec<String>> = sheet.to_string_vec();
    assert_eq!(rows[0], vec!["Name", "Age"]);
    assert_eq!(rows[1], vec!["Alice", "30"]);
}

#[test]
fn error_cell_round_trip() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("error.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("S").unwrap();
    let mut r = XlsRowData::new();
    r.add_error("#N/A");
    r.add_error("#DIV/0!");
    w.add_row(r);
    w.save(path.to_str().unwrap()).unwrap();

    let wb = XlsReader::from_path(path.to_str().unwrap()).expect("reader opens error xls");
    let sheet = wb.get_sheet_by_name("S").expect("sheet present");
    let rows: Vec<Vec<String>> = sheet.to_string_vec();
    assert_eq!(rows[0][0], "#N/A");
    assert_eq!(rows[0][1], "#DIV/0!");
}

#[test]
fn add_data_with_error_and_formula() {
    let mut w = XlsWriter::new();
    w.add_sheet("D").unwrap();
    w.add_data(&[
        vec!["Name".into(), "Status".into()],
        vec!["Alice".into(), "#N/A".into()],
        vec!["=B1".into(), "OK".into()],
    ]);
    let bytes = w.to_bytes().unwrap();
    assert_eq!(&bytes[0..8], &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
}

#[test]
fn rich_features_combined() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("rich.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("Report").unwrap();
    // Header row with merged title
    let mut r = XlsRowData::new();
    r.add_string("Sales Report");
    w.add_row(r);
    w.merge_cells(0, 0, 0, 2);
    // Column headers
    let mut r2 = XlsRowData::new();
    r2.add_string("Product");
    r2.add_string("Q1");
    r2.add_string("Q2");
    w.add_row(r2);
    // Data with formula
    let mut r3 = XlsRowData::new();
    r3.add_string("Widget");
    r3.add_number(100.0);
    r3.add_number(200.0);
    w.add_row(r3);
    let mut r4 = XlsRowData::new();
    r4.add_string("Total");
    r4.add_formula("B3+C3");
    w.add_row(r4);
    // Rich features
    w.freeze_panes(2, 0); // freeze header rows
    w.set_auto_filter(1, 0, 3, 2);
    w.set_column_width(0, 15.0);

    w.save(path.to_str().unwrap()).unwrap();

    let wb = XlsReader::from_path(path.to_str().unwrap()).expect("reader opens rich xls");
    let sheet = wb.get_sheet_by_name("Report").expect("sheet present");
    let rows: Vec<Vec<String>> = sheet.to_string_vec();
    assert_eq!(rows[0][0], "Sales Report");
    assert_eq!(rows[1], vec!["Product", "Q1", "Q2"]);
    assert_eq!(rows[2], vec!["Widget", "100", "200"]);
}

#[test]
#[ignore = "generates sample artifacts; run with --ignored --nocapture"]
fn generate_sample_xls_files() {
    use std::path::PathBuf;
    use xls_rs::excel::{RowData, WriteOptions, XlsxWriter};

    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output");
    std::fs::create_dir_all(&out_dir).unwrap();

    // ── sales_rich.xlsx: formulas, freeze panes, auto-filter ──
    {
        let options = WriteOptions {
            freeze_header: true,
            auto_filter: true,
            ..Default::default()
        };
        let mut w = XlsxWriter::with_options(options);
        w.add_sheet("Sales").unwrap();

        let mut hdr = RowData::new();
        hdr.add_string("Product");
        hdr.add_string("Category");
        hdr.add_string("Price");
        hdr.add_string("Quantity");
        hdr.add_string("Revenue");
        w.add_row(hdr);

        let products: &[(&str, &str, f64, f64)] = &[
            ("Laptop", "Electronics", 1200.0, 1.0),
            ("Mouse", "Electronics", 25.0, 2.0),
            ("Desk", "Furniture", 300.0, 1.0),
            ("Chair", "Furniture", 150.0, 4.0),
            ("Pen", "Stationery", 2.0, 10.0),
            ("Lamp", "Home", 45.0, 1.0),
        ];
        for (i, (name, cat, price, qty)) in products.iter().enumerate() {
            let mut row = RowData::new();
            row.add_string(*name);
            row.add_string(*cat);
            row.add_number(*price);
            row.add_number(*qty);
            let row_num = i + 2;
            row.add_formula(format!("C{}*D{}", row_num, row_num));
            w.add_row(row);
        }

        let mut totals = RowData::new();
        totals.add_string("Total");
        totals.add_empty();
        totals.add_empty();
        totals.add_empty();
        totals.add_formula("SUM(E2:E7)");
        w.add_row(totals);

        w.set_column_width(0, 12.0);
        w.set_column_width(1, 14.0);

        let path = out_dir.join("sales_rich.xlsx");
        w.save(std::fs::File::create(&path).unwrap()).unwrap();
        println!("Created {}", path.display());
    }

    // ── employees_rich.xlsx: merged cells, AVERAGE formula ──
    {
        let mut w = XlsxWriter::new();
        w.add_sheet("Employees").unwrap();

        let mut title = RowData::new();
        title.add_string("Employee Directory");
        w.add_row(title);
        w.add_merge_cell(0, 0, 0, 3);

        let mut hdr = RowData::new();
        hdr.add_string("ID");
        hdr.add_string("Name");
        hdr.add_string("Department");
        hdr.add_string("Salary");
        w.add_row(hdr);

        let employees: &[(f64, &str, &str, f64)] = &[
            (1.0, "Alice Johnson", "Engineering", 85000.0),
            (2.0, "Bob Smith", "Sales", 65000.0),
            (3.0, "Carol Davis", "Engineering", 92000.0),
            (4.0, "Dan Miller", "Marketing", 72000.0),
            (6.0, "Grace Anderson", "Engineering", 81000.0),
            (7.0, "Henry Wilson", "Engineering", 95000.0),
        ];
        for (id, name, dept, salary) in employees {
            let mut row = RowData::new();
            row.add_number(*id);
            row.add_string(*name);
            row.add_string(*dept);
            row.add_number(*salary);
            w.add_row(row);
        }

        let mut summary = RowData::new();
        summary.add_empty();
        summary.add_string("Average Salary");
        summary.add_empty();
        summary.add_formula("AVERAGE(D3:D8)");
        w.add_row(summary);

        let mut err_row = RowData::new();
        err_row.add_empty();
        err_row.add_string("Missing Entry");
        err_row.add_string("#N/A");
        err_row.add_string("#VALUE!");
        w.add_row(err_row);

        w.set_column_width(1, 18.0);
        w.set_column_width(2, 14.0);

        let path = out_dir.join("employees_rich.xlsx");
        w.save(std::fs::File::create(&path).unwrap()).unwrap();
        println!("Created {}", path.display());
    }

    // ── budget_rich.xlsx: multi-sheet, IF formula, cross-sheet SUM ──
    {
        let mut w = XlsxWriter::new();

        w.add_sheet("Budget").unwrap();
        let mut hdr = RowData::new();
        hdr.add_string("Item");
        hdr.add_string("Budgeted");
        hdr.add_string("Actual");
        hdr.add_string("Status");
        w.add_row(hdr);

        let budget: &[(&str, f64, f64)] = &[
            ("Rent", 2000.0, 2000.0),
            ("Food", 500.0, 620.0),
            ("Transport", 300.0, 280.0),
            ("Entertainment", 200.0, 350.0),
        ];
        for (i, (item, budgeted, actual)) in budget.iter().enumerate() {
            let mut row = RowData::new();
            row.add_string(*item);
            row.add_number(*budgeted);
            row.add_number(*actual);
            let row_num = i + 2;
            row.add_formula(format!("IF(C{}<=B{},\"OK\",\"OVER\")", row_num, row_num));
            w.add_row(row);
        }

        w.add_sheet("Summary").unwrap();
        let mut r = RowData::new();
        r.add_string("Total Budget");
        r.add_formula("SUM(Budget!B2:B5)");
        w.add_row(r);

        let mut r2 = RowData::new();
        r2.add_string("Total Actual");
        r2.add_formula("SUM(Budget!C2:C5)");
        w.add_row(r2);

        let mut r3 = RowData::new();
        r3.add_string("Over Budget?");
        r3.add_bool(true);
        w.add_row(r3);

        let path = out_dir.join("budget_rich.xlsx");
        w.save(std::fs::File::create(&path).unwrap()).unwrap();
        println!("Created {}", path.display());
    }

    // ── lookup_rich.xlsx: VLOOKUP, freeze panes, auto-filter ──
    {
        let options = WriteOptions {
            freeze_header: true,
            auto_filter: true,
            ..Default::default()
        };
        let mut w = XlsxWriter::with_options(options);
        w.add_sheet("Lookup").unwrap();

        let mut hdr = RowData::new();
        hdr.add_string("Code");
        hdr.add_string("Name");
        hdr.add_string("Price");
        w.add_row(hdr);

        let items: &[(&str, &str, f64)] = &[
            ("W", "Widget", 10.0),
            ("G", "Gadget", 25.0),
            ("S", "Sprocket", 15.0),
            ("D", "Doohickey", 50.0),
        ];
        for (code, name, price) in items {
            let mut row = RowData::new();
            row.add_string(*code);
            row.add_string(*name);
            row.add_number(*price);
            w.add_row(row);
        }

        let queries = ["W", "G", "X"];
        for (i, q) in queries.iter().enumerate() {
            let mut row = RowData::new();
            row.add_string(*q);
            let row_num = i + 6;
            row.add_formula(format!("VLOOKUP(A{},A2:C5,2,FALSE)", row_num));
            row.add_formula(format!("VLOOKUP(A{},A2:C5,3,FALSE)", row_num));
            w.add_row(row);
        }

        let path = out_dir.join("lookup_rich.xlsx");
        w.save(std::fs::File::create(&path).unwrap()).unwrap();
        println!("Created {}", path.display());
    }

    println!("\nAll sample XLSX files written to {}", out_dir.display());
}

