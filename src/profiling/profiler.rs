//! Core data profiler implementation

use crate::common::{collection, string};
use anyhow::Result;
use std::collections::HashSet;

use super::types::*;

/// Data profiler
pub struct DataProfiler {
    max_distinct_values: usize,
    sample_size: Option<usize>,
}

impl DataProfiler {
    /// Create a new profiler with options
    pub fn new() -> Self {
        Self {
            max_distinct_values: 100,
            sample_size: None,
        }
    }

    /// Set maximum distinct values to track
    pub fn with_max_distinct_values(mut self, max: usize) -> Self {
        self.max_distinct_values = max;
        self
    }

    /// Set sample size for large datasets
    pub fn with_sample_size(mut self, size: usize) -> Self {
        self.sample_size = Some(size);
        self
    }

    /// Profile data from rows
    pub fn profile(&self, data: &[Vec<String>], file_path: &str) -> Result<DataProfile> {
        if data.is_empty() {
            return Ok(DataProfile {
                file_path: file_path.to_string(),
                total_rows: 0,
                total_columns: 0,
                total_cells: 0,
                null_cells: 0,
                null_percentage: 0.0,
                duplicate_rows: 0,
                duplicate_percentage: 0.0,
                columns: Vec::new(),
                data_quality_score: 0.0,
                recommendations: Vec::new(),
                profiling_timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }

        let header = &data[0];
        let total_rows = data.len() - 1;
        let total_columns = header.len();
        let total_cells = total_rows * total_columns;

        // Sample data if needed
        let data_to_profile = if let Some(sample_size) = self.sample_size {
            if total_rows > sample_size {
                let mut sampled = vec![header.clone()];
                let step = total_rows / sample_size;
                for i in (1..=total_rows).step_by(step.max(1)) {
                    if i < data.len() {
                        sampled.push(data[i].clone());
                    }
                }
                sampled
            } else {
                data.to_vec()
            }
        } else {
            data.to_vec()
        };

        // Profile each column
        let mut columns = Vec::new();
        let mut null_cells = 0;

        for (col_idx, col_name) in header.iter().enumerate() {
            let column_data: Vec<String> = data_to_profile
                .iter()
                .skip(1)
                .filter_map(|row| row.get(col_idx).cloned())
                .collect();

            let column_profile = self.profile_column(col_name, &column_data, total_rows)?;
            null_cells += column_profile.null_count;
            columns.push(column_profile);
        }

        // Calculate duplicates
        let duplicate_rows = self.count_duplicate_rows(&data_to_profile[1..]);
        let duplicate_percentage = (duplicate_rows as f64 / total_rows as f64) * 100.0;
        let null_percentage = (null_cells as f64 / total_cells as f64) * 100.0;

        // Calculate overall quality score
        let data_quality_score =
            self.calculate_overall_quality_score(&columns, null_percentage, duplicate_percentage);

        // Generate recommendations
        let recommendations =
            self.generate_recommendations(&columns, null_percentage, duplicate_percentage);

        Ok(DataProfile {
            file_path: file_path.to_string(),
            total_rows,
            total_columns,
            total_cells,
            null_cells,
            null_percentage,
            duplicate_rows,
            duplicate_percentage,
            columns,
            data_quality_score,
            recommendations,
            profiling_timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Profile a single column
    fn profile_column(
        &self,
        name: &str,
        data: &[String],
        total_rows: usize,
    ) -> Result<ColumnProfile> {
        let null_count = data
            .iter()
            .filter(|v| string::is_empty_or_whitespace(v))
            .count();
        let null_percentage = (null_count as f64 / total_rows as f64) * 100.0;

        // Get distinct values
        let distinct_values: Vec<String> = collection::unique_preserve_order(
            &data
                .iter()
                .filter(|v| !string::is_empty_or_whitespace(v))
                .cloned()
                .collect::<Vec<_>>(),
        );

        let unique_count = distinct_values.len();
        let unique_percentage = (unique_count as f64 / total_rows as f64) * 100.0;

        // Get top values
        let top_values = self.get_value_frequencies(data);

        // Determine data type
        let data_type = self.infer_data_type(data);

        // Calculate type-specific statistics
        let length_stats = if matches!(
            data_type,
            DataType::String | DataType::Email | DataType::Url | DataType::Phone
        ) {
            Some(self.calculate_length_stats(data))
        } else {
            None
        };

        let numeric_stats = if matches!(data_type, DataType::Integer | DataType::Float) {
            self.calculate_numeric_stats(data)
        } else {
            None
        };

        let date_stats = if matches!(data_type, DataType::Date | DataType::DateTime) {
            self.calculate_date_stats(data)
        } else {
            None
        };

        let text_stats = if matches!(data_type, DataType::String) {
            Some(self.calculate_text_stats(data))
        } else {
            None
        };

        // Calculate quality score for this column
        let quality_score = self.calculate_column_quality_score(
            null_percentage,
            unique_percentage,
            &data_type,
            length_stats.as_ref(),
            numeric_stats.as_ref(),
        );

        Ok(ColumnProfile {
            name: name.to_string(),
            data_type,
            null_count,
            null_percentage,
            unique_count,
            unique_percentage,
            distinct_values: distinct_values
                .into_iter()
                .take(self.max_distinct_values)
                .collect(),
            top_values,
            length_stats,
            numeric_stats,
            date_stats,
            text_stats,
            quality_score,
        })
    }

    /// Count duplicate rows
    fn count_duplicate_rows(&self, rows: &[Vec<String>]) -> usize {
        let mut seen = HashSet::new();
        let mut duplicates = 0;

        for row in rows {
            let row_str = row.join("|");
            if seen.contains(&row_str) {
                duplicates += 1;
            } else {
                seen.insert(row_str);
            }
        }

        duplicates
    }
}

impl Default for DataProfiler {
    fn default() -> Self {
        Self::new()
    }
}
