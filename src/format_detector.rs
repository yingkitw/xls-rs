//! Format detection for file types

use crate::traits::FormatDetector;
use anyhow::Result;

/// Default format detector implementation
pub struct DefaultFormatDetector;

impl DefaultFormatDetector {
    pub fn new() -> Self {
        Self
    }
}

impl FormatDetector for DefaultFormatDetector {
    fn detect_format(&self, path: &str) -> Result<String> {
        // Check for Google Sheets URLs or IDs first
        if path.starts_with("gsheet://")
            || path.starts_with("https://docs.google.com/spreadsheets/")
            || (path.len() >= 44
                && path
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_'))
        {
            return Ok("gsheet".to_string());
        }

        // Fall back to file extension detection
        path.split('.')
            .last()
            .map(|s| s.to_lowercase())
            .ok_or_else(|| anyhow::anyhow!("No file extension found in: {}", path))
    }

    fn is_supported(&self, format: &str) -> bool {
        matches!(
            format.to_lowercase().as_str(),
            "csv" | "xlsx" | "xls" | "ods" | "parquet" | "avro" | "gsheet"
        )
    }

    fn supported_formats(&self) -> Vec<String> {
        vec![
            "csv".to_string(),
            "xlsx".to_string(),
            "xls".to_string(),
            "ods".to_string(),
            "parquet".to_string(),
            "avro".to_string(),
            "gsheet".to_string(),
        ]
    }
}
