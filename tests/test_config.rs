//! Tests for configuration module

use xls_rs::Config;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_path(prefix: &str) -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("test_{prefix}_{id}.toml")
}

#[test]
fn test_config_default() {
    let config = Config::default();

    assert!(config.default_format.is_none());
    assert!(config.date_format.is_none());
    assert!(config.output_dir.is_none());
}

#[test]
fn test_config_save_and_load() {
    let path = unique_path("config_save");

    let mut config = Config::default();
    config.default_format = Some("json".to_string());
    config.date_format = Some("%Y-%m-%d".to_string());

    config.save(&path).unwrap();

    assert!(std::path::Path::new(&path).exists());

    let loaded = Config::load_from(&path).unwrap();

    assert_eq!(loaded.default_format, Some("json".to_string()));
    assert_eq!(loaded.date_format, Some("%Y-%m-%d".to_string()));

    fs::remove_file(&path).ok();
}

#[test]
fn test_config_excel_options() {
    let path = unique_path("config_excel");

    let config_content = r#"
[excel]
header_bold = true
header_bg_color = "FF0000"
header_font_color = "FFFFFF"
auto_filter = true
freeze_header = true
auto_fit = false
"#;

    fs::write(&path, config_content).unwrap();

    let config = Config::load_from(&path).unwrap();

    assert_eq!(config.excel.header_bold, Some(true));
    assert_eq!(config.excel.header_bg_color, Some("FF0000".to_string()));
    assert_eq!(config.excel.header_font_color, Some("FFFFFF".to_string()));
    assert_eq!(config.excel.auto_filter, Some(true));
    assert_eq!(config.excel.freeze_header, Some(true));
    assert_eq!(config.excel.auto_fit, Some(false));

    fs::remove_file(&path).ok();
}

#[test]
fn test_config_csv_options() {
    let path = unique_path("config_csv");

    let config_content = r#"
[csv]
delimiter = ";"
quote = "'"
has_header = false
"#;

    fs::write(&path, config_content).unwrap();

    let config = Config::load_from(&path).unwrap();

    assert_eq!(config.csv.delimiter, Some(";".to_string()));
    assert_eq!(config.csv.quote, Some("'".to_string()));
    assert_eq!(config.csv.has_header, Some(false));

    fs::remove_file(&path).ok();
}

#[test]
fn test_config_default_content() {
    let content = Config::default_config_content();

    assert!(content.contains("default_format"));
    assert!(content.contains("date_format"));
    assert!(content.contains("[excel]"));
    assert!(content.contains("[csv]"));
    assert!(content.contains("header_bold"));
    assert!(content.contains("delimiter"));
}

#[test]
fn test_config_load_nonexistent() {
    // Should return default config when file doesn't exist
    let config = Config::load();
    assert!(config.is_ok());
}

#[test]
fn test_config_full_example() {
    let path = unique_path("config_full");

    let config_content = r#"
default_format = "markdown"
date_format = "%d/%m/%Y"
output_dir = "output"

[excel]
header_bold = true
header_bg_color = "4472C4"
auto_filter = true

[csv]
delimiter = ","
has_header = true
"#;

    fs::write(&path, config_content).unwrap();

    let config = Config::load_from(&path).unwrap();

    assert_eq!(config.default_format, Some("markdown".to_string()));
    assert_eq!(config.date_format, Some("%d/%m/%Y".to_string()));
    assert_eq!(config.output_dir, Some("output".to_string()));
    assert_eq!(config.excel.header_bold, Some(true));
    assert_eq!(config.csv.has_header, Some(true));

    fs::remove_file(&path).ok();
}
