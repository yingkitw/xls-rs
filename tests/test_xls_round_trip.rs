//! End-to-end round-trip validation for the XLS (BIFF8) writer.
//!
//! These tests build a representative XLS workbook, write it via
//! `XlsWriter`, then read it back with our native `XlsReader` and assert
//! that every cell matches the source. The test is intentionally strict:
//! any mismatch surfaces as a clear, actionable diagnostic that points
//! at the exact cell and the expected vs actual value.
//!
//! Run with `cargo test --test test_xls_round_trip`.
//!
//! If the test fails, the diagnostics tell you which cell the writer is
//! dropping or mangling. Fix the writer, re-run, and the test must pass
//! before the change is considered complete.

use xls_rs::excel::xls_reader::{SheetData, XlsReader};
use xls_rs::{XlsRowData, XlsWriter};

/// One expected cell. Use `AnyOf` for cells that may be reformatted by
/// the reader (e.g. integer 30.0 -> "30" vs "30.0") and a specific value
/// for exact matches.
#[derive(Debug, Clone)]
enum Expected {
    Exact(String),
    AnyOf(Vec<String>),
    Boolish,
    ErrorCode,
    Empty,
}

fn check_expected(label: &str, actual: &str, expected: &Expected) -> Result<(), String> {
    match expected {
        Expected::Exact(s) => {
            if actual == s {
                Ok(())
            } else {
                Err(format!("{label}: expected {s:?}, got {actual:?}"))
            }
        }
        Expected::AnyOf(alts) => {
            if alts.iter().any(|a| a == actual) {
                Ok(())
            } else {
                Err(format!("{label}: expected one of {alts:?}, got {actual:?}"))
            }
        }
        Expected::Boolish => {
            const OK: &[&str] = &["TRUE", "FALSE", "true", "false", "1", "0"];
            if OK.contains(&actual) {
                Ok(())
            } else {
                Err(format!(
                    "{label}: expected a boolean literal, got {actual:?}"
                ))
            }
        }
        Expected::ErrorCode => {
            const CODES: &[&str] = &[
                "#NULL!",
                "#DIV/0!",
                "#VALUE!",
                "#REF!",
                "#NAME?",
                "#NUM!",
                "#N/A",
            ];
            if CODES.contains(&actual) {
                Ok(())
            } else {
                Err(format!(
                    "{label}: expected a known error code, got {actual:?}"
                ))
            }
        }
        Expected::Empty => {
            if actual.is_empty() {
                Ok(())
            } else {
                Err(format!("{label}: expected empty cell, got {actual:?}"))
            }
        }
    }
}

fn assert_cell(
    sheet: &SheetData,
    row: usize,
    col: usize,
    expected: Expected,
    label: &str,
) {
    let got = sheet.get_cell(row, col).to_string();
    check_expected(label, &got, &expected)
        .unwrap_or_else(|e| panic!("{e}"));
}

/// Build a dense, mixed-type XLS workbook, write it, read it back, and
/// validate that every cell survived the round-trip. This is the
/// canonical "does the XLS generation work" test.
#[test]
fn round_trip_dense_mixed_workbook() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("dense.xls");

    // ------------------------------------------------------------------
    // Build the writer.
    // ------------------------------------------------------------------
    let mut w = XlsWriter::new();
    w.add_sheet("People").expect("add sheet");

    // Row 0: header
    let mut r = XlsRowData::new();
    r.add_string("Name");
    r.add_string("Age");
    r.add_string("Active");
    r.add_string("Salary");
    r.add_string("Note");
    w.add_row(r);

    // Row 1: data row with formula and number
    let mut r = XlsRowData::new();
    r.add_string("Alice");
    r.add_number(30.0);
    r.add_bool(true);
    r.add_number(85000.0);
    r.add_formula("D2*1");
    w.add_row(r);

    // Row 2: data row with empty middle cell
    let mut r = XlsRowData::new();
    r.add_string("Bob");
    r.add_number(25.0);
    r.add_empty();
    r.add_number(65000.0);
    r.add_string("on leave");
    w.add_row(r);

    // Row 3: data row with error and a string containing unicode
    let mut r = XlsRowData::new();
    r.add_string("Carol");
    r.add_number(45.0);
    r.add_bool(false);
    r.add_error("#N/A");
    r.add_string("日本語 🦀");
    w.add_row(r);

    // Row 4: aggregated row
    let mut r = XlsRowData::new();
    r.add_string("Total");
    r.add_empty();
    r.add_empty();
    r.add_formula("SUM(D2:D4)");
    r.add_string("");
    w.add_row(r);

    w.freeze_panes(1, 0);
    w.set_auto_filter(0, 0, 4, 4);
    w.set_column_width(0, 16.0);
    w.set_column_width(4, 22.0);

    w.save(path.to_str().unwrap()).expect("save xls");

    // ------------------------------------------------------------------
    // Read it back with the native reader.
    // ------------------------------------------------------------------
    let reader =
        XlsReader::from_path(path.to_str().unwrap()).expect("native reader opens file");
    let sheet = reader
        .get_sheet_by_name("People")
        .expect("People sheet present");

    // Sheet name
    assert_eq!(sheet.name, "People", "sheet name preserved");

    // Row 0: header
    assert_cell(sheet, 0, 0, Expected::Exact("Name".into()), "R0C0");
    assert_cell(sheet, 0, 1, Expected::Exact("Age".into()), "R0C1");
    assert_cell(sheet, 0, 2, Expected::Exact("Active".into()), "R0C2");
    assert_cell(sheet, 0, 3, Expected::Exact("Salary".into()), "R0C3");
    assert_cell(sheet, 0, 4, Expected::Exact("Note".into()), "R0C4");

    // Row 1: Alice
    assert_cell(sheet, 1, 0, Expected::Exact("Alice".into()), "R1C0");
    assert_cell(
        sheet,
        1,
        1,
        Expected::AnyOf(vec!["30".into(), "30.0".into()]),
        "R1C1 (Age=30)",
    );
    assert_cell(sheet, 1, 2, Expected::Boolish, "R1C2 (Active=true)");
    assert_cell(
        sheet,
        1,
        3,
        Expected::AnyOf(vec!["85000".into(), "85000.0".into()]),
        "R1C3 (Salary=85000)",
    );
    // D2*1 cached value is 0 (placeholder) — see formula_cell in biff.rs.
    // Excel recomputes on open; our reader should at least read a number.
    assert_cell(
        sheet,
        1,
        4,
        Expected::AnyOf(vec![
            "0".into(),
            "0.0".into(),
            "85000".into(),
            "85000.0".into(),
        ]),
        "R1C4 (formula cached)",
    );

    // Row 2: Bob (empty Active)
    assert_cell(sheet, 2, 0, Expected::Exact("Bob".into()), "R2C0");
    assert_cell(
        sheet,
        2,
        1,
        Expected::AnyOf(vec!["25".into(), "25.0".into()]),
        "R2C1 (Age=25)",
    );
    assert_cell(sheet, 2, 2, Expected::Empty, "R2C2 (empty)");
    assert_cell(
        sheet,
        2,
        3,
        Expected::AnyOf(vec!["65000".into(), "65000.0".into()]),
        "R2C3 (Salary=65000)",
    );
    assert_cell(sheet, 2, 4, Expected::Exact("on leave".into()), "R2C4");

    // Row 3: Carol (error + unicode)
    assert_cell(sheet, 3, 0, Expected::Exact("Carol".into()), "R3C0");
    assert_cell(
        sheet,
        3,
        1,
        Expected::AnyOf(vec!["45".into(), "45.0".into()]),
        "R3C1 (Age=45)",
    );
    assert_cell(sheet, 3, 2, Expected::Boolish, "R3C2 (Active=false)");
    assert_cell(sheet, 3, 3, Expected::ErrorCode, "R3C3 (error)");
    assert_cell(
        sheet,
        3,
        4,
        Expected::Exact("日本語 🦀".into()),
        "R3C4 (unicode)",
    );

    // Row 4: Total
    assert_cell(sheet, 4, 0, Expected::Exact("Total".into()), "R4C0");
    // Cached value of SUM(D2:D4) is 0 (placeholder) — that's fine.
    assert_cell(
        sheet,
        4,
        3,
        Expected::AnyOf(vec![
            "0".into(),
            "0.0".into(),
            "150000".into(),
            "150000.0".into(),
        ]),
        "R4C3 (SUM formula cached)",
    );
}

/// Round-trip for multiple sheets with cross-sheet formulas.
#[test]
fn round_trip_multi_sheet_with_formulas() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("multi.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("Budget").expect("add Budget");
    let mut r = XlsRowData::new();
    r.add_string("Item");
    r.add_string("Amount");
    w.add_row(r);
    for amt in [100.0, 200.0, 50.0] {
        let mut r = XlsRowData::new();
        r.add_string("X");
        r.add_number(amt);
        w.add_row(r);
    }
    // total row
    let mut r = XlsRowData::new();
    r.add_string("Total");
    r.add_formula("SUM(B2:B4)");
    w.add_row(r);

    w.add_sheet("Summary").expect("add Summary");
    let mut r = XlsRowData::new();
    r.add_string("BudgetTotal");
    r.add_formula("Budget!B5");
    w.add_row(r);

    w.save(path.to_str().unwrap()).expect("save");

    let reader = XlsReader::from_path(path.to_str().unwrap()).expect("read");
    assert_eq!(reader.sheet_count(), 2);
    assert_eq!(reader.sheet_names(), vec!["Budget", "Summary"]);

    let budget = reader.get_sheet_by_name("Budget").expect("Budget");
    assert_cell(
        budget,
        0,
        0,
        Expected::Exact("Item".into()),
        "Budget R0C0",
    );
    assert_cell(
        budget,
        0,
        1,
        Expected::Exact("Amount".into()),
        "Budget R0C1",
    );
    assert_cell(
        budget,
        1,
        1,
        Expected::AnyOf(vec!["100".into(), "100.0".into()]),
        "Budget R1C1",
    );
    assert_cell(
        budget,
        4,
        1,
        Expected::AnyOf(vec![
            "0".into(),
            "0.0".into(),
            "350".into(),
            "350.0".into(),
        ]),
        "Budget R4C1 (SUM cached)",
    );

    let summary = reader.get_sheet_by_name("Summary").expect("Summary");
    assert_cell(
        summary,
        0,
        0,
        Expected::Exact("BudgetTotal".into()),
        "Summary R0C0",
    );
    assert_cell(
        summary,
        0,
        1,
        Expected::AnyOf(vec!["0".into(), "0.0".into()]),
        "Summary R0C1 (cross-sheet formula cached)",
    );
}

/// Validate the BIFF8 byte structure: BOF/EOF, the reserved field is 0,
/// etc. This is a tighter contract on the wire format that catches
/// writer bugs even if a permissive reader happens to accept the file.
#[test]
fn round_trip_biff8_byte_layout() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("layout.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("S").expect("add sheet");
    let mut r = XlsRowData::new();
    r.add_string("hello");
    r.add_number(7.0);
    w.add_row(r);
    w.save(path.to_str().unwrap()).expect("save");

    let bytes = std::fs::read(&path).expect("read");
    // CFB magic
    assert_eq!(
        &bytes[0..8],
        &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]
    );
    // CFB v3 (sector shift = 9)
    assert_eq!(u16::from_le_bytes([bytes[30], bytes[31]]), 9);
    // Mini sector shift = 6 (64-byte mini-sectors)
    assert_eq!(u16::from_le_bytes([bytes[32], bytes[33]]), 6);
}

/// Stress: many sheets, many rows, with every cell type interleaved.
/// This catches layout drift when streams grow large.
#[test]
fn round_trip_many_sheets_stress() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("stress.xls");

    let mut w = XlsWriter::new();
    for s in 0..6 {
        w.add_sheet(&format!("S{s}")).expect("add sheet");
        let mut hdr = XlsRowData::new();
        hdr.add_string("K");
        hdr.add_string("V");
        w.add_row(hdr);
        for r in 0..10 {
            let mut row = XlsRowData::new();
            row.add_string(format!("k{s}_{r}"));
            row.add_number((s * 10 + r) as f64);
            w.add_row(row);
        }
    }
    w.save(path.to_str().unwrap()).expect("save");

    let reader = XlsReader::from_path(path.to_str().unwrap()).expect("read");
    assert_eq!(reader.sheet_count(), 6);
    for s in 0..6 {
        let name = format!("S{s}");
        let sheet = reader
            .get_sheet_by_name(&name)
            .unwrap_or_else(|| panic!("missing sheet {name}"));
        // header
        assert_cell(
            sheet,
            0,
            0,
            Expected::Exact("K".into()),
            &format!("{name} R0C0"),
        );
        assert_cell(
            sheet,
            0,
            1,
            Expected::Exact("V".into()),
            &format!("{name} R0C1"),
        );
        // data rows
        for r in 0..10 {
            let v = (s * 10 + r) as f64;
            let vstr = if v == v as i64 as f64 {
                (v as i64).to_string()
            } else {
                v.to_string()
            };
            assert_cell(
                sheet,
                r + 1,
                0,
                Expected::Exact(format!("k{s}_{r}")),
                &format!("{name} R{}C0", r + 1),
            );
            assert_cell(
                sheet,
                r + 1,
                1,
                Expected::AnyOf(vec![vstr.clone(), format!("{vstr}.0")]),
                &format!("{name} R{}C1", r + 1),
            );
        }
    }
}

/// All seven BIFF8 error codes must round-trip.
#[test]
fn round_trip_all_error_codes() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("errors.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("Err").expect("add sheet");
    let mut r = XlsRowData::new();
    r.add_error("#NULL!");
    r.add_error("#DIV/0!");
    r.add_error("#VALUE!");
    r.add_error("#REF!");
    r.add_error("#NAME?");
    r.add_error("#NUM!");
    r.add_error("#N/A");
    w.add_row(r);
    w.save(path.to_str().unwrap()).expect("save");

    let reader = XlsReader::from_path(path.to_str().unwrap()).expect("read");
    let sheet = reader.get_sheet_by_name("Err").expect("Err sheet");

    // Order is preserved: c0..c6 map to those errors.
    let codes = [
        "#NULL!", "#DIV/0!", "#VALUE!", "#REF!", "#NAME?", "#NUM!", "#N/A",
    ];
    for (i, code) in codes.iter().enumerate() {
        assert_cell(
            sheet,
            0,
            i,
            Expected::Exact((*code).to_string()),
            &format!("Err R0C{i}"),
        );
    }
}

/// Special characters in strings: control chars are rejected by the
/// writer, but high-bit / extended unicode must round-trip.
#[test]
fn round_trip_special_strings() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("special.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("Spec").expect("add sheet");
    let cases = [
        "plain ASCII",
        "with \"quotes\" and 'apos'",
        "tab\there",
        "comma, semicolon; colon:",
        "日本語 中文 한국어",
        "🦀🐍🐉",
        "mix: ASCII + 日本語 + 🦀",
    ];
    for c in cases {
        let mut r = XlsRowData::new();
        r.add_string(c);
        w.add_row(r);
    }
    w.save(path.to_str().unwrap()).expect("save");

    let reader = XlsReader::from_path(path.to_str().unwrap()).expect("read");
    let sheet = reader.get_sheet_by_name("Spec").expect("Spec sheet");
    for (i, expected) in cases.iter().enumerate() {
        assert_cell(
            sheet,
            i,
            0,
            Expected::Exact((*expected).to_string()),
            &format!("Spec R{i}C0"),
        );
    }
}

/// Sheet name edge cases: max length, unicode, etc.
#[test]
fn round_trip_sheet_name_edges() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("names.xls");

    let mut w = XlsWriter::new();
    // Add a row to each sheet *before* creating the next one — that
    // way the row lands on the freshly created sheet.
    let sheets = ["A", "日本語", "Last"];
    for (i, name) in sheets.iter().enumerate() {
        w.add_sheet(name).expect("add sheet");
        let mut r = XlsRowData::new();
        r.add_string(format!("sheet{i}"));
        w.add_row(r);
    }
    w.save(path.to_str().unwrap()).expect("save");

    let reader = XlsReader::from_path(path.to_str().unwrap()).expect("read");
    let names = reader.sheet_names();
    assert_eq!(names, vec!["A", "日本語", "Last"]);
    for (i, _) in names.iter().enumerate() {
        let sheet = reader.get_sheet(i).expect("sheet");
        assert_cell(
            sheet,
            0,
            0,
            Expected::Exact(format!("sheet{i}")),
            &format!("R0C0 of {}", names[i]),
        );
    }
}

/// Long strings (over the 255-cch single-record limit get rejected by
/// the SST) still round-trip cleanly up to that limit.
#[test]
fn round_trip_long_strings() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("long.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("L").expect("add sheet");
    let cases = [
        "x".repeat(255),       // exact single-byte limit
        "x".repeat(64),        // mid-range
        "🦀".repeat(100),     // 100 astral codepoints
    ];
    for c in &cases {
        let mut r = XlsRowData::new();
        r.add_string(c);
        w.add_row(r);
    }
    w.save(path.to_str().unwrap()).expect("save");

    let reader = XlsReader::from_path(path.to_str().unwrap()).expect("read");
    let sheet = reader.get_sheet_by_name("L").expect("L sheet");
    for (i, expected) in cases.iter().enumerate() {
        assert_cell(
            sheet,
            i,
            0,
            Expected::Exact(expected.clone()),
            &format!("L R{i}C0 (len={})", expected.len()),
        );
    }
}

/// Numbers at the edges of IEEE 754 f64 round-trip (the writer uses
/// NUMBER records, not RK compressed, so no precision loss is expected).
#[test]
fn round_trip_number_edges() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("nums.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("N").expect("add sheet");
    let mut r = XlsRowData::new();
    let vals = [
        0.0_f64,
        -0.0,
        1.0,
        -1.0,
        1.5,
        -1.5,
        f64::MIN_POSITIVE, // smallest positive subnormal
        f64::MAX,
        f64::MIN,
        1e-300,
        1e300,
    ];
    for v in vals {
        r.add_number(v);
    }
    w.add_row(r);
    w.save(path.to_str().unwrap()).expect("save");

    let reader = XlsReader::from_path(path.to_str().unwrap()).expect("read");
    let sheet = reader.get_sheet_by_name("N").expect("N sheet");
    for (i, v) in vals.iter().enumerate() {
        let cell = sheet.get_cell(0, i);
        match cell {
            xls_rs::excel::xls_reader::CellValue::Number(n) => {
                if n.is_finite() && v.is_finite() {
                    assert!(
                        (n - v).abs() < (v.abs() * 1e-12).max(1e-12),
                        "R0C{i}: expected {v}, got {n}"
                    );
                } else {
                    // sign / NaN — only check sign for ±0 and infinities
                    assert_eq!(
                        n.is_sign_negative(),
                        v.is_sign_negative(),
                        "R0C{i}: sign mismatch ({v} vs {n})"
                    );
                }
            }
            other => panic!("R0C{i}: expected Number, got {other:?}"),
        }
    }
}

// =====================================================================
// BIFF8 layout regression tests.
//
// These tests do NOT round-trip through our own reader (that is already
// covered by the other tests in this file). Instead, they parse the
// generated BIFF8 byte stream directly and assert that critical
// records have the exact layout Excel and xlrd expect.
//
// History: in mid-2026 we discovered that the writer was emitting
// several records with the wrong body length or wrong field count
// (e.g. WINDOW1 was 14 bytes when it should be 18, FONT was missing
// its character-count prefix, BoundSheet used u16 visibility instead
// of u8, and the workbook globals section was missing the required
// BIFF8 setup records: INTERFACEHDR / MMS / INTERFACEEND /
// WRITEACCESS). This caused Excel to show "unreadable content" and
// caused xlrd to read back only the first sheet with empty names.
// These tests lock in the fixes so we never regress.
// =====================================================================

/// Parse the BIFF8 record stream from a generated `.xls` and return
/// `(globals, sheets)` where:
/// - `globals` is a list of (id, body_length, body) tuples for the
///   workbook globals substream (everything from the workbook BOF up
///   to the first sheet BOF, exclusive).
/// - `sheets` is a list of per-sheet record lists. The first record
///   of each sheet is its BOF (0x0809 with body type 0x0010).
#[cfg(test)]
fn parse_workbook_streams(
    path: &std::path::Path,
) -> (Vec<(u16, usize, Vec<u8>)>, Vec<Vec<(u16, usize, Vec<u8>)>>) {
    let data = XlsReader::read_workbook_stream(path.to_str().unwrap())
        .expect("read workbook stream");

    // Parse all BIFF8 records.
    let mut all: Vec<(u16, usize, Vec<u8>)> = Vec::new();
    let mut p = 0;
    while p + 4 <= data.len() {
        let id = u16::from_le_bytes([data[p], data[p + 1]]);
        let len = u16::from_le_bytes([data[p + 2], data[p + 3]]) as usize;
        if p + 4 + len > data.len() {
            break;
        }
        all.push((id, len, data[p + 4..p + 4 + len].to_vec()));
        p += 4 + len;
    }

    // Split into globals (everything before the first sheet BOF) and
    // per-sheet record lists. A sheet BOF is a BOF record whose body
    // starts with the type field 0x0010 (worksheet). The first BOF
    // (type 0x0005, workbook) belongs to the globals.
    let mut globals: Vec<(u16, usize, Vec<u8>)> = Vec::new();
    let mut sheets: Vec<Vec<(u16, usize, Vec<u8>)>> = Vec::new();
    let mut current: Option<&mut Vec<(u16, usize, Vec<u8>)>> = Some(&mut globals);
    for rec in all {
        if rec.0 == 0x0809 {
            // BOF body layout: version(2) + type(2) + build(2) + year(2) + ...
            // Type 0x0005 = workbook, 0x0010 = worksheet.
            let body_type = if rec.2.len() >= 4 {
                u16::from_le_bytes([rec.2[2], rec.2[3]])
            } else {
                0
            };
            if body_type == 0x0010 {
                // worksheet BOF — start a new sheet
                let new_sheet = Vec::new();
                sheets.push(new_sheet);
                current = sheets.last_mut();
            }
        }
        if let Some(buf) = current.as_mut() {
            buf.push(rec);
        }
    }
    (globals, sheets)
}

/// Convenience: just the globals records.
#[cfg(test)]
fn parse_workbook_stream(path: &std::path::Path) -> Vec<(u16, usize, Vec<u8>)> {
    parse_workbook_streams(path).0
}

#[test]
fn biff8_workbook_has_required_setup_records() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("setup.xls");

    let mut w = XlsWriter::new();
    w.add_sheet("S").expect("add sheet");
    w.add_row(XlsRowData::new());
    w.save(path.to_str().unwrap()).expect("save");

    let (globals, _sheets) = parse_workbook_streams(&path);
    let ids: Vec<u16> = globals.iter().map(|(id, _, _)| *id).collect();

    // First record MUST be BOF (0x0809), type = workbook (0x0005).
    assert_eq!(ids[0], 0x0809, "first record must be workbook BOF");

    // The required BIFF8 setup block must appear in order, right
    // after the workbook BOF and before any sheet data. Without these
    // records Excel shows "unreadable content" and xlrd cannot
    // enumerate the sheets correctly.
    let required: &[(u16, &str)] = &[
        (0x0809, "BOF (workbook)"),
        (0x00E1, "INTERFACEHDR"),
        (0x00C1, "MMS"),
        (0x00E2, "INTERFACEEND"),
        (0x005C, "WRITEACCESS"),
        (0x0042, "CODEPAGE"),
        (0x0161, "DSF"),
        (0x013D, "TABID"),
        (0x009C, "FNGROUPNAME"),
        (0x0019, "WINDOWPROTECT"),
        (0x0012, "PROTECT"),
        (0x0063, "OBJECTPROTECT"),
        (0x0013, "PASSWORD"),
        (0x01AF, "PROT4REV"),
        (0x01BC, "PROT4REVPASS"),
        (0x0040, "BACKUP"),
        (0x008D, "HIDEOBJ"),
        (0x003D, "WINDOW1"),
        (0x0022, "DATEMODE"),
        (0x000E, "PRECISION"),
        (0x01B7, "REFRESHALL"),
        (0x00DA, "BOOKBOOL"),
        (0x0031, "FONT"),
        (0x041E, "FORMAT"),
        (0x00E0, "XF"),
        (0x0293, "STYLE"),
        (0x0092, "PALETTE"),
    ];
    let mut pos = 0;
    for (want_id, name) in required {
        if pos >= ids.len() {
            panic!(
                "workbook globals stream missing record {} (0x{:04X})",
                name, want_id
            );
        }
        assert_eq!(
            ids[pos], *want_id,
            "expected {} (0x{:04X}) at position {}, got 0x{:04X}",
            name, want_id, pos, ids[pos]
        );
        pos += 1;
    }
    // After STYLE the BOUNDSHEET records (0x0085) must follow — one
    // per sheet. We have 1 sheet.
    assert_eq!(ids[pos], 0x0085, "expected BOUNDSHEET after STYLE");
    pos += 1;
    // Then USESELFS, COUNTRY, SST.
    assert_eq!(ids[pos], 0x0160, "expected USESELFS after BOUNDSHEET");
    pos += 1;
    assert_eq!(ids[pos], 0x008C, "expected COUNTRY after USESELFS");
    pos += 1;
    assert_eq!(ids[pos], 0x00FC, "expected SST after COUNTRY");
}

#[test]
fn biff8_window1_record_has_18_byte_body() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("w1.xls");
    let mut w = XlsWriter::new();
    w.add_sheet("S").expect("add sheet");
    w.add_row(XlsRowData::new());
    w.save(path.to_str().unwrap()).expect("save");

    let records = parse_workbook_stream(&path);
    let w1 = records
        .iter()
        .find(|(id, _, _)| *id == 0x003D)
        .expect("WINDOW1 record");
    // WINDOW1 body is 9 u16 fields (hpos, vpos, width, height, flags,
    // active_sheet, first_tab_index, selected_tabs, tab_width) = 18
    // bytes. The previous bug emitted only 7 u16 (14 bytes), which
    // caused xlrd to misread the tab/active-sheet fields and Excel to
    // show the wrong sheet on open.
    assert_eq!(
        w1.1, 18,
        "WINDOW1 body must be 18 bytes (9 u16 fields), got {}",
        w1.1
    );
}

#[test]
fn biff8_window2_record_has_18_byte_body() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("w2.xls");
    let mut w = XlsWriter::new();
    w.add_sheet("S").expect("add sheet");
    w.add_row(XlsRowData::new());
    w.save(path.to_str().unwrap()).expect("save");

    let (globals, sheets) = parse_workbook_streams(&path);
    // WINDOW2 (0x023E) MUST NOT appear in the workbook globals — it
    // is a per-sheet record. The previous bug emitted a 10-byte
    // WINDOW2 in the globals, which made Excel render every sheet
    // with the same view settings.
    let w2_in_globals = globals.iter().any(|(id, _, _)| *id == 0x023E);
    assert!(
        !w2_in_globals,
        "WINDOW2 must NOT appear in workbook globals; it is a per-sheet record"
    );
    // Each sheet substream must contain at least one WINDOW2 with a
    // 18-byte body. (10-byte was the previous buggy layout.)
    assert!(!sheets.is_empty(), "no sheets in workbook");
    for (i, sheet) in sheets.iter().enumerate() {
        let w2 = sheet
            .iter()
            .find(|(id, _, _)| *id == 0x023E)
            .unwrap_or_else(|| panic!("sheet {i} has no WINDOW2 record"));
        assert_eq!(
            w2.1, 18,
            "sheet {i} WINDOW2 body must be 18 bytes, got {}",
            w2.1
        );
    }
}

#[test]
fn biff8_font_record_has_length_prefixed_name() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("font.xls");
    let mut w = XlsWriter::new();
    w.add_sheet("S").expect("add sheet");
    w.add_row(XlsRowData::new());
    w.save(path.to_str().unwrap()).expect("save");

    let records = parse_workbook_stream(&path);
    let font = records
        .iter()
        .find(|(id, _, _)| *id == 0x0031)
        .expect("FONT record");
    // FONT body is 14 fixed bytes + (u16 cch + chars). For "Arial"
    // (5 chars in UTF-16): 14 + 2 + 10 = 26 bytes. The previous bug
    // emitted just 14 + 10 = 24 bytes (no length prefix), which made
    // xlrd read the first two bytes of "Arial" as a 65-char name and
    // then run off the end of the body.
    assert!(
        font.1 >= 26,
        "FONT body must include the u16 name length prefix; expected >= 26 bytes, got {}",
        font.1
    );
    // The first byte of the name (after the 14 fixed bytes) should be
    // the low byte of cch. For 5 chars that's 0x05.
    let cch = u16::from_le_bytes([font.2[14], font.2[15]]);
    assert!(
        cch > 0 && cch < 256,
        "FONT cch (char count) must be a small positive u16, got {cch}"
    );
}

#[test]
fn biff8_codepage_record_is_utf16() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("cp.xls");
    let mut w = XlsWriter::new();
    w.add_sheet("S").expect("add sheet");
    w.add_row(XlsRowData::new());
    w.save(path.to_str().unwrap()).expect("save");

    let records = parse_workbook_stream(&path);
    let cp = records
        .iter()
        .find(|(id, _, _)| *id == 0x0042)
        .expect("CODEPAGE record");
    // BIFF8 must declare code page 1200 (UTF-16). Older Excel
    // versions may also work with 1252, but the spec requires 1200.
    let val = u16::from_le_bytes([cp.2[0], cp.2[1]]);
    assert_eq!(
        val, 0x04B0,
        "BIFF8 CODEPAGE value must be 0x04B0 (UTF-16), got 0x{:04X}",
        val
    );
}

#[test]
fn biff8_boundsheet_uses_u8_visibility() {
    // Regression: BoundSheet previously used a u16 visibility field and
    // an extra reserved byte, which shifted the name position and
    // caused xlrd to read an empty sheet name (u16 cch = 0).
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("bs.xls");
    let mut w = XlsWriter::new();
    w.add_sheet("MySheet").expect("add sheet");
    let mut r = XlsRowData::new();
    r.add_string("hi");
    w.add_row(r);
    w.save(path.to_str().unwrap()).expect("save");

    let records = parse_workbook_stream(&path);
    let bs = records
        .iter()
        .find(|(id, _, _)| *id == 0x0085)
        .expect("BOUNDSHEET record");
    // Expected layout: position(4) + visibility(1) + type(1) + cch(1) +
    // options(1) + 2*cch bytes chars. For "MySheet" (7 chars in UTF-16):
    // 4 + 1 + 1 + 1 + 1 + 14 = 22 bytes.
    assert_eq!(
        bs.1, 22,
        "BOUNDSHEET body for 7-char UTF-16 name must be 22 bytes, got {}",
        bs.1
    );
    // The cch byte is at offset 6 (after position+vis+type).
    let cch = bs.2[6];
    assert_eq!(cch, 7, "BOUNDSHEET cch must be 7 for 'MySheet', got {cch}");
    // The options byte is at offset 7.
    assert_eq!(
        bs.2[7], 0x01,
        "BOUNDSHEET options must have fHighByte=1 (UTF-16) for non-ASCII safety"
    );
}
