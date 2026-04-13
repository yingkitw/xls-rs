//! Data operations module
//!
//! Provides pandas-inspired data manipulation operations.

mod core;
mod diff;
mod histogram;
mod pandas;
mod stats;
mod transform;
pub mod types;

pub use core::DataOperations;
pub use diff::{diff, ChangedRow, DiffResult};
pub use histogram::{histogram, render_histogram};
pub use types::{AggFunc, JoinType, SortOrder};
#[allow(unused_imports)]
pub use types::{NoProgress, ProgressCallback, StderrProgress};
