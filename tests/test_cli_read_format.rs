//! CLI `read` / `read-all` output format resolution (config + flags).

use std::process::Command;

#[test]
fn read_without_format_uses_csv_when_no_config_default() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("t.csv");
    std::fs::write(&input, "A,B\n1,2\n").unwrap();

    let exe = env!("CARGO_BIN_EXE_xls-rs");
    let out = Command::new(exe)
        .args([
            "--quiet",
            "read",
            "--input",
            input.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("A"));
    assert!(stdout.contains("1"));
}

#[test]
fn read_respects_default_format_from_config() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("t.csv");
    std::fs::write(&input, "A,B\n1,2\n").unwrap();
    let cfg = dir.path().join("cfg.toml");
    std::fs::write(&cfg, "default_format = \"json\"\n").unwrap();

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
    let trimmed = stdout.trim();
    assert!(
        trimmed.starts_with('['),
        "expected JSON array, got: {trimmed:?}"
    );
}

#[test]
fn read_explicit_format_overrides_config() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("t.csv");
    std::fs::write(&input, "A,B\n1,2\n").unwrap();
    let cfg = dir.path().join("cfg.toml");
    std::fs::write(&cfg, "default_format = \"json\"\n").unwrap();

    let exe = env!("CARGO_BIN_EXE_xls-rs");
    let out = Command::new(exe)
        .args([
            "--quiet",
            "--config",
            cfg.to_str().unwrap(),
            "read",
            "--input",
            input.to_str().unwrap(),
            "--format",
            "csv",
        ])
        .output()
        .unwrap();

    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.trim_start().starts_with('['));
    assert!(stdout.contains("A"));
}
