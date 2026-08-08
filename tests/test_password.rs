use std::io::Cursor;

use xls_rs::excel::xlsx_crypto;
use xls_rs::excel::xlsx_reader::{XlsxCellValue, XlsxReader};
use xls_rs::{RowData, XlsxWriter};

/// Write a simple XLSX, encrypt it, read it back with the password,
/// and verify the cell data is preserved.
#[test]
fn test_encrypted_xlsx_roundtrip() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    let mut row = RowData::new();
    row.add_string("Hello");
    row.add_number(42.0);
    writer.add_row(row);
    let mut row2 = RowData::new();
    row2.add_string("World");
    row2.add_number(99.0);
    writer.add_row(row2);

    let mut zip_buf = Cursor::new(Vec::new());
    writer.save(&mut zip_buf).unwrap();
    let zip_data = zip_buf.into_inner();

    let encrypted = xlsx_crypto::create_encrypted_xlsx(&zip_data, "secret123").unwrap();
    assert!(xlsx_crypto::is_ole2(&encrypted));

    let reader =
        XlsxReader::from_reader_with_password(Cursor::new(encrypted), "secret123").unwrap();

    assert_eq!(reader.sheet_count(), 1);
    let sheet = reader.get_sheet(0).unwrap();
    assert_eq!(sheet.cells.len(), 2);
    assert!(matches!(&sheet.cells[0][0], XlsxCellValue::String(s) if s == "Hello"));
    assert!(matches!(&sheet.cells[0][1], XlsxCellValue::Number(n) if (*n - 42.0).abs() < 0.01));
    assert!(matches!(&sheet.cells[1][0], XlsxCellValue::String(s) if s == "World"));
    assert!(matches!(&sheet.cells[1][1], XlsxCellValue::Number(n) if (*n - 99.0).abs() < 0.01));
}

/// Wrong password should fail with a clear error.
#[test]
fn test_encrypted_xlsx_wrong_password() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("S").unwrap();
    let mut row = RowData::new();
    row.add_string("data");
    writer.add_row(row);

    let mut zip_buf = Cursor::new(Vec::new());
    writer.save(&mut zip_buf).unwrap();

    let encrypted =
        xlsx_crypto::create_encrypted_xlsx(&zip_buf.into_inner(), "correct").unwrap();

    let result = XlsxReader::from_reader_with_password(Cursor::new(encrypted), "wrong");
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("Password verification failed"),
        "Expected password verification error, got: {}",
        err
    );
}

/// A non-encrypted XLSX passed to from_reader_with_password should still work.
#[test]
fn test_from_reader_with_password_on_unencrypted() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    let mut row = RowData::new();
    row.add_string("Plain");
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let reader =
        XlsxReader::from_reader_with_password(Cursor::new(buf.into_inner()), "any").unwrap();
    assert_eq!(reader.sheet_count(), 1);
    let sheet = reader.get_sheet(0).unwrap();
    assert!(matches!(&sheet.cells[0][0], XlsxCellValue::String(s) if s == "Plain"));
}

/// is_encrypted_xlsx should correctly identify encrypted vs plain files.
#[test]
fn test_is_encrypted_xlsx_detection() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("S").unwrap();
    let mut row = RowData::new();
    row.add_string("x");
    writer.add_row(row);
    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let plain = buf.into_inner();
    assert!(!xlsx_crypto::is_encrypted_xlsx(&plain));

    let encrypted = xlsx_crypto::create_encrypted_xlsx(&plain, "pw").unwrap();
    assert!(xlsx_crypto::is_encrypted_xlsx(&encrypted));
}

/// VBA macros should survive the encrypt→decrypt→read cycle.
#[test]
fn test_encrypted_xlsx_with_vba() {
    let vba = vec![0xD0u8, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1, 0x42, 0x42];

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Data").unwrap();
    let mut row = RowData::new();
    row.add_string("cell");
    writer.add_row(row);
    writer.set_vba_project(vba.clone());

    let mut zip_buf = Cursor::new(Vec::new());
    writer.save(&mut zip_buf).unwrap();
    let zip_data = zip_buf.into_inner();

    let encrypted = xlsx_crypto::create_encrypted_xlsx(&zip_data, "pass").unwrap();
    let reader = XlsxReader::from_reader_with_password(Cursor::new(encrypted), "pass").unwrap();

    assert!(reader.has_macros());
    assert_eq!(reader.vba_project().unwrap(), &vba);
}
