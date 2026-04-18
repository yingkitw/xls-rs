//! Utility command handlers (completions, config, styled export)

use xls_rs::{
    config::Config,
    converter::Converter,
    excel::{
        chart::{ChartConfig, DataChartType},
        types::CellStyle,
        ExcelHandler, WriteOptions,
    },
};
use anyhow::{Context, Result};
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

/// Handle the completions command
///
/// Generates shell completion scripts.
pub fn handle_completions(shell: String) -> Result<()> {
    let mut cmd = crate::cli::Cli::command();

    let shell_type = match shell.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" => Shell::PowerShell,
        "elvish" => Shell::Elvish,
        _ => anyhow::bail!("Unsupported shell: {}", shell),
    };

    generate(shell_type, &mut cmd, "xls-rs", &mut io::stdout());

    Ok(())
}

/// Handle the config_init command
///
/// Creates a default configuration file.
pub fn handle_config_init() -> Result<()> {
    let path = crate::cli::runtime::get()
        .config_path
        .as_deref()
        .and_then(|p| p.to_str())
        .unwrap_or(".xls-rs.toml");
    if std::path::Path::new(path).exists() {
        anyhow::bail!("{} already exists", path);
    }

    std::fs::write(path, Config::default_config_content())
        .context(format!("Failed to write {}", path))?;
    crate::cli::runtime::log(format!("Wrote {}", path));

    Ok(())
}

/// Handle the export_styled command
///
/// Exports data to a styled Excel file.
pub fn handle_export_styled(input: String, output: String, style: Option<String>) -> Result<()> {
    let output_lower = output.to_lowercase();
    if !output_lower.ends_with(".xlsx") {
        anyhow::bail!("ExportStyled requires .xlsx output");
    }

    let converter = Converter::new();
    let data = converter.read_any_data(&input, None)?;

    let options = if let Some(ref name) = style {
        write_options_for_preset(name)?
    } else {
        WriteOptions::default()
    };

    let handler = ExcelHandler::new();
    handler.write_styled(&output, &data, &options)?;

    println!("Exported styled Excel file: {}", output);

    Ok(())
}

fn write_options_for_preset(name: &str) -> Result<WriteOptions> {
    let key = name.trim().to_lowercase();
    match key.as_str() {
        "default" => Ok(WriteOptions::default()),
        "minimal" => Ok(WriteOptions {
            sheet_name: None,
            style_header: false,
            header_style: CellStyle::default(),
            column_styles: None,
            freeze_header: false,
            auto_filter: false,
            auto_fit: true,
        }),
        "report" => Ok(WriteOptions {
            sheet_name: None,
            style_header: true,
            header_style: CellStyle::header(),
            column_styles: None,
            freeze_header: true,
            auto_filter: true,
            auto_fit: true,
        }),
        "executive" | "corporate" => Ok(WriteOptions {
            sheet_name: None,
            style_header: true,
            header_style: CellStyle {
                bold: true,
                bg_color: Some("203764".to_string()),
                font_color: Some("FFFFFF".to_string()),
                border: true,
                align: Some("center".to_string()),
                ..Default::default()
            },
            column_styles: None,
            freeze_header: true,
            auto_filter: true,
            auto_fit: true,
        }),
        _ => anyhow::bail!(
            "Unknown style preset {:?}. Use: default, minimal, report, executive.",
            name.trim()
        ),
    }
}

pub fn handle_add_chart(
    input: String,
    output: String,
    chart_type: String,
    title: Option<String>,
    category_column: Option<usize>,
    value_columns: Option<Vec<usize>>,
) -> Result<()> {
    let converter = Converter::new();
    let data = converter.read_any_data(&input, None)?;

    let chart_type_parsed = DataChartType::from_str(&chart_type)?;

    let mut chart_config = ChartConfig {
        chart_type: chart_type_parsed,
        ..Default::default()
    };

    if let Some(t) = title {
        chart_config.title = Some(t);
    }
    if let Some(cat_col) = category_column {
        chart_config.category_column = cat_col;
    }
    if let Some(val_cols) = value_columns {
        chart_config.value_columns = val_cols;
    }

    let handler = ExcelHandler::new();
    handler.write_with_chart(&output, &data, &chart_config)?;

    crate::cli::runtime::log(format!("Added chart to {}", output));

    Ok(())
}

pub fn handle_add_sparkline(
    output: String,
    data_range: String,
    sparkline_cell: String,
    sheet: Option<String>,
) -> Result<()> {
    let handler = ExcelHandler::new();
    handler.add_sparkline_formula(
        &output,
        &data_range,
        &sparkline_cell,
        sheet.as_deref(),
    )?;

    crate::cli::runtime::log(format!("Added sparkline to {}", output));

    Ok(())
}

pub fn handle_conditional_format(
    output: String,
    range: String,
    condition: String,
    bg_color: Option<String>,
    font_color: Option<String>,
    bold: Option<bool>,
    sheet: Option<String>,
) -> Result<()> {
    let cell_style = CellStyle {
        bg_color,
        font_color,
        bold: bold.unwrap_or(true),
        ..Default::default()
    };

    let handler = ExcelHandler::new();
    handler.apply_conditional_format_formula(
        &output,
        &range,
        &condition,
        &cell_style,
        None,
        sheet.as_deref(),
    )?;

    crate::cli::runtime::log(format!("Applied conditional formatting to {}", output));

    Ok(())
}

pub fn handle_apply_formula_range(
    input: String,
    output: String,
    formula: String,
    range_str: String,
    sheet: Option<String>,
) -> Result<()> {
    use xls_rs::csv_handler::CellRange;
    use xls_rs::formula::FormulaEvaluator;

    let cell_range = CellRange::parse(&range_str)?;
    let evaluator = FormulaEvaluator::new();
    let cells_affected =
        evaluator.apply_to_range(&input, &output, &formula, &cell_range, sheet.as_deref())?;

    crate::cli::runtime::log(format!(
        "Applied formula to {} cell(s) in {}",
        cells_affected,
        output
    ));

    Ok(())
}
