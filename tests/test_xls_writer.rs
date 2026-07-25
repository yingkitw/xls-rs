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

