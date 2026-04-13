//! Quality scoring and reporting methods for data profiling

use super::types::*;

impl super::profiler::DataProfiler {
    /// Calculate column quality score
    pub fn calculate_column_quality_score(
        &self,
        null_percentage: f64,
        unique_percentage: f64,
        data_type: &DataType,
        length_stats: Option<&LengthStats>,
        numeric_stats: Option<&NumericStats>,
    ) -> f64 {
        let mut score = 100.0;

        // Penalize null values
        score -= null_percentage * 0.5;

        // Penalize too many unique values for categorical data
        if matches!(
            data_type,
            DataType::String | DataType::Email | DataType::Url | DataType::Phone
        ) {
            if unique_percentage > 80.0 {
                score -= (unique_percentage - 80.0) * 0.2;
            }
        }

        // Check for consistent lengths (good for structured data)
        if let Some(length_stats) = length_stats {
            let length_variance = length_stats.std_dev_length / length_stats.avg_length;
            if length_variance < 0.1 {
                score += 5.0; // Bonus for consistent lengths
            }
        }

        // Check for reasonable numeric distributions
        if let Some(numeric_stats) = numeric_stats {
            // Penalize extreme skewness
            if numeric_stats.skewness.abs() > 2.0 {
                score -= 5.0;
            }

            // Bonus for reasonable variance
            if numeric_stats.std_dev > 0.0 && numeric_stats.std_dev < numeric_stats.mean * 2.0 {
                score += 5.0;
            }
        }

        score.max(0.0).min(100.0)
    }

    /// Calculate overall quality score
    pub fn calculate_overall_quality_score(
        &self,
        columns: &[ColumnProfile],
        null_percentage: f64,
        duplicate_percentage: f64,
    ) -> f64 {
        let column_scores: f64 = columns.iter().map(|c| c.quality_score).sum();
        let avg_column_score = column_scores / columns.len() as f64;

        let mut overall_score = avg_column_score;

        // Penalize high null percentage
        if null_percentage > 10.0 {
            overall_score -= (null_percentage - 10.0) * 0.3;
        }

        // Penalize high duplicate percentage
        if duplicate_percentage > 5.0 {
            overall_score -= (duplicate_percentage - 5.0) * 0.5;
        }

        overall_score.max(0.0).min(100.0)
    }

    /// Generate data quality recommendations
    pub fn generate_recommendations(
        &self,
        columns: &[ColumnProfile],
        null_percentage: f64,
        duplicate_percentage: f64,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Overall recommendations
        if null_percentage > 20.0 {
            recommendations.push(format!(
                "High null rate ({:.1}%). Consider data imputation or data quality checks.",
                null_percentage
            ));
        }

        if duplicate_percentage > 10.0 {
            recommendations.push(format!(
                "High duplicate rate ({:.1}%). Consider deduplication.",
                duplicate_percentage
            ));
        }

        // Column-specific recommendations
        for column in columns {
            if column.null_percentage > 30.0 {
                recommendations.push(format!(
                    "Column '{}' has high null rate ({:.1}%).",
                    column.name, column.null_percentage
                ));
            }

            if column.unique_percentage == 100.0 && column.null_percentage == 0.0 {
                recommendations.push(format!(
                    "Column '{}' might be a candidate for primary key.",
                    column.name
                ));
            }

            if matches!(
                column.data_type,
                DataType::String | DataType::Email | DataType::Url | DataType::Phone
            ) && column.unique_percentage > 95.0
            {
                recommendations.push(format!(
                    "Column '{}' has many unique values ({:.1}%). Consider if this is intended.",
                    column.name, column.unique_percentage
                ));
            }

            if let Some(numeric_stats) = &column.numeric_stats {
                if numeric_stats.skewness.abs() > 2.0 {
                    recommendations.push(format!(
                        "Column '{}' has high skewness ({:.2}). Consider transformation.",
                        column.name, numeric_stats.skewness
                    ));
                }
            }

            if let Some(length_stats) = &column.length_stats {
                if length_stats.std_dev_length / length_stats.avg_length > 0.5 {
                    recommendations.push(format!(
                        "Column '{}' has inconsistent length pattern.",
                        column.name
                    ));
                }
            }
        }

        recommendations
    }

    /// Generate a human-readable profile report
    pub fn generate_report(&self, profile: &DataProfile) -> String {
        let mut report = String::new();

        report.push_str(&format!("# Data Profile Report: {}\n\n", profile.file_path));

        report.push_str(&format!(
            "## Summary\n\n\
             - **Total Rows**: {}\n\
             - **Total Columns**: {}\n\
             - **Total Cells**: {}\n\
             - **Null Cells**: {} ({:.1}%)\n\
             - **Duplicate Rows**: {} ({:.1}%)\n\
             - **Data Quality Score**: {:.1}/100\n\n",
            profile.total_rows,
            profile.total_columns,
            profile.total_cells,
            profile.null_cells,
            profile.null_percentage,
            profile.duplicate_rows,
            profile.duplicate_percentage,
            profile.data_quality_score
        ));

        if !profile.recommendations.is_empty() {
            report.push_str("## Recommendations\n\n");
            for rec in &profile.recommendations {
                report.push_str(&format!("- {}\n", rec));
            }
            report.push_str("\n");
        }

        report.push_str("## Column Details\n\n");

        for column in &profile.columns {
            report.push_str(&format!(
                "### {}\n\n\
                 - **Type**: {:?}\n\
                 - **Quality Score**: {:.1}/100\n\
                 - **Null Count**: {} ({:.1}%)\n\
                 - **Unique Count**: {} ({:.1}%)\n",
                column.name,
                column.data_type,
                column.quality_score,
                column.null_count,
                column.null_percentage,
                column.unique_count,
                column.unique_percentage
            ));

            if !column.top_values.is_empty() {
                report.push_str("- **Top Values**:\n");
                for (i, val) in column.top_values.iter().take(5).enumerate() {
                    report.push_str(&format!(
                        "  {}. {} ({}%, {} occurrences)\n",
                        i + 1,
                        val.value,
                        val.percentage,
                        val.count
                    ));
                }
            }

            if let Some(numeric_stats) = &column.numeric_stats {
                report.push_str(&format!(
                    "- **Numeric Stats**: Min={}, Max={}, Mean={:.2}, Median={:.2}, StdDev={:.2}\n",
                    numeric_stats.min,
                    numeric_stats.max,
                    numeric_stats.mean,
                    numeric_stats.median,
                    numeric_stats.std_dev
                ));
            }

            if let Some(length_stats) = &column.length_stats {
                report.push_str(&format!(
                    "- **Length Stats**: Min={}, Max={}, Avg={:.1}, Median={}\n",
                    length_stats.min_length,
                    length_stats.max_length,
                    length_stats.avg_length,
                    length_stats.median_length
                ));
            }

            report.push('\n');
        }

        report.push_str(&format!(
            "---\n\n*Report generated on {}*",
            profile.profiling_timestamp
        ));

        report
    }
}
