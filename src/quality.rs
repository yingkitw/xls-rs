//! Automated data quality reports
//!
//! Generates comprehensive data quality reports with recommendations.

use crate::anomaly::{AnomalyDetector, AnomalyMethod};
use crate::profiling::DataProfiler;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Data quality report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    pub file_path: String,
    pub timestamp: String,
    pub overall_score: f64,
    pub categories: QualityCategories,
    pub issues: Vec<QualityIssue>,
    pub recommendations: Vec<String>,
}

/// Quality categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityCategories {
    pub completeness: f64,
    pub accuracy: f64,
    pub consistency: f64,
    pub validity: f64,
    pub uniqueness: f64,
}

/// Quality issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    pub severity: IssueSeverity,
    pub category: String,
    pub description: String,
    pub affected_rows: Option<usize>,
    pub affected_columns: Option<Vec<String>>,
}

/// Issue severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Quality report generator
pub struct QualityReportGenerator {
    profiler: DataProfiler,
}

impl QualityReportGenerator {
    pub fn new() -> Self {
        Self {
            profiler: DataProfiler::new(),
        }
    }

    /// Generate quality report
    pub fn generate(&self, data: &[Vec<String>], file_path: &str) -> Result<QualityReport> {
        // Profile data
        let profile = self.profiler.profile(data, file_path)?;

        // Calculate quality scores
        let completeness = 100.0 - profile.null_percentage;
        let uniqueness = 100.0 - profile.duplicate_percentage;

        // Check for anomalies
        let mut issues = Vec::new();
        let mut accuracy_score = 100.0;

        for (col_idx, col_profile) in profile.columns.iter().enumerate() {
            if col_profile.null_percentage > 50.0 {
                issues.push(QualityIssue {
                    severity: IssueSeverity::High,
                    category: "Completeness".to_string(),
                    description: format!(
                        "Column '{}' has {:.1}% null values",
                        col_profile.name, col_profile.null_percentage
                    ),
                    affected_rows: None,
                    affected_columns: Some(vec![col_profile.name.clone()]),
                });
                accuracy_score -= 10.0;
            }

            // Check for anomalies in numeric columns
            if matches!(
                col_profile.data_type,
                crate::profiling::DataType::Integer | crate::profiling::DataType::Float
            ) {
                let detector = AnomalyDetector::new(AnomalyMethod::ZScore { threshold: 3.0 });
                if let Ok(anomaly_result) = detector.detect(data, col_idx) {
                    if anomaly_result.anomaly_percentage > 5.0 {
                        issues.push(QualityIssue {
                            severity: IssueSeverity::Medium,
                            category: "Accuracy".to_string(),
                            description: format!(
                                "Column '{}' has {:.1}% anomalies",
                                col_profile.name, anomaly_result.anomaly_percentage
                            ),
                            affected_rows: Some(anomaly_result.total_anomalies),
                            affected_columns: Some(vec![col_profile.name.clone()]),
                        });
                        accuracy_score -= anomaly_result.anomaly_percentage;
                    }
                }
            }
        }

        let consistency = profile.data_quality_score;
        let validity = 100.0 - (issues.len() as f64 * 5.0).min(50.0);

        let categories = QualityCategories {
            completeness,
            accuracy: accuracy_score.max(0.0),
            consistency,
            validity,
            uniqueness,
        };

        let overall_score = (categories.completeness
            + categories.accuracy
            + categories.consistency
            + categories.validity
            + categories.uniqueness)
            / 5.0;

        // Generate recommendations
        let recommendations = self.generate_recommendations(&profile, &issues);

        Ok(QualityReport {
            file_path: file_path.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            overall_score,
            categories,
            issues,
            recommendations,
        })
    }

    fn generate_recommendations(
        &self,
        profile: &crate::profiling::DataProfile,
        issues: &[QualityIssue],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if profile.null_percentage > 10.0 {
            recommendations.push(format!(
                "Consider filling {}% null values using fillna command",
                profile.null_percentage
            ));
        }

        if profile.duplicate_percentage > 5.0 {
            recommendations.push(format!(
                "Remove {}% duplicate rows using dedupe command",
                profile.duplicate_percentage
            ));
        }

        for issue in issues {
            match issue.severity {
                IssueSeverity::Critical | IssueSeverity::High => {
                    recommendations.push(format!(
                        "Fix {} issue: {}",
                        issue.category, issue.description
                    ));
                }
                _ => {}
            }
        }

        recommendations
    }

    /// Save report to file
    pub fn save_report(&self, report: &QualityReport, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(report)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}
