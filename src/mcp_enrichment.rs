//! Rich JSON-RPC `error.data` for MCP: merge request context with parsed error text.

use regex::Regex;
use serde_json::{json, Value};
use std::sync::LazyLock;

/// Optional fields merged into MCP `error.data` (beyond `kind` + `detail`).
#[derive(Debug, Default, Clone)]
pub struct McpErrorContext {
    pub file: Option<String>,
    pub input: Option<String>,
    pub output: Option<String>,
    pub sheet: Option<String>,
    pub range: Option<String>,
    pub cell: Option<String>,
}

static RE_IN_FILE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"in file '([^']+)'").expect("regex"));
static RE_SHEET_QUOTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"sheet '([^']+)'").expect("regex"));
static RE_INVALID_CELL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Invalid cell reference '([^']+)'").expect("regex"));
static RE_SHEET_NOT_FOUND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Sheet '([^']+)' not found").expect("regex"));

/// Pull hints from common anyhow / IO message shapes (best-effort).
fn parse_from_message(s: &str) -> McpErrorContext {
    let mut ctx = McpErrorContext::default();
    if let Some(c) = RE_IN_FILE.captures(s) {
        ctx.file = Some(c[1].to_string());
    }
    if ctx.file.is_none() {
        if let Some(c) = Regex::new(r"File not found: ([^\n]+)")
            .ok()
            .and_then(|r| r.captures(s))
        {
            ctx.file = Some(c[1].trim().to_string());
        }
    }
    if let Some(c) = RE_SHEET_NOT_FOUND.captures(s) {
        ctx.sheet = Some(c[1].to_string());
    } else if let Some(c) = RE_SHEET_QUOTED.captures(s) {
        ctx.sheet = Some(c[1].to_string());
    }
    if let Some(c) = RE_INVALID_CELL.captures(s) {
        ctx.range = Some(c[1].to_string());
    }
    ctx
}

/// Build `error.data` object: `kind`, `detail`, `code`, and optional `file` / `sheet` / `range` / I/O paths.
pub fn mcp_error_data(detail: &str, mut ctx: McpErrorContext, code: Option<&str>) -> Value {
    let parsed = parse_from_message(detail);
    if ctx.file.is_none() {
        ctx.file = parsed.file;
    }
    if ctx.sheet.is_none() {
        ctx.sheet = parsed.sheet;
    }
    if ctx.range.is_none() {
        ctx.range = parsed.range;
    }

    let mut o = serde_json::Map::new();
    o.insert("kind".to_string(), json!("xls_rs_error"));
    o.insert("detail".to_string(), json!(detail));
    if let Some(c) = code {
        o.insert("code".to_string(), json!(c));
    }
    if let Some(v) = ctx.file {
        o.insert("file".to_string(), json!(v));
    }
    if let Some(v) = ctx.input {
        o.insert("input".to_string(), json!(v));
    }
    if let Some(v) = ctx.output {
        o.insert("output".to_string(), json!(v));
    }
    if let Some(v) = ctx.sheet {
        o.insert("sheet".to_string(), json!(v));
    }
    if let Some(v) = ctx.range {
        o.insert("range".to_string(), json!(v));
    }
    if let Some(v) = ctx.cell {
        o.insert("cell".to_string(), json!(v));
    }
    Value::Object(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_in_file() {
        let c = parse_from_message("boom in file '/tmp/x.xlsx' and more");
        assert_eq!(c.file.as_deref(), Some("/tmp/x.xlsx"));
    }

    #[test]
    fn parse_invalid_cell() {
        let c = parse_from_message("Invalid cell reference 'A1:B2'");
        assert_eq!(c.range.as_deref(), Some("A1:B2"));
    }

    #[test]
    fn mcp_error_data_includes_code() {
        let data = mcp_error_data("detail", McpErrorContext::default(), Some("file_not_found"));
        assert_eq!(data.get("code").and_then(|v| v.as_str()), Some("file_not_found"));
        assert_eq!(data.get("kind").and_then(|v| v.as_str()), Some("xls_rs_error"));
    }

    #[test]
    fn mcp_error_data_omits_code_when_none() {
        let data = mcp_error_data("detail", McpErrorContext::default(), None);
        assert!(data.get("code").is_none());
    }
}
