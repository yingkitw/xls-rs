//! Style registry for the XLSX writer.
//!
//! This module is inspired by (and adapted from) the `xlsgen` crate's
//! `StyleRegistry`. OOXML does not store rich style objects per cell:
//! each cell references an integer index into a `<cellXfs>` table, and
//! each `cellXf` references indices into `<fonts>`, `<fills>`, `<borders>`
//! and a `<numFmt>`. This module deduplicates every unique style
//! component, assigns stable indices once, and lets callers look up
//! the `s="N"` index for any cell.
//!
//! The pattern is:
//!
//! 1. The writer walks all cells once and calls `register` for every
//!    `XlsxCellStyle` it sees.
//! 2. At emit time it calls `lookup(style)` (or stores the index it
//!    got from `register`) and emits `s="N"` on each `<c>`.
//! 3. The serialized tables (`<fonts>`, `<fills>`, ..., `<cellXfs>`)
//!    are built from the registry contents.
//!
//! Number formats with built-in IDs (General, 0, 0.00, m/d/yyyy, ...)
//! are resolved to their reserved IDs without emitting a `<numFmt>`
//! entry. Custom formats are interned by code string and assigned
//! IDs starting at 164.

use std::collections::HashMap;

/// Built-in numFmt IDs reserved by Excel.
pub mod builtin_numfmt {
    pub const GENERAL: u32 = 0;
    pub const INTEGER: u32 = 1; // `0`
    pub const DECIMAL_2: u32 = 2; // `0.00`
    pub const THOUSANDS: u32 = 3; // `#,##0`
    pub const THOUSANDS_DECIMAL: u32 = 4; // `#,##0.00`
    pub const PERCENT: u32 = 9; // `0%`
    pub const PERCENT_DECIMAL: u32 = 10; // `0.00%`
    pub const DATE_SLASH: u32 = 14; // `m/d/yyyy`
    pub const TIME_COLON: u32 = 21; // `h:mm:ss AM/PM`
    pub const DATETIME: u32 = 22; // `m/d/yyyy h:mm`
}

/// Custom number formats must use IDs >= 164.
pub const FIRST_CUSTOM_NUMFMT_ID: u32 = 164;

fn builtin_id_for(code: &str) -> u32 {
    match code {
        "General" => builtin_numfmt::GENERAL,
        "0" => builtin_numfmt::INTEGER,
        "0.00" => builtin_numfmt::DECIMAL_2,
        "#,##0" => builtin_numfmt::THOUSANDS,
        "#,##0.00" => builtin_numfmt::THOUSANDS_DECIMAL,
        "0%" => builtin_numfmt::PERCENT,
        "0.00%" => builtin_numfmt::PERCENT_DECIMAL,
        "m/d/yyyy" => builtin_numfmt::DATE_SLASH,
        "h:mm:ss AM/PM" => builtin_numfmt::TIME_COLON,
        "m/d/yyyy h:mm" => builtin_numfmt::DATETIME,
        _ => 0,
    }
}

/// RGB color, accepting either `RRGGBB` or `AARRGGBB` hex strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn parse(s: &str) -> Option<Rgb> {
        let s = s.trim().trim_start_matches('#');
        if s.len() == 6 {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some(Rgb(r, g, b))
        } else if s.len() == 8 {
            let _ = u8::from_str_radix(&s[0..2], 16).ok()?;
            let r = u8::from_str_radix(&s[2..4], 16).ok()?;
            let g = u8::from_str_radix(&s[4..6], 16).ok()?;
            let b = u8::from_str_radix(&s[6..8], 16).ok()?;
            Some(Rgb(r, g, b))
        } else {
            None
        }
    }

    pub fn to_argb_hex(self) -> String {
        format!("FF{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }
}

/// Per-cell style options. Mirrors xlsgen's `CellStyle`: every field
/// is optional, and `None` falls back to the workbook default.
///
/// This struct is intentionally NOT `Hash`/`Eq` because it contains
/// `Option<f64>` (font size). The registry converts to integer-valued
/// keys (`FontKey`, `FillKey`, ...) before interning.
#[derive(Debug, Clone, Default)]
pub struct XlsxCellStyle {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub font_size: Option<f64>,
    pub font_name: Option<String>,
    /// Hex `RRGGBB` or `AARRGGBB`.
    pub font_color: Option<String>,
    /// Hex `RRGGBB` or `AARRGGBB`.
    pub fill_color: Option<String>,
    /// `"left"`, `"center"`, `"right"`.
    pub align: Option<String>,
    /// `"top"`, `"center"`, `"bottom"`.
    pub valign: Option<String>,
    pub wrap: Option<bool>,
    /// `"thin"`, `"medium"`, `"thick"`, `"dashed"`, `"dotted"`.
    pub border: Option<String>,
    /// Hex color for border.
    pub border_color: Option<String>,
    /// A built-in or custom number format code.
    pub number_format: Option<String>,
    /// Treat the value as a date; defaults to `"yyyy-mm-dd"`.
    pub date: Option<bool>,
}

impl XlsxCellStyle {
    pub fn is_empty(&self) -> bool {
        self.bold.is_none()
            && self.italic.is_none()
            && self.underline.is_none()
            && self.font_size.is_none()
            && self.font_name.is_none()
            && self.font_color.is_none()
            && self.fill_color.is_none()
            && self.align.is_none()
            && self.valign.is_none()
            && self.wrap.is_none()
            && self.border.is_none()
            && self.border_color.is_none()
            && self.number_format.is_none()
            && self.date.is_none()
    }

    /// Convenience: bold + 14pt header style.
    pub fn header() -> Self {
        Self {
            bold: Some(true),
            font_size: Some(14.0),
            ..Default::default()
        }
    }

    /// Convenience: bold + accent fill (for highlighted cells).
    pub fn highlighted() -> Self {
        Self {
            bold: Some(true),
            fill_color: Some("305496".to_string()),
            font_color: Some("FFFFFF".to_string()),
            ..Default::default()
        }
    }

    /// Convenience: italic + dim text (for notes).
    pub fn note() -> Self {
        Self {
            italic: Some(true),
            font_color: Some("595959".to_string()),
            ..Default::default()
        }
    }
}

/// A `<font>` entry, interned by key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FontKey {
    pub name: String,
    /// Excel stores font sizes as `val * 100`.
    pub size_half: u32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color_argb: Option<String>,
}

impl FontKey {
    pub fn from_style(s: &XlsxCellStyle) -> FontKey {
        let size = s.font_size.unwrap_or(11.0);
        FontKey {
            name: s.font_name.clone().unwrap_or_else(|| "Calibri".into()),
            size_half: (size * 100.0).round() as u32,
            bold: s.bold.unwrap_or(false),
            italic: s.italic.unwrap_or(false),
            underline: s.underline.unwrap_or(false),
            color_argb: s
                .font_color
                .as_deref()
                .and_then(Rgb::parse)
                .map(|c| c.to_argb_hex()),
        }
    }

    pub fn default_calibri() -> FontKey {
        FontKey {
            name: "Calibri".into(),
            size_half: 1100,
            bold: false,
            italic: false,
            underline: false,
            color_argb: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FillKey {
    pub color_argb: Option<String>,
}

impl FillKey {
    pub fn from_style(s: &XlsxCellStyle) -> FillKey {
        FillKey {
            color_argb: s
                .fill_color
                .as_deref()
                .and_then(Rgb::parse)
                .map(|c| c.to_argb_hex()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct BorderSide {
    pub style: String,
    pub color_argb: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct BorderKey {
    pub left: BorderSide,
    pub right: BorderSide,
    pub top: BorderSide,
    pub bottom: BorderSide,
}

impl BorderKey {
    pub fn from_style(s: &XlsxCellStyle) -> BorderKey {
        let style = s.border.clone().unwrap_or_default();
        let color = s
            .border_color
            .as_deref()
            .and_then(Rgb::parse)
            .map(|c| c.to_argb_hex());
        let side = |st: String, co: Option<String>| BorderSide { style: st, color_argb: co };
        if style.is_empty() {
            BorderKey {
                left: side(String::new(), None),
                right: side(String::new(), None),
                top: side(String::new(), None),
                bottom: side(String::new(), None),
            }
        } else {
            BorderKey {
                left: side(style.clone(), color.clone()),
                right: side(style.clone(), color.clone()),
                top: side(style.clone(), color.clone()),
                bottom: side(style, color),
            }
        }
    }

    pub fn empty() -> BorderKey {
        BorderKey {
            left: BorderSide { style: String::new(), color_argb: None },
            right: BorderSide { style: String::new(), color_argb: None },
            top: BorderSide { style: String::new(), color_argb: None },
            bottom: BorderSide { style: String::new(), color_argb: None },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct AlignmentKey {
    pub horizontal: Option<String>,
    pub vertical: Option<String>,
    pub wrap_text: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CellXfKey {
    pub num_fmt_id: u32,
    pub font_id: u32,
    pub fill_id: u32,
    pub border_id: u32,
    pub apply_number_format: bool,
    pub apply_font: bool,
    pub apply_fill: bool,
    pub apply_border: bool,
    pub apply_alignment: bool,
    pub alignment: AlignmentKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct NumFmtKey {
    pub id: u32,
    pub code: String,
}

/// Holds the interned style components and their XLSX indices.
#[derive(Debug, Clone)]
pub struct StyleRegistry {
    pub(crate) fonts: Vec<FontKey>,
    pub(crate) fills: Vec<FillKey>,
    pub(crate) borders: Vec<BorderKey>,
    pub(crate) num_fmts: Vec<NumFmtKey>,
    pub(crate) cell_xfs: Vec<CellXfKey>,
    font_index: HashMap<FontKey, u32>,
    fill_index: HashMap<FillKey, u32>,
    border_index: HashMap<BorderKey, u32>,
    num_fmt_id_by_code: HashMap<String, u32>,
    cell_xf_index: HashMap<CellXfKey, u32>,
}

impl StyleRegistry {
    /// Create a registry with the XLSX-required default entries at
    /// index 0 (default font, none/gray fills, no border, default xf).
    pub fn new() -> Self {
        let mut reg = StyleRegistry {
            fonts: Vec::new(),
            fills: Vec::new(),
            borders: Vec::new(),
            num_fmts: Vec::new(),
            cell_xfs: Vec::new(),
            font_index: HashMap::new(),
            fill_index: HashMap::new(),
            border_index: HashMap::new(),
            num_fmt_id_by_code: HashMap::new(),
            cell_xf_index: HashMap::new(),
        };

        let default_font = FontKey::default_calibri();
        reg.fonts.push(default_font.clone());
        reg.font_index.insert(default_font, 0);

        let none_fill = FillKey { color_argb: None };
        let gray_fill = FillKey {
            color_argb: Some("FFEFEFEF".into()),
        };
        reg.fills.push(none_fill.clone());
        reg.fills.push(gray_fill.clone());
        reg.fill_index.insert(none_fill, 0);
        reg.fill_index.insert(gray_fill, 1);

        let no_border = BorderKey::empty();
        reg.borders.push(no_border.clone());
        reg.border_index.insert(no_border, 0);

        for code in [
            "General", "0", "0.00", "#,##0", "#,##0.00",
            "0%", "0.00%", "m/d/yyyy", "h:mm:ss AM/PM", "m/d/yyyy h:mm",
        ] {
            let id = builtin_id_for(code);
            reg.num_fmt_id_by_code.insert(code.to_string(), id);
        }

        let default_xf = CellXfKey {
            num_fmt_id: 0,
            font_id: 0,
            fill_id: 0,
            border_id: 0,
            apply_number_format: false,
            apply_font: false,
            apply_fill: false,
            apply_border: false,
            apply_alignment: false,
            alignment: AlignmentKey {
                horizontal: None,
                vertical: None,
                wrap_text: false,
            },
        };
        reg.cell_xfs.push(default_xf.clone());
        reg.cell_xf_index.insert(default_xf, 0);

        reg
    }

    /// Register a custom number-format code. Built-in codes return
    /// their reserved IDs without emitting a `<numFmt>` entry.
    pub fn intern_num_fmt(&mut self, code: &str) -> u32 {
        if let Some(&id) = self.num_fmt_id_by_code.get(code) {
            return id;
        }
        let next_id = self
            .num_fmts
            .iter()
            .map(|n| n.id)
            .max()
            .unwrap_or(FIRST_CUSTOM_NUMFMT_ID - 1)
            + 1;
        let next_id = next_id.max(FIRST_CUSTOM_NUMFMT_ID);
        self.num_fmts.push(NumFmtKey {
            id: next_id,
            code: code.to_string(),
        });
        self.num_fmt_id_by_code.insert(code.to_string(), next_id);
        next_id
    }

    /// Register an `XlsxCellStyle` and return the `s="..."` index.
    pub fn register(&mut self, style: &XlsxCellStyle) -> u32 {
        let font = FontKey::from_style(style);
        let fill = FillKey::from_style(style);
        let border = BorderKey::from_style(style);

        let num_fmt_id = if let Some(code) = &style.number_format {
            self.intern_num_fmt(code)
        } else if style.date.unwrap_or(false) {
            self.intern_num_fmt("yyyy-mm-dd")
        } else {
            builtin_numfmt::GENERAL
        };

        let has_font_overrides = font != self.fonts[0];
        let has_fill_overrides = fill != self.fills[0];
        let has_border_overrides = border != self.borders[0];
        let has_align_overrides = style.align.is_some()
            || style.valign.is_some()
            || style.wrap.unwrap_or(false);
        let has_num_overrides = num_fmt_id != builtin_numfmt::GENERAL;

        let font_id = self.intern_font(font);
        let fill_id = self.intern_fill(fill);
        let border_id = self.intern_border(border);

        let xf = CellXfKey {
            num_fmt_id,
            font_id,
            fill_id,
            border_id,
            apply_number_format: has_num_overrides,
            apply_font: has_font_overrides,
            apply_fill: has_fill_overrides,
            apply_border: has_border_overrides,
            apply_alignment: has_align_overrides,
            alignment: AlignmentKey {
                horizontal: style.align.clone(),
                vertical: style.valign.clone(),
                wrap_text: style.wrap.unwrap_or(false),
            },
        };

        if let Some(&i) = self.cell_xf_index.get(&xf) {
            return i;
        }
        let i = self.cell_xfs.len() as u32;
        self.cell_xfs.push(xf.clone());
        self.cell_xf_index.insert(xf, i);
        i
    }

    /// Look up the index for an `XlsxCellStyle` that has already been
    /// registered. Returns 0 (the default cellXf) if the style is
    /// unknown — callers must call `register` first.
    pub fn lookup(&self, style: &XlsxCellStyle) -> u32 {
        let font = FontKey::from_style(style);
        let fill = FillKey::from_style(style);
        let border = BorderKey::from_style(style);

        let num_fmt_id = if let Some(code) = &style.number_format {
            self.num_fmt_id_by_code.get(code).copied().unwrap_or(builtin_numfmt::GENERAL)
        } else if style.date.unwrap_or(false) {
            self.num_fmt_id_by_code.get("yyyy-mm-dd").copied().unwrap_or(builtin_numfmt::GENERAL)
        } else {
            builtin_numfmt::GENERAL
        };

        let xf = CellXfKey {
            num_fmt_id,
            font_id: *self.font_index.get(&font).unwrap_or(&0),
            fill_id: *self.fill_index.get(&fill).unwrap_or(&0),
            border_id: *self.border_index.get(&border).unwrap_or(&0),
            apply_number_format: num_fmt_id != builtin_numfmt::GENERAL,
            apply_font: font != self.fonts[0],
            apply_fill: fill != self.fills[0],
            apply_border: border != self.borders[0],
            apply_alignment: style.align.is_some()
                || style.valign.is_some()
                || style.wrap.unwrap_or(false),
            alignment: AlignmentKey {
                horizontal: style.align.clone(),
                vertical: style.valign.clone(),
                wrap_text: style.wrap.unwrap_or(false),
            },
        };

        self.cell_xf_index.get(&xf).copied().unwrap_or(0)
    }

    fn intern_font(&mut self, key: FontKey) -> u32 {
        if let Some(&i) = self.font_index.get(&key) {
            return i;
        }
        let i = self.fonts.len() as u32;
        self.fonts.push(key.clone());
        self.font_index.insert(key, i);
        i
    }

    fn intern_fill(&mut self, key: FillKey) -> u32 {
        if let Some(&i) = self.fill_index.get(&key) {
            return i;
        }
        let i = self.fills.len() as u32;
        self.fills.push(key.clone());
        self.fill_index.insert(key, i);
        i
    }

    fn intern_border(&mut self, key: BorderKey) -> u32 {
        if let Some(&i) = self.border_index.get(&key) {
            return i;
        }
        let i = self.borders.len() as u32;
        self.borders.push(key.clone());
        self.border_index.insert(key, i);
        i
    }

    /// Number of unique styles registered.
    pub fn cell_xf_count(&self) -> usize {
        self.cell_xfs.len()
    }

    /// Number of unique number formats registered.
    pub fn num_fmt_count(&self) -> usize {
        self.num_fmts.len()
    }

    /// Number of unique fonts registered.
    pub fn font_count(&self) -> usize {
        self.fonts.len()
    }

    /// Number of unique fills registered.
    pub fn fill_count(&self) -> usize {
        self.fills.len()
    }

    /// Number of unique borders registered.
    pub fn border_count(&self) -> usize {
        self.borders.len()
    }
}

impl Default for StyleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared strings table. Strings are interned by value and
/// deduplicated; the index is what cells reference via `t="s"`.
#[derive(Debug, Clone, Default)]
pub struct SharedStrings {
    entries: Vec<String>,
    index: std::collections::BTreeMap<String, u32>,
}

impl SharedStrings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the index of `s`, inserting it if absent.
    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(&i) = self.index.get(s) {
            return i;
        }
        let i = self.entries.len() as u32;
        self.entries.push(s.to_string());
        self.index.insert(s.to_string(), i);
        i
    }

    /// Look up an existing string. Returns `None` if absent.
    pub fn get(&self, s: &str) -> Option<u32> {
        self.index.get(s).copied()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_codes_resolve_to_reserved_ids() {
        let mut reg = StyleRegistry::new();
        assert_eq!(reg.intern_num_fmt("General"), builtin_numfmt::GENERAL);
        assert_eq!(reg.intern_num_fmt("#,##0.00"), builtin_numfmt::THOUSANDS_DECIMAL);
        assert_eq!(reg.intern_num_fmt("m/d/yyyy"), builtin_numfmt::DATE_SLASH);
    }

    #[test]
    fn custom_numfmt_dedupes_by_code() {
        let mut reg = StyleRegistry::new();
        let a = reg.intern_num_fmt("yyyy-mm-dd");
        let b = reg.intern_num_fmt("yyyy-mm-dd");
        assert_eq!(a, b);
        assert_eq!(reg.num_fmt_count(), 1, "yyyy-mm-dd should be registered as custom");
    }

    #[test]
    fn date_flag_interns_iso_date_as_custom() {
        let mut reg = StyleRegistry::new();
        let s = XlsxCellStyle {
            date: Some(true),
            ..Default::default()
        };
        let _ = reg.register(&s);
        let id = reg.intern_num_fmt("yyyy-mm-dd");
        assert!(id >= FIRST_CUSTOM_NUMFMT_ID);
    }

    #[test]
    fn custom_numfmt_assigns_consecutive_ids() {
        let mut reg = StyleRegistry::new();
        let a = reg.intern_num_fmt("$#,##0.00");
        let b = reg.intern_num_fmt("0.000%");
        assert!(a >= FIRST_CUSTOM_NUMFMT_ID);
        assert!(b >= FIRST_CUSTOM_NUMFMT_ID);
        assert_ne!(a, b);
    }

    #[test]
    fn same_style_returns_same_cellxf_index() {
        let mut reg = StyleRegistry::new();
        let s = XlsxCellStyle {
            bold: Some(true),
            fill_color: Some("AAAAAA".into()),
            ..Default::default()
        };
        let a = reg.register(&s);
        let b = reg.register(&s);
        assert_eq!(a, b);
    }

    #[test]
    fn lookup_matches_register() {
        let mut reg = StyleRegistry::new();
        let s = XlsxCellStyle {
            bold: Some(true),
            number_format: Some("$#,##0.00".into()),
            ..Default::default()
        };
        let r = reg.register(&s);
        assert_eq!(reg.lookup(&s), r);
    }

    #[test]
    fn empty_style_returns_default_index() {
        let reg = StyleRegistry::new();
        assert_eq!(reg.lookup(&XlsxCellStyle::default()), 0);
    }

    #[test]
    fn rgb_parses_short_form() {
        let c = Rgb::parse("4472C4").unwrap();
        assert_eq!(c, Rgb(0x44, 0x72, 0xC4));
        assert_eq!(c.to_argb_hex(), "FF4472C4");
    }

    #[test]
    fn rgb_parses_with_hash_and_alpha() {
        let c = Rgb::parse("#FF4472C4").unwrap();
        assert_eq!(c, Rgb(0x44, 0x72, 0xC4));
        let c = Rgb::parse("FF4472C4").unwrap();
        assert_eq!(c, Rgb(0x44, 0x72, 0xC4));
    }

    #[test]
    fn rgb_rejects_garbage() {
        assert!(Rgb::parse("xyz").is_none());
        assert!(Rgb::parse("#1").is_none());
        assert!(Rgb::parse("12345").is_none());
    }

    #[test]
    fn shared_strings_dedup() {
        let mut s = SharedStrings::new();
        assert_eq!(s.intern("hello"), 0);
        assert_eq!(s.intern("hello"), 0);
        assert_eq!(s.intern("world"), 1);
        assert_eq!(s.intern("hello"), 0);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn xlsx_cell_style_is_empty_detects_unset() {
        assert!(XlsxCellStyle::default().is_empty());
        let mut s = XlsxCellStyle::default();
        s.bold = Some(true);
        assert!(!s.is_empty());
    }

    #[test]
    fn presets_produce_different_indices() {
        let mut reg = StyleRegistry::new();
        let h = XlsxCellStyle::header();
        let n = XlsxCellStyle::note();
        assert_ne!(reg.register(&h), reg.register(&n));
        assert_ne!(reg.register(&XlsxCellStyle::highlighted()), reg.register(&h));
    }
}
