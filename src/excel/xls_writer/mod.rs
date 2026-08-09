//! XLS writer (BIFF8 format, OLE2 / CFB container).
//!
//! Mirrors the `XlsxWriter` API surface so callers can swap between formats
//! with minimal changes. All file bytes are produced from scratch using only
//! `std` (no `zip`, no external crate dependencies).
//!
//! # Supported features
//!
//! - Multiple sheets (max 31 characters per name, with character validation)
//! - Cell data: strings, numbers, booleans, formulas, empty
//! - Formula support via BIFF8 PTG encoder (SUM, AVERAGE, IF, VLOOKUP, etc.)
//! - Merged cells
//! - Freeze panes
//! - Auto-filter
//! - Column width configuration
//! - Header detection is not performed; cells are written as-is.
//!
//! # Current limitations
//!
//! - No styling, fonts, fills, borders, alignment, number formats
//! - No charts, sparklines, conditional formatting, images
//! - No data validation, hyperlinks, comments
//! - No print setup, page margins
//!
//! # Round-trip
//!
//! Files produced by this writer can be read back by our native `XlsReader`,
//! LibreOffice, Excel, and other tools that understand the BIFF8 format.

use anyhow::Result;

mod cfb;
mod biff;
mod ptg;

use cfb::{build_cfb, CfbStream, ObjectType};
use biff as B;
use ptg as P;

/// Cell data type for writing.
#[derive(Debug, Clone)]
pub enum CellData {
    String(String),
    Number(f64),
    Bool(bool),
    Formula(String),
    Error(String),
    Empty,
}

/// Row data for writing.
#[derive(Debug, Clone)]
pub struct RowData {
    pub cells: Vec<CellData>,
}

impl RowData {
    pub fn new() -> Self {
        Self { cells: Vec::new() }
    }

    pub fn add_string(&mut self, value: impl Into<String>) {
        self.cells.push(CellData::String(value.into()));
    }

    pub fn add_number(&mut self, value: f64) {
        self.cells.push(CellData::Number(value));
    }

    pub fn add_bool(&mut self, value: bool) {
        self.cells.push(CellData::Bool(value));
    }

    pub fn add_formula(&mut self, formula: impl Into<String>) {
        self.cells.push(CellData::Formula(formula.into()));
    }

    pub fn add_empty(&mut self) {
        self.cells.push(CellData::Empty);
    }

    pub fn add_error(&mut self, error: impl Into<String>) {
        self.cells.push(CellData::Error(error.into()));
    }
}

impl Default for RowData {
    fn default() -> Self {
        Self::new()
    }
}

/// Sheet data structure.
#[derive(Debug, Clone)]
pub struct SheetData {
    pub name: String,
    pub rows: Vec<RowData>,
    pub column_widths: Vec<f64>,
    /// Merged cell ranges: (start_row, start_col, end_row, end_col)
    pub merge_cells: Vec<(u16, u16, u16, u16)>,
    /// Freeze panes: first unfrozen row (0 = no freeze)
    pub freeze_row: u16,
    /// Freeze panes: first unfrozen column (0 = no freeze)
    pub freeze_col: u16,
    /// Auto-filter range: (first_row, first_col, last_row, last_col)
    pub auto_filter: Option<(u16, u16, u16, u16)>,
}

impl SheetData {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            rows: Vec::new(),
            column_widths: Vec::new(),
            merge_cells: Vec::new(),
            freeze_row: 0,
            freeze_col: 0,
            auto_filter: None,
        }
    }
}

/// XLS workbook writer.
pub struct XlsWriter {
    pub sheets: Vec<SheetData>,
}

impl XlsWriter {
    pub fn new() -> Self {
        Self { sheets: Vec::new() }
    }

    /// Add a new sheet to the workbook. Returns an error if the name is
    /// invalid (longer than 31 characters or contains forbidden characters).
    pub fn add_sheet(&mut self, name: &str) -> Result<()> {
        if name.is_empty() {
            anyhow::bail!("Sheet name cannot be empty");
        }
        if name.len() > 31 {
            anyhow::bail!("Sheet name cannot exceed 31 characters");
        }
        let invalid = ['\\', '/', '?', '*', '[', ']', ':'];
        if name.chars().any(|c| invalid.contains(&c)) {
            anyhow::bail!("Sheet name contains invalid characters: \\ / ? * [ ] :");
        }
        // Disallow leading apostrophe (Excel reserves this).
        if name.starts_with('\'') {
            anyhow::bail!("Sheet name cannot start with an apostrophe");
        }
        self.sheets.push(SheetData::new(name));
        Ok(())
    }

    /// Add a row to the current (last-added) sheet.
    pub fn add_row(&mut self, row: RowData) {
        if let Some(s) = self.sheets.last_mut() {
            s.rows.push(row);
        }
    }

    /// Add data from a 2D vector of strings. Each cell is classified: if the
    /// string parses as `f64`, it becomes a number; if it matches "TRUE" or
    /// "FALSE" (case-insensitive) it becomes a boolean; if it starts with `=`
    /// it becomes a formula; if it starts with `#` and matches a known error
    /// code it becomes an error; otherwise it stays a string. Empty strings
    /// become empty cells.
    pub fn add_data(&mut self, data: &[Vec<String>]) {
        let Some(sheet) = self.sheets.last_mut() else { return };
        for row in data {
            let mut r = RowData::new();
            for cell in row {
                if cell.is_empty() {
                    r.add_empty();
                } else if let Some(stripped) = cell.strip_prefix('=') {
                    r.add_formula(stripped);
                } else if is_error(cell) {
                    r.add_error(cell);
                } else if let Some(n) = parse_number_like(cell) {
                    r.add_number(n);
                } else if is_bool(cell) {
                    r.add_bool(cell.eq_ignore_ascii_case("TRUE") || cell.eq_ignore_ascii_case("1"));
                } else {
                    r.add_string(cell);
                }
            }
            sheet.rows.push(r);
        }
    }

    /// Set the column width for the current sheet.
    pub fn set_column_width(&mut self, col: usize, width_chars: f64) {
        if let Some(s) = self.sheets.last_mut() {
            if s.column_widths.len() <= col {
                s.column_widths.resize(col + 1, 8.43);
            }
            s.column_widths[col] = width_chars;
        }
    }

    /// Merge a cell range on the current sheet.
    /// Rows and columns are 0-based, inclusive.
    pub fn merge_cells(&mut self, start_row: u16, start_col: u16, end_row: u16, end_col: u16) {
        if let Some(s) = self.sheets.last_mut() {
            s.merge_cells.push((start_row, start_col, end_row, end_col));
        }
    }

    /// Freeze panes on the current sheet.
    /// `freeze_row` is the first unfrozen row (1 = freeze first row).
    /// `freeze_col` is the first unfrozen column (1 = freeze first column).
    pub fn freeze_panes(&mut self, freeze_row: u16, freeze_col: u16) {
        if let Some(s) = self.sheets.last_mut() {
            s.freeze_row = freeze_row;
            s.freeze_col = freeze_col;
        }
    }

    /// Set auto-filter range on the current sheet.
    pub fn set_auto_filter(&mut self, first_row: u16, first_col: u16, last_row: u16, last_col: u16) {
        if let Some(s) = self.sheets.last_mut() {
            s.auto_filter = Some((first_row, first_col, last_row, last_col));
        }
    }

    /// Serialize the workbook to a byte vector.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        if self.sheets.is_empty() {
            anyhow::bail!("Workbook has no sheets");
        }

        // 1. Build the SST (collect all unique strings).
        let mut unique: Vec<String> = Vec::new();
        let mut index_of: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        for s in &self.sheets {
            for r in &s.rows {
                for c in &r.cells {
                    if let CellData::String(v) = c
                        && !index_of.contains_key(v) {
                            index_of.insert(v.clone(), unique.len() as u32);
                            unique.push(v.clone());
                        }
                }
            }
        }
        let (sst_bytes, _sst_indices) = B::sst(&unique);

        // 2. Build each sheet's BIFF8 byte stream (with known SST indices).
        let mut sheet_bytes: Vec<Vec<u8>> = Vec::new();
        for s in &self.sheets {
            sheet_bytes.push(build_sheet(s, &index_of)?);
        }

        // 3. Compute the workbook stream layout so BoundSheet positions are
        //    known. Layout (positions are byte offsets from the start of the
        //    workbook stream). Order follows the BIFF8 spec / xlwt reference:
        //
        //      [0] BOF (workbook)
        //      [1] InterfaceHdr, MMS, InterfaceEnd      (required BIFF8 markers)
        //      [2] WriteAccess                          (owner, 112 bytes)
        //      [3] CodePage                             (0x04B0 = UTF-16)
        //      [4] DSF, TabId, FnGroupName              (workbook shape)
        //      [5] WindowProtect, Protect, ObjectProtect, Password,
        //          Prot4Rev, Prot4RevPass, Backup, HideObj
        //      [6] Window1                              (workbook window)
        //      [7] DateMode, Precision, RefreshAll, BookBool
        //      [8] Font, XF, Style                      (formatting table)
        //      [9] BoundSheet * N                       (sheet directory)
        //      [A] UseSelfs, Country, SST               (strings + locale)
        //      [B] (sheet 1 BOF + content + EOF)
        //      [C] (sheet 2 BOF + content + EOF)
        //      ...
        //      [EOF] workbook EOF
        let mut wb: Vec<u8> = Vec::new();
        wb.extend_from_slice(&B::bof_workbook());

        // BIFF8 interface block (required).
        wb.extend_from_slice(&B::interface_hdr());
        wb.extend_from_slice(&B::mms());
        wb.extend_from_slice(&B::interface_end());

        // Owner / locale.
        wb.extend_from_slice(&B::write_access());
        wb.extend_from_slice(&B::codepage());

        // Workbook shape.
        wb.extend_from_slice(&B::dsf());
        wb.extend_from_slice(&B::tab_id(self.sheets.len() as u16));
        wb.extend_from_slice(&B::fn_group_name());

        // Protection family (all off).
        wb.extend_from_slice(&B::window_protect());
        wb.extend_from_slice(&B::protect());
        wb.extend_from_slice(&B::object_protect());
        wb.extend_from_slice(&B::password());
        wb.extend_from_slice(&B::prot4_rev());
        wb.extend_from_slice(&B::prot4_rev_pass());
        wb.extend_from_slice(&B::backup());
        wb.extend_from_slice(&B::hide_obj());

        // Workbook window (18-byte body).
        wb.extend_from_slice(&B::window1(0));

        // Calculation + flags.
        wb.extend_from_slice(&B::date_mode(0));
        wb.extend_from_slice(&B::precision());
        wb.extend_from_slice(&B::refresh_all());
        wb.extend_from_slice(&B::book_bool());

        // Formatting table.
        // Order: FONT, FORMAT, XF, STYLE, PALETTE — matches xlwt and
        // the BIFF8 spec. The PALETTE record sits between STYLE and
        // USESELFS in Excel's stream; we follow xlwt and place it
        // immediately after the style table.
        wb.extend_from_slice(&B::default_fonts());
        wb.extend_from_slice(&B::number_formats());
        wb.extend_from_slice(&B::default_xf());
        // Style records come after XF and before BoundSheet.
        // This emits the built-in "Normal" style XF that xlwt and Excel
        // always write; without it some versions of Excel complain about
        // a missing style table.
        wb.extend_from_slice(&B::default_styles());
        // The 56-color default palette. Excel and xlwt always emit this
        // even when the file uses no custom colors; omitting it leaves
        // the color table empty and produces an "unreadable content"
        // warning on some Excel versions.
        wb.extend_from_slice(&B::default_palette());

        let mut bs_placeholders: Vec<usize> = Vec::new();
        for s in &self.sheets {
            bs_placeholders.push(wb.len());
            wb.extend_from_slice(&B::bound_sheet(0, 0, &s.name));
        }

        // Remaining globals.
        wb.extend_from_slice(&B::use_selfs());
        wb.extend_from_slice(&B::country());
        wb.extend_from_slice(&sst_bytes);
        // BIFF8 requires an EOF terminator after the workbook globals and
        // before the first worksheet BOF. xlwt, Excel, and the spec all
        // emit one. Without it, strict readers (and some Excel versions)
        // reject the file as malformed because they look for EOF to bound
        // the globals substream before the worksheet substreams begin.
        wb.extend_from_slice(&B::eof());
        // Note: WINDOW2 is per-sheet (in `build_sheet` below), not a workbook global.

        let sheets_start = wb.len();
        // Now we know each sheet's BOF offset.
        let mut sheet_offsets: Vec<u32> = Vec::new();
        let mut cursor = sheets_start;
        for s in &sheet_bytes {
            sheet_offsets.push(cursor as u32);
            cursor += s.len();
        }

        // Patch BoundSheet placeholders (the body starts at off + 4, with
        // the first 4 bytes being the offset).
        for (i, off) in bs_placeholders.iter().enumerate() {
            wb[off + 4..off + 8].copy_from_slice(&sheet_offsets[i].to_le_bytes());
        }

        // Append sheets.
        for s in &sheet_bytes {
            wb.extend_from_slice(s);
        }
        // Final workbook EOF.
        wb.extend_from_slice(&B::eof());

        // 4. Build the CFB container.
        let streams: Vec<CfbStream> = vec![
            CfbStream {
                name: "Root Entry".to_string(),
                data: Vec::new(),
                kind: ObjectType::Root,
                clsid: [
                    0x21, 0x08, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x46,
                ],
            },
            CfbStream::stream("Workbook", wb),
            CfbStream::stream("CompObj", compobj_stream()),
            // "\x05SummaryInformation" is optional but improves compatibility with
            // some readers (e.g. older Excel versions).
            CfbStream::stream("\u{5}SummaryInformation", summary_information_stream()),
        ];
        Ok(build_cfb(&streams))
    }

    /// Write the workbook to a file path.
    pub fn save(&self, path: &str) -> Result<()> {
        let bytes = self.to_bytes()?;
        std::fs::write(path, &bytes)
            .map_err(|e| anyhow::anyhow!("failed to write {}: {}", path, e))?;
        Ok(())
    }
}

impl Default for XlsWriter {
    fn default() -> Self {
        Self::new()
    }
}

fn build_sheet(
    sheet: &SheetData,
    sst_index: &std::collections::HashMap<String, u32>,
) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(&B::bof_sheet());

    // Per-sheet setup block (the BIFF8 "Sheet Block"). xlwt and Excel
    // emit these records right after the sheet BOF and before
    // DIMENSIONS. We follow the same order so strict readers see an
    // Excel-shaped worksheet stream.
    out.extend_from_slice(&B::calc_count());
    out.extend_from_slice(&B::calc_mode());
    out.extend_from_slice(&B::ref_mode());
    out.extend_from_slice(&B::delta());
    out.extend_from_slice(&B::iteration());
    out.extend_from_slice(&B::safer_recalc());
    out.extend_from_slice(&B::window_protect());
    out.extend_from_slice(&B::protect());
    out.extend_from_slice(&B::object_protect());
    out.extend_from_slice(&B::password());
    out.extend_from_slice(&B::guts());
    out.extend_from_slice(&B::ws_bool());
    out.extend_from_slice(&B::grid_set());
    // Page margins (left, right, top, bottom). xlwt emits these even
    // when the user hasn't customised them, so we follow.
    out.extend_from_slice(&B::left_margin());
    out.extend_from_slice(&B::right_margin());
    out.extend_from_slice(&B::top_margin());
    out.extend_from_slice(&B::bottom_margin());
    out.extend_from_slice(&B::print_headers());
    out.extend_from_slice(&B::print_gridlines());
    out.extend_from_slice(&B::h_center());
    out.extend_from_slice(&B::v_center());

    // Compute dimensions: max row / max col with any data.
    let mut max_row: u32 = 0;
    let mut max_col: u16 = 0;
    for (r_idx, r) in sheet.rows.iter().enumerate() {
        for (c_idx, c) in r.cells.iter().enumerate() {
            if !matches!(c, CellData::Empty) {
                max_row = max_row.max(r_idx as u32 + 1);
                max_col = max_col.max(c_idx as u16 + 1);
            }
        }
    }
    // Emit dimensions even for empty sheets (Excel expects 1x1 minimum).
    if max_row == 0 { max_row = 1; }
    if max_col == 0 { max_col = 1; }
    out.extend_from_slice(&B::default_row_height());
    out.extend_from_slice(&B::dimensions(max_row, max_col));
    out.extend_from_slice(&B::window2());

    // Column widths (ColInfo records). Width units: 1/256 of a character cell.
    if !sheet.column_widths.is_empty() {
        // Emit DEFCOLWIDTH (0x0055) — the default column width for any
        // column that doesn't have an explicit ColInfo entry. Excel's
        // default is 8.43 characters; we use the same value. We
        // express it in characters, NOT in 1/256-of-a-char units
        // (ColInfo stores 1/256 but DEFCOLWIDTH stores characters
        // directly). 8.43 in characters → 0x0843 (2115/256 rounded
        // up so the width matches what Excel emits).
        out.extend_from_slice(&B::def_col_width(0x0843));
        for (i, w) in sheet.column_widths.iter().enumerate() {
            out.extend_from_slice(&col_info(i as u16, i as u16, *w));
        }
    }

    for (r_idx, r) in sheet.rows.iter().enumerate() {
        if r.cells.iter().all(|c| matches!(c, CellData::Empty)) {
            continue;
        }
        let last_col = r
            .cells
            .iter()
            .rposition(|c| !matches!(c, CellData::Empty))
            .unwrap_or(0) as u16;
        out.extend_from_slice(&B::row(r_idx as u32, 0, last_col));
        for (c_idx, c) in r.cells.iter().enumerate() {
            let row = r_idx as u16;
            let col = c_idx as u16;
            match c {
                CellData::String(s) => {
                    let idx = sst_index.get(s).copied().unwrap_or(0);
                    out.extend_from_slice(&B::labelsst_cell(row, col, 0, idx));
                }
                CellData::Number(n) => {
                    out.extend_from_slice(&B::number_cell(row, col, 0, *n));
                }
                CellData::Bool(b) => {
                    out.extend_from_slice(&B::bool_cell(row, col, 0, *b));
                }
                CellData::Formula(expr) => {
                    let ptg = P::encode(expr)
                        .map_err(|e| anyhow::anyhow!("bad formula '{}': {:?}", expr, e))?;
                    out.extend_from_slice(&B::formula_cell(row, col, 0, &ptg));
                }
                CellData::Error(e) => {
                    out.extend_from_slice(&B::error_cell(row, col, 0, e));
                }
                CellData::Empty => {}
            }
        }
    }

    // Merged cells
    if !sheet.merge_cells.is_empty() {
        out.extend_from_slice(&B::merged_cells(&sheet.merge_cells));
    }

    // Freeze panes
    if sheet.freeze_row > 0 || sheet.freeze_col > 0 {
        out.extend_from_slice(&B::pane(sheet.freeze_row, sheet.freeze_col));
    }

    // Auto-filter
    if let Some((fr, fc, lr, lc)) = sheet.auto_filter {
        out.extend_from_slice(&B::auto_filter(fr, fc, lr, lc));
    }

    out.extend_from_slice(&B::eof());
    Ok(out)
}

fn col_info(first_col: u16, last_col: u16, width_chars: f64) -> Vec<u8> {
    // ColInfo body: firstcol(2) lastcol(2) width(2) xf(2) grbit(2) reserved(2)
    let width = (width_chars * 256.0) as u16;
    let mut body = Vec::new();
    body.extend_from_slice(&first_col.to_le_bytes());
    body.extend_from_slice(&last_col.to_le_bytes());
    body.extend_from_slice(&width.to_le_bytes());
    body.extend_from_slice(&0u16.to_le_bytes()); // xf
    body.extend_from_slice(&0u16.to_le_bytes()); // grbit
    body.extend_from_slice(&0u16.to_le_bytes()); // reserved
    let mut out = Vec::new();
    out.extend_from_slice(&0x007Du16.to_le_bytes()); // ColInfo id
    out.extend_from_slice(&(body.len() as u16).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

fn parse_number_like(s: &str) -> Option<f64> {
    if s.is_empty() {
        return None;
    }
    // Reject things that look like dates, booleans, or contain non-numeric
    // characters (other than leading sign, decimal point, exponent, %).
    let mut percent = false;
    let s = s.trim();
    let s = if let Some(stripped) = s.strip_suffix('%') {
        percent = true;
        stripped
    } else {
        s
    };
    let n: f64 = s.parse().ok()?;
    if !n.is_finite() {
        return None;
    }
    Some(if percent { n / 100.0 } else { n })
}

fn is_bool(s: &str) -> bool {
    matches!(s.to_ascii_uppercase().as_str(), "TRUE" | "FALSE")
}

fn is_error(s: &str) -> bool {
    matches!(
        s,
        "#NULL!" | "#DIV/0!" | "#VALUE!" | "#REF!" | "#NAME?" | "#NUM!" | "#N/A"
    )
}

fn compobj_stream() -> Vec<u8> {
    // CompObj stream (OLE link info). Excel uses this to identify the
    // file type via the user type ("Microsoft Excel Worksheet") and
    // the clipboard format ("Excel.Sheet.8"). A truncated or wrong
    // CompObj is one of the most common causes of Excel showing
    // "unreadable content" errors, even when the BIFF records are
    // otherwise valid.
    //
    // Layout (all multi-byte fields little-endian):
    //   0x00: 0xFFFE 0x0001   byte order + version
    //   0x04: 0x0002 0x0001   ???
    //   0x08: 0xFFFFFFFF      reserved
    //   0x0C: 16 bytes        CLSID (workbook: 2082...046)
    //   0x1C: 4 bytes (u32)   user type length (in chars, UTF-16)
    //   0x20: variable        user type (UTF-16LE) — "Microsoft Excel Worksheet"
    //   ...                  user type padded to 4-byte boundary
    //   4 bytes (u32)        clipboard format size (incl. size field)
    //   4 bytes              reserved (0x0000000E)
    //   variable             clipboard format name — "Excel.Sheet.8"
    //   4 bytes (u32)        0x00000000 reserved
    let mut out = Vec::new();

    // Header (12 bytes)
    out.extend_from_slice(&0x0001u16.to_le_bytes()); // [0..2] — must be 0x0001
    out.extend_from_slice(&0xFFFEu16.to_le_bytes()); // [2..4] — byte order
    out.extend_from_slice(&0x0002u16.to_le_bytes()); // [4..6] — version 2
    out.extend_from_slice(&0x0001u16.to_le_bytes()); // [6..8] — ?
    out.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // [8..12] — reserved

    // CLSID for Excel workbook: 2082...046
    let clsid: [u8; 16] = [
        0x08, 0x20, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x46,
    ];
    out.extend_from_slice(&clsid);

    // User type: "Microsoft Excel Worksheet" as UTF-16LE.
    let user_type = "Microsoft Excel Worksheet";
    let units: Vec<u16> = user_type.encode_utf16().collect();
    out.extend_from_slice(&(units.len() as u32).to_le_bytes());
    for u in &units {
        out.extend_from_slice(&u.to_le_bytes());
    }
    while out.len() % 4 != 0 {
        out.push(0);
    }

    // Clipboard format: "Excel.Sheet.8". Size includes the size field
    // itself (4 bytes) plus the reserved (4 bytes) plus the name.
    let cf_name = "Excel.Sheet.8";
    let cf_bytes: Vec<u16> = cf_name.encode_utf16().collect();
    let cf_total_size = 4 + 4 + (cf_bytes.len() * 2) as u32;
    out.extend_from_slice(&cf_total_size.to_le_bytes());
    out.extend_from_slice(&0x0000_000Eu32.to_le_bytes());
    for u in &cf_bytes {
        out.extend_from_slice(&u.to_le_bytes());
    }
    while out.len() % 4 != 0 {
        out.push(0);
    }
    out.extend_from_slice(&0x0000_0000u32.to_le_bytes());

    out
}

fn summary_information_stream() -> Vec<u8> {
    // "\u{5}SummaryInformation" property set header. Most readers accept
    // an empty property set, so we emit the minimal valid header with
    // zero sections. This is what Excel writes by default for new
    // workbooks.
    let mut out = Vec::new();
    // OS indicator + CLSID + section count.
    out.extend_from_slice(&0xFFFEu16.to_le_bytes()); // byte order
    out.extend_from_slice(&0x0000u16.to_le_bytes()); // version
    out.extend_from_slice(&0x0000u32.to_le_bytes()); // OS (0 = Win16)
    out.extend_from_slice(&[0u8; 16]); // CLSID
    out.extend_from_slice(&0u32.to_le_bytes()); // # sections
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn dump_first_bytes(b: &[u8], n: usize) -> String {
        b.iter().take(n).map(|x| format!("{:02x}", x)).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn empty_workbook_fails() {
        let w = XlsWriter::new();
        assert!(w.to_bytes().is_err());
    }

    #[test]
    fn minimal_workbook() {
        let mut w = XlsWriter::new();
        w.add_sheet("Sheet1").unwrap();
        let mut row = RowData::new();
        row.add_string("Hello");
        row.add_number(1.0);
        w.add_row(row);
        let bytes = w.to_bytes().unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[0..8], &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
    }

    #[test]
    fn file_starts_with_cfb_magic() {
        let mut w = XlsWriter::new();
        w.add_sheet("S").unwrap();
        w.add_row(RowData::new());
        let b = w.to_bytes().unwrap();
        assert_eq!(&b[0..8], &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
        eprintln!("first 16 bytes: {}", dump_first_bytes(&b, 16));
    }

    #[test]
    fn cfb_readable_via_signature_scan() {
        let mut w = XlsWriter::new();
        w.add_sheet("S").unwrap();
        let mut r = RowData::new();
        r.add_string("hi");
        w.add_row(r);
        let b = w.to_bytes().unwrap();
        // Magic and v3 header
        assert_eq!(&b[0..8], &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
        assert_eq!(u16::from_le_bytes([b[26], b[27]]), 0x0003);
    }

    #[test]
    fn save_to_disk_then_read_magic() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.xls");
        let mut w = XlsWriter::new();
        w.add_sheet("Data").unwrap();
        let mut r = RowData::new();
        r.add_string("a");
        r.add_number(2.5);
        r.add_bool(true);
        w.add_row(r);
        w.save(path.to_str().unwrap()).unwrap();
        let mut f = std::fs::File::open(&path).unwrap();
        let mut buf = [0u8; 8];
        f.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
    }
}
