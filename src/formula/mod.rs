//! Formula evaluation module
//!
//! Supports Excel-like formulas: SUM, AVERAGE, MIN, MAX, COUNT, IF, CONCAT, VLOOKUP, etc.

mod evaluator;
mod functions;
mod parser;
mod types;

pub use evaluator::FormulaEvaluator;

// Public API export - unused internally but part of library interface
#[allow(unused_imports)]
pub use types::FormulaResult;
