//! Integration tests for worksheet and workbook protection (write + read).

use std::io::Cursor;
use xls_rs::{SheetProtection, WorkbookProtection, XlsxReader, XlsxWriter};
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
fn test_sheet_protection_writes_xml() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_sheet_protection(SheetProtection::locked());

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let sheet = read_zip_part(buf.get_ref(), "xl/worksheets/sheet1.xml").unwrap();

    assert!(
        sheet.contains("<sheetProtection"),
        "missing sheetProtection"
    );
    assert!(sheet.contains(r#"sheet="1""#), "missing sheet flag");
    assert!(sheet.contains(r#"formatCells="1""#), "missing formatCells");
    assert!(
        sheet.contains(r#"selectLockedCells="1""#),
        "missing selectLockedCells"
    );
}

#[test]
fn test_sheet_protection_data_entry_mode() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_sheet_protection(SheetProtection::data_entry());

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let sheet = read_zip_part(buf.get_ref(), "xl/worksheets/sheet1.xml").unwrap();

    assert!(
        sheet.contains("<sheetProtection"),
        "missing sheetProtection"
    );
    assert!(sheet.contains(r#"sheet="1""#), "missing sheet flag");
    // data_entry mode allows selecting cells, sorting, filtering
    assert!(
        !sheet.contains(r#"selectLockedCells="1""#),
        "should not lock cell selection"
    );
    assert!(!sheet.contains(r#"sort="1""#), "should not lock sorting");
}

#[test]
fn test_sheet_protection_with_password() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_sheet_protection(SheetProtection::locked().with_password("secret"));

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let sheet = read_zip_part(buf.get_ref(), "xl/worksheets/sheet1.xml").unwrap();

    assert!(sheet.contains(r#"password="#), "missing password attribute");
    // The hash should be a 4-hex-digit string
    assert!(sheet.contains("password=\""), "password value missing");
}

#[test]
fn test_sheet_protection_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("protected.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_sheet_protection(SheetProtection::locked());

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let prot = reader.sheet_protection(0).unwrap();
    assert!(prot.sheet, "sheet flag should be true");
    assert!(prot.format_cells, "format_cells should be true");
    assert!(
        prot.select_locked_cells,
        "select_locked_cells should be true"
    );
    assert!(prot.sort, "sort should be true");
}

#[test]
fn test_sheet_protection_round_trip_with_password() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pw.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    let prot = SheetProtection::locked().with_password("test123");
    let expected_hash = prot.password.clone().unwrap();
    writer.set_sheet_protection(prot);

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let prot = reader.sheet_protection(0).unwrap();
    assert_eq!(prot.password, Some(expected_hash));
}

#[test]
fn test_no_sheet_protection_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unprotected.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.add_data(&[vec!["A".into()], vec!["1".into()]]);

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    assert!(
        reader.sheet_protection(0).is_none(),
        "should have no protection"
    );
}

#[test]
fn test_workbook_protection_writes_xml() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_workbook_protection(WorkbookProtection {
        lock_structure: true,
        lock_windows: true,
        ..Default::default()
    });

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let wb = read_zip_part(buf.get_ref(), "xl/workbook.xml").unwrap();

    assert!(
        wb.contains("<workbookProtection"),
        "missing workbookProtection"
    );
    assert!(wb.contains(r#"lockStructure="1""#), "missing lockStructure");
    assert!(wb.contains(r#"lockWindows="1""#), "missing lockWindows");
}

#[test]
fn test_workbook_protection_lock_structure_only() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_workbook_protection(WorkbookProtection {
        lock_structure: true,
        ..Default::default()
    });

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let wb = read_zip_part(buf.get_ref(), "xl/workbook.xml").unwrap();

    assert!(wb.contains(r#"lockStructure="1""#), "missing lockStructure");
    assert!(
        !wb.contains(r#"lockWindows="1""#),
        "should not have lockWindows"
    );
}

#[test]
fn test_protection_with_data_preserves_cells() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("protected_data.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.add_data(&[
        vec!["Name".into(), "Value".into()],
        vec!["A".into(), "1".into()],
    ]);
    writer.set_sheet_protection(SheetProtection::locked());
    writer.set_workbook_protection(WorkbookProtection {
        lock_structure: true,
        ..Default::default()
    });

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let sheet = reader.get_sheet(0).unwrap();
    assert_eq!(sheet.row_count(), 2);
    assert_eq!(sheet.col_count(), 2);
    assert!(reader.sheet_protection(0).is_some());
}

#[test]
fn test_password_hash_deterministic() {
    // Same password should produce the same hash
    let h1 = SheetProtection::password_hash("test");
    let h2 = SheetProtection::password_hash("test");
    assert_eq!(h1, h2);
    // Different passwords should produce different hashes
    let h3 = SheetProtection::password_hash("other");
    assert_ne!(h1, h3);
    // Hash should be 4 hex digits
    assert_eq!(h1.len(), 4);
}

#[test]
fn test_sheet_protection_on_second_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("multi_protect.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("First").unwrap();
    writer.add_sheet("Second").unwrap();
    // Only protect the second sheet
    writer.set_sheet_protection(SheetProtection::locked());

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    assert!(
        reader.sheet_protection(0).is_none(),
        "first sheet should not be protected"
    );
    assert!(
        reader.sheet_protection(1).is_some(),
        "second sheet should be protected"
    );
}

#[test]
fn test_workbook_protection_round_trip_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wb_prot_rt.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_workbook_protection(WorkbookProtection {
        lock_structure: true,
        lock_windows: true,
        revisions_password: Some("abc".into()),
    });

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    // Note: the reader currently only exposes sheet_protection, not
    // workbook_protection. Verify the XML is present in the file.
    let bytes = std::fs::read(&path).unwrap();
    let wb = read_zip_part(&bytes, "xl/workbook.xml").unwrap();
    assert!(
        wb.contains("<workbookProtection"),
        "missing workbookProtection"
    );
    assert!(wb.contains(r#"lockStructure="1""#), "missing lockStructure");
    assert!(wb.contains(r#"lockWindows="1""#), "missing lockWindows");
}

#[test]
fn test_sheet_protection_individual_flags_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sheet_flags.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    // Set individual flags — only some locked
    writer.set_sheet_protection(SheetProtection {
        sheet: true,
        objects: false,
        scenarios: false,
        format_cells: true,
        format_columns: false,
        format_rows: true,
        insert_columns: true,
        insert_rows: false,
        insert_hyperlinks: false,
        delete_columns: false,
        delete_rows: true,
        select_locked_cells: false,
        sort: true,
        auto_filter: false,
        pivot_tables: true,
        select_unlocked_cells: false,
        password: None,
    });

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let prot = reader.sheet_protection(0).unwrap();
    assert!(prot.sheet, "sheet flag");
    assert!(prot.format_cells, "format_cells");
    assert!(!prot.format_columns, "format_columns should be false");
    assert!(prot.format_rows, "format_rows");
    assert!(prot.insert_columns, "insert_columns");
    assert!(!prot.insert_rows, "insert_rows should be false");
    assert!(!prot.delete_columns, "delete_columns should be false");
    assert!(prot.delete_rows, "delete_rows");
    assert!(
        !prot.select_locked_cells,
        "select_locked_cells should be false"
    );
    assert!(prot.sort, "sort");
    assert!(!prot.auto_filter, "auto_filter should be false");
    assert!(prot.pivot_tables, "pivot_tables");
    assert!(
        !prot.select_unlocked_cells,
        "select_unlocked_cells should be false"
    );
}

#[test]
fn test_sheet_protection_with_data_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prot_data.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Data").unwrap();
    writer.add_data(&[
        vec!["Name".into(), "Score".into()],
        vec!["Alice".into(), "95".into()],
        vec!["Bob".into(), "87".into()],
        vec!["Carol".into(), "92".into()],
    ]);
    writer.set_sheet_protection(SheetProtection::locked());

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    // Data should be preserved alongside protection
    let sheet = reader.get_sheet(0).unwrap();
    assert_eq!(sheet.row_count(), 4);
    assert_eq!(sheet.col_count(), 2);
    assert_eq!(sheet.get_cell(0, 0).to_string(), "Name");
    assert_eq!(sheet.get_cell(1, 0).to_string(), "Alice");
    assert_eq!(sheet.get_cell(2, 1).to_string(), "87");
    // Protection should also be present
    let prot = reader.sheet_protection(0).unwrap();
    assert!(prot.sheet, "sheet should be protected");
}

#[test]
fn test_sheet_protection_locked_preset_flags() {
    // The locked() preset should lock most operations
    let prot = SheetProtection::locked();
    assert!(prot.sheet);
    assert!(prot.format_cells);
    assert!(prot.format_columns);
    assert!(prot.format_rows);
    assert!(prot.insert_columns);
    assert!(prot.insert_rows);
    assert!(prot.delete_columns);
    assert!(prot.delete_rows);
    assert!(prot.select_locked_cells);
    assert!(prot.sort);
    assert!(prot.auto_filter);
    assert!(prot.pivot_tables);
    assert!(prot.select_unlocked_cells);
}

#[test]
fn test_sheet_protection_data_entry_preset_flags() {
    // The data_entry() preset should allow selection, sort, filter
    let prot = SheetProtection::data_entry();
    assert!(prot.sheet, "sheet should still be protected");
    // But allow user interactions
    assert!(!prot.select_locked_cells, "should allow cell selection");
    assert!(!prot.sort, "should allow sorting");
    assert!(!prot.auto_filter, "should allow filtering");
}

#[test]
fn test_protection_with_images_preserves_both() {
    use xls_rs::Image;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prot_img.xlsx");

    let png = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.add_data(&[vec!["A".into()], vec!["1".into()]]);
    writer.insert_image(Image::new(png.clone(), 3, 0));
    writer.set_sheet_protection(SheetProtection::locked());
    writer.set_workbook_protection(WorkbookProtection {
        lock_structure: true,
        ..Default::default()
    });

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    // Both image and protection should survive round-trip
    let images = reader.images(0).unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].data, png);
    let prot = reader.sheet_protection(0).unwrap();
    assert!(prot.sheet);
}
