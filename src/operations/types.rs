//! Type definitions for operations

use anyhow::Result;

/// Progress callback for long-running operations
pub trait ProgressCallback: Send {
    fn on_progress(&mut self, current: usize, total: Option<usize>, message: &str);
}

/// Simple progress reporter that prints to stderr
pub struct StderrProgress {
    last_percent: usize,
}

impl StderrProgress {
    pub fn new() -> Self {
        Self { last_percent: 0 }
    }
}

impl ProgressCallback for StderrProgress {
    fn on_progress(&mut self, current: usize, total: Option<usize>, message: &str) {
        if let Some(total) = total {
            let percent = if total > 0 {
                (current * 100) / total
            } else {
                0
            };
            if percent != self.last_percent {
                eprintln!("\r{}: {}% ({}/{})", message, percent, current, total);
                self.last_percent = percent;
            }
        } else {
            eprintln!("\r{}: {} processed", message, current);
        }
    }
}

/// No-op progress callback
pub struct NoProgress;

impl ProgressCallback for NoProgress {
    fn on_progress(&mut self, _current: usize, _total: Option<usize>, _message: &str) {}
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

/// Join type for merge operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Outer,
}

impl JoinType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "inner" => Ok(JoinType::Inner),
            "left" => Ok(JoinType::Left),
            "right" => Ok(JoinType::Right),
            "outer" | "full" => Ok(JoinType::Outer),
            _ => anyhow::bail!("Unknown join type: {}. Use: inner, left, right, outer", s),
        }
    }
}

/// Aggregation functions for groupby
#[derive(Debug, Clone, Copy)]
pub enum AggFunc {
    Sum,
    Count,
    Mean,
    Min,
    Max,
}

impl AggFunc {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "sum" => Ok(AggFunc::Sum),
            "count" => Ok(AggFunc::Count),
            "mean" | "avg" | "average" => Ok(AggFunc::Mean),
            "min" => Ok(AggFunc::Min),
            "max" => Ok(AggFunc::Max),
            _ => anyhow::bail!(
                "Unknown aggregation: {}. Use: sum, count, mean, min, max",
                s
            ),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            AggFunc::Sum => "sum",
            AggFunc::Count => "count",
            AggFunc::Mean => "mean",
            AggFunc::Min => "min",
            AggFunc::Max => "max",
        }
    }

    pub fn apply(&self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        match self {
            AggFunc::Sum => values.iter().sum(),
            AggFunc::Count => values.len() as f64,
            AggFunc::Mean => values.iter().sum::<f64>() / values.len() as f64,
            AggFunc::Min => values.iter().cloned().fold(f64::INFINITY, f64::min),
            AggFunc::Max => values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        }
    }
}
