//! Statistical analysis methods for data profiling

use crate::common::string;
use chrono::{Datelike, NaiveDate};
use std::collections::HashMap;

use super::types::*;

impl super::profiler::DataProfiler {
    /// Infer data type from sample values
    pub fn infer_data_type(&self, data: &[String]) -> DataType {
        let non_null_values: Vec<&str> = data
            .iter()
            .filter(|v| !string::is_empty_or_whitespace(v))
            .map(|v| v.as_str())
            .collect();

        if non_null_values.is_empty() {
            return DataType::Unknown;
        }

        let sample_size = non_null_values.len().min(100);
        let sample = &non_null_values[..sample_size];

        // Check for boolean
        let boolean_count = sample
            .iter()
            .filter(|v| {
                matches!(
                    v.to_lowercase().as_str(),
                    "true" | "false" | "1" | "0" | "yes" | "no"
                )
            })
            .count();

        if boolean_count as f64 / sample_size as f64 > 0.8 {
            return DataType::Boolean;
        }

        // Check for email
        let email_regex =
            regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        let email_count = sample.iter().filter(|v| email_regex.is_match(v)).count();

        if email_count as f64 / sample_size as f64 > 0.8 {
            return DataType::Email;
        }

        // Check for URL
        let url_regex = regex::Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap();
        let url_count = sample.iter().filter(|v| url_regex.is_match(v)).count();

        if url_count as f64 / sample_size as f64 > 0.8 {
            return DataType::Url;
        }

        // Check for phone
        let phone_regex = regex::Regex::new(r"^\+?[\d\s\-\(\)]{10,}$").unwrap();
        let phone_count = sample.iter().filter(|v| phone_regex.is_match(v)).count();

        if phone_count as f64 / sample_size as f64 > 0.8 {
            return DataType::Phone;
        }

        // Check for date/datetime
        let date_formats = vec![
            "%Y-%m-%d",
            "%d/%m/%Y",
            "%m/%d/%Y",
            "%Y-%m-%d %H:%M:%S",
            "%d/%m/%Y %H:%M:%S",
        ];

        for format in &date_formats {
            let date_count = sample
                .iter()
                .filter(|v| {
                    chrono::NaiveDate::parse_from_str(v, format).is_ok()
                        || chrono::NaiveDateTime::parse_from_str(v, format).is_ok()
                })
                .count();

            if date_count as f64 / sample_size as f64 > 0.8 {
                return if format.contains("%H") {
                    DataType::DateTime
                } else {
                    DataType::Date
                };
            }
        }

        // Check for numeric
        let numeric_count = sample.iter().filter(|v| string::is_numeric(v)).count();

        if numeric_count as f64 / sample_size as f64 > 0.8 {
            // Check if all are integers
            let int_count = sample.iter().filter(|v| v.parse::<i64>().is_ok()).count();

            return if int_count as f64 / numeric_count as f64 > 0.8 {
                DataType::Integer
            } else {
                DataType::Float
            };
        }

        DataType::String
    }

    /// Get value frequencies
    pub fn get_value_frequencies(&self, data: &[String]) -> Vec<ValueFrequency> {
        let mut frequency_map: HashMap<String, usize> = HashMap::new();
        let total = data.len();

        for value in data {
            if !string::is_empty_or_whitespace(value) {
                *frequency_map.entry(value.clone()).or_insert(0) += 1;
            }
        }

        let mut frequencies: Vec<ValueFrequency> = frequency_map
            .into_iter()
            .map(|(value, count)| ValueFrequency {
                value,
                count,
                percentage: (count as f64 / total as f64) * 100.0,
            })
            .collect();

        // Sort by count (descending), then by value (ascending) for deterministic ordering
        frequencies.sort_by(|a, b| match b.count.cmp(&a.count) {
            std::cmp::Ordering::Equal => a.value.cmp(&b.value),
            other => other,
        });
        frequencies.truncate(10); // Top 10 values
        frequencies
    }

    /// Calculate length statistics
    pub fn calculate_length_stats(&self, data: &[String]) -> LengthStats {
        let lengths: Vec<usize> = data
            .iter()
            .filter(|v| !string::is_empty_or_whitespace(v))
            .map(|v| v.len())
            .collect();

        if lengths.is_empty() {
            return LengthStats {
                min_length: 0,
                max_length: 0,
                avg_length: 0.0,
                median_length: 0,
                std_dev_length: 0.0,
            };
        }

        let min_length = *lengths.iter().min().unwrap();
        let max_length = *lengths.iter().max().unwrap();
        let avg_length = lengths.iter().sum::<usize>() as f64 / lengths.len() as f64;

        let mut sorted_lengths = lengths.clone();
        sorted_lengths.sort_unstable();
        let median_length = if sorted_lengths.len() % 2 == 0 {
            let mid = sorted_lengths.len() / 2;
            (sorted_lengths[mid - 1] + sorted_lengths[mid]) / 2
        } else {
            sorted_lengths[sorted_lengths.len() / 2]
        };

        let variance = lengths
            .iter()
            .map(|&len| (len as f64 - avg_length).powi(2))
            .sum::<f64>()
            / lengths.len() as f64;
        let std_dev_length = variance.sqrt();

        LengthStats {
            min_length,
            max_length,
            avg_length,
            median_length,
            std_dev_length,
        }
    }

    /// Calculate numeric statistics
    pub fn calculate_numeric_stats(&self, data: &[String]) -> Option<NumericStats> {
        let numbers: Vec<f64> = data
            .iter()
            .filter(|v| !string::is_empty_or_whitespace(v))
            .filter_map(|v| string::to_number(v))
            .collect();

        if numbers.is_empty() {
            return None;
        }

        let min = numbers.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = numbers.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let mean = numbers.iter().sum::<f64>() / numbers.len() as f64;

        let mut sorted_numbers = numbers.clone();
        sorted_numbers.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = if sorted_numbers.len() % 2 == 0 {
            let mid = sorted_numbers.len() / 2;
            (sorted_numbers[mid - 1] + sorted_numbers[mid]) / 2.0
        } else {
            sorted_numbers[sorted_numbers.len() / 2]
        };

        // Calculate mode
        let mut frequency_map: HashMap<i64, usize> = HashMap::new();
        for &num in &numbers {
            let rounded = num.round() as i64;
            *frequency_map.entry(rounded).or_insert(0) += 1;
        }

        let max_freq = frequency_map.values().max().unwrap();
        let mode: Vec<String> = frequency_map
            .iter()
            .filter(|&(_, &freq)| freq == *max_freq)
            .map(|(val, _)| val.to_string())
            .collect();

        let variance =
            numbers.iter().map(|&num| (num - mean).powi(2)).sum::<f64>() / numbers.len() as f64;
        let std_dev = variance.sqrt();

        // Calculate quartiles
        let q1_idx = (sorted_numbers.len() as f64 * 0.25) as usize;
        let q3_idx = (sorted_numbers.len() as f64 * 0.75) as usize;
        let q1 = sorted_numbers[q1_idx];
        let q3 = sorted_numbers[q3_idx];
        let iqr = q3 - q1;

        // Calculate skewness and kurtosis
        let skewness = if std_dev > 0.0 {
            numbers
                .iter()
                .map(|&num| ((num - mean) / std_dev).powi(3))
                .sum::<f64>()
                / numbers.len() as f64
        } else {
            0.0
        };

        let kurtosis = if std_dev > 0.0 {
            numbers
                .iter()
                .map(|&num| ((num - mean) / std_dev).powi(4))
                .sum::<f64>()
                / numbers.len() as f64
                - 3.0 // Excess kurtosis
        } else {
            0.0
        };

        Some(NumericStats {
            min,
            max,
            mean,
            median,
            mode,
            std_dev,
            variance,
            q1,
            q3,
            iqr,
            skewness,
            kurtosis,
        })
    }

    /// Calculate date statistics
    pub fn calculate_date_stats(&self, data: &[String]) -> Option<DateStats> {
        let dates: Vec<NaiveDate> = data
            .iter()
            .filter(|v| !string::is_empty_or_whitespace(v))
            .filter_map(|v| {
                // Try different date formats
                if let Ok(date) = NaiveDate::parse_from_str(v, "%Y-%m-%d") {
                    Some(date)
                } else if let Ok(date) = NaiveDate::parse_from_str(v, "%d/%m/%Y") {
                    Some(date)
                } else if let Ok(date) = NaiveDate::parse_from_str(v, "%m/%d/%Y") {
                    Some(date)
                } else {
                    None
                }
            })
            .collect();

        if dates.is_empty() {
            return None;
        }

        let min_date = dates.iter().min().unwrap();
        let max_date = dates.iter().max().unwrap();
        let date_range_days = (max_date.signed_duration_since(*min_date)).num_days();

        // Most common year, month, and day of week
        let mut year_count: HashMap<u32, usize> = HashMap::new();
        let mut month_count: HashMap<u32, usize> = HashMap::new();
        let mut dow_count: HashMap<String, usize> = HashMap::new();

        for date in &dates {
            *year_count.entry(date.year() as u32).or_insert(0) += 1;
            *month_count.entry(date.month()).or_insert(0) += 1;

            let dow = match date.weekday() {
                chrono::Weekday::Mon => "Monday",
                chrono::Weekday::Tue => "Tuesday",
                chrono::Weekday::Wed => "Wednesday",
                chrono::Weekday::Thu => "Thursday",
                chrono::Weekday::Fri => "Friday",
                chrono::Weekday::Sat => "Saturday",
                chrono::Weekday::Sun => "Sunday",
            };
            *dow_count.entry(dow.to_string()).or_insert(0) += 1;
        }

        let most_common_year = year_count
            .iter()
            .max_by_key(|&(_, &count)| count)
            .map(|(&year, _)| year)
            .unwrap_or(0);

        let most_common_month = month_count
            .iter()
            .max_by_key(|&(_, &count)| count)
            .map(|(&month, _)| month)
            .unwrap_or(0);

        let most_common_day_of_week = dow_count
            .iter()
            .max_by_key(|&(_, &count)| count)
            .map(|(dow, _)| dow.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        Some(DateStats {
            min_date: min_date.format("%Y-%m-%d").to_string(),
            max_date: max_date.format("%Y-%m-%d").to_string(),
            date_range_days,
            most_common_year,
            most_common_month,
            most_common_day_of_week,
        })
    }

    /// Calculate text statistics
    pub fn calculate_text_stats(&self, data: &[String]) -> TextStats {
        let non_empty: Vec<&str> = data
            .iter()
            .filter(|v| !string::is_empty_or_whitespace(v))
            .map(|v| v.as_str())
            .collect();

        if non_empty.is_empty() {
            return TextStats {
                avg_word_count: 0.0,
                max_word_count: 0,
                min_word_count: 0,
                contains_numbers: false,
                contains_special_chars: false,
                all_uppercase: 0,
                all_lowercase: 0,
                title_case: 0,
                mixed_case: 0,
            };
        }

        let word_counts: Vec<usize> = non_empty
            .iter()
            .map(|text| text.split_whitespace().count())
            .collect();

        let max_word_count = *word_counts.iter().max().unwrap();
        let min_word_count = *word_counts.iter().min().unwrap();
        let avg_word_count = word_counts.iter().sum::<usize>() as f64 / word_counts.len() as f64;

        let contains_numbers = non_empty
            .iter()
            .any(|text| text.chars().any(|c| c.is_numeric()));
        let contains_special_chars = non_empty.iter().any(|text| {
            text.chars()
                .any(|c| !c.is_alphanumeric() && !c.is_whitespace())
        });

        let mut all_uppercase = 0;
        let mut all_lowercase = 0;
        let mut title_case = 0;
        let mut mixed_case = 0;

        for text in &non_empty {
            if text.chars().all(|c| !c.is_alphabetic() || c.is_uppercase())
                && text.chars().any(|c| c.is_alphabetic())
            {
                all_uppercase += 1;
            } else if text.chars().all(|c| !c.is_alphabetic() || c.is_lowercase())
                && text.chars().any(|c| c.is_alphabetic())
            {
                all_lowercase += 1;
            } else if text.chars().next().map_or(false, |c| c.is_uppercase())
                && text
                    .chars()
                    .skip(1)
                    .all(|c| !c.is_alphabetic() || c.is_lowercase())
            {
                title_case += 1;
            } else if text.chars().any(|c| c.is_alphabetic()) {
                mixed_case += 1;
            }
        }

        TextStats {
            avg_word_count,
            max_word_count,
            min_word_count,
            contains_numbers,
            contains_special_chars,
            all_uppercase,
            all_lowercase,
            title_case,
            mixed_case,
        }
    }
}
