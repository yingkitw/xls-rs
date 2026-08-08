//! Data types for XLSX writer

/// Cell data type for writing
#[derive(Debug, Clone)]
pub enum CellData {
    String(String),
    Number(f64),
    Bool(bool),
    Formula(String),
    Empty,
}

/// Row data for writing
#[derive(Debug, Clone)]
pub struct RowData {
    pub cells: Vec<CellData>,
    /// Per-cell style index into the workbook's `StyleRegistry`. The
    /// vector is aligned with `cells`; a `None` (or missing entry)
    /// means "use the workbook default style". Only emitted on `<c>`
    /// when the value is `Some(idx) && idx != 0`.
    pub cell_styles: Vec<Option<u32>>,
}

impl Default for RowData {
    fn default() -> Self {
        Self::new()
    }
}

impl RowData {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            cell_styles: Vec::new(),
        }
    }

    pub fn add_string(&mut self, value: &str) {
        self.cells.push(CellData::String(value.to_string()));
        self.cell_styles.push(None);
    }

    pub fn add_number(&mut self, value: f64) {
        self.cells.push(CellData::Number(value));
        self.cell_styles.push(None);
    }

    pub fn add_formula(&mut self, formula: impl Into<String>) {
        self.cells.push(CellData::Formula(formula.into()));
        self.cell_styles.push(None);
    }

    pub fn add_bool(&mut self, value: bool) {
        self.cells.push(CellData::Bool(value));
        self.cell_styles.push(None);
    }

    pub fn add_empty(&mut self) {
        self.cells.push(CellData::Empty);
        self.cell_styles.push(None);
    }

    /// Attach a style index (returned from
    /// `XlsxWriter::register_cell_style`) to the cell in column
    /// `col_idx` (0-based). Panics if `col_idx` is out of bounds or
    /// refers to an `Empty` cell — styles on empty cells are dropped.
    pub fn set_cell_style(&mut self, col_idx: usize, style_idx: u32) {
        if col_idx >= self.cell_styles.len() {
            panic!(
                "set_cell_style: column {col_idx} is out of bounds (row has {} cells)",
                self.cell_styles.len()
            );
        }
        if matches!(self.cells[col_idx], CellData::Empty) {
            return;
        }
        self.cell_styles[col_idx] = Some(style_idx);
    }

    /// Convenience: style the just-appended cell.
    pub fn style_last(&mut self, style_idx: u32) {
        let last = self.cell_styles.len().saturating_sub(1);
        if last < self.cells.len() && !matches!(self.cells[last], CellData::Empty) {
            self.cell_styles[last] = Some(style_idx);
        }
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

/// Row/column outline grouping
#[derive(Debug, Clone)]
pub struct RowGroup {
    pub start_row: usize, // 0-based, inclusive
    pub end_row: usize,   // 0-based, inclusive
    pub level: u8,        // outline level (1-7)
    pub collapsed: bool,
}

/// Column outline grouping
#[derive(Debug, Clone)]
pub struct ColGroup {
    pub start_col: usize, // 0-based, inclusive
    pub end_col: usize,   // 0-based, inclusive
    pub level: u8,        // outline level (1-7)
    pub collapsed: bool,
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
    pub row_groups: Vec<RowGroup>,
    pub col_groups: Vec<ColGroup>,
    pub tables: Vec<Table>,
}

/// Excel structured table (auto-expanding range with headers, banded rows, etc.)
#[derive(Debug, Clone)]
pub struct Table {
    /// Display name (must be unique in the workbook, no spaces)
    pub name: String,
    /// 0-based start row (header row)
    pub start_row: usize,
    /// 0-based start column
    pub start_col: usize,
    /// 0-based end row (last data row, inclusive)
    pub end_row: usize,
    /// 0-based end column (inclusive)
    pub end_col: usize,
    /// Column names. If empty, auto-generated from the first row or default names.
    pub column_names: Vec<String>,
    /// Show banded rows (alternating row colors)
    pub show_banded_rows: bool,
    /// Show banded columns (alternating column colors)
    pub show_banded_columns: bool,
    /// Show filter button in header
    pub show_filter_button: bool,
    /// Show totals row
    pub show_totals_row: bool,
    /// Style info (built-in table style name)
    pub style: Option<TableStyleInfo>,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            name: String::new(),
            start_row: 0,
            start_col: 0,
            end_row: 0,
            end_col: 0,
            column_names: Vec::new(),
            show_banded_rows: true,
            show_banded_columns: false,
            show_filter_button: true,
            show_totals_row: false,
            style: Some(TableStyleInfo::default()),
        }
    }
}

/// Built-in table style info
#[derive(Debug, Clone)]
pub struct TableStyleInfo {
    /// Style name, e.g. "TableStyleMedium2"
    pub name: String,
    /// Show first column emphasis
    pub show_first_column: bool,
    /// Show last column emphasis
    pub show_last_column: bool,
}

impl Default for TableStyleInfo {
    fn default() -> Self {
        Self {
            name: "TableStyleMedium2".to_string(),
            show_first_column: false,
            show_last_column: false,
        }
    }
}
