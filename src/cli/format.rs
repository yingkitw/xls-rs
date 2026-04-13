//! Output format options for the read command

use anyhow::Result;
use clap::ValueEnum;

/// Output format for read command
///
/// Determines how data is displayed when using the read command.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    /// CSV format (comma-separated values)
    #[default]
    Csv,

    /// JSON format (array of objects)
    Json,

    /// JSON Lines / NDJSON (one JSON object per line, streaming-friendly)
    Jsonl,

    /// Markdown table format
    Markdown,
}

impl OutputFormat {
    /// Parse values allowed in `default_format` (config) or CLI strings.
    pub fn from_config_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "json" => Some(Self::Json),
            "jsonl" | "ndjson" => Some(Self::Jsonl),
            "markdown" | "md" => Some(Self::Markdown),
            _ => None,
        }
    }

    /// If `explicit` is set, use it; otherwise `default_format` from config, else CSV.
    pub fn resolve_for_read(explicit: Option<Self>) -> Result<Self> {
        if let Some(f) = explicit {
            return Ok(f);
        }
        let cfg = crate::cli::runtime::load_cli_config()?;
        if let Some(ref s) = cfg.default_format {
            if let Some(f) = Self::from_config_str(s) {
                return Ok(f);
            }
            anyhow::bail!(
                "Invalid default_format in config: {:?}. Use csv, json, jsonl, or markdown.",
                s
            );
        }
        Ok(Self::Csv)
    }

    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Csv => "csv",
            OutputFormat::Json => "json",
            OutputFormat::Jsonl => "jsonl",
            OutputFormat::Markdown => "md",
        }
    }

    /// Get the MIME type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            OutputFormat::Csv => "text/csv",
            OutputFormat::Json => "application/json",
            OutputFormat::Jsonl => "application/x-ndjson",
            OutputFormat::Markdown => "text/markdown",
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Csv => write!(f, "csv"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Jsonl => write!(f, "jsonl"),
            OutputFormat::Markdown => write!(f, "markdown"),
        }
    }
}
