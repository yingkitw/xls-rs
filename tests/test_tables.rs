use std::io::Cursor;

use xls_rs::excel::xlsx_reader::{XlsxCellValue, XlsxReader};
use xls_rs::{RowData, Table, TableStyleInfo, XlsxWriter};

/// Write an XLSX with a structured table, read it back, and verify
/// the table definition round-trips correctly.
#[test]
fn test_table_write_read_roundtrip() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    // Header row
    let mut header = RowData::new();
    header.add_string("Name");
    header.add_string("Score");
    header.add_string("Grade");
    writer.add_row(header);

    // Data rows
    let mut r1 = RowData::new();
    r1.add_string("Alice");
    r1.add_number(95.0);
    r1.add_string("A");
    writer.add_row(r1);

    let mut r2 = RowData::new();
    r2.add_string("Bob");
    r2.add_number(78.0);
    r2.add_string("B");
    writer.add_row(r2);

    let mut r3 = RowData::new();
    r3.add_string("Carol");
    r3.add_number(88.0);
    r3.add_string("B+");
    writer.add_row(r3);

    // Add a structured table covering A1:C4 (0-based: rows 0-3, cols 0-2)
    writer.add_table(Table {
        name: "StudentTable".to_string(),
        start_row: 0,
        start_col: 0,
        end_row: 3,
        end_col: 2,
        column_names: vec!["Name".to_string(), "Score".to_string(), "Grade".to_string()],
        show_banded_rows: true,
        show_banded_columns: false,
        show_filter_button: true,
        show_totals_row: false,
        style: Some(TableStyleInfo::default()),
    });

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let reader = XlsxReader::from_reader(Cursor::new(buf.into_inner())).unwrap();

    // Verify sheet data
    assert_eq!(reader.sheet_count(), 1);
    let sheet = reader.get_sheet(0).unwrap();
    assert_eq!(sheet.cells.len(), 4);
    assert!(matches!(&sheet.cells[0][0], XlsxCellValue::String(s) if s == "Name"));
    assert!(matches!(&sheet.cells[3][2], XlsxCellValue::String(s) if s == "B+"));

    // Verify table was read back
    let tables = reader.tables(0).expect("Sheet should have tables");
    assert_eq!(tables.len(), 1);
    let table = &tables[0];
    assert_eq!(table.name, "StudentTable");
    assert_eq!(table.range, "A1:C4");
    assert_eq!(table.start_row, 0);
    assert_eq!(table.start_col, 0);
    assert_eq!(table.end_row, 3);
    assert_eq!(table.end_col, 2);
    assert_eq!(table.column_names, vec!["Name", "Score", "Grade"]);
    assert_eq!(table.style_name.as_deref(), Some("TableStyleMedium2"));
}

/// A sheet without tables should return an empty slice.
#[test]
fn test_no_tables() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Empty").unwrap();
    let mut row = RowData::new();
    row.add_string("data");
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let reader = XlsxReader::from_reader(Cursor::new(buf.into_inner())).unwrap();
    let tables = reader.tables(0).expect("Sheet 0 should exist");
    assert!(tables.is_empty());
}

/// Table with custom style and banded columns.
#[test]
fn test_table_custom_style() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("S").unwrap();

    let mut h = RowData::new();
    h.add_string("X");
    h.add_string("Y");
    writer.add_row(h);

    let mut r = RowData::new();
    r.add_number(1.0);
    r.add_number(2.0);
    writer.add_row(r);

    writer.add_table(Table {
        name: "DataTable".to_string(),
        start_row: 0,
        start_col: 0,
        end_row: 1,
        end_col: 1,
        column_names: vec!["X".to_string(), "Y".to_string()],
        show_banded_rows: false,
        show_banded_columns: true,
        show_filter_button: false,
        show_totals_row: false,
        style: Some(TableStyleInfo {
            name: "TableStyleLight9".to_string(),
            show_first_column: true,
            show_last_column: false,
        }),
    });

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let reader = XlsxReader::from_reader(Cursor::new(buf.into_inner())).unwrap();
    let tables = reader.tables(0).unwrap();
    assert_eq!(tables.len(), 1);
    let t = &tables[0];
    assert_eq!(t.name, "DataTable");
    assert_eq!(t.range, "A1:B2");
    assert_eq!(t.style_name.as_deref(), Some("TableStyleLight9"));
}

/// Table with auto-generated column names (no explicit names provided).
#[test]
fn test_table_auto_column_names() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("S").unwrap();

    let mut h = RowData::new();
    h.add_string("A");
    h.add_string("B");
    writer.add_row(h);

    let mut r = RowData::new();
    r.add_number(10.0);
    r.add_number(20.0);
    writer.add_row(r);

    writer.add_table(Table {
        name: "AutoTable".to_string(),
        start_row: 0,
        start_col: 0,
        end_row: 1,
        end_col: 1,
        column_names: vec![], // empty → auto-generated
        show_banded_rows: true,
        show_banded_columns: false,
        show_filter_button: true,
        show_totals_row: false,
        style: None,
    });

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let reader = XlsxReader::from_reader(Cursor::new(buf.into_inner())).unwrap();
    let tables = reader.tables(0).unwrap();
    assert_eq!(tables.len(), 1);
    let t = &tables[0];
    assert_eq!(t.name, "AutoTable");
    // Auto-generated names should be "Column1", "Column2"
    assert_eq!(t.column_names, vec!["Column1", "Column2"]);
}

/// Multiple tables on the same sheet.
#[test]
fn test_multiple_tables_one_sheet() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("S").unwrap();

    // First table data (A1:B2)
    let mut h1 = RowData::new();
    h1.add_string("P");
    h1.add_string("Q");
    writer.add_row(h1);

    let mut r1 = RowData::new();
    r1.add_number(1.0);
    r1.add_number(2.0);
    writer.add_row(r1);

    // Gap row
    writer.add_row(RowData::new());

    // Second table data (A4:B5)
    let mut h2 = RowData::new();
    h2.add_string("R");
    h2.add_string("S");
    writer.add_row(h2);

    let mut r2 = RowData::new();
    r2.add_number(3.0);
    r2.add_number(4.0);
    writer.add_row(r2);

    writer.add_table(Table {
        name: "Table1".to_string(),
        start_row: 0,
        start_col: 0,
        end_row: 1,
        end_col: 1,
        column_names: vec!["P".to_string(), "Q".to_string()],
        ..Table::default()
    });

    writer.add_table(Table {
        name: "Table2".to_string(),
        start_row: 3,
        start_col: 0,
        end_row: 4,
        end_col: 1,
        column_names: vec!["R".to_string(), "S".to_string()],
        ..Table::default()
    });

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let reader = XlsxReader::from_reader(Cursor::new(buf.into_inner())).unwrap();
    let tables = reader.tables(0).unwrap();
    assert_eq!(tables.len(), 2);
    assert_eq!(tables[0].name, "Table1");
    assert_eq!(tables[0].range, "A1:B2");
    assert_eq!(tables[1].name, "Table2");
    assert_eq!(tables[1].range, "A4:B5");
}
