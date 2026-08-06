//! BIFF8 record encoder.
//!
//! Produces the byte sequences for the records used by a minimal XLS writer:
//! workbook globals (BOF, CodePage, Window1, Font, Format, XF, Style, BoundSheet,
//! SST, DateMode, Window2) and sheet records (BOF, Index, Dimensions, Row, cells).
//!
//! References:
//! - [MS-XLS] §2.3 (records)
//! - [MS-XLS] §2.5 (workbook stream)
//! - [MS-XLS] §2.4 (worksheet stream)
//!
//! All multi-byte integers are little-endian.

use std::io::Write;

/// BIFF8 record identifier.
#[allow(dead_code)] // Variants retained for BIFF8 structure completeness
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum RecordId {
    Bof = 0x0809,
    Eof = 0x000A,
    CodePage = 0x0042,
    DateMode = 0x0022,
    Dimensions = 0x0200,
    Index = 0x020B,
    Window1 = 0x003D,
    Window2 = 0x023E,
    Font = 0x0031,
    Format = 0x041E,
    Xf = 0x00E0,
    Style = 0x0293,
    BoundSheet = 0x0085,
    Sst = 0x00FC,
    ExtSst = 0x00FF,
    Number = 0x0203,
    Rk = 0x027E,
    BoolErr = 0x0205,
    LabelSst = 0x00FD,
    Formula = 0x0006,
    Row = 0x0208,
    ColInfo = 0x007D,
    ColWidth = 0x0024,  // per-column width (legacy COLWIDTH record)
    Guts = 0x0080,
    Blank = 0x0201,
    MulRk = 0x00BD,
    Label = 0x0204,
    UseSelfs = 0x0160,
    Country = 0x008C,
    Obj = 0x005D,
    TxO = 0x01B6,
    WriteProtect = 0x0086,
    MergedCells = 0x00E5,
    Pane = 0x0041,
    AutoFilter = 0x009E,
    String = 0x0207,
    Selection = 0x001D,
    // BIFF8 setup records (required for Excel/xlrd compatibility)
    InterfaceHdr = 0x00E1,
    InterfaceEnd = 0x00E2,
    Mms = 0x00C1,
    WriteAccess = 0x005C,
    Dsf = 0x0161,
    TabId = 0x013D,
    FnGroupName = 0x009C,
    WindowProtect = 0x0019,
    Protect = 0x0012,
    ObjectProtect = 0x0063,
    Password = 0x0013,
    Prot4Rev = 0x01AF,
    Prot4RevPass = 0x01BC,
    Backup = 0x0040,
    HideObj = 0x008D,
    Precision = 0x000E,
    RefreshAll = 0x01B7,
    BookBool = 0x00DA,
    Palette = 0x0092,
    Continue = 0x003C,
    DefColWidth = 0x0055, // default column width in characters (BIFF2+ record)
    // Per-sheet setup records that xlwt/Excel emit right after the
    // sheet BOF and before DIMENSIONS. These are the records the BIFF8
    // spec lists in the worksheet stream's "Sheet Block": defaults,
    // view state, page setup, and per-sheet protection.
    CalcCount = 0x000C,   // number of calc iterations (100 = automatic)
    CalcMode = 0x000D,    // 1 = automatic, 0 = manual
    RefMode = 0x000F,     // 1 = R1C1, 0 = A1 (we always emit A1)
    Iteration = 0x0011,   // iteration enabled flag
    Delta = 0x0010,       // iteration step (f64)
    SaferRecalc = 0x005F, // "recalc before save" (we leave it off)
    WsBool = 0x0081,      // per-sheet view boolean (manual break, fit-to-page)
    GridSet = 0x0082,     // 1 = print gridlines
    HCenter = 0x0083,     // 1 = center horizontally on page
    VCenter = 0x0084,     // 1 = center vertically on page
    DefaultRowHeight = 0x0225, // default row height when not in a ROW record
    // Page setup / margin records
    LeftMargin = 0x0026,
    RightMargin = 0x0027,
    TopMargin = 0x0028,
    BottomMargin = 0x0029,
    PrintHeaders = 0x002A,
    PrintGridlines = 0x002B,
}

impl RecordId {
    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

/// Builder for a single BIFF record. Append fields in order; call `finish` to
/// emit `[id u16][size u16]body`.
pub struct Record<'a> {
    id: u16,
    buf: Vec<u8>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> Record<'a> {
    pub fn new(id: RecordId) -> Self {
        Self {
            id: id.as_u16(),
            buf: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn u8(mut self, v: u8) -> Self {
        self.buf.push(v);
        self
    }

    pub fn u16(mut self, v: u16) -> Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn u32(mut self, v: u32) -> Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn f64(mut self, v: f64) -> Self {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn bytes(mut self, v: &[u8]) -> Self {
        self.buf.extend_from_slice(v);
        self
    }

    pub fn utf16(mut self, s: &str) -> Self {
        for unit in s.encode_utf16() {
            self.buf.extend_from_slice(&unit.to_le_bytes());
        }
        self
    }

    /// Write the record header + body into `out`.
    pub fn finish<W: Write>(&self, out: &mut W) -> std::io::Result<()> {
        out.write_all(&self.id.to_le_bytes())?;
        out.write_all(&(self.buf.len() as u16).to_le_bytes())?;
        out.write_all(&self.buf)?;
        Ok(())
    }
}

pub fn record(id: RecordId) -> Record<'static> {
    Record::new(id)
}

/// Build the standard CodePage record (id 0x0042, code page 0x04B0 = 1200 / UTF-16).
/// Per the BIFF8 spec and xlwt, the code page in BIFF8 must be 1200 (UTF-16)
/// because all strings in BIFF8 are stored as UTF-16LE.
pub fn codepage() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::CodePage)
        .u16(0x04B0)
        .finish(&mut out)
        .unwrap();
    out
}

/// BOF record for a workbook stream.
///
/// BIFF8 BOF layout (16 bytes total):
/// - version (2): 0x0600 (BIFF8)
/// - type (2):    0x0005 (workbook)
/// - build id (2): e.g. 0x0DBB
/// - build year (2): 1900-based, e.g. 0x07CC (1996)
/// - reserved (4): MUST be 0
/// - file history (4): typically 1, opaque
pub fn bof_workbook() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Bof)
        .u16(0x0600) // version
        .u16(0x0005) // type (workbook)
        .u16(0x0DBB) // build id
        .u16(0x07CC) // build year (1996)
        .u32(0x0000_0000) // reserved (MUST be 0)
        .u32(0x0000_0001) // file history
        .finish(&mut out)
        .unwrap();
    out
}

/// BOF record for a worksheet stream.
pub fn bof_sheet() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Bof)
        .u16(0x0600) // version
        .u16(0x0010) // type (worksheet)
        .u16(0x0DBB) // build id
        .u16(0x07CC) // build year
        .u32(0x0000_0000) // reserved (MUST be 0)
        .u32(0x0000_0001) // file history
        .finish(&mut out)
        .unwrap();
    out
}

/// EOF record (no body).
pub fn eof() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Eof).finish(&mut out).unwrap();
    out
}

/// Window1 record (initial workbook window). 18-byte body per MS-XLS §2.4.355.
///
/// Layout (all u16 little-endian):
/// - hpos_twips:        horizontal position of the document window
/// - vpos_twips:        vertical position of the document window
/// - width_twips:       width of the document window
/// - height_twips:      height of the document window
/// - flags:             0x0001 hidden, 0x0002 minimised, 0x0008 hscroll,
///                      0x0010 vscroll, 0x0020 tabbar visible
/// - active_sheet:      zero-based index of the active worksheet
/// - first_tab_index:   index of first visible tab
/// - selected_tabs:     number of selected worksheets
/// - tab_width:         width of tab bar in 1/1000 of window width
pub fn window1(active_sheet: u16) -> Vec<u8> {
    let mut out = Vec::new();
    let flags: u16 = 0x0038; // hscroll + vscroll + tabbar visible
    Record::new(RecordId::Window1)
        .u16(0x0000) // hpos_twips
        .u16(0x0000) // vpos_twips
        .u16(0x55F0) // width_twips  (~ 55 twips * 256 = 14,080)
        .u16(0x36B0) // height_twips
        .u16(flags) // flags
        .u16(active_sheet) // active sheet index
        .u16(0x0000) // first tab index
        .u16(0x0001) // selected tabs (1 = just the active sheet)
        .u16(0x0258) // tab width (600/1000 = 60% of window)
        .finish(&mut out)
        .unwrap();
    out
}

/// Window2 record (sheet view). 18-byte body per MS-XLS §2.4.356.
///
/// Layout:
/// - grbit (2):                option flags
/// - first_visible_row (2):    top row visible
/// - first_visible_col (2):    leftmost column visible
/// - grid_colour_index (2):    color of gridlines (0x7FFF = system default)
/// - reserved (2):             MUST be 0
/// - preview_magn (2):         magnification in page-break preview
/// - normal_magn (2):          magnification in normal view (0 = default 100%)
/// - reserved (4):             MUST be 0
pub fn window2() -> Vec<u8> {
    let mut out = Vec::new();
    // grbit flags:
    // 0x0002 - fDspGrid: show gridlines
    // 0x0004 - fDspRwCol: show row/column headers
    // 0x0080 - fDspGuts: show outline symbols
    // 0x0200 - fSelected: sheet tab is selected
    // 0x0400 - fVisible: sheet is visible
    let grbit: u16 = 0x0006 | 0x0080 | 0x0200 | 0x0400;
    Record::new(RecordId::Window2)
        .u16(grbit) // grbit
        .u16(0x0000) // first visible row
        .u16(0x0000) // first visible col
        .u16(0x7FFF) // grid colour index (system default)
        .u16(0x0000) // reserved
        .u16(0x0000) // preview magnification (0 = default 60%)
        .u16(0x0000) // normal magnification (0 = default 100%)
        .u32(0x0000_0000) // reserved
        .finish(&mut out)
        .unwrap();
    out
}

/// Standard font record. We always emit one minimal font.
///
/// BIFF8 FONT body layout (per MS-XLS §2.4.150):
/// - height (2):      font size in 1/20 of a point (200 = 10pt)
/// - options (2):     bit flags (bold, italic, underline, ...)
/// - color_idx (2):   color palette index
/// - weight (2):      100-1000 (400 = normal, 700 = bold)
/// - escapement (2):  0=none, 1=super, 2=sub
/// - underline (1):   underline type
/// - family (1):      font family
/// - charset (1):     character set
/// - reserved (1):    MUST be 0
/// - name (XlsUnicodeStringNoCch): cch(2)+chars — for BIFF8 fonts the length
///   is a 16-bit count of characters, then the characters in UTF-16LE.
pub fn default_fonts() -> Vec<u8> {
    let mut out = Vec::new();
    // Font 0: bold off, size 10, name "Arial".
    Record::new(RecordId::Font)
        .u16(0x00C8) // height (10 * 20 = 200)
        .u16(0x0000) // options
        .u16(0x7FFF) // colour index (system default = 0x7FFF per spec)
        .u16(0x0190) // weight (400 = normal)
        .u16(0x0000) // escapement
        .u8(0x00)    // underline
        .u8(0x00)    // family
        .u8(0x01)    // charset (0x01 = system default; avoids ANSI-only assumption)
        .u8(0x00)    // reserved
        // Name: 16-bit character count (5), then 5 chars UTF-16LE.
        .u16(0x0005) // char count
        .utf16("Arial")
        .finish(&mut out)
        .unwrap();
    out
}

/// Number format record (BIFF8 Format, 0x041E).
///
/// Per MS-XLS §2.4.128 / xlwt reference, each Format record is:
///
///   - format_idx (2)  — index into the Format table. Built-in formats
///                       (0..=0x003E in BIFF8) are NOT stored; user-defined
///                       formats start at index 0x0064 (100). The first
///                       user-defined format the writer emits uses index 0x0064.
///   - size (2)        — length of the format string in bytes (always
///                       even; for UTF-16 strings this is 2*cch).
///   - options (1)     — bit flags: 0 = ASCII/1-byte, 1 = UTF-16.
///                       (Some writers also set bit 0x80 for "built-in".)
///   - format_string   — the format text (ASCII or UTF-16LE).
///
/// Excel and xlwt always emit at least one user-defined Format record
/// (typically "General") even when the file uses no custom formats, so
/// the style table is unambiguous. We follow the same convention.
pub fn number_formats() -> Vec<u8> {
    let mut out = Vec::new();
    // Format index 0x0064 (100): the first user-defined format slot.
    // String is ASCII "General" (1 byte per char, so 7 bytes; padded to
    // even length by adding a trailing '\0' if needed).
    let fmt_text = b"General\0";
    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(&0x0064u16.to_le_bytes()); // format_idx
    body.extend_from_slice(&(fmt_text.len() as u16).to_le_bytes()); // size
    body.push(0x00); // options: 1-byte ASCII, not built-in
    body.extend_from_slice(fmt_text);
    Record::new(RecordId::Format)
        .bytes(&body)
        .finish(&mut out)
        .unwrap();
    out
}

/// Default 56-color palette (BIFF8 PALETTE record, 0x0092).
///
/// Per MS-XLS §2.4.232, the palette has 56 RGB entries stored as
/// little-endian u32 each (the on-disk byte order is 0x00 RR GG BB;
/// in registers the value is 0x00BBGGRR — xlwt's `excel_default_palette_b8`
/// matches this). Indices 0..=7 are the built-in colors and 8..=63
/// are user-defined; we keep the standard Excel palette so that any
/// color index in our FONT/XF records points to a known value.
pub fn default_palette() -> Vec<u8> {
    // The canonical Excel BIFF8 default palette. First 8 entries are
    // the built-in colors; the rest match what Excel and xlwt emit.
    const PALETTE: [u32; 56] = [
        0x00000000, 0x00FFFFFF, 0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFF00, 0x00FF00FF,
        0x0000FFFF, 0x00800000, 0x00008000, 0x00000080, 0x00808000, 0x00800080, 0x00008080,
        0x00C0C0C0, 0x00808080, 0x009999FF, 0x00993366, 0x00FFFFCC, 0x00CCFFFF, 0x00660066,
        0x00FF8080, 0x000066CC, 0x00CCCCFF, 0x00000080, 0x00FF00FF, 0x00FFFF00, 0x0000FFFF,
        0x00800080, 0x00800000, 0x00008080, 0x000000FF, 0x0000CCFF, 0x00CCFFFF, 0x00CCFFCC,
        0x00FFFF99, 0x0099CCFF, 0x00FF99CC, 0x00CC99FF, 0x00FFCC99, 0x003366FF, 0x0033CCCC,
        0x0099CC00, 0x00FFCC00, 0x00FF9900, 0x00FF6600, 0x00666699, 0x00969696, 0x00003366,
        0x00339966, 0x00003300, 0x00333300, 0x00993300, 0x00993366, 0x00333399, 0x00333333,
    ];
    let mut out = Vec::new();
    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(&(PALETTE.len() as u16).to_le_bytes());
    for c in &PALETTE {
        body.extend_from_slice(&c.to_le_bytes());
    }
    Record::new(RecordId::Palette)
        .bytes(&body)
        .finish(&mut out)
        .unwrap();
    out
}

/// XF record for the default cell format (index 0 = "Normal" style XF).
///
/// Per MS-XLS §2.4.353, a BIFF8 XF is 20 bytes:
/// - font_idx (2):   index into Font record table
/// - format_idx (2): index into Format record table
/// - prot_parent (2): low 3 bits = cell protection (locked, hidden, type),
///                    high 12 bits = parent style XF index (0xFFF for style XF)
/// - alignment (1):  horz(3)|wrap(1)|vert(3)
/// - rotation (1):   0-180 or 255=stacked
/// - indent (1):     indent(4)|shrink(1)|merge(1)|direction(2)
/// - used_attrib (1): low 2 bits=fmt/font overrides, high 6 bits=other overrides
/// - borders1 (4):   16 bits border styles + 16 bits diag flags
/// - borders2 (4):   border colors + fill pattern
/// - pattern (2):    pattern foreground/background colors
///
/// A "Normal" style XF has prot=0x5 (locked+cell-type, no formula hidden) and
/// parent=0xFFF (no parent — this IS the parent for cell XFs).
pub fn default_xf() -> Vec<u8> {
    let mut out = Vec::new();
    let body: [u8; 20] = [
        0x00, 0x00, // font_idx = 0
        0x00, 0x00, // format_idx = 0 (General)
        0xF5, 0xFF, // prot=0x5 (locked+cell, no hidden), parent=0xFFF
        0x20,       // alignment: horz=0, wrap=0, vert=1 (centred)
        0x00,       // rotation
        0x00,       // indent
        0xF4,       // used_attrib: all overrides
        0x00, 0x00, 0x00, 0x00, // borders1
        0x00, 0x00, 0x00, 0x00, // borders2
        0x00, 0x00, // pattern colors
    ];
    let mut hdr = Vec::new();
    Record::new(RecordId::Xf)
        .bytes(&body)
        .finish(&mut hdr)
        .unwrap();
    out.extend_from_slice(&hdr);
    out
}

/// Build a default Style record (id 0x0293) for a built-in style.
/// Per MS-XLS §2.4.265: `xfIndex` (2 bytes) + `builtIn` (1 byte) +
/// `level` (1 byte). For built-in styles, the high bit (0x8000) of
/// `xfIndex` MUST be set. The built-in id mapping is:
///   0x00 = Normal (no Style record needed)
///   0x01 = RowLevel_lv
///   0x02 = ColLevel_lv
///   0x03 = Comma
///   0x04 = Currency
///   0x05 = Percent
///   0x06 = Comma [0]
///   0x07 = Currency [0]
pub fn default_styles() -> Vec<u8> {
    // Emit a single STYLE record for the implicit "Normal" style.
    // Excel expects at least one STYLE record; the built‑in Normal style
    // has an xfIndex with the high bit set (0x8000) and built‑in id 0.
    // The level byte is 0xFF, meaning "not applicable" for built‑in
    // styles.
    let mut out = Vec::new();
    let xf_index: u16 = 0x8000; // high bit set indicates built‑in style
    Record::new(RecordId::Style)
        .u16(xf_index)
        .u8(0u8) // built‑in id 0 = Normal
        .u8(0xFF) // level (N/A)
        .finish(&mut out)
        .unwrap();
    out
}

// =====================================================================
// Per-sheet setup records (the "Sheet Block")
//
// These records appear at the very start of each worksheet stream,
// between the sheet BOF and DIMENSIONS. xlwt, Excel, and the BIFF8 spec
// all expect them. Omitting them makes some readers (notably Excel
// itself) fall back to unspecified defaults and complain about an
// "unreadable" file. Each helper below emits one such record.
// =====================================================================

/// CALCCOUNT (0x000C): number of iterations for circular-reference
/// resolution. 100 is the Excel default for automatic calculation.
pub fn calc_count() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::CalcCount)
        .u16(100)
        .finish(&mut out)
        .unwrap();
    out
}

/// CALCMODE (0x000D): 0 = manual, 1 = automatic. Excel's default is
/// automatic, and we don't expose a knob for it.
pub fn calc_mode() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::CalcMode)
        .u16(1)
        .finish(&mut out)
        .unwrap();
    out
}

/// REFMODE (0x000F): 0 = A1 reference style, 1 = R1C1. xlwt always
/// writes 0; we follow suit.
pub fn ref_mode() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::RefMode)
        .u16(0)
        .finish(&mut out)
        .unwrap();
    out
}

/// DELTA (0x0010): iteration step (f64) for circular references.
/// Always 0.001 in Excel. We never enable iterations so the value
/// does not matter in practice, but xlwt writes it.
pub fn delta() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Delta)
        .f64(0.001)
        .finish(&mut out)
        .unwrap();
    out
}

/// ITERATION (0x0011): 0 = no iteration, 1 = iterate. xlwt writes 0.
pub fn iteration() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Iteration)
        .u16(0)
        .finish(&mut out)
        .unwrap();
    out
}

/// SAFERECALC (0x005F): "recalculate before save" flag, off.
pub fn safer_recalc() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::SaferRecalc)
        .u16(0)
        .finish(&mut out)
        .unwrap();
    out
}

/// GUTS (0x0080): empty row/col outline gutters, no summary rows.
/// Layout: rwColLevel(2) + rwRowLevel(2) + colGutter(2) + rowGutter(2) = 8 bytes.
pub fn guts() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Guts)
        .u16(0) // rwColLevel
        .u16(0) // rwRowLevel
        .u16(0) // colGutter
        .u16(0) // rowGutter
        .finish(&mut out)
        .unwrap();
    out
}

/// WSBOOL (0x0081): per-sheet view flags. xlwt writes a single u16
/// with value 0x0001 (fRowSumsBelow) + 0x0008 (fSyncHoriz) + 0x0010
/// (fSyncVert) + 0x0020 (fShowRowColHeaders). We emit the same.
pub fn ws_bool() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::WsBool)
        .u16(0x0001 | 0x0008 | 0x0010 | 0x0020)
        .finish(&mut out)
        .unwrap();
    out
}

/// GRIDSET (0x0082): 1 = print gridlines (we leave it off).
pub fn grid_set() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::GridSet)
        .u16(0)
        .finish(&mut out)
        .unwrap();
    out
}

/// DEFAULTROWHEIGHT (0x0225): default row height in twips when not
/// set by a ROW record. Per MS-XLS §2.4.86, the body is 4 bytes:
///
///   - options (2): bit 0 = "height differs from default font height",
///                  bit 1 = hidden, bit 2 = extra space above, bit 3 = extra space below.
///   - def_height (2): default row height in twips (1/20 of a point).
///
/// We use 255 twips (~12 pt) as a sensible Excel default and clear
/// the option flags.
pub fn default_row_height() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::DefaultRowHeight)
        .u16(0x0000) // options
        .u16(255)    // def_height in twips
        .finish(&mut out)
        .unwrap();
    out
}

/// HCENTER (0x0083): 1 = center the sheet horizontally when printing.
/// xlwt writes 0 by default; we follow.
pub fn h_center() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::HCenter)
        .u16(0)
        .finish(&mut out)
        .unwrap();
    out
}

/// VCENTER (0x0084): 1 = center the sheet vertically when printing.
pub fn v_center() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::VCenter)
        .u16(0)
        .finish(&mut out)
        .unwrap();
    out
}

/// LEFTMARGIN (0x0026): left page margin in inches (f64). Excel
/// default is 0.7".
pub fn left_margin() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::LeftMargin)
        .f64(0.7)
        .finish(&mut out)
        .unwrap();
    out
}

/// RIGHTMARGIN (0x0027): right page margin in inches (f64). Default 0.7".
pub fn right_margin() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::RightMargin)
        .f64(0.7)
        .finish(&mut out)
        .unwrap();
    out
}

/// TOPMARGIN (0x0028): top page margin in inches (f64). Default 0.75".
pub fn top_margin() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::TopMargin)
        .f64(0.75)
        .finish(&mut out)
        .unwrap();
    out
}

/// BOTTOMMARGIN (0x0029): bottom page margin in inches (f64). Default 0.75".
pub fn bottom_margin() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::BottomMargin)
        .f64(0.75)
        .finish(&mut out)
        .unwrap();
    out
}

/// PRINTHEADERS (0x002A): 1 = print row & column headers.
pub fn print_headers() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::PrintHeaders)
        .u16(0)
        .finish(&mut out)
        .unwrap();
    out
}

/// PRINTGRIDLINES (0x002B): 1 = print gridlines.
pub fn print_gridlines() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::PrintGridlines)
        .u16(0)
        .finish(&mut out)
        .unwrap();
    out
}

/// Date mode record: 0 = 1900-based (Windows), 1 = 1904-based (Mac).
pub fn date_mode(mode: u16) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::DateMode)
        .u16(mode)
        .finish(&mut out)
        .unwrap();
    out
}

/// Dimensions record: defines the used range of a sheet.
/// (rwTop=0, rwBot=n_rows-1, colLeft=0, colRight=n_cols-1, reserved=0)
pub fn dimensions(n_rows: u32, n_cols: u16) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Dimensions)
        .u32(0)
        .u32(n_rows.saturating_sub(1))
        .u16(0)
        .u16(n_cols.saturating_sub(1))
        .u16(0)
        .finish(&mut out)
        .unwrap();
    out
}

/// DEFCOLWIDTH (0x0055) record: default column width in characters
/// (counted using the zero character of the first FONT record). We
/// emit Excel's default of 8.43 characters so files without explicit
/// per-column widths still render the way Excel expects.
pub fn def_col_width(width_chars: u16) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::DefColWidth)
        .u16(width_chars)
        .finish(&mut out)
        .unwrap();
    out
}

/// Row record (BIFF8 layout per MS-XLS §2.4.307, 18-byte body).
///
///   - rowx       (2)  row index (0-based)
///   - colFirst   (2)  first used column
///   - colLast    (2)  last used column (0xFFFF = until end)
///   - rwHeight   (2)  height in twips (0xFF = "auto")
///   - irwMac     (2)  used by Excel for optimisation (0)
///   - reserved   (2)  MUST be 0
///   - grbit      (4)  option flags: 0x0001 = height matches default,
///                                 0x0002 = hidden,
///                                 0x0004 = thick top border,
///                                 0x0008 = thick bottom border
///   - ixfe       (2)  default XF index for cells in this row
pub fn row(r: u32, first_col: u16, last_col: u16) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Row)
        .u16(r as u16) // rowx
        .u16(first_col) // colFirst
        .u16(last_col) // colLast
        .u16(0x00FF) // rwHeight (0x00FF = "auto")
        .u16(0x0000) // irwMac
        .u16(0x0000) // reserved
        .u32(0x0000_0000) // grbit (4 bytes per spec — was 2-byte grbit + 2-byte xf in the buggy version)
        .u16(0x0000) // ixfe (default XF index 0)
        .finish(&mut out)
        .unwrap();
    out
}

/// Number cell (IEEE 754 double).
pub fn number_cell(row: u16, col: u16, xf: u16, value: f64) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Number)
        .u16(row)
        .u16(col)
        .u16(xf)
        .f64(value)
        .finish(&mut out)
        .unwrap();
    out
}

/// BoolErr cell.
pub fn bool_cell(row: u16, col: u16, xf: u16, value: bool) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::BoolErr)
        .u16(row)
        .u16(col)
        .u16(xf)
        .u8(if value { 1 } else { 0 })
        .u8(0) // type: 0 = bool, 1 = error
        .finish(&mut out)
        .unwrap();
    out
}

/// Error cell (BoolErr record with error type).
pub fn error_cell(row: u16, col: u16, xf: u16, error: &str) -> Vec<u8> {
    let code = match error {
        "#NULL!" | "NULL!" => 0x00u8,
        "#DIV/0!" | "DIV/0!" => 0x07,
        "#VALUE!" | "VALUE!" => 0x0F,
        "#REF!" | "REF!" => 0x17,
        "#NAME?" | "NAME?" => 0x1D,
        "#NUM!" | "NUM!" => 0x24,
        "#N/A" | "N/A" => 0x2A,
        _ => 0x2A, // default to #N/A
    };
    let mut out = Vec::new();
    Record::new(RecordId::BoolErr)
        .u16(row)
        .u16(col)
        .u16(xf)
        .u8(code)
        .u8(1) // type: 1 = error
        .finish(&mut out)
        .unwrap();
    out
}

/// LabelSst cell: a string by index into the SST.
pub fn labelsst_cell(row: u16, col: u16, xf: u16, sst_index: u32) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::LabelSst)
        .u16(row)
        .u16(col)
        .u16(xf)
        .u32(sst_index)
        .finish(&mut out)
        .unwrap();
    out
}

/// Blank cell (empty cell that still has formatting).
pub fn blank_cell(row: u16, col: u16, xf: u16) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Blank)
        .u16(row)
        .u16(col)
        .u16(xf)
        .finish(&mut out)
        .unwrap();
    out
}

/// Formula cell. `ptg_bytes` is the encoded RPN expression. The cell stores
/// a placeholder cached value of 0.0; Excel will recompute on open.
pub fn formula_cell(row: u16, col: u16, xf: u16, ptg_bytes: &[u8]) -> Vec<u8> {
    // Body layout: rw(2) col(2) ixfe(2) val(8) grbit(2) chn(4) cce(2) rgce(cce)
    // = 22 + cce bytes. Total record size includes the 4-byte header.
    let mut body = Vec::new();
    body.extend_from_slice(&row.to_le_bytes());
    body.extend_from_slice(&col.to_le_bytes());
    body.extend_from_slice(&xf.to_le_bytes());
    body.extend_from_slice(&0f64.to_le_bytes()); // cached value (placeholder)
    body.extend_from_slice(&0u16.to_le_bytes()); // grbit
    body.extend_from_slice(&0u32.to_le_bytes()); // chn
    body.extend_from_slice(&(ptg_bytes.len() as u16).to_le_bytes()); // cce
    body.extend_from_slice(ptg_bytes);
    // Pad to 4-byte boundary.
    while body.len() % 4 != 0 {
        body.push(0);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&(RecordId::Formula as u16).to_le_bytes());
    out.extend_from_slice(&(body.len() as u16).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// Formula cell with a cached numeric result.
pub fn formula_cell_cached(row: u16, col: u16, xf: u16, cached: f64, ptg_bytes: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&row.to_le_bytes());
    body.extend_from_slice(&col.to_le_bytes());
    body.extend_from_slice(&xf.to_le_bytes());
    body.extend_from_slice(&cached.to_le_bytes()); // cached numeric value
    body.extend_from_slice(&0u16.to_le_bytes()); // grbit
    body.extend_from_slice(&0u32.to_le_bytes()); // chn
    body.extend_from_slice(&(ptg_bytes.len() as u16).to_le_bytes()); // cce
    body.extend_from_slice(ptg_bytes);
    while body.len() % 4 != 0 {
        body.push(0);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&(RecordId::Formula as u16).to_le_bytes());
    out.extend_from_slice(&(body.len() as u16).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// Formula cell with a cached string result.
/// Writes the FORMULA record with string sentinel, then a STRING record.
pub fn formula_cell_string(row: u16, col: u16, xf: u16, cached: &str, ptg_bytes: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&row.to_le_bytes());
    body.extend_from_slice(&col.to_le_bytes());
    body.extend_from_slice(&xf.to_le_bytes());
    // String sentinel: 0xFFFF, type=0 (string), reserved=0
    body.extend_from_slice(&0xFFFFu16.to_le_bytes());
    body.push(0x00); // type: string
    body.push(0x00); // reserved
    body.extend_from_slice(&0u16.to_le_bytes()); // grbit
    body.extend_from_slice(&0u32.to_le_bytes()); // chn
    body.extend_from_slice(&(ptg_bytes.len() as u16).to_le_bytes()); // cce
    body.extend_from_slice(ptg_bytes);
    while body.len() % 4 != 0 {
        body.push(0);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&(RecordId::Formula as u16).to_le_bytes());
    out.extend_from_slice(&(body.len() as u16).to_le_bytes());
    out.extend_from_slice(&body);

    // STRING record: contains the cached string value
    let units: Vec<u16> = cached.encode_utf16().collect();
    let len = units.len().min(0x7FFF) as u16;
    let mut str_body = Vec::new();
    str_body.extend_from_slice(&len.to_le_bytes());
    str_body.push(0x01); // flags: UTF-16
    for unit in &units[..len as usize] {
        str_body.extend_from_slice(&unit.to_le_bytes());
    }
    out.extend_from_slice(&(RecordId::String as u16).to_le_bytes());
    out.extend_from_slice(&(str_body.len() as u16).to_le_bytes());
    out.extend_from_slice(&str_body);
    out
}

/// Formula cell with a cached boolean result.
pub fn formula_cell_bool(row: u16, col: u16, xf: u16, cached: bool, ptg_bytes: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&row.to_le_bytes());
    body.extend_from_slice(&col.to_le_bytes());
    body.extend_from_slice(&xf.to_le_bytes());
    body.extend_from_slice(&0xFFFFu16.to_le_bytes());
    body.push(0x01); // type: boolean
    body.push(if cached { 1 } else { 0 }); // value
    body.extend_from_slice(&0u16.to_le_bytes()); // grbit
    body.extend_from_slice(&0u32.to_le_bytes()); // chn
    body.extend_from_slice(&(ptg_bytes.len() as u16).to_le_bytes()); // cce
    body.extend_from_slice(ptg_bytes);
    while body.len() % 4 != 0 {
        body.push(0);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&(RecordId::Formula as u16).to_le_bytes());
    out.extend_from_slice(&(body.len() as u16).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// Merged cells record. Each merge is (start_row, start_col, end_row, end_col).
pub fn merged_cells(merges: &[(u16, u16, u16, u16)]) -> Vec<u8> {
    if merges.is_empty() {
        return Vec::new();
    }
    let mut body = Vec::new();
    body.extend_from_slice(&(merges.len() as u16).to_le_bytes());
    for &(r1, c1, r2, c2) in merges {
        body.extend_from_slice(&r1.to_le_bytes());
        body.extend_from_slice(&c1.to_le_bytes());
        body.extend_from_slice(&r2.to_le_bytes());
        body.extend_from_slice(&c2.to_le_bytes());
    }
    let mut out = Vec::new();
    out.extend_from_slice(&(RecordId::MergedCells as u16).to_le_bytes());
    out.extend_from_slice(&(body.len() as u16).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// Pane record for freeze panes.
/// `freeze_row` / `freeze_col` are the first unfrozen row/column (0 = no freeze).
pub fn pane(freeze_row: u16, freeze_col: u16) -> Vec<u8> {
    let mut out = Vec::new();
    let has_freeze = freeze_row > 0 || freeze_col > 0;
    Record::new(RecordId::Pane)
        .u16(if freeze_col > 0 { freeze_col } else { 0 }) // x (col split)
        .u16(if freeze_row > 0 { freeze_row } else { 0 }) // y (row split)
        .u16(0) // top row visible
        .u16(0) // left col visible
        .u8(if has_freeze { 0x02 } else { 0x00 }) // active pane: 0=none, 2=frozen
        .u8(0) // reserved
        .u16(0) // reserved
        .finish(&mut out)
        .unwrap();
    out
}

/// AutoFilter record. Specifies the filter range on a sheet.
pub fn auto_filter(first_row: u16, first_col: u16, last_row: u16, last_col: u16) -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::AutoFilter)
        .u16(first_row)
        .u16(last_row)
        .u16(first_col)
        .u16(last_col)
        .finish(&mut out)
        .unwrap();
    out
}

/// Country record (system settings). Optional but expected.
pub fn country() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Country)
        .u16(0x0001) // default country
        .u16(0x0001) // user country
        .finish(&mut out)
        .unwrap();
    out
}

/// UseSelfs record (workbook recalculation flags).
pub fn use_selfs() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::UseSelfs)
        .u16(0x0001)
        .finish(&mut out)
        .unwrap();
    out
}

// === BIFF8 setup records =================================================
//
// These records are required for proper Excel/xlrd compatibility. They
// appear in the workbook globals substream in the order defined by the
// spec. Omitting them causes Excel to show "unreadable content" or
// "repairs" the file on open, and causes xlrd to fall back to default
// values that are subtly wrong (e.g. it ignores sheet names).

/// InterfaceHdr (0x00E1): required marker that the workbook uses the
/// "BIFF8 / Office 97" interface. Body is the version of the interface
/// (0x04B0 = BIFF8).
pub fn interface_hdr() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::InterfaceHdr)
        .u16(0x04B0)
        .finish(&mut out)
        .unwrap();
    out
}

/// InterfaceEnd (0x00E2): required terminator for the BIFF8 interface
/// block. Body is empty.
pub fn interface_end() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::InterfaceEnd)
        .finish(&mut out)
        .unwrap();
    out
}

/// MMS (0x00C1): "Modifiable Media Stream" marker. Body is a single u16
/// that MUST be 0.
pub fn mms() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Mms)
        .u16(0x0000)
        .finish(&mut out)
        .unwrap();
    out
}

/// WriteAccess (0x005C): user name of last saver.
///
/// BIFF8 body is an XlsUnicodeString (u16 nchars + UTF-16LE chars). The
/// total body length should be at most 112 bytes (0x70) including the
/// 2-byte length prefix; older Excel versions truncate at 0x70 and pad
/// with nulls. We emit a short owner name ("xls-rs", 6 chars) without
/// trailing padding so the u16 length prefix is unambiguous.
pub fn write_access() -> Vec<u8> {
    let mut out = Vec::new();
    let owner = "xls-rs";
    let mut body: Vec<u8> = Vec::new();
    let units: Vec<u16> = owner.encode_utf16().collect();
    body.extend_from_slice(&(units.len() as u16).to_le_bytes());
    for u in &units {
        body.extend_from_slice(&u.to_le_bytes());
    }
    // Pad to 112 bytes (0x70) with nulls for compatibility with older
    // Excel versions that expect a fixed-size slot.
    while body.len() < 0x70 {
        body.push(0);
    }
    let mut hdr = Vec::new();
    Record::new(RecordId::WriteAccess)
        .bytes(&body)
        .finish(&mut hdr)
        .unwrap();
    out.extend_from_slice(&hdr);
    out
}

/// DSF (0x0161): "Double Stream File" flag. Body is a single u16 that
/// MUST be 0 (only the BIFF8 stream is present).
pub fn dsf() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Dsf)
        .u16(0x0000)
        .finish(&mut out)
        .unwrap();
    out
}

/// TabId (0x013D): one u16 per sheet, each holding the (1-based) tab id.
/// The i-th entry MUST equal i+1.
pub fn tab_id(sheet_count: u16) -> Vec<u8> {
    let mut out = Vec::new();
    let mut rec = Record::new(RecordId::TabId);
    for i in 0..sheet_count {
        rec = rec.u16(i as u16 + 1);
    }
    rec.finish(&mut out).unwrap();
    out
}

/// FnGroupName (0x009C): function group name count. Body is two bytes
/// (count, reserved) = (0x0E, 0x00) — matching xlwt.
pub fn fn_group_name() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::FnGroupName)
        .u8(0x0E)
        .u8(0x00)
        .finish(&mut out)
        .unwrap();
    out
}

/// WindowProtect (0x0019): workbook window protection flag.
pub fn window_protect() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::WindowProtect)
        .u16(0x0000)
        .finish(&mut out)
        .unwrap();
    out
}

/// Protect (0x0012): workbook structure protection flag.
pub fn protect() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Protect)
        .u16(0x0000)
        .finish(&mut out)
        .unwrap();
    out
}

/// ObjectProtect (0x0063): object protection flag.
pub fn object_protect() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::ObjectProtect)
        .u16(0x0000)
        .finish(&mut out)
        .unwrap();
    out
}

/// Password (0x0013): zero hash (no password set).
pub fn password() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Password)
        .u16(0x0000)
        .finish(&mut out)
        .unwrap();
    out
}

/// Prot4Rev (0x01AF): "Protect for Revisions" flag (off).
pub fn prot4_rev() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Prot4Rev)
        .u16(0x0000)
        .finish(&mut out)
        .unwrap();
    out
}

/// Prot4RevPass (0x01BC): password for "Protect for Revisions" (none).
pub fn prot4_rev_pass() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Prot4RevPass)
        .u16(0x0000)
        .finish(&mut out)
        .unwrap();
    out
}

/// Backup (0x0040): "make backup on save" flag (off).
pub fn backup() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Backup)
        .u16(0x0000)
        .finish(&mut out)
        .unwrap();
    out
}

/// HideObj (0x008D): object display mode (0 = show all).
pub fn hide_obj() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::HideObj)
        .u16(0x0000)
        .finish(&mut out)
        .unwrap();
    out
}

/// Precision (0x000E): "use real cell values for calculation" flag.
/// 1 = use real values (Excel's default).
pub fn precision() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::Precision)
        .u16(0x0001)
        .finish(&mut out)
        .unwrap();
    out
}

/// RefreshAll (0x01B7): "refresh all on load" flag (off).
pub fn refresh_all() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::RefreshAll)
        .u16(0x0000)
        .finish(&mut out)
        .unwrap();
    out
}

/// BookBool (0x00DA): "save external linked values" flag.
/// 0 = save external linked values (Excel's default).
pub fn book_bool() -> Vec<u8> {
    let mut out = Vec::new();
    Record::new(RecordId::BookBool)
        .u16(0x0000)
        .finish(&mut out)
        .unwrap();
    out
}

/// Build the SST record from a list of unique strings. Returns the bytes and
/// the index of each string in the SST (a Vec parallel to `strings`).
///
/// SST body layout (XlsUnicodeStringNoCch per string):
///   - total u32 (total strings including duplicates)
///   - unique u32
///   - per string:
///     - cch (u16) — number of characters
///     - optionFlags (u8):
///         bit 0: fHighByte (1 = UTF-16, 0 = 1-byte)
///         bit 1: reserved
///         bit 2: fExtSt
///         bit 3: fRichSt
///     - characters: cch bytes (if fHighByte = 0) or cch * 2 bytes (UTF-16LE)
pub fn sst(strings: &[String]) -> (Vec<u8>, Vec<u32>) {
    let total = strings.len() as u32;
    let unique = strings.len() as u32;
    let mut body = Vec::new();
    body.extend_from_slice(&total.to_le_bytes());
    body.extend_from_slice(&unique.to_le_bytes());

    let mut indices = Vec::with_capacity(strings.len());
    for s in strings {
        indices.push(body.len() as u32);
        write_sst_string(&mut body, s);
    }

    let mut out = Vec::new();
    Record::new(RecordId::Sst)
        .bytes(&body)
        .finish(&mut out)
        .unwrap();
    (out, indices)
}

fn write_sst_string(body: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    let is_ascii = bytes.iter().all(|b| b.is_ascii());
    if is_ascii {
        let len = bytes.len();
        let cch = len.min(0x7FFF) as u16;
        body.extend_from_slice(&cch.to_le_bytes());
        body.push(0x00); // flags: no high byte, no ext, no rich
        body.extend_from_slice(&bytes[..len.min(0x7FFF)]);
    } else {
        let units: Vec<u16> = s.encode_utf16().collect();
        let len = units.len().min(0x7FFF);
        body.extend_from_slice(&(len as u16).to_le_bytes());
        body.push(0x01); // flags: high byte (UTF-16)
        for u in &units[..len] {
            body.extend_from_slice(&u.to_le_bytes());
        }
    }
}

/// BoundSheet record (BIFF8). `stream_pos` is the absolute byte offset
/// of the sheet's BOF record within the workbook stream. `state = 0`
/// (visible), `kind = 0` (worksheet).
///
/// Body layout per MS-XLS §2.4.86 (and matching what xlwt / xlrd use):
///   - Position  (4 bytes u32)
///   - Visibility (1 byte u8) — 0 = visible, 1 = hidden, 2 = very hidden
///   - Type (1 byte u8)       — 0 = worksheet, 1 = macro, 2 = chart, ...
///   - Name (XlsUnicodeStringNoCch: cch 1 byte, options 1 byte, chars)
///
/// XlsUnicodeStringNoCch has an 8-bit character count, not 16-bit —
/// contrary to the spec doc which calls it "16-bit string length" but
/// in practice BIFF8 readers (xlrd, LibreOffice) and writers (xlwt)
/// all use 8-bit here.
pub fn bound_sheet(sheet_index: u32, stream_pos: u32, name: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&stream_pos.to_le_bytes()); // 4 bytes
    body.push(0u8); // 1 byte — visibility (0 = visible)
    body.push(0u8); // 1 byte — type (0 = worksheet)
    // XlsUnicodeStringNoCch: cch (1 byte), optionFlags (1 byte), chars.
    // We always use UTF-16 (fHighByte = 1) so non-ASCII sheet names
    // round-trip cleanly.
    let units: Vec<u16> = name.encode_utf16().collect();
    let n = units.len().min(255) as u8;
    body.push(n); // 1 byte — character count
    body.push(0x01); // 1 byte — optionFlags: fHighByte = 1 (UTF-16)
    for u in units.iter().take(n as usize) {
        body.extend_from_slice(&u.to_le_bytes());
    }
    let mut out = Vec::new();
    let rec_id: u16 = 0x0085; // BoundSheet
    out.extend_from_slice(&rec_id.to_le_bytes());
    out.extend_from_slice(&(body.len() as u16).to_le_bytes());
    out.extend_from_slice(&body);
    let _ = sheet_index;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_records(bytes: &[u8]) -> Vec<(u16, Vec<u8>)> {
        let mut out = Vec::new();
        let mut p = 0;
        while p + 4 <= bytes.len() {
            let id = u16::from_le_bytes([bytes[p], bytes[p + 1]]);
            let len = u16::from_le_bytes([bytes[p + 2], bytes[p + 3]]);
            let end = p + 4 + len as usize;
            if end > bytes.len() {
                break;
            }
            out.push((id, bytes[p + 4..end].to_vec()));
            p = end;
        }
        out
    }

    #[test]
    fn codepage_record_layout() {
        let bytes = codepage();
        // BIFF8 CodePage record id is 0x0042, value is 0x04B0 (UTF-16).
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x0042);
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 2);
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 0x04B0);
    }

    #[test]
    fn eof_record() {
        let bytes = eof();
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x000A);
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 0);
    }

    #[test]
    fn bof_workbook_record_layout() {
        // BIFF8 BOF body is 16 bytes (per MS-XLS §2.4.21).
        // Total record on the wire: 4-byte header + 16-byte body = 20 bytes.
        let bytes = bof_workbook();
        assert_eq!(bytes.len(), 20, "BOF total must be 20 bytes (4-byte header + 16-byte body)");
        // Header
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x0809); // BOF id
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 16);     // body size
        // Body
        let body = &bytes[4..];
        assert_eq!(u16::from_le_bytes([body[0], body[1]]), 0x0600);   // version
        assert_eq!(u16::from_le_bytes([body[2], body[3]]), 0x0005);   // type (workbook)
        // Build id and year (opaque, but non-zero is fine)
        let build_id = u16::from_le_bytes([body[4], body[5]]);
        let build_year = u16::from_le_bytes([body[6], body[7]]);
        assert!(build_id > 0, "build id should be set");
        assert!(build_year > 0, "build year should be set");
        // Reserved (MUST be 0)
        let reserved = u32::from_le_bytes([body[8], body[9], body[10], body[11]]);
        assert_eq!(reserved, 0, "reserved field MUST be 0");
    }

    #[test]
    fn bof_sheet_record_layout() {
        let bytes = bof_sheet();
        assert_eq!(bytes.len(), 20, "BOF total must be 20 bytes (4-byte header + 16-byte body)");
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x0809);
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 16);
        let body = &bytes[4..];
        assert_eq!(u16::from_le_bytes([body[0], body[1]]), 0x0600);
        assert_eq!(u16::from_le_bytes([body[2], body[3]]), 0x0010); // type (worksheet)
        let reserved = u32::from_le_bytes([body[8], body[9], body[10], body[11]]);
        assert_eq!(reserved, 0, "reserved field MUST be 0");
    }

    #[test]
    fn number_cell_record() {
        let bytes = number_cell(0, 0, 0, std::f64::consts::PI);
        let recs = parse_records(&bytes);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].0, 0x0203);
        let body = &recs[0].1;
        assert_eq!(u16::from_le_bytes([body[0], body[1]]), 0);
        assert_eq!(u16::from_le_bytes([body[2], body[3]]), 0);
        assert_eq!(u16::from_le_bytes([body[4], body[5]]), 0);
        let v = f64::from_le_bytes(body[6..14].try_into().unwrap());
        assert!((v - std::f64::consts::PI).abs() < 1e-12);
    }

    #[test]
    fn sst_record_layout() {
        let strings = vec!["foo".to_string(), "bar".to_string(), "baz".to_string()];
        let (bytes, indices) = sst(&strings);
        assert_eq!(indices.len(), 3);
        // Verify each string can be re-parsed (length=correct).
        let recs = parse_records(&bytes);
        assert_eq!(recs[0].0, 0x00FC);
        let body = &recs[0].1;
        assert_eq!(u32::from_le_bytes([body[0], body[1], body[2], body[3]]), 3);
        assert_eq!(u32::from_le_bytes([body[4], body[5], body[6], body[7]]), 3);
    }

    #[test]
    fn bound_sheet_record_layout() {
        let bytes = bound_sheet(0, 0x100, "Sheet1");
        let recs = parse_records(&bytes);
        assert_eq!(recs[0].0, 0x0085);
    }
}
