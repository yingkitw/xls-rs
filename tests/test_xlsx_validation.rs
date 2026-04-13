//! XLSX file validation tests
//!
//! These tests verify that generated XLSX files are properly formatted
//! and can be opened in Excel, Numbers, and other spreadsheet applications.

use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use zip::ZipArchive;
use xls_rs::excel::xlsx_writer::{RowData, XlsxWriter};
use xls_rs::excel::{CellStyle, WriteOptions};

/// Validate that a ZIP file has the correct XLSX structure
fn validate_xlsx_structure<P: AsRef<Path>>(path: P) -> Result<(), String> {
    let file = File::open(path.as_ref())
        .map_err(|e| format!("Failed to open file: {}", e))?;
    let mut zip = ZipArchive::new(file)
        .map_err(|e| format!("Not a valid ZIP file: {}", e))?;

    // Check for required files
    let required_files = vec![
        "[Content_Types].xml",
        "_rels/.rels",
        "xl/workbook.xml",
        "xl/_rels/workbook.xml.rels",
        "xl/styles.xml",
        "xl/theme/theme1.xml",
    ];

    for required in &required_files {
        zip.by_name(required)
            .map_err(|e| format!("Missing required file '{}': {}", required, e))?;
    }

    // Check for at least one worksheet
    let worksheet_name = "xl/worksheets/sheet1.xml";
    zip.by_name(worksheet_name)
        .map_err(|e| format!("Missing worksheet '{}': {}", worksheet_name, e))?;

    Ok(())
}

/// Validate XML content in XLSX file
fn validate_xml_content<P: AsRef<Path>>(path: P) -> Result<(), String> {
    let file = File::open(path.as_ref())
        .map_err(|e| format!("Failed to open file: {}", e))?;
    let mut zip = ZipArchive::new(file)
        .map_err(|e| format!("Not a valid ZIP file: {}", e))?;

    // Validate workbook.xml
    {
        let mut workbook_file = zip.by_name("xl/workbook.xml")
            .map_err(|e| format!("Failed to open workbook.xml: {}", e))?;
        let mut workbook_content = String::new();
        workbook_file.read_to_string(&mut workbook_content)
            .map_err(|e| format!("Failed to read workbook.xml: {}", e))?;

        if !workbook_content.contains("<sheets>") {
            return Err("workbook.xml is missing <sheets> element".to_string());
        }
        if !workbook_content.contains("</sheets>") {
            return Err("workbook.xml is missing closing </sheets> element".to_string());
        }
    }

    // Validate worksheet
    {
        let mut worksheet_file = zip.by_name("xl/worksheets/sheet1.xml")
            .map_err(|e| format!("Failed to open worksheet: {}", e))?;
        let mut worksheet_content = String::new();
        worksheet_file.read_to_string(&mut worksheet_content)
            .map_err(|e| format!("Failed to read worksheet: {}", e))?;

        if !worksheet_content.contains("<worksheet") {
            return Err("worksheet.xml is missing <worksheet> element".to_string());
        }
        if !worksheet_content.contains("<sheetData>") {
            return Err("worksheet.xml is missing <sheetData> element".to_string());
        }
    }

    Ok(())
}

#[test]
fn test_xlsx_structure_valid() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Test").unwrap();

    let mut row = RowData::new();
    row.add_string("Header");
    row.add_number(100.0);
    writer.add_row(row);

    let mut buffer = Cursor::new(Vec::new());
    writer.save(&mut buffer).unwrap();

    // Write to temp file for validation
    let temp_path = "/tmp/test_structure.xlsx";
    let mut file = File::create(temp_path).unwrap();
    file.write_all(&buffer.into_inner()).unwrap();

    let result = validate_xlsx_structure(temp_path);
    assert!(result.is_ok(), "XLSX structure validation failed: {:?}", result.err());

    std::fs::remove_file(temp_path).ok();
}

#[test]
fn test_xlsx_xml_content_valid() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Test").unwrap();

    let mut row = RowData::new();
    row.add_string("Name");
    row.add_string("Value");
    writer.add_row(row);

    let mut row = RowData::new();
    row.add_string("Alice");
    row.add_number(42.0);
    writer.add_row(row);

    let mut buffer = Cursor::new(Vec::new());
    writer.save(&mut buffer).unwrap();

    let temp_path = "/tmp/test_xml_content.xlsx";
    let mut file = File::create(temp_path).unwrap();
    file.write_all(&buffer.into_inner()).unwrap();

    let result = validate_xml_content(temp_path);
    assert!(result.is_ok(), "XLSX XML validation failed: {:?}", result.err());

    std::fs::remove_file(temp_path).ok();
}

#[test]
fn test_xlsx_with_freeze_and_autofilter() {
    let options = WriteOptions {
        sheet_name: None,
        style_header: false,
        header_style: CellStyle::default(),
        column_styles: None,
        freeze_header: true,
        auto_filter: true,
        auto_fit: false,
    };

    let mut writer = XlsxWriter::with_options(options);
    writer.add_sheet("Data").unwrap();

    // Add header row
    let mut header = RowData::new();
    header.add_string("ID");
    header.add_string("Name");
    header.add_string("Value");
    writer.add_row(header);

    // Add data rows
    for i in 1..=5 {
        let mut row = RowData::new();
        row.add_number(i as f64);
        row.add_string(&format!("Item {}", i));
        row.add_number((i * 10) as f64);
        writer.add_row(row);
    }

    let mut buffer = Cursor::new(Vec::new());
    writer.save(&mut buffer).unwrap();

    let temp_path = "/tmp/test_freeze_autofilter.xlsx";
    let mut file = File::create(temp_path).unwrap();
    file.write_all(&buffer.into_inner()).unwrap();

    // Validate structure
    let result = validate_xlsx_structure(temp_path);
    assert!(result.is_ok(), "XLSX structure validation failed: {:?}", result.err());

    // Validate XML contains freeze panes and autofilter
    {
        let file = File::open(temp_path).unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        let mut worksheet_file = zip.by_name("xl/worksheets/sheet1.xml").unwrap();
        let mut content = String::new();
        worksheet_file.read_to_string(&mut content).unwrap();

        assert!(content.contains("ySplit"), "Missing freeze panes (ySplit)");
        assert!(content.contains("autoFilter"), "Missing autoFilter");
    }

    std::fs::remove_file(temp_path).ok();
}

#[test]
fn test_xlsx_with_formulas() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Formulas").unwrap();

    let mut header = RowData::new();
    header.add_string("A");
    header.add_string("B");
    header.add_string("C");
    header.add_string("Sum");
    writer.add_row(header);

    let mut row = RowData::new();
    row.add_number(10.0);
    row.add_number(20.0);
    row.add_number(30.0);
    row.add_formula("=A2+B2+C2");
    writer.add_row(row);

    let mut buffer = Cursor::new(Vec::new());
    writer.save(&mut buffer).unwrap();

    let temp_path = "/tmp/test_formulas.xlsx";
    let mut file = File::create(temp_path).unwrap();
    file.write_all(&buffer.into_inner()).unwrap();

    let result = validate_xlsx_structure(temp_path);
    assert!(result.is_ok(), "XLSX structure validation failed: {:?}", result.err());

    // Validate formula cell
    {
        let file = File::open(temp_path).unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        let mut worksheet_file = zip.by_name("xl/worksheets/sheet1.xml").unwrap();
        let mut content = String::new();
        worksheet_file.read_to_string(&mut content).unwrap();

        assert!(content.contains("<f>"), "Missing formula cell (<f>)");
        assert!(content.contains("A2+B2+C2"), "Missing formula content");
    }

    std::fs::remove_file(temp_path).ok();
}

#[test]
fn test_xlsx_multiple_sheets() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.add_sheet("Sheet2").unwrap();
    writer.add_sheet("Sheet3").unwrap();

    // Add data to each sheet
    for _ in 0..3 {
        let mut row = RowData::new();
        row.add_string("Test");
        row.add_number(1.0);
        writer.add_row(row);
    }

    let mut buffer = Cursor::new(Vec::new());
    writer.save(&mut buffer).unwrap();

    let temp_path = "/tmp/test_multiple_sheets.xlsx";
    let mut file = File::create(temp_path).unwrap();
    file.write_all(&buffer.into_inner()).unwrap();

    // Validate all three sheets exist
    {
        let file = File::open(temp_path).unwrap();
        let mut zip = ZipArchive::new(file).unwrap();

        assert!(zip.by_name("xl/worksheets/sheet1.xml").is_ok());
        assert!(zip.by_name("xl/worksheets/sheet2.xml").is_ok());
        assert!(zip.by_name("xl/worksheets/sheet3.xml").is_ok());

        // Validate workbook references all sheets
        let mut workbook_file = zip.by_name("xl/workbook.xml").unwrap();
        let mut content = String::new();
        workbook_file.read_to_string(&mut content).unwrap();

        assert!(content.contains("Sheet1"));
        assert!(content.contains("Sheet2"));
        assert!(content.contains("Sheet3"));
    }

    std::fs::remove_file(temp_path).ok();
}

#[test]
fn test_xlsx_column_widths() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Widths").unwrap();

    writer.set_column_width(0, 15.5);
    writer.set_column_width(1, 20.0);
    writer.set_column_width(2, 12.0);

    let mut row = RowData::new();
    row.add_string("A");
    row.add_string("B");
    row.add_string("C");
    writer.add_row(row);

    let mut buffer = Cursor::new(Vec::new());
    writer.save(&mut buffer).unwrap();

    let temp_path = "/tmp/test_column_widths.xlsx";
    let mut file = File::create(temp_path).unwrap();
    file.write_all(&buffer.into_inner()).unwrap();

    // Validate column widths in XML
    {
        let file = File::open(temp_path).unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        let mut worksheet_file = zip.by_name("xl/worksheets/sheet1.xml").unwrap();
        let mut content = String::new();
        worksheet_file.read_to_string(&mut content).unwrap();

        assert!(content.contains("<cols>"), "Missing column definitions");
        assert!(content.contains("width=\"15.5\""), "Missing column A width");
        assert!(content.contains("width=\"20\""), "Missing column B width");
        assert!(content.contains("width=\"12\""), "Missing column C width");
    }

    std::fs::remove_file(temp_path).ok();
}

#[test]
fn test_xlsx_special_characters() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Special").unwrap();

    let mut row = RowData::new();
    row.add_string("Test & < > \" ' data");
    row.add_string("Special: äöü ñ");
    row.add_string("Math: ∑ ∆ ∞");
    writer.add_row(row);

    let mut buffer = Cursor::new(Vec::new());
    writer.save(&mut buffer).unwrap();

    let temp_path = "/tmp/test_special_chars.xlsx";
    let mut file = File::create(temp_path).unwrap();
    file.write_all(&buffer.into_inner()).unwrap();

    let result = validate_xlsx_structure(temp_path);
    assert!(result.is_ok(), "XLSX structure validation failed: {:?}", result.err());

    // Validate XML escaping
    {
        let file = File::open(temp_path).unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        let mut worksheet_file = zip.by_name("xl/worksheets/sheet1.xml").unwrap();
        let mut content = String::new();
        worksheet_file.read_to_string(&mut content).unwrap();

        // Check that special characters are properly escaped
        assert!(content.contains("&amp;"), "Ampersand not escaped");
        assert!(content.contains("&lt;"), "Less-than not escaped");
        assert!(content.contains("&gt;"), "Greater-than not escaped");
        assert!(content.contains("&quot;"), "Quote not escaped");
        assert!(content.contains("&apos;"), "Apostrophe not escaped");
    }

    std::fs::remove_file(temp_path).ok();
}

#[test]
fn test_xlsx_empty_cells() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Empty").unwrap();

    let mut row = RowData::new();
    row.add_string("A");
    row.add_empty();
    row.add_string("C");
    row.add_empty();
    row.add_string("E");
    writer.add_row(row);

    let mut buffer = Cursor::new(Vec::new());
    writer.save(&mut buffer).unwrap();

    let temp_path = "/tmp/test_empty_cells.xlsx";
    let mut file = File::create(temp_path).unwrap();
    file.write_all(&buffer.into_inner()).unwrap();

    let result = validate_xlsx_structure(temp_path);
    assert!(result.is_ok(), "XLSX structure validation failed: {:?}", result.err());

    // Empty cells should not create <c> elements (they're implicit)
    {
        let file = File::open(temp_path).unwrap();
        let mut zip = ZipArchive::new(file).unwrap();
        let mut worksheet_file = zip.by_name("xl/worksheets/sheet1.xml").unwrap();
        let mut content = String::new();
        worksheet_file.read_to_string(&mut content).unwrap();

        // Count cell elements - should only have 3 (A, C, E) not 5
        let cell_count = content.matches("<c ").count();
        assert_eq!(cell_count, 3, "Expected 3 cells, got {}", cell_count);
    }

    std::fs::remove_file(temp_path).ok();
}
