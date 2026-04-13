//! Anomaly detection operations
//!
//! Provides statistical anomaly detection using methods like Z-score,
//! IQR (Interquartile Range), and isolation forest.

use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Anomaly detection method
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AnomalyMethod {
    ZScore { threshold: f64 },
    IQR { multiplier: f64 },
    Percentile { lower: f64, upper: f64 },
}

/// Anomaly detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyResult {
    pub anomalies: Vec<Anomaly>,
    pub total_anomalies: usize,
    pub anomaly_percentage: f64,
}

/// Individual anomaly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub row: usize,
    pub column: String,
    pub value: String,
    pub score: f64,
    pub reason: String,
}

/// Anomaly detector
pub struct AnomalyDetector {
    method: AnomalyMethod,
}

impl AnomalyDetector {
    pub fn new(method: AnomalyMethod) -> Self {
        Self { method }
    }

    /// Detect anomalies in a column
    pub fn detect(&self, data: &[Vec<String>], column: usize) -> Result<AnomalyResult> {
        if data.is_empty() || column >= data[0].len() {
            return Ok(AnomalyResult {
                anomalies: Vec::new(),
                total_anomalies: 0,
                anomaly_percentage: 0.0,
            });
        }

        // Extract numeric values
        let values: Vec<f64> = data
            .iter()
            .skip(1) // Skip header
            .filter_map(|row| row.get(column))
            .filter_map(|v| v.parse::<f64>().ok())
            .collect();

        if values.is_empty() {
            return Ok(AnomalyResult {
                anomalies: Vec::new(),
                total_anomalies: 0,
                anomaly_percentage: 0.0,
            });
        }

        let anomalies = match self.method {
            AnomalyMethod::ZScore { threshold } => {
                self.detect_zscore(&values, column, threshold)?
            }
            AnomalyMethod::IQR { multiplier } => self.detect_iqr(&values, column, multiplier)?,
            AnomalyMethod::Percentile { lower, upper } => {
                self.detect_percentile(&values, column, lower, upper)?
            }
        };

        let total_anomalies = anomalies.len();
        let anomaly_percentage = (total_anomalies as f64 / values.len() as f64) * 100.0;

        Ok(AnomalyResult {
            anomalies,
            total_anomalies,
            anomaly_percentage,
        })
    }

    fn detect_zscore(&self, values: &[f64], column: usize, threshold: f64) -> Result<Vec<Anomaly>> {
        let mean = values.par_iter().sum::<f64>() / values.len() as f64;
        let variance = values.par_iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();

        if std_dev == 0.0 {
            return Ok(Vec::new());
        }

        let anomalies: Vec<Anomaly> = values
            .par_iter()
            .enumerate()
            .filter_map(|(idx, value)| {
                let z_score = (value - mean).abs() / std_dev;
                if z_score > threshold {
                    Some(Anomaly {
                        row: idx + 1, // +1 for header
                        column: format!("col_{column}"),
                        value: value.to_string(),
                        score: z_score,
                        reason: format!("Z-score {z_score:.2} exceeds threshold {threshold:.2}"),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(anomalies)
    }

    fn detect_iqr(&self, values: &[f64], column: usize, multiplier: f64) -> Result<Vec<Anomaly>> {
        let mut sorted = values.to_vec();
        sorted.par_sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let q1_idx = sorted.len() / 4;
        let q3_idx = (sorted.len() * 3) / 4;

        let q1 = sorted[q1_idx];
        let q3 = sorted[q3_idx];
        let iqr = q3 - q1;

        let lower_bound = q1 - multiplier * iqr;
        let upper_bound = q3 + multiplier * iqr;

        let anomalies: Vec<Anomaly> = values
            .par_iter()
            .enumerate()
            .filter_map(|(idx, value)| {
                if *value < lower_bound || *value > upper_bound {
                    let reason = if *value < lower_bound {
                        format!("Value {value:.2} below lower bound {lower_bound:.2}")
                    } else {
                        format!("Value {value:.2} above upper bound {upper_bound:.2}")
                    };

                    Some(Anomaly {
                        row: idx + 1,
                        column: format!("col_{column}"),
                        value: value.to_string(),
                        score: if *value < lower_bound {
                            (lower_bound - value) / iqr
                        } else {
                            (value - upper_bound) / iqr
                        },
                        reason,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(anomalies)
    }

    fn detect_percentile(
        &self,
        values: &[f64],
        column: usize,
        lower: f64,
        upper: f64,
    ) -> Result<Vec<Anomaly>> {
        let mut sorted = values.to_vec();
        sorted.par_sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let lower_idx = (sorted.len() as f64 * lower / 100.0) as usize;
        let upper_idx = (sorted.len() as f64 * upper / 100.0) as usize;

        let lower_bound = sorted[lower_idx];
        let upper_bound = sorted[upper_idx];

        let anomalies: Vec<Anomaly> = values
            .par_iter()
            .enumerate()
            .filter_map(|(idx, value)| {
                if *value < lower_bound || *value > upper_bound {
                    Some(Anomaly {
                        row: idx + 1,
                        column: format!("col_{column}"),
                        value: value.to_string(),
                        score: 1.0,
                        reason: format!("Value outside {lower:.1}%-{upper:.1}% percentile range"),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(anomalies)
    }
}
