//! Batch processing command handlers

use xls_rs::{converter::Converter, operations::DataOperations, traits::SortOperator};
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

/// Handle the batch command
///
/// Processes multiple files with the same operation.
pub fn handle_batch(
    inputs: String,
    output_dir: String,
    operation: String,
    args: Vec<String>,
) -> Result<()> {
    // Ensure output directory exists
    std::fs::create_dir_all(&output_dir)
        .context(format!("Failed to create output directory {output_dir}"))?;

    // Parse input files
    let input_files: Vec<String> = if inputs.contains('*') {
        glob::glob(&inputs)
            .context("Failed to parse glob pattern")?
            .filter_map(|entry| match entry {
                Ok(path) => Some(path),
                Err(e) => {
                    eprintln!("Warning: glob error for pattern '{}': {}", inputs, e);
                    None
                }
            })
            .filter(|entry| entry.is_file())
            .map(|entry| entry.to_string_lossy().to_string())
            .collect()
    } else {
        inputs.split(',').map(|s| s.trim().to_string()).collect()
    };

    if input_files.is_empty() {
        anyhow::bail!("No input files found for pattern: {inputs}");
    }

    println!(
        "Processing {} files with operation '{operation}' in parallel...",
        input_files.len()
    );

    let success_count = Arc::new(Mutex::new(0));
    let error_count = Arc::new(Mutex::new(0));

    // Process files in parallel using rayon
    input_files.par_iter().for_each(|input_file| {
        // Generate output filename
        let file_stem = std::path::Path::new(input_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let output_file = format!("{}/{}.csv", output_dir, file_stem);

        // Execute operation based on type
        let result = match operation.as_str() {
            "convert" => {
                if args.is_empty() {
                    Err(anyhow::anyhow!(
                        "Convert operation requires output format argument"
                    ))
                } else {
                    let format = &args[0];
                    let output_with_ext = format!("{}/{}.{}", output_dir, file_stem, format);
                    batch_convert(input_file, &output_with_ext)
                }
            }
            "sort" => {
                if args.is_empty() {
                    Err(anyhow::anyhow!("Sort operation requires column argument"))
                } else {
                    batch_sort(input_file, &output_file, &args[0], true)
                }
            }
            "filter" => {
                if args.is_empty() {
                    Err(anyhow::anyhow!(
                        "Filter operation requires where clause argument"
                    ))
                } else {
                    batch_filter(input_file, &output_file, &args[0])
                }
            }
            "dedupe" => batch_dedupe(input_file, &output_file),
            "normalize" => {
                if args.is_empty() {
                    Err(anyhow::anyhow!(
                        "Normalize operation requires column argument"
                    ))
                } else {
                    batch_normalize(input_file, &output_file, &args[0])
                }
            }
            "zscore" => {
                if args.is_empty() {
                    Err(anyhow::anyhow!(
                        "Zscore operation requires column argument"
                    ))
                } else {
                    batch_zscore(input_file, &output_file, &args[0])
                }
            }
            "rolling" => {
                if args.len() < 3 {
                    Err(anyhow::anyhow!(
                        "Rolling operation requires: column, window (usize), agg (mean|sum)"
                    ))
                } else {
                    match args[1].parse::<usize>() {
                        Ok(window) => {
                            batch_rolling(input_file, &output_file, &args[0], window, &args[2])
                        }
                        Err(_) => Err(anyhow::anyhow!(
                            "Rolling: window must be a positive integer"
                        )),
                    }
                }
            }
            _ => Err(anyhow::anyhow!("Unknown batch operation: {}", operation)),
        };

        match result {
            Ok(_) => {
                println!("  ✓ {}", input_file);
                *success_count.lock().unwrap() += 1;
            }
            Err(e) => {
                println!("  ✗ {input_file}: {e}");
                *error_count.lock().unwrap() += 1;
            }
        }
    });

    let final_success = *success_count.lock().unwrap();
    let final_errors = *error_count.lock().unwrap();

    println!(
        "Batch processing complete: {} successful, {} failed",
        final_success, final_errors
    );

    if final_errors > 0 {
        anyhow::bail!("Some batch operations failed");
    }

    Ok(())
}

/// Batch convert operation
fn batch_convert(input_file: &str, output_file: &str) -> Result<()> {
    let converter = Converter::new();
    let data = converter.read_any_data(input_file, None)?;
    converter.write_any_data(output_file, &data, None)?;
    Ok(())
}

/// Batch sort operation
fn batch_sort(input_file: &str, output_file: &str, column: &str, ascending: bool) -> Result<()> {
    let converter = Converter::new();
    let ops = DataOperations::new();

    let mut data = converter.read_any_data(input_file, None)?;
    let col_idx = find_column_index(&data, column)?;
    ops.sort(&mut data, col_idx, ascending)?;
    converter.write_any_data(output_file, &data, None)?;
    Ok(())
}

/// Batch filter operation
fn batch_filter(input_file: &str, output_file: &str, where_clause: &str) -> Result<()> {
    let converter = Converter::new();
    let ops = DataOperations::new();

    let data = converter.read_any_data(input_file, None)?;
    let filtered = ops.query(&data, where_clause)?;
    converter.write_any_data(output_file, &filtered, None)?;
    Ok(())
}

/// Batch dedupe operation
fn batch_dedupe(input_file: &str, output_file: &str) -> Result<()> {
    let converter = Converter::new();
    let ops = DataOperations::new();

    let data = converter.read_any_data(input_file, None)?;
    let deduped = ops.deduplicate(&data);
    converter.write_any_data(output_file, &deduped, None)?;
    Ok(())
}

/// Batch normalize operation
fn batch_normalize(input_file: &str, output_file: &str, column: &str) -> Result<()> {
    let converter = Converter::new();
    let ops = DataOperations::new();

    let mut data = converter.read_any_data(input_file, None)?;
    let col_idx = find_column_index(&data, column)?;
    ops.normalize(&mut data, col_idx)?;
    converter.write_any_data(output_file, &data, None)?;
    Ok(())
}

/// Batch rolling mean or sum
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

/// Batch z-score standardization
fn batch_zscore(input_file: &str, output_file: &str, column: &str) -> Result<()> {
    let converter = Converter::new();
    let ops = DataOperations::new();

    let mut data = converter.read_any_data(input_file, None)?;
    let col_idx = find_column_index(&data, column)?;
    ops.zscore(&mut data, col_idx)?;
    converter.write_any_data(output_file, &data, None)?;
    Ok(())
}

/// Find column index by name or number
fn find_column_index(data: &[Vec<String>], column: &str) -> Result<usize> {
    if data.is_empty() {
        anyhow::bail!("Data is empty");
    }

    let header = &data[0];

    // Try to parse as number first
    if let Ok(index) = column.parse::<usize>() {
        if index == 0 {
            anyhow::bail!("Column indices start from 1");
        }
        return Ok(index - 1);
    }

    // Try to find by name
    header
        .iter()
        .position(|col_name| col_name == column)
        .ok_or_else(|| anyhow::anyhow!("Column '{}' not found", column))
}
