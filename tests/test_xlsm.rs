use std::io::Cursor;

use xls_rs::excel::xlsx_reader::XlsxReader;
use xls_rs::{RowData, XlsxWriter};

/// VBA project bin files start with the OLE2 magic bytes `D0 CF 11 E0`.
/// We use a minimal fake VBA blob — real `.bin` files are OLE2 compound
/// documents containing the VBA storage stream. For round-trip testing
/// we only need to verify the bytes are preserved, not that they are
/// valid VBA.
fn fake_vba_blob() -> Vec<u8> {
    let mut blob = vec![0xD0u8, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    blob.extend_from_slice(b"fake-vba-project-data-for-testing");
    blob
}

#[test]
fn test_write_xlsm_read_back_vba() {
    let vba = fake_vba_blob();

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    let mut row = RowData::new();
    row.add_string("Hello");
    row.add_number(42.0);
    writer.add_row(row);
    writer.set_vba_project(vba.clone());

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let reader = XlsxReader::from_reader(Cursor::new(buf.into_inner())).unwrap();
    assert!(reader.has_macros());
    let read_vba = reader.vba_project().unwrap();
    assert_eq!(read_vba, &vba);
}

#[test]
fn test_no_vba_returns_none() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    let mut row = RowData::new();
    row.add_string("No macros");
    writer.add_row(row);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let reader = XlsxReader::from_reader(Cursor::new(buf.into_inner())).unwrap();
    assert!(!reader.has_macros());
    assert!(reader.vba_project().is_none());
}

#[test]
fn test_vba_round_trip_preserves_data() {
    let vba = fake_vba_blob();

    // Write .xlsm with VBA
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Data").unwrap();
    let mut row = RowData::new();
    row.add_string("A1");
    row.add_number(1.0);
    writer.add_row(row);
    writer.set_vba_project(vba.clone());

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let zip_bytes = buf.into_inner();

    // Read it back
    let reader = XlsxReader::from_reader(Cursor::new(zip_bytes)).unwrap();
    let read_vba = reader.vba_project().unwrap().to_vec();

    // Write it again with the preserved VBA
    let mut writer2 = XlsxWriter::new();
    writer2.add_sheet("Data").unwrap();
    let mut row2 = RowData::new();
    row2.add_string("A1");
    row2.add_number(1.0);
    writer2.add_row(row2);
    writer2.set_vba_project(read_vba.clone());

    let mut buf2 = Cursor::new(Vec::new());
    writer2.save(&mut buf2).unwrap();

    // Read the second file and verify VBA is still there
    let reader2 = XlsxReader::from_reader(Cursor::new(buf2.into_inner())).unwrap();
    assert!(reader2.has_macros());
    assert_eq!(reader2.vba_project().unwrap(), &vba);
}

#[test]
fn test_xlsm_content_types_has_vba() {
    use std::io::Read;
    let vba = fake_vba_blob();

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_vba_project(vba);

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let zip_bytes = buf.into_inner();

    let cursor = std::io::Cursor::new(&zip_bytes);
    let mut za = zip::ZipArchive::new(cursor).unwrap();
    let mut ct = String::new();
    za.by_name("[Content_Types].xml").unwrap().read_to_string(&mut ct).unwrap();

    assert!(ct.contains("vbaProject"), "Content types should mention vbaProject");
    assert!(ct.contains("macroEnabled"), "Content types should use macroEnabled");
}

#[test]
fn test_xlsm_vba_bin_in_archive() {
    use std::io::Read;
    let vba = fake_vba_blob();

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_vba_project(vba.clone());

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let zip_bytes = buf.into_inner();

    let cursor = std::io::Cursor::new(&zip_bytes);
    let mut za = zip::ZipArchive::new(cursor).unwrap();
    let mut bin = Vec::new();
    za.by_name("xl/vbaProject.bin").unwrap().read_to_end(&mut bin).unwrap();

    assert_eq!(bin, vba);
}
