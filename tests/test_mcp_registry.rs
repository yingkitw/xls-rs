//! Minimal capability-registry tests (same entry points MCP tools use).

use std::sync::Arc;
use xls_rs::capabilities::{
    BatchCapability, CapabilityRegistry, ConvertCapability, EncryptCapability, FilterCapability,
    ProfileCapability, ReadExcelCapability, SortCapability, StreamCapability, ValidateCapability,
};
use xls_rs::{Converter, DataWriter};

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

#[test]
fn registry_read_excel_returns_data_rows_and_columns() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.xlsx");

    // Create a small Excel file via library
    let handler = xls_rs::ExcelHandler::new();
    let data = vec![
        vec!["Name".to_string(), "Score".to_string()],
        vec!["Alice".to_string(), "95".to_string()],
        vec!["Bob".to_string(), "87".to_string()],
    ];
    handler.write(input.to_string_lossy().as_ref(), &data, Default::default()).unwrap();

    let reg = CapabilityRegistry::new();
    reg.register(Arc::new(ReadExcelCapability));
    let args = serde_json::json!({
        "input": input.to_string_lossy(),
    });
    let r = reg.execute("read_excel", args).unwrap();
    assert_eq!(r["status"], "success");
    assert_eq!(r["rows"], 3);
    assert_eq!(r["columns"], 2);

    let returned = r["data"].as_array().unwrap();
    assert_eq!(returned[0][0], "Name");
    assert_eq!(returned[1][0], "Alice");
}

#[test]
fn registry_validate_returns_status_and_errors() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.csv");
    std::fs::write(&input, "A,B\n1,2\n3,4\n").unwrap();

    let reg = CapabilityRegistry::new();
    reg.register(Arc::new(ValidateCapability));
    let args = serde_json::json!({
        "input": input.to_string_lossy(),
        "rules": "auto",
    });
    let r = reg.execute("validate", args).unwrap();
    assert_eq!(r["status"], "passed");
    assert_eq!(r["total_rows"], 2);
}

#[test]
fn registry_profile_returns_quality_score() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.csv");
    std::fs::write(&input, "Name,Score\nAlice,95\nBob,87\n").unwrap();

    let reg = CapabilityRegistry::new();
    reg.register(Arc::new(ProfileCapability));
    let args = serde_json::json!({
        "input": input.to_string_lossy(),
    });
    let r = reg.execute("profile", args).unwrap();
    assert_eq!(r["status"], "success");
    assert_eq!(r["total_rows"], 2);
    assert_eq!(r["total_columns"], 2);
    assert!(r["data_quality_score"].as_f64().unwrap() > 0.0);
    let cols = r["columns"].as_array().unwrap();
    assert_eq!(cols.len(), 2);
}

#[test]
fn registry_stream_copies_csv_in_chunks() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.csv");
    let output = dir.path().join("out.csv");
    std::fs::write(&input, "A,B\n1,a\n2,b\n3,c\n").unwrap();

    let reg = CapabilityRegistry::new();
    reg.register(Arc::new(StreamCapability));
    let args = serde_json::json!({
        "input": input.to_string_lossy(),
        "output": output.to_string_lossy(),
        "chunk_size": 2,
    });
    let r = reg.execute("stream", args).unwrap();
    assert_eq!(r["status"], "success");
    assert_eq!(r["mode"], "streaming");
    assert_eq!(r["total_rows"], 4);

    let conv = Converter::new();
    let data = conv
        .read_any_data(output.to_string_lossy().as_ref(), None)
        .unwrap();
    assert_eq!(data[0], vec!["A", "B"]);
    assert_eq!(data[1], vec!["1", "a"]);
}

#[test]
fn registry_encrypts_file_with_xor() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.txt");
    let output = dir.path().join("out.enc");
    std::fs::write(&input, "hello world").unwrap();

    let reg = CapabilityRegistry::new();
    reg.register(Arc::new(EncryptCapability));
    let args = serde_json::json!({
        "input": input.to_string_lossy(),
        "output": output.to_string_lossy(),
        "algorithm": "xor",
        "key": "secret",
    });
    let r = reg.execute("encrypt", args).unwrap();
    assert_eq!(r["status"], "success");
    assert!(output.exists());
}

#[test]
fn registry_batch_converts_files() {
    let dir = tempfile::tempdir().unwrap();
    let input1 = dir.path().join("a.csv");
    let input2 = dir.path().join("b.csv");
    let output_dir = dir.path().join("out");
    std::fs::write(&input1, "x,y\n1,2\n").unwrap();
    std::fs::write(&input2, "x,y\n3,4\n").unwrap();

    let reg = CapabilityRegistry::new();
    reg.register(Arc::new(BatchCapability));
    let args = serde_json::json!({
        "inputs": format!("{}, {}", input1.to_string_lossy(), input2.to_string_lossy()),
        "output_dir": output_dir.to_string_lossy(),
        "operation": "convert",
        "args": ["csv"],
    });
    let r = reg.execute("batch", args).unwrap();
    assert_eq!(r["status"], "success");
    assert_eq!(r["success"], 2);
    assert_eq!(r["errors"], 0);
}
