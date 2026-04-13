//! Minimal capability-registry tests (same entry points MCP tools use).

use std::sync::Arc;
use xls_rs::capabilities::{
    CapabilityRegistry, ConvertCapability, FilterCapability, SortCapability,
};
use xls_rs::Converter;

#[test]
fn registry_sort_writes_sorted_csv() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.csv");
    let output = dir.path().join("out.csv");
    std::fs::write(&input, "A,B\n2,b\n1,a\n").unwrap();

    let reg = CapabilityRegistry::new();
    reg.register(Arc::new(SortCapability));
    let args = serde_json::json!({
        "input": input.to_string_lossy(),
        "output": output.to_string_lossy(),
        "column": "A",
        "ascending": true
    });
    let r = reg.execute("sort", args).unwrap();
    assert_eq!(r["status"], "success");

    let conv = Converter::new();
    let data = conv
        .read_any_data(output.to_string_lossy().as_ref(), None)
        .unwrap();
    assert_eq!(data[1][0], "1");
    assert_eq!(data[2][0], "2");
}

#[test]
fn registry_filter_respects_column_condition() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.csv");
    let output = dir.path().join("out.csv");
    std::fs::write(&input, "Name,Score\na,1\nb,2\nc,1\n").unwrap();

    let reg = CapabilityRegistry::new();
    reg.register(Arc::new(FilterCapability));
    let args = serde_json::json!({
        "input": input.to_string_lossy(),
        "output": output.to_string_lossy(),
        "column": "Score",
        "operator": "=",
        "value": "1"
    });
    let r = reg.execute("filter", args).unwrap();
    assert_eq!(r["status"], "success");

    let conv = Converter::new();
    let data = conv
        .read_any_data(output.to_string_lossy().as_ref(), None)
        .unwrap();
    assert_eq!(data.len(), 3);
}

#[test]
fn registry_convert_invokes_converter() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.csv");
    let output = dir.path().join("b.csv");
    std::fs::write(&input, "x,y\n1,2\n").unwrap();

    let reg = CapabilityRegistry::new();
    reg.register(Arc::new(ConvertCapability));
    let args = serde_json::json!({
        "input": input.to_string_lossy(),
        "output": output.to_string_lossy(),
    });
    let r = reg.execute("convert", args).unwrap();
    assert_eq!(r["status"], "success");

    let data = Converter::new()
        .read_any_data(output.to_string_lossy().as_ref(), None)
        .unwrap();
    assert_eq!(data[0], vec!["x", "y"]);
    assert_eq!(data[1], vec!["1", "2"]);
}
