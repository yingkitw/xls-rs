//! Integration test: write styled XLSX, read it back, verify styles round-trip.

use std::io::Cursor;
use xls_rs::excel::xlsx_reader::XlsxReader;
use xls_rs::XlsxCellStyle;
use xls_rs::XlsxWriter;
use xls_rs::RowData;

#[test]
fn test_style_round_trip_bold_fill() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let header_idx = writer.register_cell_style(&XlsxCellStyle {
        bold: Some(true),
        fill_color: Some("305496".into()),
        font_color: Some("FFFFFF".into()),
        align: Some("center".into()),
        ..Default::default()
    });

    let mut row = RowData::new();
    row.add_string("Name");
    row.add_string("Amount");
    row.set_cell_style(0, header_idx);
    row.set_cell_style(1, header_idx);
    writer.add_row(row);

    let mut data_row = RowData::new();
    data_row.add_string("Widget");
    data_row.add_number(99.99);
    writer.add_row(data_row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let reader = XlsxReader::from_reader(Cursor::new(buf.into_inner())).unwrap();

    // Cell A1 should have the header style
    let style = reader.cell_style(0, 0, 0).expect("A1 should have a style");
    assert_eq!(style.bold, Some(true));
    assert_eq!(style.fill_color.as_deref(), Some("305496"));
    assert_eq!(style.font_color.as_deref(), Some("FFFFFF"));
    assert_eq!(style.align.as_deref(), Some("center"));

    // Cell B1 should also have the header style
    let style_b = reader.cell_style(0, 0, 1).expect("B1 should have a style");
    assert_eq!(style_b.bold, Some(true));

    // Cell A2 should have no style (returns None)
    assert!(reader.cell_style(0, 1, 0).is_none());
}

#[test]
fn test_style_round_trip_number_format() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Data").unwrap();

    let money_idx = writer.register_cell_style(&XlsxCellStyle {
        number_format: Some("$#,##0.00".into()),
        ..Default::default()
    });

    let mut row = RowData::new();
    row.add_string("Price");
    row.add_number(1234.56);
    row.set_cell_style(1, money_idx);
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let reader = XlsxReader::from_reader(Cursor::new(buf.into_inner())).unwrap();

    let style = reader.cell_style(0, 0, 1).expect("B1 should have a style");
    assert_eq!(style.number_format.as_deref(), Some("$#,##0.00"));
}

#[test]
fn test_style_round_trip_date_format() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Dates").unwrap();

    let date_idx = writer.register_cell_style(&XlsxCellStyle {
        date: Some(true),
        ..Default::default()
    });

    let mut row = RowData::new();
    row.add_string("Date");
    row.add_number(45000.0); // Excel serial date
    row.set_cell_style(1, date_idx);
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let reader = XlsxReader::from_reader(Cursor::new(buf.into_inner())).unwrap();

    let style = reader.cell_style(0, 0, 1).expect("B1 should have a style");
    assert_eq!(style.date, Some(true));
    assert!(style.number_format.is_some());
}

#[test]
fn test_style_round_trip_border() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Bordered").unwrap();

    let border_idx = writer.register_cell_style(&XlsxCellStyle {
        border: Some("thin".into()),
        border_color: Some("000000".into()),
        ..Default::default()
    });

    let mut row = RowData::new();
    row.add_string("Cell");
    row.set_cell_style(0, border_idx);
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let reader = XlsxReader::from_reader(Cursor::new(buf.into_inner())).unwrap();

    let style = reader.cell_style(0, 0, 0).expect("A1 should have a style");
    assert_eq!(style.border.as_deref(), Some("thin"));
    assert_eq!(style.border_color.as_deref(), Some("000000"));
}

#[test]
fn test_style_round_trip_italic_wrap() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Wrapped").unwrap();

    let style_idx = writer.register_cell_style(&XlsxCellStyle {
        italic: Some(true),
        wrap: Some(true),
        valign: Some("top".into()),
        ..Default::default()
    });

    let mut row = RowData::new();
    row.add_string("Long text that wraps");
    row.set_cell_style(0, style_idx);
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let reader = XlsxReader::from_reader(Cursor::new(buf.into_inner())).unwrap();

    let style = reader.cell_style(0, 0, 0).expect("A1 should have a style");
    assert_eq!(style.italic, Some(true));
    assert_eq!(style.wrap, Some(true));
    assert_eq!(style.valign.as_deref(), Some("top"));
}

#[test]
fn test_style_table_access() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("S").unwrap();

    let _ = writer.register_cell_style(&XlsxCellStyle {
        bold: Some(true),
        ..Default::default()
    });

    let mut row = RowData::new();
    row.add_string("Test");
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let reader = XlsxReader::from_reader(Cursor::new(buf.into_inner())).unwrap();
    let styles = reader.styles();
    // Should have at least 2 cellXfs: default + the bold one
    assert!(styles.cell_xf_count() >= 2);
}

#[test]
fn test_no_styles_returns_none() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Plain").unwrap();

    let mut row = RowData::new();
    row.add_string("No style");
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let reader = XlsxReader::from_reader(Cursor::new(buf.into_inner())).unwrap();
    // Cell with no s attribute should return None
    assert!(reader.cell_style(0, 0, 0).is_none());
}
