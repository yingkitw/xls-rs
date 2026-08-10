//! Format detection for file types

use crate::traits::FormatDetector;
use anyhow::Result;

/// Default format detector implementation
pub struct DefaultFormatDetector;

impl Default for DefaultFormatDetector {
    fn default() -> Self {
        Self
    }
}

impl DefaultFormatDetector {
    pub fn new() -> Self {
        Self
    }
}

impl FormatDetector for DefaultFormatDetector {
    fn detect_format(&self, path: &str) -> Result<String> {
        path.split('.')
            .next_back()
            .map(|s| s.to_lowercase())
            .ok_or_else(|| anyhow::anyhow!("No file extension found in: {}", path))
    }

    fn is_supported(&self, format: &str) -> bool {
        format.to_lowercase() == "xlsx"
    }

    fn supported_formats(&self) -> Vec<String> {
        vec!["xlsx".to_string()]
    }
}
