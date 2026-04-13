//! Formula types

/// Result of formula evaluation - can be number or string
#[derive(Debug, Clone)]
pub enum FormulaResult {
    Number(f64),
    Text(String),
}

impl std::fmt::Display for FormulaResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormulaResult::Number(n) => write!(f, "{}", n),
            FormulaResult::Text(s) => write!(f, "{}", s),
        }
    }
}

impl FormulaResult {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            FormulaResult::Number(n) => Some(*n),
            FormulaResult::Text(s) => s.parse().ok(),
        }
    }
}

/// Internal cell range representation
#[derive(Clone)]
pub(crate) struct CellRange {
    pub start_row: u32,
    pub start_col: u16,
    pub end_row: u32,
    pub end_col: u16,
}
