//! XLSX style table reader — parses `xl/styles.xml` into structured data.
//!
//! The XLSX style sheet contains five tables that cells reference by index:
//! - `<numFmts>` — custom number format codes (built-ins use reserved IDs)
//! - `<fonts>` — font definitions (name, size, bold, italic, underline, color)
//! - `<fills>` — fill definitions (pattern fill with fg/bg color)
//! - `<borders>` — border definitions (per-side style + color)
//! - `<cellXfs>` — cell format entries that tie the above together with alignment
//!
//! Each `<c>` element in a worksheet may carry an `s="N"` attribute that
//! indexes into `<cellXfs>`. This module resolves that index to an
//! `XlsxCellStyle` so callers can inspect or re-write styles.

use std::collections::HashMap;

use super::xlsx_reader::{XmlScanner, xml_unescape};
use super::xlsx_writer::XlsxCellStyle;

/// Parsed font entry from `<fonts>`.
#[derive(Debug, Clone, Default)]
pub struct FontInfo {
    pub name: String,
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// ARGB hex string like `"FF305496"`.
    pub color: Option<String>,
}

/// Parsed fill entry from `<fills>`.
#[derive(Debug, Clone, Default)]
pub struct FillInfo {
    /// ARGB hex string for solid pattern fills.
    pub color: Option<String>,
}

/// One side of a border.
#[derive(Debug, Clone, Default)]
pub struct BorderSideInfo {
    pub style: String,
    pub color: Option<String>,
}

/// Parsed border entry from `<borders>`.
#[derive(Debug, Clone, Default)]
pub struct BorderInfo {
    pub left: BorderSideInfo,
    pub right: BorderSideInfo,
    pub top: BorderSideInfo,
    pub bottom: BorderSideInfo,
}

/// Parsed number format from `<numFmts>`.
#[derive(Debug, Clone)]
pub struct NumFmtInfo {
    pub id: u32,
    pub code: String,
}

/// Parsed alignment from `<alignment>` inside a `<xf>`.
#[derive(Debug, Clone, Default)]
pub struct AlignmentInfo {
    pub horizontal: Option<String>,
    pub vertical: Option<String>,
    pub wrap_text: bool,
}

/// Parsed cell format from `<cellXfs>`.
#[derive(Debug, Clone)]
pub struct CellXfInfo {
    pub num_fmt_id: u32,
    pub font_id: u32,
    pub fill_id: u32,
    pub border_id: u32,
    pub alignment: AlignmentInfo,
}

/// The complete parsed style table from `styles.xml`.
#[derive(Debug, Clone, Default)]
pub struct XlsxStyleTable {
    pub num_fmts: Vec<NumFmtInfo>,
    pub fonts: Vec<FontInfo>,
    pub fills: Vec<FillInfo>,
    pub borders: Vec<BorderInfo>,
    pub cell_xfs: Vec<CellXfInfo>,
    /// Quick lookup: numFmtId → format code (includes built-ins).
    num_fmt_by_id: HashMap<u32, String>,
}

/// Returns the format code for a built-in numFmt ID, if it is one.
fn builtin_numfmt_code(id: u32) -> Option<&'static str> {
    match id {
        0 => Some("General"),
        1 => Some("0"),
        2 => Some("0.00"),
        3 => Some("#,##0"),
        4 => Some("#,##0.00"),
        9 => Some("0%"),
        10 => Some("0.00%"),
        11 => Some("0.00E+00"),
        12 => Some("# ?/?"),
        13 => Some("# ??/??"),
        14 => Some("m/d/yyyy"),
        15 => Some("d-mmm-yy"),
        16 => Some("d-mmm"),
        17 => Some("mmm-yy"),
        18 => Some("h:mm AM/PM"),
        19 => Some("h:mm:ss AM/PM"),
        20 => Some("h:mm"),
        21 => Some("h:mm:ss"),
        22 => Some("m/d/yyyy h:mm"),
        37 => Some("#,##0 ;(#,##0)"),
        38 => Some("#,##0 ;[Red](#,##0)"),
        39 => Some("#,##0.00;(#,##0.00)"),
        40 => Some("#,##0.00;[Red](#,##0.00)"),
        45 => Some("mm:ss"),
        46 => Some("[h]:mm:ss"),
        47 => Some("mmss.0"),
        48 => Some("##0.0E+0"),
        49 => Some("@"),
        _ => None,
    }
}

/// Heuristic: does this number format code represent a date/time?
fn is_date_format(code: &str) -> bool {
    let lower = code.to_ascii_lowercase();
    // Unambiguous date/time tokens
    lower.contains("yy")
        || lower.contains("dd")
        || lower.contains("hh")
        || lower.contains(":ss")
        || lower.contains("am/pm")
        || lower.contains("mmm")
}

impl XlsxStyleTable {
    /// Parse `styles.xml` bytes into a style table.
    pub fn parse(xml: &[u8]) -> Self {
        let xml_str = String::from_utf8_lossy(xml);
        let mut scanner = XmlScanner::new(xml_str.as_bytes());
        scanner.skip_declaration();

        let mut table = XlsxStyleTable::default();

        // Populate built-in numFmt lookups
        for id in 0..=49u32 {
            if let Some(code) = builtin_numfmt_code(id) {
                table.num_fmt_by_id.insert(id, code.to_string());
            }
        }

        // Parse sections in document order. The OOXML schema requires
        // numFmts, fonts, fills, borders, cellStyleXfs, cellXfs, cellStyles,
        // dxfs, tableStyles, extLst — but we only need the first six.
        // We scan for each top-level child by name.

        // <numFmts>
        let save = scanner.pos;
        if scanner.find_open_tag("numFmts").is_some() {
            let nfmts_start = scanner.pos;
            if scanner.is_self_closing(nfmts_start) {
                scanner.skip_open_tag();
            } else {
                scanner.skip_open_tag();
                loop {
                    let s2 = scanner.pos;
                    if scanner.find_open_tag("numFmt").is_none() {
                        scanner.pos = s2;
                        break;
                    }
                    let tag_start = scanner.pos;
                    let tag_name = scanner.read_tag_name(tag_start);
                    let (attrs, _) = scanner.parse_attributes(tag_start + tag_name.len());
                    let id = attrs.get("numFmtId").and_then(|s| s.parse::<u32>().ok());
                    let code = attrs.get("formatCode").cloned().unwrap_or_default();
                    if let Some(id) = id {
                        let info = NumFmtInfo { id, code: code.clone() };
                        table.num_fmts.push(info);
                        table.num_fmt_by_id.insert(id, code);
                    }
                    scanner.skip_open_tag();
                }
            }
        } else {
            scanner.pos = save;
        }

        // <fonts>
        let save = scanner.pos;
        if scanner.find_open_tag("fonts").is_some() {
            let fonts_start = scanner.pos;
            if !scanner.is_self_closing(fonts_start) {
                scanner.skip_open_tag();
                loop {
                    let s2 = scanner.pos;
                    if scanner.find_open_tag("font").is_none() {
                        scanner.pos = s2;
                        break;
                    }
                    let font_start = scanner.pos;
                    if scanner.is_self_closing(font_start) {
                        scanner.skip_open_tag();
                        table.fonts.push(FontInfo::default());
                        continue;
                    }
                    scanner.skip_open_tag();
                    table.fonts.push(parse_font(&mut scanner));
                }
            } else {
                scanner.skip_open_tag();
            }
        } else {
            scanner.pos = save;
        }

        // <fills>
        let save = scanner.pos;
        if scanner.find_open_tag("fills").is_some() {
            let fills_start = scanner.pos;
            if !scanner.is_self_closing(fills_start) {
                scanner.skip_open_tag();
                loop {
                    let s2 = scanner.pos;
                    if scanner.find_open_tag("fill").is_none() {
                        scanner.pos = s2;
                        break;
                    }
                    let fill_start = scanner.pos;
                    if scanner.is_self_closing(fill_start) {
                        scanner.skip_open_tag();
                        table.fills.push(FillInfo::default());
                        continue;
                    }
                    scanner.skip_open_tag();
                    table.fills.push(parse_fill(&mut scanner));
                }
            } else {
                scanner.skip_open_tag();
            }
        } else {
            scanner.pos = save;
        }

        // <borders>
        let save = scanner.pos;
        if scanner.find_open_tag("borders").is_some() {
            let borders_start = scanner.pos;
            if !scanner.is_self_closing(borders_start) {
                scanner.skip_open_tag();
                loop {
                    let s2 = scanner.pos;
                    if scanner.find_open_tag("border").is_none() {
                        scanner.pos = s2;
                        break;
                    }
                    let border_start = scanner.pos;
                    if scanner.is_self_closing(border_start) {
                        scanner.skip_open_tag();
                        table.borders.push(BorderInfo::default());
                        continue;
                    }
                    scanner.skip_open_tag();
                    table.borders.push(parse_border(&mut scanner));
                }
            } else {
                scanner.skip_open_tag();
            }
        } else {
            scanner.pos = save;
        }

        // <cellXfs> (skip <cellStyleXfs> which comes before it)
        let save = scanner.pos;
        if scanner.find_open_tag("cellXfs").is_some() {
            let xfs_start = scanner.pos;
            if !scanner.is_self_closing(xfs_start) {
                scanner.skip_open_tag();
                loop {
                    let s2 = scanner.pos;
                    if scanner.find_open_tag("xf").is_none() {
                        scanner.pos = s2;
                        break;
                    }
                    let xf_start = scanner.pos;
                    let tag_name = scanner.read_tag_name(xf_start);
                    let (attrs, _) = scanner.parse_attributes(xf_start + tag_name.len());
                    let num_fmt_id = attrs.get("numFmtId").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                    let font_id = attrs.get("fontId").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                    let fill_id = attrs.get("fillId").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                    let border_id = attrs.get("borderId").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);

                    let is_self_closing = scanner.is_self_closing(xf_start);
                    scanner.skip_open_tag();

                    let alignment = if !is_self_closing {
                        parse_alignment(&mut scanner)
                    } else {
                        AlignmentInfo::default()
                    };

                    table.cell_xfs.push(CellXfInfo {
                        num_fmt_id,
                        font_id,
                        fill_id,
                        border_id,
                        alignment,
                    });
                }
            } else {
                scanner.skip_open_tag();
            }
        } else {
            scanner.pos = save;
        }

        table
    }

    /// Resolve a `cellXfs` index to an `XlsxCellStyle`.
    /// Returns `None` if the index is out of range.
    pub fn resolve_style(&self, xf_index: u32) -> Option<XlsxCellStyle> {
        let xf = self.cell_xfs.get(xf_index as usize)?;

        let font = self.fonts.get(xf.font_id as usize);
        let fill = self.fills.get(xf.fill_id as usize);
        let border = self.borders.get(xf.border_id as usize);
        let num_fmt_code = self.num_fmt_by_id.get(&xf.num_fmt_id).cloned();

        let mut style = XlsxCellStyle::default();

        if let Some(f) = font {
            if f.bold { style.bold = Some(true); }
            if f.italic { style.italic = Some(true); }
            if f.underline { style.underline = Some(true); }
            if !f.name.is_empty() && f.name != "Calibri" {
                style.font_name = Some(f.name.clone());
            }
            if f.size > 0.0 && (f.size - 11.0).abs() > 0.01 {
                style.font_size = Some(f.size);
            }
            if let Some(c) = &f.color {
                style.font_color = Some(strip_alpha(c));
            }
        }

        if let Some(fl) = fill {
            if let Some(c) = &fl.color {
                style.fill_color = Some(strip_alpha(c));
            }
        }

        if let Some(b) = border {
            // Report the first non-empty border side (writer applies same to all)
            for side in [&b.left, &b.right, &b.top, &b.bottom] {
                if !side.style.is_empty() {
                    style.border = Some(side.style.clone());
                    if let Some(c) = &side.color {
                        style.border_color = Some(strip_alpha(c));
                    }
                    break;
                }
            }
        }

        if let Some(code) = &num_fmt_code {
            if code != "General" {
                style.number_format = Some(code.clone());
                if is_date_format(code) {
                    style.date = Some(true);
                }
            }
        }

        if let Some(h) = &xf.alignment.horizontal {
            style.align = Some(h.clone());
        }
        if let Some(v) = &xf.alignment.vertical {
            style.valign = Some(v.clone());
        }
        if xf.alignment.wrap_text {
            style.wrap = Some(true);
        }

        Some(style)
    }

    /// Number of cell formats.
    pub fn cell_xf_count(&self) -> usize {
        self.cell_xfs.len()
    }
}

/// Strip the leading alpha channel from an ARGB hex string (e.g. "FF305496" → "305496").
fn strip_alpha(argb: &str) -> String {
    if argb.len() == 8 {
        argb[2..].to_string()
    } else {
        argb.to_string()
    }
}

fn parse_font(scanner: &mut XmlScanner) -> FontInfo {
    let mut info = FontInfo {
        name: String::new(),
        size: 0.0,
        bold: false,
        italic: false,
        underline: false,
        color: None,
    };

    let end_pos = scanner.find_close_tag("font", scanner.pos);

    loop {
        let save = scanner.pos;
        if let Some(end) = end_pos {
            if scanner.pos >= end {
                break;
            }
        }
        match scanner.find_any_open_tag() {
            None => {
                scanner.pos = save;
                break;
            }
            Some((tag_name, tag_start)) => {
                if let Some(end) = end_pos {
                    if tag_start >= end {
                        scanner.pos = save;
                        break;
                    }
                }
                let (attrs, _) = scanner.parse_attributes(tag_start + tag_name.len());
                match tag_name.as_str() {
                    "sz" => {
                        if let Some(v) = attrs.get("val") {
                            info.size = v.parse::<f64>().unwrap_or(0.0);
                        }
                        scanner.skip_open_tag();
                    }
                    "name" => {
                        if let Some(v) = attrs.get("val") {
                            info.name = xml_unescape(v);
                        }
                        scanner.skip_open_tag();
                    }
                    "color" => {
                        if let Some(rgb) = attrs.get("rgb") {
                            info.color = Some(rgb.clone());
                        }
                        scanner.skip_open_tag();
                    }
                    "b" => { info.bold = true; scanner.skip_open_tag(); }
                    "i" => { info.italic = true; scanner.skip_open_tag(); }
                    "u" => { info.underline = true; scanner.skip_open_tag(); }
                    _ => { scanner.skip_open_tag(); }
                }
            }
        }
    }

    info
}

fn parse_fill(scanner: &mut XmlScanner) -> FillInfo {
    let mut info = FillInfo::default();
    let end_pos = scanner.find_close_tag("fill", scanner.pos);

    loop {
        let save = scanner.pos;
        if let Some(end) = end_pos {
            if scanner.pos >= end {
                break;
            }
        }
        match scanner.find_any_open_tag() {
            None => {
                scanner.pos = save;
                break;
            }
            Some((tag_name, tag_start)) => {
                if let Some(end) = end_pos {
                    if tag_start >= end {
                        scanner.pos = save;
                        break;
                    }
                }
                let (attrs, _) = scanner.parse_attributes(tag_start + tag_name.len());
                match tag_name.as_str() {
                    "patternFill" => {
                        let is_self_closing = scanner.is_self_closing(tag_start);
                        scanner.skip_open_tag();
                        if !is_self_closing {
                            let pf_end = scanner.find_close_tag("patternFill", scanner.pos);
                            loop {
                                let save2 = scanner.pos;
                                if let Some(end) = pf_end {
                                    if scanner.pos >= end {
                                        break;
                                    }
                                }
                                match scanner.find_any_open_tag() {
                                    None => { scanner.pos = save2; break; }
                                    Some((sub_name, sub_start)) => {
                                        if let Some(end) = pf_end {
                                            if sub_start >= end {
                                                scanner.pos = save2;
                                                break;
                                            }
                                        }
                                        let (sub_attrs, _) = scanner.parse_attributes(sub_start + sub_name.len());
                                        if sub_name == "fgColor" {
                                            if let Some(rgb) = sub_attrs.get("rgb") {
                                                info.color = Some(rgb.clone());
                                            }
                                        }
                                        scanner.skip_open_tag();
                                    }
                                }
                            }
                        }
                    }
                    _ => { scanner.skip_open_tag(); }
                }
            }
        }
    }

    info
}

fn parse_border(scanner: &mut XmlScanner) -> BorderInfo {
    let mut info = BorderInfo::default();
    let end_pos = scanner.find_close_tag("border", scanner.pos);

    loop {
        let save = scanner.pos;
        if let Some(end) = end_pos {
            if scanner.pos >= end {
                break;
            }
        }
        match scanner.find_any_open_tag() {
            None => {
                scanner.pos = save;
                break;
            }
            Some((tag_name, tag_start)) => {
                if let Some(end) = end_pos {
                    if tag_start >= end {
                        scanner.pos = save;
                        break;
                    }
                }
                let (attrs, _) = scanner.parse_attributes(tag_start + tag_name.len());
                let is_self_closing = scanner.is_self_closing(tag_start);
                scanner.skip_open_tag();

                let side: Option<&mut BorderSideInfo> = match tag_name.as_str() {
                    "left" => Some(&mut info.left),
                    "right" => Some(&mut info.right),
                    "top" => Some(&mut info.top),
                    "bottom" => Some(&mut info.bottom),
                    _ => None,
                };

                if let Some(side) = side {
                    if let Some(style) = attrs.get("style") {
                        side.style = style.clone();
                    }
                    if let Some(c) = attrs.get("color") {
                        side.color = Some(c.clone());
                    }
                    if !is_self_closing {
                        let side_end = scanner.find_close_tag(&tag_name, scanner.pos);
                        loop {
                            let save2 = scanner.pos;
                            if let Some(end) = side_end {
                                if scanner.pos >= end {
                                    break;
                                }
                            }
                            match scanner.find_any_open_tag() {
                                None => { scanner.pos = save2; break; }
                                Some((sub_name, sub_start)) => {
                                    if let Some(end) = side_end {
                                        if sub_start >= end {
                                            scanner.pos = save2;
                                            break;
                                        }
                                    }
                                    let (sub_attrs, _) = scanner.parse_attributes(sub_start + sub_name.len());
                                    if sub_name == "color" {
                                        if let Some(rgb) = sub_attrs.get("rgb") {
                                            side.color = Some(rgb.clone());
                                        }
                                    }
                                    scanner.skip_open_tag();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    info
}

fn parse_alignment(scanner: &mut XmlScanner) -> AlignmentInfo {
    let mut info = AlignmentInfo::default();
    let end_pos = scanner.find_close_tag("xf", scanner.pos);

    loop {
        let save = scanner.pos;
        if let Some(end) = end_pos {
            if scanner.pos >= end {
                break;
            }
        }
        match scanner.find_any_open_tag() {
            None => {
                scanner.pos = save;
                break;
            }
            Some((tag_name, tag_start)) => {
                if let Some(end) = end_pos {
                    if tag_start >= end {
                        scanner.pos = save;
                        break;
                    }
                }
                if tag_name == "alignment" {
                    let (attrs, _) = scanner.parse_attributes(tag_start + tag_name.len());
                    if let Some(h) = attrs.get("horizontal") {
                        info.horizontal = Some(h.clone());
                    }
                    if let Some(v) = attrs.get("vertical") {
                        info.vertical = Some(v.clone());
                    }
                    if attrs.get("wrapText").map(|s| s == "1" || s.eq_ignore_ascii_case("true")).unwrap_or(false) {
                        info.wrap_text = true;
                    }
                    scanner.skip_open_tag();
                    break;
                }
                scanner.skip_open_tag();
            }
        }
    }

    info
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_styles_xml() -> Vec<u8> {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <numFmts count="1">
    <numFmt numFmtId="164" formatCode="$#,##0.00"/>
  </numFmts>
  <fonts count="3">
    <font>
      <sz val="11"/>
      <name val="Calibri"/>
      <color theme="1"/>
    </font>
    <font>
      <sz val="14"/>
      <name val="Arial"/>
      <b/>
      <color rgb="FF305496"/>
    </font>
    <font>
      <sz val="11"/>
      <name val="Calibri"/>
      <i/>
      <u val="single"/>
      <color rgb="FF595959"/>
    </font>
  </fonts>
  <fills count="3">
    <fill><patternFill/></fill>
    <fill><patternFill patternType="gray125"/></fill>
    <fill><patternFill patternType="solid"><fgColor rgb="FF305496"/><bgColor rgb="FF305496"/></patternFill></fill>
  </fills>
  <borders count="2">
    <border>
      <left/><right/><top/><bottom/><diagonal/>
    </border>
    <border>
      <left style="thin"><color rgb="FF000000"/></left>
      <right style="thin"><color rgb="FF000000"/></right>
      <top style="thin"><color rgb="FF000000"/></top>
      <bottom style="thin"><color rgb="FF000000"/></bottom>
      <diagonal/>
    </border>
  </borders>
  <cellStyleXfs count="1">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0"/>
  </cellStyleXfs>
  <cellXfs count="4">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
    <xf numFmtId="164" fontId="1" fillId="2" borderId="0" xfId="0" applyNumberFormat="1" applyFont="1" applyFill="1">
      <alignment horizontal="center" vertical="center"/>
    </xf>
    <xf numFmtId="14" fontId="2" fillId="0" borderId="1" xfId="0" applyNumberFormat="1" applyFont="1" applyBorder="1">
      <alignment horizontal="right" wrapText="1"/>
    </xf>
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
  </cellXfs>
  <cellStyles count="1">
    <cellStyle name="Normal" xfId="0" builtinId="0"/>
  </cellStyles>
</styleSheet>"#;
        xml.as_bytes().to_vec()
    }

    #[test]
    fn parse_num_fmts() {
        let table = XlsxStyleTable::parse(&make_styles_xml());
        assert_eq!(table.num_fmts.len(), 1);
        assert_eq!(table.num_fmts[0].id, 164);
        assert_eq!(table.num_fmts[0].code, "$#,##0.00");
    }

    #[test]
    fn parse_fonts() {
        let table = XlsxStyleTable::parse(&make_styles_xml());
        assert_eq!(table.fonts.len(), 3);
        // Default font
        assert_eq!(table.fonts[0].name, "Calibri");
        assert_eq!(table.fonts[0].size, 11.0);
        assert!(!table.fonts[0].bold);
        // Bold font
        assert_eq!(table.fonts[1].name, "Arial");
        assert_eq!(table.fonts[1].size, 14.0);
        assert!(table.fonts[1].bold);
        assert_eq!(table.fonts[1].color.as_deref(), Some("FF305496"));
        // Italic + underline
        assert!(table.fonts[2].italic);
        assert!(table.fonts[2].underline);
        assert_eq!(table.fonts[2].color.as_deref(), Some("FF595959"));
    }

    #[test]
    fn parse_fills() {
        let table = XlsxStyleTable::parse(&make_styles_xml());
        assert_eq!(table.fills.len(), 3);
        assert!(table.fills[0].color.is_none());
        assert!(table.fills[1].color.is_none());
        assert_eq!(table.fills[2].color.as_deref(), Some("FF305496"));
    }

    #[test]
    fn parse_borders() {
        let table = XlsxStyleTable::parse(&make_styles_xml());
        assert_eq!(table.borders.len(), 2);
        assert!(table.borders[0].left.style.is_empty());
        assert_eq!(table.borders[1].left.style, "thin");
        assert_eq!(table.borders[1].left.color.as_deref(), Some("FF000000"));
    }

    #[test]
    fn parse_cell_xfs() {
        let table = XlsxStyleTable::parse(&make_styles_xml());
        assert_eq!(table.cell_xfs.len(), 4);
        // xf[0] — default
        assert_eq!(table.cell_xfs[0].font_id, 0);
        // xf[1] — bold + fill + custom numFmt + center alignment
        assert_eq!(table.cell_xfs[1].num_fmt_id, 164);
        assert_eq!(table.cell_xfs[1].font_id, 1);
        assert_eq!(table.cell_xfs[1].fill_id, 2);
        assert_eq!(table.cell_xfs[1].alignment.horizontal.as_deref(), Some("center"));
        assert_eq!(table.cell_xfs[1].alignment.vertical.as_deref(), Some("center"));
        // xf[2] — date format + border + wrap
        assert_eq!(table.cell_xfs[2].num_fmt_id, 14);
        assert_eq!(table.cell_xfs[2].border_id, 1);
        assert!(table.cell_xfs[2].alignment.wrap_text);
    }

    #[test]
    fn resolve_default_style() {
        let table = XlsxStyleTable::parse(&make_styles_xml());
        let style = table.resolve_style(0).unwrap();
        assert!(style.is_empty());
    }

    #[test]
    fn resolve_bold_fill_style() {
        let table = XlsxStyleTable::parse(&make_styles_xml());
        let style = table.resolve_style(1).unwrap();
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.font_name.as_deref(), Some("Arial"));
        assert_eq!(style.font_size, Some(14.0));
        assert_eq!(style.font_color.as_deref(), Some("305496"));
        assert_eq!(style.fill_color.as_deref(), Some("305496"));
        assert_eq!(style.number_format.as_deref(), Some("$#,##0.00"));
        assert_eq!(style.align.as_deref(), Some("center"));
        assert_eq!(style.valign.as_deref(), Some("center"));
    }

    #[test]
    fn resolve_date_border_style() {
        let table = XlsxStyleTable::parse(&make_styles_xml());
        let style = table.resolve_style(2).unwrap();
        assert_eq!(style.italic, Some(true));
        assert_eq!(style.underline, Some(true));
        assert_eq!(style.font_color.as_deref(), Some("595959"));
        assert_eq!(style.border.as_deref(), Some("thin"));
        assert_eq!(style.border_color.as_deref(), Some("000000"));
        assert_eq!(style.number_format.as_deref(), Some("m/d/yyyy"));
        assert_eq!(style.date, Some(true));
        assert_eq!(style.align.as_deref(), Some("right"));
        assert_eq!(style.wrap, Some(true));
    }

    #[test]
    fn resolve_out_of_range_returns_none() {
        let table = XlsxStyleTable::parse(&make_styles_xml());
        assert!(table.resolve_style(999).is_none());
    }

    #[test]
    fn empty_styles_xml() {
        let xml = br#"<?xml version="1.0"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <numFmts count="0"/>
  <fonts count="0"/>
  <fills count="0"/>
  <borders count="0"/>
  <cellStyleXfs count="0"/>
  <cellXfs count="0"/>
</styleSheet>"#;
        let table = XlsxStyleTable::parse(xml);
        assert_eq!(table.cell_xf_count(), 0);
        assert!(table.resolve_style(0).is_none());
    }

    #[test]
    fn is_date_format_detection() {
        assert!(is_date_format("m/d/yyyy"));
        assert!(is_date_format("yyyy-mm-dd"));
        assert!(is_date_format("h:mm:ss"));
        assert!(is_date_format("mm/dd/yyyy hh:mm"));
        assert!(!is_date_format("$#,##0.00"));
        assert!(!is_date_format("0%"));
        assert!(!is_date_format("General"));
    }
}
