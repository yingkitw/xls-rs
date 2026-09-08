//! Integration tests for document (core) properties (write + read).

use std::io::Cursor;
use xls_rs::{DocumentProperties, XlsxReader, XlsxWriter};
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
fn test_document_properties_writes_core_xml() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_document_properties(DocumentProperties {
        title: Some("My Report".into()),
        creator: Some("Jane Doe".into()),
        subject: Some("Q3 Financials".into()),
        description: Some("Quarterly financial summary".into()),
        keywords: Some("finance q3 2026".into()),
        category: Some("Finance".into()),
        content_status: Some("Draft".into()),
        created: Some("2026-01-15T10:30:00Z".into()),
        modified: Some("2026-01-16T14:00:00Z".into()),
        last_modified_by: Some("John Smith".into()),
    });

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let core = read_zip_part(buf.get_ref(), "docProps/core.xml").unwrap();
    assert!(core.contains("<dc:title>My Report</dc:title>"));
    assert!(core.contains("<dc:creator>Jane Doe</dc:creator>"));
    assert!(core.contains("<dc:subject>Q3 Financials</dc:subject>"));
    assert!(core.contains("<dc:description>Quarterly financial summary</dc:description>"));
    assert!(core.contains("<cp:keywords>finance q3 2026</cp:keywords>"));
    assert!(core.contains("<cp:category>Finance</cp:category>"));
    assert!(core.contains("<cp:contentStatus>Draft</cp:contentStatus>"));
    assert!(core.contains("<dcterms:created"));
    assert!(core.contains("2026-01-15T10:30:00Z"));
    assert!(core.contains("<dcterms:modified"));
    assert!(core.contains("2026-01-16T14:00:00Z"));
    assert!(core.contains("<cp:lastModifiedBy>John Smith</cp:lastModifiedBy>"));
}

#[test]
fn test_document_properties_content_types_and_rels() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_document_properties(DocumentProperties {
        title: Some("Test".into()),
        ..Default::default()
    });

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    let ct = read_zip_part(buf.get_ref(), "[Content_Types].xml").unwrap();
    assert!(
        ct.contains(r#"PartName="/docProps/core.xml""#),
        "missing core.xml content type"
    );

    let rels = read_zip_part(buf.get_ref(), "_rels/.rels").unwrap();
    assert!(
        rels.contains("core-properties"),
        "missing core-properties relationship"
    );
    assert!(
        rels.contains("docProps/core.xml"),
        "missing core.xml target"
    );
}

#[test]
fn test_document_properties_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("props.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_document_properties(DocumentProperties {
        title: Some("Round Trip".into()),
        creator: Some("Tester".into()),
        subject: Some("Testing".into()),
        keywords: Some("test round-trip".into()),
        created: Some("2026-01-01T00:00:00Z".into()),
        ..Default::default()
    });

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let props = reader.document_properties().unwrap();
    assert_eq!(props.title, Some("Round Trip".into()));
    assert_eq!(props.creator, Some("Tester".into()));
    assert_eq!(props.subject, Some("Testing".into()));
    assert_eq!(props.keywords, Some("test round-trip".into()));
    assert_eq!(props.created, Some("2026-01-01T00:00:00Z".into()));
}

#[test]
fn test_document_properties_partial() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_document_properties(DocumentProperties {
        title: Some("Only Title".into()),
        ..Default::default()
    });

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let core = read_zip_part(buf.get_ref(), "docProps/core.xml").unwrap();
    assert!(core.contains("<dc:title>Only Title</dc:title>"));
    assert!(!core.contains("<dc:creator>"), "should not have creator");
    assert!(!core.contains("<dc:subject>"), "should not have subject");
}

#[test]
fn test_no_document_properties_no_core_xml() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();

    // core.xml should not exist
    assert!(read_zip_part(buf.get_ref(), "docProps/core.xml").is_none());
    // Content types should not have core.xml override
    let ct = read_zip_part(buf.get_ref(), "[Content_Types].xml").unwrap();
    assert!(!ct.contains("docProps/core.xml"));
    // Rels should not have core-properties relationship
    let rels = read_zip_part(buf.get_ref(), "_rels/.rels").unwrap();
    assert!(!rels.contains("core-properties"));
}

#[test]
fn test_no_document_properties_reader_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("noprops.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.add_data(&[vec!["A".into()], vec!["1".into()]]);

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    assert!(reader.document_properties().is_none());
}

#[test]
fn test_document_properties_with_data_preserves_cells() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("props_data.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.add_data(&[
        vec!["Name".into(), "Value".into()],
        vec!["A".into(), "1".into()],
    ]);
    writer.set_document_properties(DocumentProperties {
        title: Some("Data Report".into()),
        creator: Some("Data Team".into()),
        ..Default::default()
    });

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let sheet = reader.get_sheet(0).unwrap();
    assert_eq!(sheet.row_count(), 2);
    assert_eq!(sheet.col_count(), 2);
    let props = reader.document_properties().unwrap();
    assert_eq!(props.title, Some("Data Report".into()));
    assert_eq!(props.creator, Some("Data Team".into()));
}

#[test]
fn test_document_properties_xml_escaping() {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_document_properties(DocumentProperties {
        title: Some("A & B < C > D".into()),
        creator: Some("Jane \"Quote\" Doe".into()),
        ..Default::default()
    });

    let mut buf = Cursor::new(Vec::new());
    writer.save(&mut buf).unwrap();
    let core = read_zip_part(buf.get_ref(), "docProps/core.xml").unwrap();
    // XML special characters should be properly escaped in the output.
    // Check for escaped entity suffixes (amp;, lt;, gt;, quot;) which
    // only appear in properly escaped XML.
    assert!(core.contains("amp;"), "ampersand not escaped: {}", core);
    assert!(core.contains("lt;"), "less-than not escaped: {}", core);
    assert!(core.contains("gt;"), "greater-than not escaped: {}", core);
    assert!(core.contains("quot;"), "quote not escaped: {}", core);
}

#[test]
fn test_document_properties_all_fields_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("all_props.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_document_properties(DocumentProperties {
        title: Some("Complete Report".into()),
        creator: Some("Author Name".into()),
        subject: Some("Annual Review".into()),
        description: Some("A comprehensive annual review document.".into()),
        keywords: Some("annual report finance 2026".into()),
        category: Some("Reports".into()),
        content_status: Some("Final".into()),
        created: Some("2026-01-15T09:00:00Z".into()),
        modified: Some("2026-02-20T17:30:00Z".into()),
        last_modified_by: Some("Editor Name".into()),
    });

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let props = reader.document_properties().unwrap();
    assert_eq!(props.title, Some("Complete Report".into()));
    assert_eq!(props.creator, Some("Author Name".into()));
    assert_eq!(props.subject, Some("Annual Review".into()));
    assert_eq!(
        props.description,
        Some("A comprehensive annual review document.".into())
    );
    assert_eq!(props.keywords, Some("annual report finance 2026".into()));
    assert_eq!(props.category, Some("Reports".into()));
    assert_eq!(props.content_status, Some("Final".into()));
    assert_eq!(props.created, Some("2026-01-15T09:00:00Z".into()));
    assert_eq!(props.modified, Some("2026-02-20T17:30:00Z".into()));
    assert_eq!(props.last_modified_by, Some("Editor Name".into()));
}

#[test]
fn test_document_properties_with_multiple_sheets() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("multi_props.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Summary").unwrap();
    writer.add_data(&[vec!["Total".into()], vec!["100".into()]]);
    writer.add_sheet("Details").unwrap();
    writer.add_data(&[
        vec!["Item".into(), "Qty".into()],
        vec!["A".into(), "5".into()],
    ]);
    writer.set_document_properties(DocumentProperties {
        title: Some("Multi-Sheet Report".into()),
        creator: Some("Multi-Team".into()),
        ..Default::default()
    });

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    // Both sheets should be readable
    let s0 = reader.get_sheet(0).unwrap();
    assert_eq!(s0.row_count(), 2);
    let s1 = reader.get_sheet(1).unwrap();
    assert_eq!(s1.row_count(), 2);
    // Properties should also be present
    let props = reader.document_properties().unwrap();
    assert_eq!(props.title, Some("Multi-Sheet Report".into()));
    assert_eq!(props.creator, Some("Multi-Team".into()));
}

#[test]
fn test_document_properties_only_modified_and_last_modified_by() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mod_props.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_document_properties(DocumentProperties {
        modified: Some("2026-03-01T12:00:00Z".into()),
        last_modified_by: Some("Last Editor".into()),
        ..Default::default()
    });

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let props = reader.document_properties().unwrap();
    assert_eq!(props.modified, Some("2026-03-01T12:00:00Z".into()));
    assert_eq!(props.last_modified_by, Some("Last Editor".into()));
    // Fields not set should be None
    assert!(props.title.is_none());
    assert!(props.creator.is_none());
    assert!(props.created.is_none());
}

#[test]
fn test_document_properties_combined_with_protection() {
    use xls_rs::{SheetProtection, WorkbookProtection};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("props_prot.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.add_data(&[vec!["A".into()], vec!["1".into()]]);
    writer.set_document_properties(DocumentProperties {
        title: Some("Protected Report".into()),
        creator: Some("Security Team".into()),
        ..Default::default()
    });
    writer.set_sheet_protection(SheetProtection::locked());
    writer.set_workbook_protection(WorkbookProtection {
        lock_structure: true,
        ..Default::default()
    });

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    // Document properties
    let props = reader.document_properties().unwrap();
    assert_eq!(props.title, Some("Protected Report".into()));
    assert_eq!(props.creator, Some("Security Team".into()));
    // Sheet protection
    let prot = reader.sheet_protection(0).unwrap();
    assert!(prot.sheet);
    // Data preserved
    let sheet = reader.get_sheet(0).unwrap();
    assert_eq!(sheet.row_count(), 2);
    assert_eq!(sheet.get_cell(0, 0).to_string(), "A");
    assert_eq!(sheet.get_cell(1, 0).to_string(), "1");
}

#[test]
fn test_document_properties_unicode_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unicode_props.xlsx");

    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();
    writer.set_document_properties(DocumentProperties {
        title: Some("日本語タイトル".into()),
        creator: Some("José García".into()),
        subject: Some("Übersicht".into()),
        ..Default::default()
    });

    let file = std::fs::File::create(&path).unwrap();
    writer.save(file).unwrap();

    let reader = XlsxReader::from_path(path.to_str().unwrap()).unwrap();
    let props = reader.document_properties().unwrap();
    assert_eq!(props.title, Some("日本語タイトル".into()));
    assert_eq!(props.creator, Some("José García".into()));
    assert_eq!(props.subject, Some("Übersicht".into()));
}
