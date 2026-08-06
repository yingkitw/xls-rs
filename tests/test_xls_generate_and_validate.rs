//! Generate-and-validate loop for the XLSX writer.
//!
//! Builds a representative XLSX workbook, writes it to a temp file, then
//! validates it with the native XlsxReader. If the reader reports any
//! errors, this test fails.
//!
//! Run with `cargo test --test test_xls_generate_and_validate
//! -- --nocapture` to see the per-step output.

use xls_rs::excel::xlsx_reader::XlsxReader;
use xls_rs::excel::{RowData, WriteOptions, XlsxWriter};

/// Validate an XLSX file with the native reader. Returns (ok, message).
fn run_validator(path: &std::path::Path) -> (bool, String) {
    match XlsxReader::from_path(path.to_str().unwrap()) {
        Ok(_) => (true, "ok".to_string()),
        Err(e) => (false, format!("native reader failed: {e}")),
    }
}

/// Build a dense workbook and validate it. This is the
/// headline "does the XLSX generation work" test.
#[test]
fn generate_and_validate_dense_workbook() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("dense.xlsx");

    // ---- 1. Generate ----
    let options = WriteOptions {
        freeze_header: true,
        auto_filter: true,
        ..Default::default()
    };
    let mut w = XlsxWriter::with_options(options);
    w.add_sheet("People").expect("add sheet");

    let mut hdr = RowData::new();
    hdr.add_string("Name");
    hdr.add_string("Age");
    hdr.add_string("Active");
    hdr.add_string("Salary");
    w.add_row(hdr);

    let mut r = RowData::new();
    r.add_string("Alice");
    r.add_number(30.0);
    r.add_bool(true);
    r.add_number(85000.0);
    w.add_row(r);

    let mut r = RowData::new();
    r.add_string("Bob");
    r.add_number(25.0);
    r.add_bool(false);
    r.add_number(65000.0);
    w.add_row(r);

    let mut r = RowData::new();
    r.add_string("Carol");
    r.add_number(45.0);
    r.add_bool(true);
    r.add_string("#N/A");
    w.add_row(r);

    let mut total = RowData::new();
    total.add_string("Total");
    total.add_empty();
    total.add_empty();
    total.add_formula("SUM(D2:D4)");
    w.add_row(total);

    w.set_column_width(0, 14.0);
    w.save(std::fs::File::create(&path).expect("create file"))
        .expect("save xlsx");

    // ---- 2. Validate ----
    let (ok, output) = run_validator(&path);
    println!("validator output: {output}");
    assert!(
        ok,
        "validation failed for {}\noutput:\n{output}",
        path.display()
    );
}

/// Same as above but exercises the rich-features path: merged cells,
/// unicode strings, multiple sheets, etc. This is the
/// stress-and-validate case.
#[test]
fn generate_and_validate_rich_workbook() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("rich.xlsx");

    // ---- 1. Generate ----
    let mut w = XlsxWriter::new();

    // Sheet 1: report with merged title + unicode
    w.add_sheet("Report").expect("add Report");
    let mut title = RowData::new();
    title.add_string("Q1 Report — résumé");
    w.add_row(title);
    w.add_merge_cell(0, 0, 0, 3);

    let mut hdr = RowData::new();
    hdr.add_string("Product");
    hdr.add_string("Q1");
    hdr.add_string("Q2");
    hdr.add_string("Q3");
    w.add_row(hdr);

    for (name, q1, q2, q3) in [
        ("Widget", 10.0, 12.0, 15.0),
        ("Gadget", 5.0, 7.0, 9.0),
        ("日本語", 1.0, 2.0, 3.0),
    ] {
        let mut r = RowData::new();
        r.add_string(name);
        r.add_number(q1);
        r.add_number(q2);
        r.add_number(q3);
        w.add_row(r);
    }

    let mut total = RowData::new();
    total.add_string("Total");
    total.add_formula("SUM(B2:B4)");
    total.add_formula("SUM(C2:C4)");
    total.add_formula("SUM(D2:D4)");
    w.add_row(total);

    w.set_column_width(0, 16.0);

    // Sheet 2: lookup table
    w.add_sheet("Lookup").expect("add Lookup");
    let mut hdr = RowData::new();
    hdr.add_string("Code");
    hdr.add_string("Name");
    w.add_row(hdr);
    for (code, name) in [("W", "Widget"), ("G", "Gadget"), ("S", "Sprocket")] {
        let mut r = RowData::new();
        r.add_string(code);
        r.add_string(name);
        w.add_row(r);
    }

    w.save(std::fs::File::create(&path).expect("create file"))
        .expect("save xlsx");

    // ---- 2. Validate ----
    let (ok, output) = run_validator(&path);
    println!("validator output: {output}");
    assert!(
        ok,
        "validation failed for {}\noutput:\n{output}",
        path.display()
    );
}

/// Edge-case workbook: 1 sheet, 1 row, single string cell. The smallest
/// valid .xlsx the writer can produce. Catches empty-stream bugs.
#[test]
fn generate_and_validate_minimal_workbook() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("minimal.xlsx");

    let mut w = XlsxWriter::new();
    w.add_sheet("S").expect("add sheet");
    let mut r = RowData::new();
    r.add_string("hi");
    w.add_row(r);
    w.save(std::fs::File::create(&path).expect("create file"))
        .expect("save");

    let (ok, output) = run_validator(&path);
    assert!(
        ok,
        "validation failed for minimal file\noutput:\n{output}"
    );
}
