//! Integration tests for rich text writing (multi-run formatted strings).

use std::io::Cursor;
use xls_rs::{RichTextRun, RichTextRunStyle, XlsxReader, XlsxWriter};
use zip::ZipArchive;

fn read_zip_part(zip_bytes: &[u8], name: &str) -> Option<String> {
    let cursor = Cursor::new(zip_bytes);
    let mut za = ZipArchive::new(cursor).unwrap();
    let mut s = String::new();
    use std::io::Read;
    za.by_name(name).ok()?.read_to_string(&mut s).unwrap();
    Some(s)
}

#[test]
fn test_rich_text_writes_runs() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let mut row = xls_rs::RowData::new();
    row.add_rich_text(vec![
        RichTextRun::plain("Hello "),
        RichTextRun::bold("World"),
        RichTextRun::plain("!"),
    ]);
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let sheet = read_zip_part(buf.get_ref(), "xl/worksheets/sheet1.xml").unwrap();

    assert!(sheet.contains("<is>"), "missing inline string");
    assert!(sheet.contains("<r>"), "missing rich text run");
    assert!(sheet.contains("<rPr><b/></rPr>"), "missing bold rPr");
    // "Hello " has trailing space → xml:space="preserve"
    assert!(
        sheet.contains(r#"<t xml:space="preserve">Hello </t>"#),
        "missing first run text"
    );
    assert!(sheet.contains("<t>World</t>"), "missing bold run text");
    assert!(sheet.contains("<t>!</t>"), "missing last run text");
}

#[test]
fn test_rich_text_with_italic_and_color() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let mut row = xls_rs::RowData::new();
    row.add_rich_text(vec![
        RichTextRun {
            text: "Red ".into(),
            style: RichTextRunStyle {
                color: Some("FF0000".into()),
                ..Default::default()
            },
        },
        RichTextRun {
            text: "Italic".into(),
            style: RichTextRunStyle {
                italic: true,
                ..Default::default()
            },
        },
    ]);
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let sheet = read_zip_part(buf.get_ref(), "xl/worksheets/sheet1.xml").unwrap();

    assert!(
        sheet.contains(r#"<color rgb="FFFF0000"/>"#),
        "missing color"
    );
    assert!(sheet.contains("<i/>"), "missing italic");
}

#[test]
fn test_rich_text_with_font_size_and_name() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let mut row = xls_rs::RowData::new();
    row.add_rich_text(vec![
        RichTextRun {
            text: "Big".into(),
            style: RichTextRunStyle {
                font_size: Some(24.0),
                font_name: Some("Arial".into()),
                ..Default::default()
            },
        },
        RichTextRun::plain(" small"),
    ]);
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let sheet = read_zip_part(buf.get_ref(), "xl/worksheets/sheet1.xml").unwrap();

    assert!(sheet.contains(r#"<sz val="24"/>"#), "missing font size");
    assert!(
        sheet.contains(r#"<rFont val="Arial"/>"#),
        "missing font name"
    );
}

#[test]
fn test_rich_text_plain_run_has_no_rpr() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let mut row = xls_rs::RowData::new();
    row.add_rich_text(vec![RichTextRun::plain("no formatting")]);
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let sheet = read_zip_part(buf.get_ref(), "xl/worksheets/sheet1.xml").unwrap();

    // Plain run should have <r><t> without <rPr>
    assert!(
        sheet.contains("<r><t>no formatting</t></r>"),
        "plain run should not have rPr"
    );
}

#[test]
fn test_rich_text_preserves_leading_space() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let mut row = xls_rs::RowData::new();
    row.add_rich_text(vec![
        RichTextRun::bold("Bold"),
        RichTextRun::plain(" with space"),
    ]);
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let sheet = read_zip_part(buf.get_ref(), "xl/worksheets/sheet1.xml").unwrap();

    assert!(
        sheet.contains(r#"<t xml:space="preserve"> with space</t>"#),
        "leading space should be preserved with xml:space"
    );
}

#[test]
fn test_rich_text_round_trip_concatenated() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rich.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let mut row = xls_rs::RowData::new();
    row.add_rich_text(vec![
        RichTextRun::plain("Hello "),
        RichTextRun::bold("World"),
        RichTextRun::plain("!"),
    ]);
    writer.add_row(row);

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let sheet = reader.get_sheet(0).unwrap();
    // The reader concatenates rich text runs (existing behavior)
    let cell = sheet.get_cell(0, 0);
    assert_eq!(cell.to_string(), "Hello World!");
}

#[test]
fn test_rich_text_with_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rich_data.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let mut header = xls_rs::RowData::new();
    header.add_rich_text(vec![RichTextRun {
        text: "Header".into(),
        style: RichTextRunStyle {
            bold: true,
            color: Some("FFFFFF".into()),
            ..Default::default()
        },
    }]);
    writer.add_row(header);

    let mut data = xls_rs::RowData::new();
    data.add_string("value");
    data.add_number(42.0);
    writer.add_row(data);

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let sheet = reader.get_sheet(0).unwrap();
    assert_eq!(sheet.row_count(), 2);
    assert_eq!(sheet.col_count(), 2);
    assert_eq!(sheet.get_cell(0, 0).to_string(), "Header");
    assert_eq!(sheet.get_cell(1, 0).to_string(), "value");
    assert_eq!(sheet.get_cell(1, 1).to_string(), "42");
}

#[test]
fn test_rich_text_underline() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let mut row = xls_rs::RowData::new();
    row.add_rich_text(vec![RichTextRun {
        text: "underlined".into(),
        style: RichTextRunStyle {
            underline: true,
            ..Default::default()
        },
    }]);
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let sheet = read_zip_part(buf.get_ref(), "xl/worksheets/sheet1.xml").unwrap();
    assert!(sheet.contains("<u/>"), "missing underline");
}

#[test]
fn test_rich_text_all_formatting_combined() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let mut row = xls_rs::RowData::new();
    row.add_rich_text(vec![RichTextRun {
        text: "Fancy".into(),
        style: RichTextRunStyle {
            bold: true,
            italic: true,
            underline: true,
            font_size: Some(18.0),
            font_name: Some("Comic Sans MS".into()),
            color: Some("00FF00".into()),
        },
    }]);
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let sheet = read_zip_part(buf.get_ref(), "xl/worksheets/sheet1.xml").unwrap();
    assert!(sheet.contains("<b/>"), "missing bold");
    assert!(sheet.contains("<i/>"), "missing italic");
    assert!(sheet.contains("<u/>"), "missing underline");
    assert!(sheet.contains(r#"<sz val="18"/>"#), "missing font size");
    assert!(
        sheet.contains(r#"<rFont val="Comic Sans MS"/>"#),
        "missing font name"
    );
    assert!(
        sheet.contains(r#"<color rgb="FF00FF00"/>"#),
        "missing color"
    );
}

#[test]
fn test_rich_text_multiple_formatted_runs_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rich_multi.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    let mut row = xls_rs::RowData::new();
    row.add_rich_text(vec![
        RichTextRun {
            text: "Bold ".into(),
            style: RichTextRunStyle {
                bold: true,
                ..Default::default()
            },
        },
        RichTextRun {
            text: "italic ".into(),
            style: RichTextRunStyle {
                italic: true,
                ..Default::default()
            },
        },
        RichTextRun {
            text: "underlined".into(),
            style: RichTextRunStyle {
                underline: true,
                ..Default::default()
            },
        },
    ]);
    writer.add_row(row);

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let sheet = reader.get_sheet(0).unwrap();
    // Reader concatenates all runs into one cell value
    let cell = sheet.get_cell(0, 0);
    let text = cell.to_string();
    assert_eq!(text, "Bold italic underlined");
}

#[test]
fn test_rich_text_preserves_inter_run_spaces() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rich_spaces.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    let mut row = xls_rs::RowData::new();
    row.add_rich_text(vec![
        RichTextRun::plain("Hello"),
        RichTextRun::plain(" "),
        RichTextRun::plain("World"),
    ]);
    writer.add_row(row);

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let sheet = reader.get_sheet(0).unwrap();
    let text = sheet.get_cell(0, 0).to_string();
    // The middle run is a single space — must be preserved verbatim
    assert_eq!(text, "Hello World");
}

#[test]
fn test_rich_text_with_special_characters() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rich_special.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    let mut row = xls_rs::RowData::new();
    row.add_rich_text(vec![
        RichTextRun::plain("A & B"),
        RichTextRun::bold(" < C > "),
        RichTextRun::plain("D"),
    ]);
    writer.add_row(row);

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let sheet = reader.get_sheet(0).unwrap();
    let text = sheet.get_cell(0, 0).to_string();
    assert_eq!(text, "A & B < C > D");
}

#[test]
fn test_rich_text_empty_cell_no_runs() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    let mut row = xls_rs::RowData::new();
    row.add_rich_text(vec![]);
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    // Should not panic; the writer should handle empty rich text gracefully
    let sheet = read_zip_part(buf.get_ref(), "xl/worksheets/sheet1.xml").unwrap();
    // Empty rich text should still produce a valid cell
    assert!(
        sheet.contains("<c r=\"A1\""),
        "missing cell for empty rich text"
    );
}

#[test]
fn test_rich_text_mixed_with_other_cells_in_row() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rich_mixed.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    let mut row = xls_rs::RowData::new();
    row.add_rich_text(vec![RichTextRun::bold("Header")]);
    row.add_number(42.0);
    row.add_string("plain");
    writer.add_row(row);

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let sheet = reader.get_sheet(0).unwrap();
    assert_eq!(sheet.get_cell(0, 0).to_string(), "Header");
    assert_eq!(sheet.get_cell(0, 1).to_string(), "42");
    assert_eq!(sheet.get_cell(0, 2).to_string(), "plain");
}
