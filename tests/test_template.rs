//! End-to-end tests for template-based XLSX generation.
//!
//! Verifies that a template with `{{placeholder}}` cells can be read, filled,
//! and written back while preserving the non-placeholder cells.

use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

use xls_rs::{ExcelHandler, RowData, TemplateFiller, TemplateReader, XlsxWriter};

/// Returns a path inside `dir` for a test artifact. Per-test `TempDir`
/// isolation makes name collisions impossible, so no global counter is
/// needed — and the dir is cleaned up automatically on drop (no manual
/// `remove_file`, no artifacts leaking into the repo root on panic).
fn unique_path(dir: &TempDir, name: &str, ext: &str) -> String {
    dir.path()
        .join(format!("{name}.{ext}"))
        .to_string_lossy()
        .to_string()
}

/// Build a simple template XLSX with placeholders on disk and return its path.
fn build_template(path: &str) {
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Invoice").unwrap();

    let mut header = RowData::new();
    header.add_string("Invoice #{{invoice_id}}");
    writer.add_row(header);

    let mut customer = RowData::new();
    customer.add_string("Customer:");
    customer.add_string("{{customer_name}}");
    writer.add_row(customer);

    let mut amount = RowData::new();
    amount.add_string("Amount:");
    amount.add_string("{{total}}");
    writer.add_row(amount);

    let mut note = RowData::new();
    note.add_string("Note: {{customer_name}} owes {{total}}");
    writer.add_row(note);

    let file = fs::File::create(path).unwrap();
    let buffered = std::io::BufWriter::new(file);
    writer.save(buffered).unwrap();
}

#[test]
fn test_template_reader_detects_placeholders() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = unique_path(&dir, "read", "xlsx");
    build_template(&tpl);

    let reader = TemplateReader::new().unwrap();
    let data = reader.read_template(&tpl, None).unwrap();

    let names = data.placeholder_names();
    assert!(names.contains(&"invoice_id".to_string()));
    assert!(names.contains(&"customer_name".to_string()));
    assert!(names.contains(&"total".to_string()));
}

#[test]
fn test_template_filler_replaces_single_placeholder() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = unique_path(&dir, "fill1", "xlsx");
    let out = unique_path(&dir, "out1", "xlsx");
    build_template(&tpl);

    let mut values = HashMap::new();
    values.insert("invoice_id".to_string(), "INV-42".to_string());

    TemplateFiller::fill_from_file(&tpl, &out, &values, None).unwrap();

    // Verify by reading back
    let handler = ExcelHandler::new();
    let content = handler.read(&out).unwrap();
    assert!(
        content.contains("Invoice #INV-42"),
        "content was: {content}"
    );
    // Unfilled placeholders are preserved as-is
    assert!(content.contains("{{customer_name}}"));
}

#[test]
fn test_template_filler_replaces_all_placeholders() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = unique_path(&dir, "fill2", "xlsx");
    let out = unique_path(&dir, "out2", "xlsx");
    build_template(&tpl);

    let mut values = HashMap::new();
    values.insert("invoice_id".to_string(), "INV-100".to_string());
    values.insert("customer_name".to_string(), "Acme Corp".to_string());
    values.insert("total".to_string(), "$1,250.00".to_string());

    TemplateFiller::fill_from_file(&tpl, &out, &values, None).unwrap();

    let handler = ExcelHandler::new();
    let content = handler.read(&out).unwrap();
    assert!(content.contains("Invoice #INV-100"));
    assert!(content.contains("Acme Corp"));
    assert!(content.contains("$1,250.00"));
    assert!(
        !content.contains("{{"),
        "unfilled placeholder remains: {content}"
    );
}

#[test]
fn test_template_filler_in_string_interpolation() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = unique_path(&dir, "interp", "xlsx");
    let out = unique_path(&dir, "interp_out", "xlsx");
    build_template(&tpl);

    let mut values = HashMap::new();
    values.insert("customer_name".to_string(), "Globex".to_string());
    values.insert("total".to_string(), "99.99".to_string());

    TemplateFiller::fill_from_file(&tpl, &out, &values, None).unwrap();

    let handler = ExcelHandler::new();
    let content = handler.read(&out).unwrap();
    // The note row interpolates multiple placeholders
    assert!(content.contains("Note: Globex owes 99.99"));
}

#[test]
fn test_template_get_required_placeholders() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = unique_path(&dir, "req", "xlsx");
    build_template(&tpl);

    let required = TemplateFiller::get_required_placeholders(&tpl, None).unwrap();
    assert_eq!(required.len(), 3);
    assert!(required.contains(&"invoice_id".to_string()));
    assert!(required.contains(&"customer_name".to_string()));
    assert!(required.contains(&"total".to_string()));
}

#[test]
fn test_template_validate_missing_placeholders() {
    use xls_rs::PlaceholderInfo;
    use xls_rs::TemplateData;

    let mut data = TemplateData::new("Sheet1".to_string());
    data.placeholders.push(PlaceholderInfo {
        cell_ref: "A1".to_string(),
        row: 0,
        col: 0,
        name: "name".to_string(),
        full_value: "{{name}}".to_string(),
    });

    let empty_values = HashMap::new();
    let result = TemplateFiller::validate_placeholders(&data, &empty_values);
    assert!(result.is_err());

    let mut full_values = HashMap::new();
    full_values.insert("name".to_string(), "Alice".to_string());
    let result = TemplateFiller::validate_placeholders(&data, &full_values);
    assert!(result.is_ok());
}
