//! Time series operations
//!
//! Provides time series analysis capabilities including resampling,
//! rolling windows, trend analysis, and seasonal decomposition.

use crate::common::string;
use anyhow::Result;
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Time series resampling intervals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResampleInterval {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
    Hourly,
    Minute,
    Custom(Duration),
}

/// Aggregation functions for resampling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeSeriesAgg {
    Sum,
    Mean,
    Median,
    Min,
    Max,
    First,
    Last,
    Count,
}

/// Rolling window configuration
#[derive(Debug, Clone)]
pub struct RollingWindow {
    pub window_size: Duration,
    pub min_periods: usize,
    pub center: bool,
}

/// Time series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: NaiveDateTime,
    pub value: f64,
}

/// Time series statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesStats {
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
    pub total_points: usize,
    pub missing_points: usize,
    pub trend_direction: TrendDirection,
    pub seasonality_detected: bool,
    pub autocorrelation: f64,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stationary,
    Unknown,
}

/// Time series processor
pub struct TimeSeriesProcessor {
    date_format: String,
}

impl TimeSeriesProcessor {
    /// Create a new time series processor
    pub fn new(date_format: &str) -> Self {
        Self {
            date_format: date_format.to_string(),
        }
    }

    /// Parse date string to NaiveDateTime
    pub fn parse_date(&self, date_str: &str) -> Result<NaiveDateTime> {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, &self.date_format) {
            Ok(date.and_hms_opt(0, 0, 0).unwrap())
        } else if let Ok(datetime) = NaiveDateTime::parse_from_str(date_str, &self.date_format) {
            Ok(datetime)
        } else {
            // Try common formats
            let common_formats = vec![
                "%Y-%m-%d",
                "%Y-%m-%d %H:%M:%S",
                "%d/%m/%Y",
                "%d/%m/%Y %H:%M:%S",
                "%m/%d/%Y",
                "%m/%d/%Y %H:%M:%S",
            ];

            for format in common_formats {
                if let Ok(date) = NaiveDate::parse_from_str(date_str, format) {
                    return Ok(date.and_hms_opt(0, 0, 0).unwrap());
                }
                if let Ok(datetime) = NaiveDateTime::parse_from_str(date_str, format) {
                    return Ok(datetime);
                }
            }

            anyhow::bail!("Unable to parse date: {}", date_str);
        }
    }

    /// Convert CSV data to time series
    pub fn csv_to_timeseries(
        &self,
        data: &[Vec<String>],
        date_col: usize,
        value_col: usize,
    ) -> Result<Vec<TimeSeriesPoint>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut points = Vec::new();

        for (i, row) in data.iter().enumerate().skip(1) {
            // Skip header
            if let (Some(date_str), Some(value_str)) = (row.get(date_col), row.get(value_col)) {
                let timestamp = self.parse_date(date_str)?;
                let value = string::to_number(value_str).ok_or_else(|| {
                    anyhow::anyhow!("Invalid number at row {}: {}", i + 1, value_str)
                })?;

                points.push(TimeSeriesPoint { timestamp, value });
            }
        }

        // Sort by timestamp
        points.sort_by_key(|p| p.timestamp);

        Ok(points)
    }

    /// Resample time series data
    pub fn resample(
        &self,
        data: &[TimeSeriesPoint],
        interval: &ResampleInterval,
        agg: &TimeSeriesAgg,
    ) -> Result<Vec<TimeSeriesPoint>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let grouped = self.group_by_interval(data, interval)?;
        let mut resampled = Vec::new();

        for (timestamp, values) in grouped {
            let aggregated_value = self.aggregate_values(&values, agg)?;
            resampled.push(TimeSeriesPoint {
                timestamp,
                value: aggregated_value,
            });
        }

        resampled.sort_by_key(|p| p.timestamp);
        Ok(resampled)
    }

    /// Group time series by interval
    fn group_by_interval(
        &self,
        data: &[TimeSeriesPoint],
        interval: &ResampleInterval,
    ) -> Result<HashMap<NaiveDateTime, Vec<f64>>> {
        let mut groups: HashMap<NaiveDateTime, Vec<f64>> = HashMap::new();

        for point in data {
            let key = self.get_interval_key(point.timestamp, interval);
            groups.entry(key).or_insert_with(Vec::new).push(point.value);
        }

        Ok(groups)
    }

    /// Get interval key for timestamp
    fn get_interval_key(
        &self,
        timestamp: NaiveDateTime,
        interval: &ResampleInterval,
    ) -> NaiveDateTime {
        match interval {
            ResampleInterval::Daily => timestamp.date().and_hms_opt(0, 0, 0).unwrap(),
            ResampleInterval::Weekly => {
                let week_start = timestamp.date()
                    - Duration::days(timestamp.weekday().num_days_from_sunday() as i64);
                week_start.and_hms_opt(0, 0, 0).unwrap()
            }
            ResampleInterval::Monthly => {
                NaiveDate::from_ymd_opt(timestamp.year(), timestamp.month(), 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
            }
            ResampleInterval::Quarterly => {
                let quarter = ((timestamp.month() - 1) / 3) + 1;
                let month = (quarter - 1) * 3 + 1;
                NaiveDate::from_ymd_opt(timestamp.year(), month, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
            }
            ResampleInterval::Yearly => NaiveDate::from_ymd_opt(timestamp.year(), 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            ResampleInterval::Hourly => timestamp
                .date()
                .and_hms_opt(timestamp.hour(), 0, 0)
                .unwrap(),
            ResampleInterval::Minute => timestamp
                .date()
                .and_hms_opt(timestamp.hour(), timestamp.minute(), 0)
                .unwrap(),
            ResampleInterval::Custom(duration) => {
                let epoch = NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
                    chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                );
                let duration_since_epoch = timestamp.signed_duration_since(epoch);
                let rounded_duration = (duration_since_epoch.num_seconds() as i64
                    / duration.num_seconds())
                    * duration.num_seconds();
                epoch + Duration::seconds(rounded_duration)
            }
        }
    }

    /// Aggregate values using specified function
    fn aggregate_values(&self, values: &[f64], agg: &TimeSeriesAgg) -> Result<f64> {
        if values.is_empty() {
            return Err(anyhow::anyhow!("Cannot aggregate empty values"));
        }

        match agg {
            TimeSeriesAgg::Sum => Ok(values.iter().sum()),
            TimeSeriesAgg::Mean => Ok(values.iter().sum::<f64>() / values.len() as f64),
            TimeSeriesAgg::Median => {
                let mut sorted = values.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let mid = sorted.len() / 2;
                if sorted.len() % 2 == 0 {
                    Ok((sorted[mid - 1] + sorted[mid]) / 2.0)
                } else {
                    Ok(sorted[mid])
                }
            }
            TimeSeriesAgg::Min => Ok(values.iter().fold(f64::INFINITY, |a, &b| a.min(b))),
            TimeSeriesAgg::Max => Ok(values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))),
            TimeSeriesAgg::First => Ok(values[0]),
            TimeSeriesAgg::Last => Ok(values[values.len() - 1]),
            TimeSeriesAgg::Count => Ok(values.len() as f64),
        }
    }

    /// Calculate rolling window statistics
    pub fn rolling_mean(
        &self,
        data: &[TimeSeriesPoint],
        window: &RollingWindow,
    ) -> Result<Vec<TimeSeriesPoint>> {
        if data.len() < window.min_periods {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();
        let _window_size_secs = window.window_size.num_seconds() as usize;

        for i in 0..data.len() {
            let current_time = data[i].timestamp;
            let window_start = current_time - Duration::seconds(window.window_size.num_seconds());

            // Find all points within the window
            let window_values: Vec<f64> = data
                .iter()
                .filter(|p| p.timestamp >= window_start && p.timestamp <= current_time)
                .map(|p| p.value)
                .collect();

            if window_values.len() >= window.min_periods {
                let mean = window_values.iter().sum::<f64>() / window_values.len() as f64;
                result.push(TimeSeriesPoint {
                    timestamp: current_time,
                    value: mean,
                });
            }
        }

        Ok(result)
    }

    /// Detect trend in time series
    pub fn detect_trend(&self, data: &[TimeSeriesPoint]) -> TrendDirection {
        if data.len() < 2 {
            return TrendDirection::Unknown;
        }

        // Simple linear regression to detect trend
        let n = data.len() as f64;
        let x_sum: f64 = (0..data.len()).map(|i| i as f64).sum();
        let y_sum: f64 = data.iter().map(|p| p.value).sum();
        let xy_sum: f64 = data
            .iter()
            .enumerate()
            .map(|(i, p)| i as f64 * p.value)
            .sum();
        let x2_sum: f64 = (0..data.len()).map(|i| (i as f64).powi(2)).sum();

        let slope = (n * xy_sum - x_sum * y_sum) / (n * x2_sum - x_sum.powi(2));

        if slope.abs() < 0.001 {
            TrendDirection::Stationary
        } else if slope > 0.0 {
            TrendDirection::Increasing
        } else {
            TrendDirection::Decreasing
        }
    }

    /// Calculate basic statistics
    pub fn calculate_stats(&self, data: &[TimeSeriesPoint]) -> Result<TimeSeriesStats> {
        if data.is_empty() {
            return Err(anyhow::anyhow!("Empty time series"));
        }

        let start_date = data[0].timestamp;
        let end_date = data[data.len() - 1].timestamp;
        let total_points = data.len();

        // Check for missing points (simplified - assumes daily data)
        let expected_points = (end_date - start_date).num_days() + 1;
        let missing_points = (expected_points as usize).saturating_sub(total_points);

        let trend_direction = self.detect_trend(data);
        let seasonality_detected = self.detect_seasonality(data);
        let autocorrelation = self.calculate_autocorrelation(data, 1);

        Ok(TimeSeriesStats {
            start_date,
            end_date,
            total_points,
            missing_points,
            trend_direction,
            seasonality_detected,
            autocorrelation,
        })
    }

    /// Simple seasonality detection
    fn detect_seasonality(&self, data: &[TimeSeriesPoint]) -> bool {
        if data.len() < 12 {
            return false;
        }

        // Simple approach: check if there's a pattern in monthly averages
        let mut monthly_data: HashMap<u32, Vec<f64>> = HashMap::new();

        for point in data {
            let month = point.timestamp.month();
            monthly_data
                .entry(month)
                .or_insert_with(Vec::new)
                .push(point.value);
        }

        // Calculate variance of monthly means
        let monthly_means: Vec<f64> = monthly_data
            .values()
            .map(|values| values.iter().sum::<f64>() / values.len() as f64)
            .collect();

        if monthly_means.len() < 2 {
            return false;
        }

        let mean = monthly_means.iter().sum::<f64>() / monthly_means.len() as f64;
        let variance = monthly_means
            .iter()
            .map(|m| (m - mean).powi(2))
            .sum::<f64>()
            / monthly_means.len() as f64;

        // If variance is significant relative to mean, assume seasonality
        variance > mean * 0.1
    }

    /// Calculate autocorrelation at given lag
    fn calculate_autocorrelation(&self, data: &[TimeSeriesPoint], lag: usize) -> f64 {
        if data.len() <= lag {
            return 0.0;
        }

        let values: Vec<f64> = data.iter().map(|p| p.value).collect();
        let n = values.len() - lag;

        let mean = values.iter().sum::<f64>() / values.len() as f64;

        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for i in 0..n {
            numerator += (values[i] - mean) * (values[i + lag] - mean);
        }

        for i in 0..values.len() {
            denominator += (values[i] - mean).powi(2);
        }

        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }

    /// Convert time series back to CSV format
    pub fn timeseries_to_csv(&self, data: &[TimeSeriesPoint]) -> Vec<Vec<String>> {
        let mut result = vec![vec!["timestamp".to_string(), "value".to_string()]];

        for point in data {
            result.push(vec![
                point.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
                point.value.to_string(),
            ]);
        }

        result
    }
}

impl Default for TimeSeriesProcessor {
    fn default() -> Self {
        Self::new("%Y-%m-%d")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_parse_date() {
        let processor = TimeSeriesProcessor::new("%Y-%m-%d");

        assert!(processor.parse_date("2023-01-01").is_ok());
        assert!(processor.parse_date("2023-01-01 12:00:00").is_ok());
    }

    #[test]
    fn test_detect_trend() {
        let processor = TimeSeriesProcessor::new("%Y-%m-%d");

        // Increasing trend
        let increasing_data: Vec<TimeSeriesPoint> = (0..10)
            .map(|i| TimeSeriesPoint {
                timestamp: NaiveDate::from_ymd_opt(2023, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    + Duration::days(i),
                value: i as f64,
            })
            .collect();

        assert!(matches!(
            processor.detect_trend(&increasing_data),
            TrendDirection::Increasing
        ));

        // Decreasing trend
        let decreasing_data: Vec<TimeSeriesPoint> = (0..10)
            .map(|i| TimeSeriesPoint {
                timestamp: NaiveDate::from_ymd_opt(2023, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    + Duration::days(i),
                value: (10 - i) as f64,
            })
            .collect();

        assert!(matches!(
            processor.detect_trend(&decreasing_data),
            TrendDirection::Decreasing
        ));
    }

    #[test]
    fn test_resample() {
        let processor = TimeSeriesProcessor::new("%Y-%m-%d");

        let data: Vec<TimeSeriesPoint> = (0..30)
            .map(|i| TimeSeriesPoint {
                timestamp: NaiveDate::from_ymd_opt(2023, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    + Duration::days(i),
                value: (i % 7) as f64,
            })
            .collect();

        let resampled = processor
            .resample(&data, &ResampleInterval::Weekly, &TimeSeriesAgg::Mean)
            .unwrap();

        assert!(!resampled.is_empty());
        assert!(resampled.len() < data.len());
    }
}
