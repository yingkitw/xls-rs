//! Create a small XLSX workbook from scratch using the
//! `XlsxWriter` API. Demonstrates multi-sheet creation, mixed cell types,
//! formulas, merged cells, freeze panes, auto-filter, and unicode strings.
//!
//! Run with: `cargo run --example write_xls`

use xls_rs::excel::{RowData, WriteOptions, XlsxWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = WriteOptions {
        freeze_header: true,
        auto_filter: true,
        ..Default::default()
    };
    let mut w = XlsxWriter::with_options(options);

    // Sheet 1: simple header + data with freeze panes and auto-filter.
    w.add_sheet("People")?;
    let mut header = RowData::new();
    header.add_string("Name");
    header.add_string("Age");
    header.add_string("Active");
    w.add_row(header);

    let mut r1 = RowData::new();
    r1.add_string("Alice");
    r1.add_number(30.0);
    r1.add_bool(true);
    w.add_row(r1);

    let mut r2 = RowData::new();
    r2.add_string("Bob");
    r2.add_number(25.0);
    r2.add_bool(false);
    w.add_row(r2);

    // Sheet 2: a totals row that references sheet 1.
    w.add_sheet("Totals")?;
    let mut r3 = RowData::new();
    r3.add_number(10.0);
    r3.add_number(20.0);
    r3.add_number(30.0);
    w.add_row(r3);

    let mut r4 = RowData::new();
    r4.add_formula("SUM(A1:C1)");
    w.add_row(r4);

    // Sheet 3: merged cells.
    w.add_sheet("Report")?;
    let mut title = RowData::new();
    title.add_string("Quarterly Summary");
    w.add_row(title);
    w.add_merge_cell(0, 0, 0, 2);

    let mut data_row = RowData::new();
    data_row.add_string("Total");
    data_row.add_number(500.0);
    data_row.add_string("#N/A");
    w.add_row(data_row);

    // Sheet 4: unicode strings.
    w.add_sheet("I18N")?;
    let mut r5 = RowData::new();
    r5.add_string("héllo");
    r5.add_string("wörld");
    r5.add_string("日本語");
    r5.add_string("🦀 rust");
    w.add_row(r5);

    let path = "/tmp/demo.xlsx";
    w.save(std::fs::File::create(path)?)?;
    println!("wrote {} ({} bytes)", path, std::fs::metadata(path)?.len());
    Ok(())
}
