//! Create a small XLS (legacy BIFF8) workbook from scratch using the
//! `XlsWriter` API. Demonstrates multi-sheet creation, mixed cell types,
//! and basic formulas.
//!
//! Run with: `cargo run --example write_xls`

use xls_rs::{XlsRowData, XlsWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut w = XlsWriter::new();

    // Sheet 1: simple header + data.
    w.add_sheet("People")?;
    let mut header = XlsRowData::new();
    header.add_string("Name");
    header.add_string("Age");
    header.add_string("Active");
    w.add_row(header);

    let mut r1 = XlsRowData::new();
    r1.add_string("Alice");
    r1.add_number(30.0);
    r1.add_bool(true);
    w.add_row(r1);

    let mut r2 = XlsRowData::new();
    r2.add_string("Bob");
    r2.add_number(25.0);
    r2.add_bool(false);
    w.add_row(r2);

    // Sheet 2: a totals row that references sheet 1.
    w.add_sheet("Totals")?;
    let mut r3 = XlsRowData::new();
    r3.add_number(10.0);
    r3.add_number(20.0);
    r3.add_number(30.0);
    w.add_row(r3);

    let mut r4 = XlsRowData::new();
    r4.add_formula("SUM(A1:C1)");
    w.add_row(r4);

    // Sheet 3: unicode strings (UTF-16 in BIFF8).
    w.add_sheet("I18N")?;
    let mut r5 = XlsRowData::new();
    r5.add_string("héllo");
    r5.add_string("wörld");
    r5.add_string("日本語");
    r5.add_string("🦀 rust");
    w.add_row(r5);

    w.save("/tmp/demo.xls")?;
    println!("wrote /tmp/demo.xls ({} bytes)", std::fs::metadata("/tmp/demo.xls")?.len());
    Ok(())
}
