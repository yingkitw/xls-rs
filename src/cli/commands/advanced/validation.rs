//! Data validation command handler

use xls_rs::{converter::Converter, validation::DataValidator};
use anyhow::{Context, Result};

/// Handle the validate command
///
/// Validates data against a set of rules.
pub fn handle_validate(
    input: String,
    rules: String,
    output: Option<String>,
    report: Option<String>,
) -> Result<()> {
    let converter = Converter::new();
    let data = converter.read_any_data(&input, None)?;

    // Load validation rules
    let validator = if rules.ends_with(".json") {
        DataValidator::from_config_file(&rules)?
    } else {
        // Create default rules if no file provided
        let config = xls_rs::validation::create_sample_config();
        DataValidator::new(config)
    };

    // Validate data
    let result = validator.validate(&data)?;

    // Output results
    if let Some(output_path) = output {
        validator.save_result(&result, &output_path)?;
        println!("Validation results saved to {}", output_path);
    }

    if let Some(report_path) = report {
        let report = validator.generate_report(&result);
        std::fs::write(&report_path, report)
            .context(format!("Failed to write report to {report_path}"))?;
        println!("Validation report saved to {}", report_path);
    }

    // Print summary
    println!("Validation Summary:");
    println!("  Total rows: {}", result.stats.total_rows);
    println!("  Valid rows: {}", result.stats.valid_rows);
    println!("  Invalid rows: {}", result.stats.invalid_rows);
    println!("  Errors: {}", result.errors.len());

    Ok(())
}
