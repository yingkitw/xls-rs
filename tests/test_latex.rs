//! CLI LaTeX table export tests.

use std::process::Command;

/// Test that `read --format latex` produces a LaTeX tabular environment.
#[test]
fn read_latex_format_basic() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("t.csv");
    std::fs::write(&input, "Name,Score,Grade\nAlice,95,A\nBob,78,B\n").unwrap();

    let exe = env!("CARGO_BIN_EXE_xls-rs");
    let out = Command::new(exe)
        .args([
            "--quiet",
            "read",
            "--input",
            input.to_str().unwrap(),
            "--format",
            "latex",
        ])
        .output()
        .unwrap();

    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(r"\begin{tabular}"), "missing begin tabular: {stdout:?}");
    assert!(stdout.contains(r"\end{tabular}"), "missing end tabular: {stdout:?}");
    assert!(stdout.contains(r"\hline"), "missing hline: {stdout:?}");
    assert!(stdout.contains("Name & Score & Grade"), "missing header row: {stdout:?}");
    assert!(stdout.contains("Alice & 95 & A"), "missing data row: {stdout:?}");
    assert!(stdout.contains("Bob & 78 & B"), "missing data row: {stdout:?}");
}

/// Test LaTeX format with empty data.
#[test]
fn read_latex_format_empty() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("t.csv");
    std::fs::write(&input, "").unwrap();

    let exe = env!("CARGO_BIN_EXE_xls-rs");
    let out = Command::new(exe)
        .args([
            "--quiet",
            "read",
            "--input",
            input.to_str().unwrap(),
            "--format",
            "latex",
        ])
        .output()
        .unwrap();

    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(r"\begin{tabular}"), "expected LaTeX output: {stdout:?}");
    assert!(stdout.contains(r"\end{tabular}"), "expected LaTeX output: {stdout:?}");
}

/// Test LaTeX format with a single column.
#[test]
fn read_latex_format_single_column() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("t.csv");
    std::fs::write(&input, "Value\n42\n99\n").unwrap();

    let exe = env!("CARGO_BIN_EXE_xls-rs");
    let out = Command::new(exe)
        .args([
            "--quiet",
            "read",
            "--input",
            input.to_str().unwrap(),
            "--format",
            "latex",
        ])
        .output()
        .unwrap();

    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(r"\begin{tabular}{l}"), "expected single-column spec: {stdout:?}");
    assert!(stdout.contains("Value \\"));
    assert!(stdout.contains("42 \\"));
    assert!(stdout.contains("99 \\"));
}

/// Test LaTeX format via config default_format.
#[test]
fn read_latex_format_from_config() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("t.csv");
    std::fs::write(&input, "A,B\n1,2\n").unwrap();
    let cfg = dir.path().join("cfg.toml");
    std::fs::write(&cfg, "default_format = \"latex\"\n").unwrap();

    let exe = env!("CARGO_BIN_EXE_xls-rs");
    let out = Command::new(exe)
        .args([
            "--quiet",
            "--config",
            cfg.to_str().unwrap(),
            "read",
            "--input",
            input.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(r"\begin{tabular}"), "expected LaTeX output: {stdout:?}");
}
