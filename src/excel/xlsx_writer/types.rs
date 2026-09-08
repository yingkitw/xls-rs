//! Data types for XLSX writer

/// Cell data type for writing
#[derive(Debug, Clone)]
pub enum CellData {
    String(String),
    Number(f64),
    Bool(bool),
    Formula(String),
    /// Legacy CSE / dynamic-array formula: `<f t="array" ref="B2:B4">…</f>`
    /// on the anchor cell. `reference` is the spill range in A1 notation.
    ArrayFormula {
        formula: String,
        reference: String,
    },
    /// Rich text — multiple formatted runs in a single cell. Emitted as
    /// `<is><r><rPr>…</rPr><t>…</t></r>…</is>` (inline rich string).
    RichText(Vec<RichTextRun>),
    Empty,
}

/// A single run within a rich-text cell. The `text` is the literal
/// content; `style` carries optional per-run formatting that maps to
/// `<rPr>` properties (bold, italic, color, font, size).
#[derive(Debug, Clone)]
pub struct RichTextRun {
    pub text: String,
    pub style: RichTextRunStyle,
}

impl RichTextRun {
    /// Create a plain (unformatted) run.
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: RichTextRunStyle::default(),
        }
    }

    /// Create a bold run.
    pub fn bold(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: RichTextRunStyle {
                bold: true,
                ..Default::default()
            },
        }
    }

    /// Create an italic run.
    pub fn italic(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: RichTextRunStyle {
                italic: true,
                ..Default::default()
            },
        }
    }
}

/// Formatting for a single rich-text run. All fields default to
/// `false`/`None` (no formatting). Maps to `<rPr>` child elements.
#[derive(Debug, Clone, Default)]
pub struct RichTextRunStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub font_size: Option<f64>,
    pub font_name: Option<String>,
    /// Hex RGB color without alpha, e.g. "FF0000" for red.
    pub color: Option<String>,
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
    }

    /// Add an array (CSE) formula anchored at this cell, spilling over
    /// `reference` (e.g. `"B2:B4"`). The formula is stored without its
    /// surrounding braces; Excel computes the spilled values.
    pub fn add_array_formula(&mut self, formula: impl Into<String>, reference: impl Into<String>) {
        self.cells.push(CellData::ArrayFormula {
            formula: formula.into(),
            reference: reference.into(),
        });
    }

    pub fn add_bool(&mut self, value: bool) {
        self.cells.push(CellData::Bool(value));
        self.cell_styles.push(None);
    }

    pub fn add_empty(&mut self) {
        self.cells.push(CellData::Empty);
        self.cell_styles.push(None);
    }

    /// Add a rich-text cell (multiple formatted runs). The cell is
    /// emitted as an inline rich string (`<is><r>…</r>…</is>`).
    pub fn add_rich_text(&mut self, runs: Vec<RichTextRun>) {
        self.cells.push(CellData::RichText(runs));
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
    List {
        source: String,
    }, // comma-separated or formula
    Whole {
        operator: Operator,
        formula1: String,
        formula2: Option<String>,
    },
    Decimal {
        operator: Operator,
        formula1: String,
        formula2: Option<String>,
    },
    Date {
        operator: Operator,
        formula1: String,
        formula2: Option<String>,
    },
    TextLength {
        operator: Operator,
        formula1: String,
    },
    Custom {
        formula: String,
    },
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
    pub images: Vec<Image>,
    pub sheet_protection: Option<SheetProtection>,
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

/// Supported embedded image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    Bmp,
}

impl ImageFormat {
    /// File extension used inside `xl/media/` (without the dot).
    pub fn extension(self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpeg",
            ImageFormat::Gif => "gif",
            ImageFormat::Bmp => "bmp",
        }
    }

    /// MIME content type for `[Content_Types].xml` `<Default>` entries.
    pub fn content_type(self) -> &'static str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Gif => "image/gif",
            ImageFormat::Bmp => "image/bmp",
        }
    }

    /// Detect the format from image header magic bytes. Returns `None`
    /// when the bytes do not match a known signature.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() >= 8 && bytes[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
            return Some(ImageFormat::Png);
        }
        if bytes.len() >= 3 && &bytes[..3] == b"\xFF\xD8\xFF" {
            return Some(ImageFormat::Jpeg);
        }
        if bytes.len() >= 6 && (&bytes[..6] == b"GIF87a" || &bytes[..6] == b"GIF89a") {
            return Some(ImageFormat::Gif);
        }
        if bytes.len() >= 2 && &bytes[..2] == b"BM" {
            return Some(ImageFormat::Bmp);
        }
        None
    }
}

/// An embedded image anchored to a worksheet cell.
///
/// The image bytes are stored verbatim and written to `xl/media/imageN.ext`.
/// Pixel dimensions are auto-detected from the header when not supplied;
/// callers may override with explicit EMU dimensions for precise placement.
#[derive(Debug, Clone)]
pub struct Image {
    /// Raw image bytes (PNG/JPEG/GIF/BMP).
    pub data: Vec<u8>,
    /// Image format — must match the actual byte content.
    pub format: ImageFormat,
    /// 0-based row of the anchor cell (top-left corner).
    pub anchor_row: usize,
    /// 0-based column of the anchor cell (top-left corner).
    pub anchor_col: usize,
    /// Optional explicit width in EMU (English Metric Units; 1 px = 9525
    /// EMU). When `None`, the pixel width is read from the image header.
    pub width_emu: Option<u64>,
    /// Optional explicit height in EMU. When `None`, the pixel height is
    /// read from the image header.
    pub height_emu: Option<u64>,
}

impl Image {
    /// Create an image from raw bytes, auto-detecting the format from the
    /// header. Panics if the format is unrecognized — use [`Image::try_new`]
    /// for fallible construction.
    pub fn new(data: Vec<u8>, anchor_row: usize, anchor_col: usize) -> Self {
        let format = ImageFormat::from_bytes(&data)
            .expect("Image::new: unrecognized image format (expected PNG/JPEG/GIF/BMP)");
        Self {
            data,
            format,
            anchor_row,
            anchor_col,
            width_emu: None,
            height_emu: None,
        }
    }

    /// Fallible constructor — returns `None` when the format is unrecognized.
    pub fn try_new(data: Vec<u8>, anchor_row: usize, anchor_col: usize) -> Option<Self> {
        let format = ImageFormat::from_bytes(&data)?;
        Some(Self {
            data,
            format,
            anchor_row,
            anchor_col,
            width_emu: None,
            height_emu: None,
        })
    }

    /// Set explicit EMU dimensions, overriding auto-detection.
    pub fn with_emu(mut self, width: u64, height: u64) -> Self {
        self.width_emu = Some(width);
        self.height_emu = Some(height);
        self
    }
}

/// Worksheet protection settings. When `sheet` is `true`, the sheet is
/// protected and the individual flags control which actions are
/// restricted. An optional password hash can be set to require
/// authentication to unprotect.
///
/// The password is stored as a hex string of the Excel 16-bit hash
/// (see [`SheetProtection::password_hash`]).
#[derive(Debug, Clone, Default)]
pub struct SheetProtection {
    /// Enable sheet protection.
    pub sheet: bool,
    /// Protect objects.
    pub objects: bool,
    /// Protect scenarios.
    pub scenarios: bool,
    /// Prevent formatting cells.
    pub format_cells: bool,
    /// Prevent formatting columns.
    pub format_columns: bool,
    /// Prevent formatting rows.
    pub format_rows: bool,
    /// Prevent inserting columns.
    pub insert_columns: bool,
    /// Prevent inserting rows.
    pub insert_rows: bool,
    /// Prevent inserting hyperlinks.
    pub insert_hyperlinks: bool,
    /// Prevent deleting columns.
    pub delete_columns: bool,
    /// Prevent deleting rows.
    pub delete_rows: bool,
    /// Prevent selecting locked cells.
    pub select_locked_cells: bool,
    /// Prevent sorting.
    pub sort: bool,
    /// Prevent auto-filter.
    pub auto_filter: bool,
    /// Prevent pivot tables.
    pub pivot_tables: bool,
    /// Prevent selecting unlocked cells.
    pub select_unlocked_cells: bool,
    /// Optional password hash (4-hex-digit Excel hash). When set,
    /// unprotecting the sheet requires this password.
    pub password: Option<String>,
}

impl SheetProtection {
    /// Create a protection config that locks everything (most restrictive).
    pub fn locked() -> Self {
        Self {
            sheet: true,
            objects: true,
            scenarios: true,
            format_cells: true,
            format_columns: true,
            format_rows: true,
            insert_columns: true,
            insert_rows: true,
            insert_hyperlinks: true,
            delete_columns: true,
            delete_rows: true,
            select_locked_cells: true,
            sort: true,
            auto_filter: true,
            pivot_tables: true,
            select_unlocked_cells: true,
            password: None,
        }
    }

    /// Create a protection config that protects the sheet but allows
    /// common data-entry actions (select cells, sort, filter).
    pub fn data_entry() -> Self {
        Self {
            sheet: true,
            select_locked_cells: false,
            select_unlocked_cells: false,
            sort: false,
            auto_filter: false,
            ..Default::default()
        }
    }

    /// Compute the Excel 16-bit password hash from a plaintext password.
    /// This is a simple, well-known algorithm — NOT cryptographically
    /// secure. It exists for compatibility with Excel's protection model.
    pub fn password_hash(password: &str) -> String {
        // Excel password hash: rotate-and-XOR over the password bytes
        // (reversed), starting with 0x0000.
        let mut hash: u16 = 0;
        for (i, ch) in password.chars().rev().enumerate() {
            let ch_val = ch as u16;
            let rotated = (ch_val >> (i + 1)) | (ch_val << (15 - (i % 15)));
            hash ^= rotated;
            hash = hash.wrapping_add(1);
        }
        // XOR with password length
        hash ^= password.len() as u16;
        format!("{:04X}", hash)
    }

    /// Set the password from a plaintext string (computes the hash).
    pub fn with_password(mut self, password: &str) -> Self {
        self.password = Some(Self::password_hash(password));
        self
    }
}

/// Workbook protection settings.
#[derive(Debug, Clone, Default)]
pub struct WorkbookProtection {
    /// Lock workbook structure (prevent adding/deleting/reordering sheets).
    pub lock_structure: bool,
    /// Lock workbook windows (prevent resizing/moving windows).
    pub lock_windows: bool,
    /// Optional password hash for revisions protection.
    pub revisions_password: Option<String>,
}

/// Document (core) properties written to `docProps/core.xml`.
///
/// These are the Dublin Core metadata fields that Excel stores in the
/// XLSX package. Timestamps are ISO 8601 strings (e.g.
/// `"2026-01-01T00:00:00Z"`).
#[derive(Debug, Clone, Default)]
pub struct DocumentProperties {
    pub title: Option<String>,
    pub creator: Option<String>,
    pub subject: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub category: Option<String>,
    pub content_status: Option<String>,
    /// ISO 8601 creation timestamp, e.g. `"2026-01-01T00:00:00Z"`.
    pub created: Option<String>,
    /// ISO 8601 modification timestamp.
    pub modified: Option<String>,
    pub last_modified_by: Option<String>,
}
