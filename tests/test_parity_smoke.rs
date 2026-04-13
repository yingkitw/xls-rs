//! Parity smoke tests between library and CLI.
//!
//! Full MCP parity needs an end-to-end harness; this file keeps a minimal
//! regression test so core behaviors don't diverge.

use std::process::Command;

#[test]
fn test_library_and_cli_can_read_csv() {
    // Arrange: create a temp CSV
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("sales.csv");
    std::fs::write(
        &input,
        "Product,Category,Price\nLaptop,Electronics,1200\nMouse,Electronics,25\n",
    )
    .unwrap();

    // Library read
    let converter = xls_rs::Converter::new();
    let data = converter
        .read_any_data(input.to_string_lossy().as_ref(), None)
        .unwrap();
    assert_eq!(data[0][0], "Product");

    // CLI read (use the compiled test binary path)
    let exe = env!("CARGO_BIN_EXE_xls-rs");
    let out = Command::new(exe)
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

