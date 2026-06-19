//! Batch capability

use crate::capabilities::{Capability, CapabilityMetadata};
use crate::converter::Converter;
use crate::operations::DataOperations;
use crate::traits::SortOperator;
use anyhow::{Context, Result};
use serde_json::{json, Value};

pub struct BatchCapability;

impl Capability for BatchCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "batch".to_string(),
            description: "Batch process multiple files with the same operation".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "inputs": { "type": "string", "description": "Comma-separated input file paths or glob pattern" },
                    "output_dir": { "type": "string", "description": "Output directory for results" },
                    "operation": { "type": "string", "description": "Operation: convert, sort, filter, dedupe, normalize, zscore, rolling" },
                    "args": { "type": "array", "items": { "type": "string" }, "description": "Operation-specific arguments" }
                },
                "required": ["inputs", "output_dir", "operation"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let inputs = args["inputs"].as_str().context("Missing inputs")?;
        let output_dir = args["output_dir"].as_str().context("Missing output_dir")?;
        let operation = args["operation"].as_str().context("Missing operation")?;
        let args_arr: Vec<String> = args["args"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        std::fs::create_dir_all(output_dir)
            .context(format!("Failed to create output directory {output_dir}"))?;

        let input_files: Vec<String> = if inputs.contains('*') {
            glob::glob(inputs)
                .context("Failed to parse glob pattern")?
                .filter_map(|entry| match entry {
                    Ok(path) if path.is_file() => Some(path.to_string_lossy().to_string()),
                    _ => None,
                })
                .collect()
        } else {
            inputs.split(',').map(|s| s.trim().to_string()).collect()
        };

        if input_files.is_empty() {
            anyhow::bail!("No input files found for: {inputs}");
        }

        let mut success_count = 0;
        let mut error_count = 0;
        let mut errors = Vec::new();

        for input_file in &input_files {
            let file_stem = std::path::Path::new(input_file)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            let output_file = format!("{}/{}.csv", output_dir, file_stem);

            let result = match operation {
                "convert" => {
                    if args_arr.is_empty() {
                        Err(anyhow::anyhow!("Convert requires output format argument"))
                    } else {
                        let ext = &args_arr[0];
                        let out = format!("{}/{}.{}", output_dir, file_stem, ext);
                        batch_convert(input_file, &out)
                    }
                }
                "sort" => {
                    if args_arr.is_empty() {
                        Err(anyhow::anyhow!("Sort requires column argument"))
                    } else {
                        batch_sort(input_file, &output_file, &args_arr[0], true)
                    }
                }
                "filter" => {
                    if args_arr.is_empty() {
                        Err(anyhow::anyhow!("Filter requires where clause argument"))
                    } else {
                        batch_filter(input_file, &output_file, &args_arr[0])
                    }
                }
                "dedupe" => batch_dedupe(input_file, &output_file),
                "normalize" => {
                    if args_arr.is_empty() {
                        Err(anyhow::anyhow!("Normalize requires column argument"))
                    } else {
                        batch_normalize(input_file, &output_file, &args_arr[0])
                    }
                }
                "zscore" => {
                    if args_arr.is_empty() {
                        Err(anyhow::anyhow!("Zscore requires column argument"))
                    } else {
                        batch_zscore(input_file, &output_file, &args_arr[0])
                    }
                }
                "rolling" => {
                    if args_arr.len() < 3 {
                        Err(anyhow::anyhow!("Rolling requires: column, window, agg"))
                    } else {
                        match args_arr[1].parse::<usize>() {
                            Ok(window) => {
                                batch_rolling(input_file, &output_file, &args_arr[0], window, &args_arr[2])
                            }
                            Err(_) => Err(anyhow::anyhow!("Rolling: window must be an integer")),
                        }
                    }
                }
                _ => Err(anyhow::anyhow!("Unknown batch operation: {operation}")),
            };

            match result {
                Ok(_) => success_count += 1,
                Err(e) => {
                    error_count += 1;
                    errors.push(format!("{}: {}", input_file, e));
                }
            }
        }

        Ok(json!({
            "status": if error_count == 0 { "success" } else { "partial" },
            "total": input_files.len(),
            "success": success_count,
            "errors": error_count,
            "error_details": errors,
        }))
    }
}

fn batch_convert(input_file: &str, output_file: &str) -> Result<()> {
    let converter = Converter::new();
    let data = converter.read_any_data(input_file, None)?;
    converter.write_any_data(output_file, &data, None)?;
    Ok(())
}

fn batch_sort(input_file: &str, output_file: &str, column: &str, ascending: bool) -> Result<()> {
    let converter = Converter::new();
    let ops = DataOperations::new();
    let mut data = converter.read_any_data(input_file, None)?;
    let col_idx = find_column_index(&data, column)?;
    ops.sort(&mut data, col_idx, ascending)?;
    converter.write_any_data(output_file, &data, None)?;
    Ok(())
}

fn batch_filter(input_file: &str, output_file: &str, where_clause: &str) -> Result<()> {
    let converter = Converter::new();
    let ops = DataOperations::new();
    let data = converter.read_any_data(input_file, None)?;
    let filtered = ops.query(&data, where_clause)?;
    converter.write_any_data(output_file, &filtered, None)?;
    Ok(())
}

fn batch_dedupe(input_file: &str, output_file: &str) -> Result<()> {
    let converter = Converter::new();
    let ops = DataOperations::new();
    let data = converter.read_any_data(input_file, None)?;
    let deduped = ops.deduplicate(&data);
    converter.write_any_data(output_file, &deduped, None)?;
    Ok(())
}

fn batch_normalize(input_file: &str, output_file: &str, column: &str) -> Result<()> {
    let converter = Converter::new();
    let ops = DataOperations::new();
    let mut data = converter.read_any_data(input_file, None)?;
    let col_idx = find_column_index(&data, column)?;
    ops.normalize(&mut data, col_idx)?;
    converter.write_any_data(output_file, &data, None)?;
    Ok(())
}

fn batch_zscore(input_file: &str, output_file: &str, column: &str) -> Result<()> {
    let converter = Converter::new();
    let ops = DataOperations::new();
    let mut data = converter.read_any_data(input_file, None)?;
    let col_idx = find_column_index(&data, column)?;
    ops.zscore(&mut data, col_idx)?;
    converter.write_any_data(output_file, &data, None)?;
    Ok(())
}

fn batch_rolling(
    input_file: &str,
    output_file: &str,
    column: &str,
    window: usize,
    agg: &str,
) -> Result<()> {
    let converter = Converter::new();
    let ops = DataOperations::new();
    let mut data = converter.read_any_data(input_file, None)?;
    let col_idx = find_column_index(&data, column)?;
    let agg_lower = agg.to_lowercase();
    match agg_lower.as_str() {
        "sum" => {
            let new_name = format!("{column}_roll{window}_sum");
            ops.rolling_sum_column(&mut data, col_idx, window, &new_name)?;
        }
        "mean" | "avg" => {
            let new_name = format!("{column}_roll{window}_mean");
            ops.rolling_mean_column(&mut data, col_idx, window, &new_name)?;
        }
        other => anyhow::bail!("rolling batch: unknown agg '{other}' (use mean or sum)"),
    }
    converter.write_any_data(output_file, &data, None)?;
    Ok(())
}

fn find_column_index(data: &[Vec<String>], column: &str) -> Result<usize> {
    if data.is_empty() {
        anyhow::bail!("Data is empty");
    }
    let header = &data[0];
    if let Ok(index) = column.parse::<usize>() {
        if index == 0 {
            anyhow::bail!("Column indices start from 1");
        }
        return Ok(index - 1);
    }
    header
        .iter()
        .position(|col_name| col_name == column)
        .ok_or_else(|| anyhow::anyhow!("Column '{}' not found", column))
}
