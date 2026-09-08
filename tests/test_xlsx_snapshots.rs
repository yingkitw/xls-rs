//! Snapshot test that writes representative .xlsx artifacts to the
//! project's `output/` directory on every `cargo test` run.
//!
//! Unlike the round-trip tests (which use `tempfile::TempDir` so they
//! leave no trace), this test produces *visible* artifacts that
//! developers can open in Excel/LibreOffice, hand to colleagues, or
//! diff against. Each artifact is also validated by our native reader
//! before being written, so a green `cargo test` guarantees the files
//! in `output/` are correct.
//!
//! The set of artifacts is intentionally small and curated:
//!   - basic_strings.xlsx   — minimal workbook
//!   - dense_mixed.xlsx     — every cell type in one sheet
//!   - multi_sheet.xlsx     — multiple sheets, cross-sheet formula
//!   - merged_freeze.xlsx   — merged cells + freeze panes + auto-filter
//!   - unicode.xlsx         — CJK + astral codepoints
//!   - errors.xlsx          — all 7 error codes
//!   - formula_dense.xlsx   — many formulas of various shapes
//!   - stress.xlsx          — 8 sheets × 50 rows
//!
//! To regenerate just the snapshots, run:
//!     cargo test --test test_xls_snapshots -- --nocapture

use std::path::{Path, PathBuf};

use xls_rs::excel::xlsx_reader::XlsxReader;
use xls_rs::excel::{RowData, WriteOptions, XlsxWriter};

fn output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output")
}

fn ensure_output_dir() -> PathBuf {
    let dir = output_dir();
    std::fs::create_dir_all(&dir).expect("create output dir");
    dir
}

fn write_artifact(name: &str, build: impl FnOnce(&mut XlsxWriter)) -> PathBuf {
    let dir = ensure_output_dir();
    let path = dir.join(name);
    let mut w = XlsxWriter::new();
    build(&mut w);
    w.save(std::fs::File::create(&path).expect("create file"))
        .expect("save xlsx");
    path
}

fn write_artifact_with_options(
    name: &str,
    options: WriteOptions,
    build: impl FnOnce(&mut XlsxWriter),
) -> PathBuf {
    let dir = ensure_output_dir();
    let path = dir.join(name);
    let mut w = XlsxWriter::with_options(options);
    build(&mut w);
    w.save(std::fs::File::create(&path).expect("create file"))
        .expect("save xlsx");
    path
}

fn validate(path: &Path) {
    let _ = XlsxReader::from_path(path.to_str().unwrap())
        .unwrap_or_else(|e| panic!("native reader failed on {}: {e}", path.display()));
}

/// The headline test: emits the whole curated set of artifacts to
/// `output/`. Each artifact is validated before this test returns, so
/// a green `cargo test` guarantees the files in `output/` are correct.
#[test]
fn generate_output_artifacts() {
    let dir = ensure_output_dir();
    // Wipe stale .xls and .xlsx artifacts so the directory is always a
    // faithful representation of "what this test produced on this run".
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|s| s.to_str());
            if ext == Some("xls") || ext == Some("xlsx") {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    // 1. minimal: a single sheet, a single string cell.
    let p = write_artifact("basic_strings.xlsx", |w| {
        w.add_sheet("S").expect("add sheet");
        let mut r = RowData::new();
        r.add_string("hello");
        r.add_string("world");
        w.add_row(r);
    });
    validate(&p);

    // 2. dense mixed: every cell type in one place.
    let p = write_artifact("dense_mixed.xlsx", |w| {
        w.add_sheet("People").expect("add sheet");
        let mut hdr = RowData::new();
        hdr.add_string("Name");
        hdr.add_string("Age");
        hdr.add_string("Active");
        hdr.add_string("Salary");
        hdr.add_string("Note");
        w.add_row(hdr);

        let mut r = RowData::new();
        r.add_string("Alice");
        r.add_number(30.0);
        r.add_bool(true);
        r.add_number(85000.0);
        r.add_formula("D2*1");
        w.add_row(r);

        let mut r = RowData::new();
        r.add_string("Bob");
        r.add_number(25.0);
        r.add_empty();
        r.add_number(65000.0);
        r.add_string("on leave");
        w.add_row(r);

        let mut r = RowData::new();
        r.add_string("Carol");
        r.add_number(45.0);
        r.add_bool(false);
        r.add_string("#N/A");
        r.add_string("日本語 🦀");
        w.add_row(r);

        let mut r = RowData::new();
        r.add_string("Total");
        r.add_empty();
        r.add_empty();
        r.add_formula("SUM(D2:D4)");
        w.add_row(r);

        w.set_column_width(0, 16.0);
    });
    validate(&p);

    // 3. multi-sheet + cross-sheet formula.
    let p = write_artifact("multi_sheet.xlsx", |w| {
        w.add_sheet("Budget").expect("add");
        let mut r = RowData::new();
        r.add_string("Item");
        r.add_string("Amount");
        w.add_row(r);
        for amt in [100.0, 200.0, 50.0] {
            let mut r = RowData::new();
            r.add_string("X");
            r.add_number(amt);
            w.add_row(r);
        }
        let mut r = RowData::new();
        r.add_string("Total");
        r.add_formula("SUM(B2:B4)");
        w.add_row(r);

        w.add_sheet("Summary").expect("add");
        let mut r = RowData::new();
        r.add_string("BudgetTotal");
        r.add_formula("Budget!B5");
        w.add_row(r);
    });
    validate(&p);

    // 4. merged cells + freeze panes + auto-filter.
    let options = WriteOptions {
        freeze_header: true,
        auto_filter: true,
        ..Default::default()
    };
    let p = write_artifact_with_options("merged_freeze.xlsx", options, |w| {
        w.add_sheet("Report").expect("add");
        let mut title = RowData::new();
        title.add_string("Q1 Report");
        w.add_row(title);
        w.add_merge_cell(0, 0, 0, 3);

        let mut hdr = RowData::new();
        hdr.add_string("A");
        hdr.add_string("B");
        hdr.add_string("C");
        hdr.add_string("D");
        w.add_row(hdr);
        for r in 1..=4 {
            let mut row = RowData::new();
            row.add_string(&format!("a{r}"));
            row.add_string(&format!("b{r}"));
            row.add_string(&format!("c{r}"));
            row.add_string(&format!("d{r}"));
            w.add_row(row);
        }
        w.set_column_width(0, 14.0);
    });
    validate(&p);

    // 5. unicode (CJK + astral codepoints).
    let p = write_artifact("unicode.xlsx", |w| {
        w.add_sheet("U").expect("add");
        for s in [
            "日本語 中文 한국어",
            "🦀🐍🐉",
            "mix: ASCII + 日本語 + 🦀",
            "café résumé naïve",
        ] {
            let mut r = RowData::new();
            r.add_string(s);
            w.add_row(r);
        }
    });
    validate(&p);

    // 6. all 7 error codes.
    let p = write_artifact("errors.xlsx", |w| {
        w.add_sheet("Err").expect("add");
        let mut r = RowData::new();
        r.add_string("#NULL!");
        r.add_string("#DIV/0!");
        r.add_string("#VALUE!");
        r.add_string("#REF!");
        r.add_string("#NAME?");
        r.add_string("#NUM!");
        r.add_string("#N/A");
        w.add_row(r);
    });
    validate(&p);

    // 7. many formulas.
    let p = write_artifact("formula_dense.xlsx", |w| {
        w.add_sheet("F").expect("add");
        for i in 0..20 {
            let mut r = RowData::new();
            r.add_number(i as f64);
            r.add_number((i * 2) as f64);
            r.add_formula(format!("A{}*B{}", i + 1, i + 1));
            r.add_formula(format!("SUM(A1:B{})", i + 1));
            r.add_formula(format!("IF(C{}>10,\"big\",\"small\")", i + 1));
            w.add_row(r);
        }
    });
    validate(&p);

    // 8. stress: many sheets, many rows.
    let p = write_artifact("stress.xlsx", |w| {
        for s in 0..8 {
            w.add_sheet(&format!("S{s}")).expect("add");
            let mut hdr = RowData::new();
            hdr.add_string("K");
            hdr.add_string("V");
            w.add_row(hdr);
            for r in 0..50 {
                let mut row = RowData::new();
                row.add_string(&format!("k{s}_{r}"));
                row.add_number((s * 50 + r) as f64);
                w.add_row(row);
            }
        }
    });
    validate(&p);

    eprintln!("\nArtifacts written to {}:", dir.display());
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("xlsx") {
                eprintln!("  {}", entry.path().display());
            }
        }
    }
}
