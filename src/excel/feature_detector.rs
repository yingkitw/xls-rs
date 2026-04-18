//! Excel feature detection and structured errors
//!
//! This module provides utilities for detecting Excel features that may not be fully supported
//! and returns structured error messages with actionable guidance.

use anyhow::{anyhow, Result};

/// Unsupported Excel feature with structured error information
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsupportedFeature {
    /// Merged cells (partial support - cells are read but merge status is lost)
    MergedCells {
        sheet: String,
        range: String,
    },
    /// Pivot tables (not supported - data may be read but pivot structure is lost)
    PivotTable {
        sheet: String,
    },
    /// Data validation (not supported - data is readable but validation rules are lost)
    DataValidation {
        sheet: String,
        range: String,
    },
    /// Conditional formatting (read-only - formats are visible but not editable)
    ConditionalFormatting {
        sheet: String,
    },
    /// Array formulas (limited support - formulas may be read as static values)
    ArrayFormulas {
        sheet: String,
    },
    /// Protected sheets (read-only - content readable but cannot be modified)
    ProtectedSheet {
        sheet: String,
        password_protected: bool,
    },
    /// External references/links (not supported - references may be broken)
    ExternalReferences {
        sheet: String,
    },
    /// Charts (read-only - data visible but chart configuration is lost)
    Charts {
        sheet: String,
        count: usize,
    },
    /// Images/Objects (not supported - visual elements are lost)
    EmbeddedObjects {
        sheet: String,
        object_type: String,
    },
}

impl UnsupportedFeature {
    /// Get a user-friendly description of the unsupported feature
    pub fn description(&self) -> String {
        match self {
            Self::MergedCells { sheet, range } => {
                format!(
                    "Merged cells detected in sheet '{}' range '{}'. Merged cells will be read as individual cells. Merge structure will be lost on write.",
                    sheet, range
                )
            }
            Self::PivotTable { sheet } => {
                format!(
                    "Pivot table detected in sheet '{}'. Pivot tables are not fully supported - data will be read as static values. Pivot structure, filters, and calculations will be lost.",
                    sheet
                )
            }
            Self::DataValidation { sheet, range } => {
                format!(
                    "Data validation detected in sheet '{}' range '{}'. Validation rules will be lost when modifying or writing this file.",
                    sheet, range
                )
            }
            Self::ConditionalFormatting { sheet } => {
                format!(
                    "Conditional formatting detected in sheet '{}'. Formatting rules will be preserved on read but may not be editable through xls-rs.",
                    sheet
                )
            }
            Self::ArrayFormulas { sheet } => {
                format!(
                    "Array formulas detected in sheet '{}'. Array formulas may be read as static values. Dynamic calculation behavior may be lost.",
                    sheet
                )
            }
            Self::ProtectedSheet { sheet, password_protected } => {
                if *password_protected {
                    format!(
                        "Sheet '{}' is password protected. Content is readable but cannot be modified. Remove protection to enable editing.",
                        sheet
                    )
                } else {
                    format!(
                        "Sheet '{}' is protected. Content is readable but editing may be limited.",
                        sheet
                    )
                }
            }
            Self::ExternalReferences { sheet } => {
                format!(
                    "External references detected in sheet '{}'. External links may be broken or not accessible. Consider consolidating data.",
                    sheet
                )
            }
            Self::Charts { sheet, count } => {
                format!(
                    "Charts detected in sheet '{}' ({} chart(s)). Charts are read-only through xls-rs - data is visible but chart configuration cannot be modified.",
                    sheet, count
                )
            }
            Self::EmbeddedObjects { sheet, object_type } => {
                format!(
                    "Embedded {} detected in sheet '{}'. Visual elements like images and shapes are not fully supported - they may be lost on read/write.",
                    object_type, sheet
                )
            }
        }
    }

    /// Get the severity level of the unsupported feature
    pub fn severity(&self) -> FeatureSeverity {
        match self {
            Self::MergedCells { .. } => FeatureSeverity::Warning,
            Self::PivotTable { .. } => FeatureSeverity::Limitation,
            Self::DataValidation { .. } => FeatureSeverity::Warning,
            Self::ConditionalFormatting { .. } => FeatureSeverity::Warning,
            Self::ArrayFormulas { .. } => FeatureSeverity::Limitation,
            Self::ProtectedSheet { password_protected: true, .. } => FeatureSeverity::Error,
            Self::ProtectedSheet { password_protected: false, .. } => FeatureSeverity::Warning,
            Self::ExternalReferences { .. } => FeatureSeverity::Warning,
            Self::Charts { .. } => FeatureSeverity::Warning,
            Self::EmbeddedObjects { .. } => FeatureSeverity::Limitation,
        }
    }

    /// Get actionable guidance for working around the limitation
    pub fn guidance(&self) -> Option<String> {
        match self {
            Self::MergedCells { .. } => Some(
                "To preserve merged cells, consider using Excel directly or exporting to a format that maintains merge structure.".to_string()
            ),
            Self::PivotTable { .. } => Some(
                "For full pivot table support, use Excel directly. To work with pivot data, consider flattening the pivot table to static values first.".to_string()
            ),
            Self::DataValidation { .. } => Some(
                "Data validation rules can be re-applied after modification using the conditional-format command.".to_string()
            ),
            Self::ProtectedSheet { password_protected: true, .. } => Some(
                "Unprotect the sheet in Excel with the password to enable full editing capabilities.".to_string()
            ),
            Self::ExternalReferences { .. } => Some(
                "Replace external references with static values or consolidate external data into the workbook.".to_string()
            ),
            _ => None,
        }
    }

    /// Convert to an anyhow::Error with full context
    pub fn to_error(&self) -> anyhow::Error {
        let desc = self.description();
        let severity = self.severity();
        let guidance = self.guidance();

        let msg = if let Some(g) = guidance {
            format!("[{}] {}. Guidance: {}", severity, desc, g)
        } else {
            format!("[{}] {}", severity, desc)
        };

        anyhow!(msg)
    }
}

/// Severity level of unsupported features
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FeatureSeverity {
    /// Informational - feature is read-only but functional
    Info,
    /// Warning - feature has limited support but data is preserved
    Warning,
    /// Limitation - feature is partially supported with some data loss
    Limitation,
    /// Error - feature prevents the operation from completing
    Error,
}

impl std::fmt::Display for FeatureSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeatureSeverity::Info => write!(f, "INFO"),
            FeatureSeverity::Warning => write!(f, "WARNING"),
            FeatureSeverity::Limitation => write!(f, "LIMITATION"),
            FeatureSeverity::Error => write!(f, "ERROR"),
        }
    }
}

/// Excel file feature detector
///
/// This struct provides methods to detect potentially unsupported features
/// in Excel files before attempting operations that may fail or lose data.
pub struct FeatureDetector;

impl FeatureDetector {
    /// Detect features that may affect read/write operations
    ///
    /// This is a placeholder for future implementation. Currently, calamine
    /// doesn't expose detailed feature information. This would be enhanced
    /// when using a more advanced Excel library or adding custom parsing.
    pub fn detect_potential_issues(_path: &str) -> Result<Vec<UnsupportedFeature>> {
        // TODO: Implement actual feature detection
        // This would require:
        // 1. Parsing Excel XML structure directly
        // 2. Detecting merged cells, pivot tables, etc.
        // 3. Or using a library that exposes this information

        // For now, return empty vector
        Ok(Vec::new())
    }

    /// Check if a file is likely to contain unsupported features
    ///
    /// This is a heuristic check based on file extension and size.
    /// Complex Excel files (large, multiple sheets) are more likely
    /// to have unsupported features.
    pub fn heuristic_check(path: &str) -> Vec<UnsupportedFeature> {
        let mut issues = Vec::new();

        let path_lower = path.to_lowercase();

        // Check for very large files (more likely to have complex features)
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > 10 * 1024 * 1024 {
                // File > 10MB
                issues.push(UnsupportedFeature::PivotTable {
                    sheet: "unknown".to_string(),
                });
            }
        }

        // ODS files have different feature set
        if path_lower.ends_with(".ods") {
            // ODS may have different limitations
        }

        issues
    }

    /// Validate that a file doesn't contain features that would prevent write operations
    pub fn validate_for_write(path: &str) -> Result<()> {
        // Check for potential issues
        let issues = Self::detect_potential_issues(path)?;

        // Filter to only error-level issues for write validation
        let errors: Vec<_> = issues
            .into_iter()
            .filter(|f| f.severity() == FeatureSeverity::Error)
            .collect();

        if !errors.is_empty() {
            let error_messages: Vec<String> = errors
                .iter()
                .map(|f| f.description())
                .collect();
            return Err(anyhow!(
                "File contains features that prevent write operations:\n{}",
                error_messages.join("\n")
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unsupported_feature_description() {
        let feature = UnsupportedFeature::MergedCells {
            sheet: "Sheet1".to_string(),
            range: "A1:B2".to_string(),
        };
        let desc = feature.description();
        assert!(desc.contains("Merged cells"));
        assert!(desc.contains("Sheet1"));
        assert!(desc.contains("A1:B2"));
    }

    #[test]
    fn test_feature_severity() {
        assert_eq!(
            UnsupportedFeature::MergedCells {
                sheet: "S".to_string(),
                range: "A1".to_string(),
            }
            .severity(),
            FeatureSeverity::Warning
        );

        assert_eq!(
            UnsupportedFeature::ProtectedSheet {
                sheet: "S".to_string(),
                password_protected: true,
            }
            .severity(),
            FeatureSeverity::Error
        );
    }

    #[test]
    fn test_to_error() {
        let feature = UnsupportedFeature::PivotTable {
            sheet: "Sheet1".to_string(),
        };
        let error = feature.to_error();
        assert!(error.to_string().contains("Pivot table"));
    }
}
