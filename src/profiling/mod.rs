//! Data profiling operations
//!
//! Provides comprehensive data profiling capabilities including
//! statistical analysis, data quality metrics, and insights.

pub mod analysis;
pub mod profiler;
pub mod quality;
pub mod types;

// Re-export main types for convenience
pub use profiler::DataProfiler;
pub use types::*;
