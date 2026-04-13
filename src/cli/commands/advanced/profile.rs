//! Data profiling command handler

use xls_rs::{converter::Converter, profiling::DataProfiler};
use anyhow::{Context, Result};

/// Handle the profile command
///
/// Generates a data profile report.
pub fn handle_profile(input: String, output: Option<String>) -> Result<()> {
    let converter = Converter::new();
    let data = converter.read_any_data(&input, None)?;

    let profiler = DataProfiler::new();
    let profile = profiler.profile(&data, &input)?;

    let report = serde_json::to_string_pretty(&profile)?;

    if let Some(output_path) = output {
        std::fs::write(&output_path, report)
            .context(format!("Failed to write profile to {output_path}"))?;
        println!("Profile saved to {}", output_path);
    } else {
        println!("{}", report);
    }

    Ok(())
}
