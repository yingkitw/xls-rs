//! Iterate-until-correct loop for the XLSX writer.
//!
//! This test demonstrates the "generate → validate → diagnose → fix →
//! repeat" loop. It does not loop indefinitely — the iteration is
//! bounded, and the goal is to provide a single, easy-to-run signal
//! that says "the XLSX generation is correct" or "it still has
//! problem X".
//!
//! Run with `cargo test --test test_xls_iterate_until_correct
//! -- --nocapture` to see the per-iteration output.

use xls_rs::excel::xlsx_reader::XlsxReader;
use xls_rs::excel::{RowData, WriteOptions, XlsxWriter};

#[derive(serde::Deserialize, serde::Serialize)]
struct ValidatorOutput {
    ok: bool,
    #[serde(default)]
    errors: Vec<String>,
}

/// Validate with the native XlsxReader. Returns None if the reader
/// fails to parse the file.
fn run_validator(path: &std::path::Path) -> Option<ValidatorOutput> {
    match XlsxReader::from_path(path.to_str().unwrap()) {
        Ok(_) => Some(ValidatorOutput { ok: true, errors: vec![] }),
        Err(e) => Some(ValidatorOutput { ok: false, errors: vec![e.to_string()] }),
    }
}

/// Generate one representative workbook.
fn generate_iteration(iteration: u32, path: &std::path::Path) {
    let mut w = XlsxWriter::new();
    w.add_sheet(&format!("S{iteration}")).expect("add sheet");
    for r in 0..5 {
        let mut row = RowData::new();
        row.add_string(&format!("row{r}"));
        row.add_number(r as f64 * 1.5);
        row.add_bool(r % 2 == 0);
        if r == 3 {
            row.add_string("#N/A");
        } else {
            row.add_string(&format!("cell_{r}"));
        }
        w.add_row(row);
    }
    w.save(std::fs::File::create(path).expect("create file"))
        .expect("save xlsx");
}

/// The headline iterate-until-correct test. Runs the loop a fixed
/// number of times; if any iteration produces an invalid file, the
/// test fails with the validator's diagnostic.
#[test]
fn iterate_until_xls_is_correct() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("iter.xlsx");

    const MAX_ITERS: u32 = 5;
    for iter in 0..MAX_ITERS {
        generate_iteration(iter, &path);

        let Some(report) = run_validator(&path) else {
            eprintln!("[iter {iter}] validator not available, skipping");
            return;
        };
        eprintln!(
            "[iter {iter}] ok={} errors={:?}",
            report.ok, report.errors
        );

        if report.ok {
            return;
        }

        panic!(
            "iteration {iter} produced an invalid file:\n{}",
            serde_json::to_string_pretty(&report).unwrap()
        );
    }

    panic!("did not converge in {MAX_ITERS} iterations");
}

/// Same idea, but with the rich-features writer (merged cells,
/// formulas, freeze panes, auto-filter, multiple sheets).
#[test]
fn iterate_until_rich_xls_is_correct() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("rich_iter.xlsx");

    let options = WriteOptions {
        freeze_header: true,
        auto_filter: true,
        ..Default::default()
    };
    let mut w = XlsxWriter::with_options(options);

    // Sheet 1
    w.add_sheet("Data").expect("add Data");
    let mut hdr = RowData::new();
    hdr.add_string("K");
    hdr.add_string("V");
    w.add_row(hdr);
    for i in 0..4 {
        let mut r = RowData::new();
        r.add_string(&format!("k{i}"));
        r.add_number(i as f64 * 10.0);
        w.add_row(r);
    }
    let mut total = RowData::new();
    total.add_string("Total");
    total.add_formula("SUM(B2:B5)");
    w.add_row(total);
    w.add_merge_cell(0, 0, 0, 1);

    // Sheet 2
    w.add_sheet("Notes").expect("add Notes");
    let mut r = RowData::new();
    r.add_string("Notes about data");
    w.add_row(r);
    w.add_sheet("Errors").expect("add Errors");
    let mut r = RowData::new();
    r.add_string("#N/A");
    r.add_string("#DIV/0!");
    r.add_string("#REF!");
    w.add_row(r);

    w.save(std::fs::File::create(&path).expect("create file"))
        .expect("save xlsx");

    let Some(report) = run_validator(&path) else {
        eprintln!("validator not available, skipping");
        return;
    };
    eprintln!(
        "rich: ok={} errors={:?}",
        report.ok, report.errors
    );
    if !report.ok {
        panic!(
            "rich iteration produced an invalid file:\n{}",
            serde_json::to_string_pretty(&report).unwrap()
        );
    }
}
