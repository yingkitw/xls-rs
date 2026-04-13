//! Data validation operations
//!
//! Provides comprehensive data validation capabilities including
//! rule-based validation, data quality checks, and reporting.

use crate::common::string;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validation rule types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ValidationRule {
    /// Check if value is not null/empty
    NotNull,
    /// Check if value matches a regex pattern
    Regex { pattern: String },
    /// Check if value is within a numeric range
    Range { min: Option<f64>, max: Option<f64> },
    /// Check if value is one of allowed values
    Enum { values: Vec<String> },
    /// Check if value has specific length
    Length {
        min: Option<usize>,
        max: Option<usize>,
    },
    /// Check if value is a valid email
    Email,
    /// Check if value is a valid URL
    Url,
    /// Check if value is numeric
    Numeric,
    /// Check if value is a valid date
    Date { format: String },
    /// Custom validation using expression
    Custom { expression: String },
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub stats: ValidationStats,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub row: usize,
    pub column: String,
    pub value: String,
    pub rule: String,
    pub message: String,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub row: usize,
    pub column: String,
    pub value: String,
    pub message: String,
}

/// Validation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStats {
    pub total_rows: usize,
    pub valid_rows: usize,
    pub invalid_rows: usize,
    pub total_errors: usize,
    pub total_warnings: usize,
    pub columns_validated: usize,
}

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub rules: HashMap<String, Vec<ValidationRule>>,
    pub strict_mode: bool,
    pub stop_on_first_error: bool,
    pub max_errors: Option<usize>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            rules: HashMap::new(),
            strict_mode: false,
            stop_on_first_error: false,
            max_errors: None,
        }
    }
}

/// Data validator
pub struct DataValidator {
    config: ValidationConfig,
}

impl DataValidator {
    /// Create a new validator with configuration
    pub fn new(config: ValidationConfig) -> Self {
        Self { config }
    }

    /// Create a validator from JSON configuration file
    pub fn from_config_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: ValidationConfig = serde_json::from_str(&content)?;
        Ok(Self::new(config))
    }

    /// Validate data rows
    pub fn validate(&self, data: &[Vec<String>]) -> Result<ValidationResult> {
        if data.is_empty() {
            return Ok(ValidationResult {
                is_valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
                stats: ValidationStats {
                    total_rows: 0,
                    valid_rows: 0,
                    invalid_rows: 0,
                    total_errors: 0,
                    total_warnings: 0,
                    columns_validated: 0,
                },
            });
        }

        let header = &data[0];
        let mut errors = Vec::new();
        let warnings = Vec::new();
        let mut valid_rows = 0;

        for (row_idx, row) in data.iter().enumerate().skip(1) {
            let mut row_valid = true;

            for (col_idx, cell_value) in row.iter().enumerate() {
                if let Some(column_name) = header.get(col_idx) {
                    if let Some(rules) = self.config.rules.get(column_name) {
                        for rule in rules {
                            match self.validate_value(cell_value, rule) {
                                Ok(()) => {} // Valid
                                Err(e) => {
                                    let error = ValidationError {
                                        row: row_idx,
                                        column: column_name.clone(),
                                        value: cell_value.clone(),
                                        rule: format!("{:?}", rule),
                                        message: e.to_string(),
                                    };
                                    errors.push(error);
                                    row_valid = false;

                                    if self.config.stop_on_first_error {
                                        break;
                                    }

                                    if let Some(max) = self.config.max_errors {
                                        if errors.len() >= max {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if row_valid {
                valid_rows += 1;
            }

            if self.config.stop_on_first_error && !errors.is_empty() {
                break;
            }

            if let Some(max) = self.config.max_errors {
                if errors.len() >= max {
                    break;
                }
            }
        }

        let total_rows = data.len() - 1; // Exclude header
        let invalid_rows = total_rows - valid_rows;
        let is_valid = if self.config.strict_mode {
            errors.is_empty() && warnings.is_empty()
        } else {
            errors.is_empty()
        };

        let total_errors = errors.len();
        let total_warnings = warnings.len();

        Ok(ValidationResult {
            is_valid,
            errors,
            warnings,
            stats: ValidationStats {
                total_rows,
                valid_rows,
                invalid_rows,
                total_errors,
                total_warnings,
                columns_validated: self.config.rules.len(),
            },
        })
    }

    /// Validate a single value against a rule
    fn validate_value(&self, value: &str, rule: &ValidationRule) -> Result<()> {
        match rule {
            ValidationRule::NotNull => {
                if string::is_empty_or_whitespace(value) {
                    return Err(anyhow::anyhow!("Value cannot be null or empty"));
                }
            }
            ValidationRule::Regex { pattern } => {
                let re = regex::Regex::new(pattern)?;
                if !re.is_match(value) {
                    return Err(anyhow::anyhow!("Value does not match pattern: {}", pattern));
                }
            }
            ValidationRule::Range { min, max } => {
                if let Some(num) = string::to_number(value) {
                    if let Some(min_val) = min {
                        if num < *min_val {
                            return Err(anyhow::anyhow!(
                                "Value {} is below minimum {}",
                                num,
                                min_val
                            ));
                        }
                    }
                    if let Some(max_val) = max {
                        if num > *max_val {
                            return Err(anyhow::anyhow!(
                                "Value {} is above maximum {}",
                                num,
                                max_val
                            ));
                        }
                    }
                } else {
                    return Err(anyhow::anyhow!("Value is not numeric"));
                }
            }
            ValidationRule::Enum { values } => {
                if !values.contains(&value.to_string()) {
                    return Err(anyhow::anyhow!(
                        "Value '{}' is not in allowed values: {:?}",
                        value,
                        values
                    ));
                }
            }
            ValidationRule::Length { min, max } => {
                let len = value.len();
                if let Some(min_len) = min {
                    if len < *min_len {
                        return Err(anyhow::anyhow!(
                            "Length {} is below minimum {}",
                            len,
                            min_len
                        ));
                    }
                }
                if let Some(max_len) = max {
                    if len > *max_len {
                        return Err(anyhow::anyhow!(
                            "Length {} is above maximum {}",
                            len,
                            max_len
                        ));
                    }
                }
            }
            ValidationRule::Email => {
                let email_regex =
                    regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")?;
                if !email_regex.is_match(value) {
                    return Err(anyhow::anyhow!("Invalid email format"));
                }
            }
            ValidationRule::Url => {
                let url_regex = regex::Regex::new(r"^https?://[^\s/$.?#].[^\s]*$")?;
                if !url_regex.is_match(value) {
                    return Err(anyhow::anyhow!("Invalid URL format"));
                }
            }
            ValidationRule::Numeric => {
                if !string::is_numeric(value) {
                    return Err(anyhow::anyhow!("Value is not numeric"));
                }
            }
            ValidationRule::Date { format } => {
                chrono::NaiveDate::parse_from_str(value, format)
                    .map_err(|_| anyhow::anyhow!("Invalid date format for {}", format))?;
            }
            ValidationRule::Custom { expression } => {
                // Simple custom expression evaluation
                // In a real implementation, this would use a proper expression parser
                if expression.contains("not_empty") && string::is_empty_or_whitespace(value) {
                    return Err(anyhow::anyhow!("Custom validation failed: {}", expression));
                }
            }
        }

        Ok(())
    }

    /// Generate validation report
    pub fn generate_report(&self, result: &ValidationResult) -> String {
        let mut report = String::new();

        report.push_str("# Data Validation Report\n\n");

        // Summary
        report.push_str("## Summary\n\n");
        report.push_str(&format!(
            "- **Total Rows**: {}\n\
             - **Valid Rows**: {}\n\
             - **Invalid Rows**: {}\n\
             - **Total Errors**: {}\n\
             - **Total Warnings**: {}\n\
             - **Columns Validated**: {}\n\
             - **Overall Status**: {}\n\n",
            result.stats.total_rows,
            result.stats.valid_rows,
            result.stats.invalid_rows,
            result.stats.total_errors,
            result.stats.total_warnings,
            result.stats.columns_validated,
            if result.is_valid {
                "✅ PASSED"
            } else {
                "❌ FAILED"
            }
        ));

        // Errors
        if !result.errors.is_empty() {
            report.push_str("## Errors\n\n");
            for error in &result.errors {
                report.push_str(&format!(
                    "- **Row {}**, Column `{}`: {} (value: `{}`)\n",
                    error.row + 1,
                    error.column,
                    error.message,
                    error.value
                ));
            }
            report.push('\n');
        }

        // Warnings
        if !result.warnings.is_empty() {
            report.push_str("## Warnings\n\n");
            for warning in &result.warnings {
                report.push_str(&format!(
                    "- **Row {}**, Column `{}`: {} (value: `{}`)\n",
                    warning.row + 1,
                    warning.column,
                    warning.message,
                    warning.value
                ));
            }
            report.push('\n');
        }

        report
    }

    /// Save validation result to file
    pub fn save_result(&self, result: &ValidationResult, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(result)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

/// Create a sample validation configuration
pub fn create_sample_config() -> ValidationConfig {
    let mut rules = HashMap::new();

    // Email validation
    rules.insert(
        "email".to_string(),
        vec![ValidationRule::Email, ValidationRule::NotNull],
    );

    // Age validation
    rules.insert(
        "age".to_string(),
        vec![
            ValidationRule::Numeric,
            ValidationRule::Range {
                min: Some(0.0),
                max: Some(150.0),
            },
        ],
    );

    // Name validation
    rules.insert(
        "name".to_string(),
        vec![
            ValidationRule::NotNull,
            ValidationRule::Length {
                min: Some(1),
                max: Some(100),
            },
        ],
    );

    // Status validation
    rules.insert(
        "status".to_string(),
        vec![ValidationRule::Enum {
            values: vec![
                "active".to_string(),
                "inactive".to_string(),
                "pending".to_string(),
            ],
        }],
    );

    ValidationConfig {
        rules,
        strict_mode: false,
        stop_on_first_error: false,
        max_errors: Some(1000),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_not_null() {
        let validator = DataValidator::new(ValidationConfig::default());

        // Test valid value
        assert!(
            validator
                .validate_value("test", &ValidationRule::NotNull)
                .is_ok()
        );

        // Test invalid value
        assert!(
            validator
                .validate_value("", &ValidationRule::NotNull)
                .is_err()
        );
        assert!(
            validator
                .validate_value("   ", &ValidationRule::NotNull)
                .is_err()
        );
    }

    #[test]
    fn test_validation_numeric() {
        let validator = DataValidator::new(ValidationConfig::default());

        // Test valid numbers
        assert!(
            validator
                .validate_value("123", &ValidationRule::Numeric)
                .is_ok()
        );
        assert!(
            validator
                .validate_value("-45.67", &ValidationRule::Numeric)
                .is_ok()
        );

        // Test invalid numbers
        assert!(
            validator
                .validate_value("abc", &ValidationRule::Numeric)
                .is_err()
        );
        assert!(
            validator
                .validate_value("", &ValidationRule::Numeric)
                .is_err()
        );
    }

    #[test]
    fn test_validation_range() {
        let validator = DataValidator::new(ValidationConfig::default());
        let rule = ValidationRule::Range {
            min: Some(0.0),
            max: Some(100.0),
        };

        // Test valid range
        assert!(validator.validate_value("50", &rule).is_ok());
        assert!(validator.validate_value("0", &rule).is_ok());
        assert!(validator.validate_value("100", &rule).is_ok());

        // Test invalid range
        assert!(validator.validate_value("-1", &rule).is_err());
        assert!(validator.validate_value("101", &rule).is_err());
    }

    #[test]
    fn test_validation_enum() {
        let validator = DataValidator::new(ValidationConfig::default());
        let rule = ValidationRule::Enum {
            values: vec!["red".to_string(), "green".to_string(), "blue".to_string()],
        };

        // Test valid enum values
        assert!(validator.validate_value("red", &rule).is_ok());
        assert!(validator.validate_value("green", &rule).is_ok());

        // Test invalid enum value
        assert!(validator.validate_value("yellow", &rule).is_err());
    }
}
