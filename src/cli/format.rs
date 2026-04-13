//! Output format options for the read command

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
