//! Formula types

/// Result of formula evaluation - number, string, or boolean
#[derive(Debug, Clone)]
pub enum FormulaResult {
    Number(f64),
    Text(String),
    Bool(bool),
}

impl std::fmt::Display for FormulaResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormulaResult::Number(n) => write!(f, "{}", n),
            FormulaResult::Text(s) => write!(f, "{}", s),
            // Excel renders booleans as TRUE / FALSE
            FormulaResult::Bool(b) => write!(f, "{}", if *b { "TRUE" } else { "FALSE" }),
        }
    }
}

impl FormulaResult {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            FormulaResult::Number(n) => Some(*n),
            FormulaResult::Text(s) => s.parse().ok(),
            // Excel coercion: TRUE = 1, FALSE = 0
            FormulaResult::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
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
