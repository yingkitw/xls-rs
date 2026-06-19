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

/// Merged cell range (0-based indices, inclusive)
#[derive(Debug, Clone)]
pub struct MergeCell {
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
}

/// Data validation rule
#[derive(Debug, Clone)]
pub struct DataValidation {
    pub range: String, // e.g. "A1:A10"
    pub validation_type: ValidationType,
    pub allow_blank: bool,
    pub show_dropdown: bool,
    pub error_title: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ValidationType {
    List { source: String },               // comma-separated or formula
    Whole { operator: Operator, formula1: String, formula2: Option<String> },
    Decimal { operator: Operator, formula1: String, formula2: Option<String> },
    Date { operator: Operator, formula1: String, formula2: Option<String> },
    TextLength { operator: Operator, formula1: String },
    Custom { formula: String },
}

#[derive(Debug, Clone, Copy)]
pub enum Operator {
    Between,
    NotBetween,
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

/// Hyperlink in a cell
#[derive(Debug, Clone)]
pub struct Hyperlink {
    pub cell_ref: String, // e.g. "A1"
    pub url: String,
    pub tooltip: Option<String>,
}

/// Print setup options
#[derive(Debug, Clone, Default)]
pub struct PrintSetup {
    pub orientation: Option<PageOrientation>,
    pub paper_size: Option<u16>, // e.g. 1=Letter, 9=A4
    pub scale: Option<u16>,      // 10..400 (percent)
    pub fit_to_width: Option<u16>,
    pub fit_to_height: Option<u16>,
    pub print_area: Option<String>, // e.g. "A1:D100"
    pub margins: Option<PageMargins>,
}

#[derive(Debug, Clone, Copy)]
pub enum PageOrientation {
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Copy)]
pub struct PageMargins {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
    pub header: f64,
    pub footer: f64,
}

impl Default for PageMargins {
    fn default() -> Self {
        Self {
            left: 0.75,
            right: 0.75,
            top: 1.0,
            bottom: 1.0,
            header: 0.5,
            footer: 0.5,
        }
    }
}

/// Cell comment
#[derive(Debug, Clone)]
pub struct CellComment {
    pub cell_ref: String, // e.g. "A1"
    pub text: String,
    pub author: Option<String>,
}

/// Sheet data structure
pub struct SheetData {
    pub name: String,
    pub rows: Vec<RowData>,
    pub column_widths: Vec<f64>,
    pub conditional_formats: Vec<super::cond_fmt_xml::ConditionalFormat>,
    pub sparkline_groups: Vec<super::sparkline_xml::SparklineGroup>,
    pub merge_cells: Vec<MergeCell>,
    pub data_validations: Vec<DataValidation>,
    pub hyperlinks: Vec<Hyperlink>,
    pub print_setup: Option<PrintSetup>,
    pub comments: Vec<CellComment>,
}
