//! Types for data profiling operations

use serde::{Deserialize, Serialize};

/// Column profile information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnProfile {
    pub name: String,
    pub data_type: DataType,
    pub null_count: usize,
    pub null_percentage: f64,
    pub unique_count: usize,
    pub unique_percentage: f64,
    pub distinct_values: Vec<String>,
    pub top_values: Vec<ValueFrequency>,
    pub length_stats: Option<LengthStats>,
    pub numeric_stats: Option<NumericStats>,
    pub date_stats: Option<DateStats>,
    pub text_stats: Option<TextStats>,
    pub quality_score: f64,
}

/// Data type classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    String,
    Integer,
    Float,
    Boolean,
    Date,
    DateTime,
    Email,
    Url,
    Phone,
    Unknown,
}

/// Value frequency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueFrequency {
    pub value: String,
    pub count: usize,
    pub percentage: f64,
}

/// Length statistics for text columns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthStats {
    pub min_length: usize,
    pub max_length: usize,
    pub avg_length: f64,
    pub median_length: usize,
    pub std_dev_length: f64,
}

/// Numeric statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericStats {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub mode: Vec<String>,
    pub std_dev: f64,
    pub variance: f64,
    pub q1: f64,
    pub q3: f64,
    pub iqr: f64,
    pub skewness: f64,
    pub kurtosis: f64,
}

/// Date statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateStats {
    pub min_date: String,
    pub max_date: String,
    pub date_range_days: i64,
    pub most_common_year: u32,
    pub most_common_month: u32,
    pub most_common_day_of_week: String,
}

/// Text statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStats {
    pub avg_word_count: f64,
    pub max_word_count: usize,
    pub min_word_count: usize,
    pub contains_numbers: bool,
    pub contains_special_chars: bool,
    pub all_uppercase: usize,
    pub all_lowercase: usize,
    pub title_case: usize,
    pub mixed_case: usize,
}

/// Overall data profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataProfile {
    pub file_path: String,
    pub total_rows: usize,
    pub total_columns: usize,
    pub total_cells: usize,
    pub null_cells: usize,
    pub null_percentage: f64,
    pub duplicate_rows: usize,
    pub duplicate_percentage: f64,
    pub columns: Vec<ColumnProfile>,
    pub data_quality_score: f64,
    pub recommendations: Vec<String>,
    pub profiling_timestamp: String,
}
