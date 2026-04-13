//! Utility command handlers (completions, config, styled export)

use xls_rs::{
    config::Config,
    converter::Converter,
    excel::{CellStyle, ExcelHandler, WriteOptions},
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
