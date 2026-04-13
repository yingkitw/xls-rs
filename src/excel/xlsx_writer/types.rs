//! Data types for XLSX writer

/// Cell data type for writing
#[derive(Debug, Clone)]
pub enum CellData {
    String(String),
    Number(f64),
    Formula(String),
    Empty,
}

/// Row data for writing
#[derive(Debug, Clone)]
pub struct RowData {
    pub cells: Vec<CellData>,
}

impl RowData {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
        }
    }

    pub fn add_string(&mut self, value: &str) {
        self.cells.push(CellData::String(value.to_string()));
    }

    pub fn add_number(&mut self, value: f64) {
        self.cells.push(CellData::Number(value));
    }

    pub fn add_formula(&mut self, formula: &str) {
        self.cells.push(CellData::Formula(formula.to_string()));
    }

    pub fn add_empty(&mut self) {
        self.cells.push(CellData::Empty);
    }
}

/// Sheet data structure
pub struct SheetData {
    pub name: String,
    pub rows: Vec<RowData>,
    pub column_widths: Vec<f64>,
    pub conditional_formats: Vec<super::cond_fmt_xml::ConditionalFormat>,
    pub sparkline_groups: Vec<super::sparkline_xml::SparklineGroup>,
}
