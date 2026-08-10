//! Parity smoke tests between library and CLI.
//!
//! Full MCP parity needs an end-to-end harness; this file keeps a minimal
//! regression test so core behaviors don't diverge.

use std::process::Command;
use xls_rs::DataWriter;

fn xls_rs_exe() -> &'static str {
    env!("CARGO_BIN_EXE_xls-rs")
}

#[test]
fn test_library_and_cli_can_read_xlsx() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("sales.xlsx");

    // Create XLSX via library
    let data = vec![
        vec!["Product".to_string(), "Category".to_string(), "Price".to_string()],
        vec!["Laptop".to_string(), "Electronics".to_string(), "1200".to_string()],
        vec!["Mouse".to_string(), "Electronics".to_string(), "25".to_string()],
    ];
    let converter = xls_rs::Converter::new();
    converter
        .write_any_data(input.to_string_lossy().as_ref(), &data, None)
        .unwrap();

    // Library read
    let read_data = converter
        .read_any_data(input.to_string_lossy().as_ref(), None)
        .unwrap();
    assert_eq!(read_data[0][0], "Product");

    // CLI read
    let out = Command::new(xls_rs_exe())
        .args([
            "--quiet",
            "read",
            "--input",
            input.to_string_lossy().as_ref(),
            "--format",
            "csv",
        ])
        .output()
        .unwrap();
    if !out.status.success() {
        panic!(
            "CLI failed.\nstatus: {}\nstderr:\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Product"));
    assert!(stdout.contains("Laptop"));
}

#[test]
fn test_cli_write_range_mode_preserve() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("output.xlsx");

    // Create baseline XLSX via library
    let handler = xls_rs::ExcelHandler::new();
    let baseline = vec![
        vec!["A".to_string(), "B".to_string()],
        vec!["1".to_string(), "2".to_string()],
    ];
    handler
        .write(output.to_string_lossy().as_ref(), &baseline, Default::default())
        .unwrap();

    // Create patch XLSX
    let patch_path = dir.path().join("patch.xlsx");
    let patch_data = vec![
        vec!["X".to_string()],
        vec!["99".to_string()],
    ];
    let converter = xls_rs::Converter::new();
    converter
        .write_any_data(patch_path.to_string_lossy().as_ref(), &patch_data, None)
        .unwrap();

    // CLI write-range --mode preserve at B2 (row 1, col 1)
    let out = Command::new(xls_rs_exe())
        .args([
            "--quiet",
            "--overwrite",
            "write-range",
            "--input",
            patch_path.to_string_lossy().as_ref(),
            "--output",
            output.to_string_lossy().as_ref(),
            "--start",
            "B2",
            "--mode",
            "preserve",
        ])
        .output()
        .unwrap();
    if !out.status.success() {
        panic!(
            "CLI write-range failed.\nstatus: {}\nstderr:\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // Verify the file still exists and can be read back
    let data = xls_rs::Converter::new()
        .read_any_data(output.to_string_lossy().as_ref(), None)
        .unwrap();
    assert!(!data.is_empty());
}

