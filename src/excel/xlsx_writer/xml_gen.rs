//! XML generation for XLSX files
//!
//! Generates proper Office Open XML (OOXML) that is compatible with
//! Microsoft Excel, Apple Numbers, and LibreOffice Calc.

use anyhow::Result;
use std::io::{Seek, Write as IoWrite};
use std::fmt::Write as FmtWrite;
use zip::ZipWriter;
use zip::write::FileOptions;

use super::style_registry::StyleRegistry;
use super::types::{CellData, SheetData};
use super::WriteOptions;

/// Escape special XML characters
pub fn escape_xml(s: &str) -> String {
    let mut out = String::new();
    escape_xml_into(s, &mut out);
    out
}

/// Escape special XML characters directly into a buffer, avoiding an
/// intermediate allocation. Fast path: strings with no special characters
/// are copied verbatim.
fn escape_xml_into(s: &str, out: &mut String) {
    let needs_escape = s.bytes().any(|b| matches!(b, b'&' | b'<' | b'>' | b'"' | b'\''));
    if !needs_escape {
        out.push_str(s);
        return;
    }
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
}

/// Convert column number to Excel column letter (1=A, 26=Z, 27=AA, etc.)
pub fn col_num_to_letter(col: usize) -> String {
    if col == 0 {
        return "A".to_string();
    }
    let mut col = col;
    let mut result = String::new();
    while col > 0 {
        col -= 1;
        result.insert(0, ((b'A') + (col % 26) as u8) as char);
        col /= 26;
    }
    result
}

/// Convert column number to Excel column letters into a reusable buffer,
/// avoiding per-cell allocation. Returns a borrow of `buf`.
fn col_num_to_letter_into(col: usize, buf: &mut String) {
    buf.clear();
    if col == 0 {
        buf.push('A');
        return;
    }
    let mut col = col;
    let mut tmp = [0u8; 3];
    let mut len = 0;
    while col > 0 {
        col -= 1;
        tmp[len] = b'A' + (col % 26) as u8;
        len += 1;
        col /= 26;
    }
    for i in (0..len).rev() {
        buf.push(tmp[i] as char);
    }
}

/// Add [Content_Types].xml
pub fn add_content_types<W: IoWrite + Seek>(
    zip: &mut ZipWriter<W>,
    sheet_count: usize,
) -> Result<()> {
    let no_flags = vec![false; sheet_count];
    add_content_types_ext(zip, sheet_count, &no_flags, &no_flags, false)
}

/// Add [Content_Types].xml with optional chart/drawing/comment/VBA content types
pub fn add_content_types_ext<W: IoWrite + Seek>(
    zip: &mut ZipWriter<W>,
    sheet_count: usize,
    chart_flags: &[bool],
    comment_flags: &[bool],
    has_vba: bool,
) -> Result<()> {
    let mut xml = String::with_capacity(1024);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#);
    xml.push_str(r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#);
    xml.push_str(r#"<Default Extension="xml" ContentType="application/xml"/>"#);
    if has_vba {
        xml.push_str(r#"<Default Extension="bin" ContentType="application/vnd.ms-office.vbaProject"/>"#);
    }
    let workbook_ct = if has_vba {
        "application/vnd.ms-excel.sheet.macroEnabled.main+xml"
    } else {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"
    };
    xml.push_str(&format!(r#"<Override PartName="/xl/workbook.xml" ContentType="{}"/>"#, workbook_ct));
    for idx in 0..sheet_count {
        xml.push_str(&format!(
            r#"<Override PartName="/xl/worksheets/sheet{}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#,
            idx + 1
        ));
    }
    xml.push_str(r#"<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>"#);
    xml.push_str(r#"<Override PartName="/xl/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>"#);

    // Chart and drawing content types
    add_chart_content_types(&mut xml, sheet_count, chart_flags);
    // Comments content types
    add_comment_content_types(&mut xml, comment_flags);

    xml.push_str(r#"</Types>"#);

    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("[Content_Types].xml", opts)?;
    zip.write_all(xml.as_bytes())?;
    Ok(())
}

/// Add _rels/.rels
pub fn add_rels<W: IoWrite + Seek>(zip: &mut ZipWriter<W>) -> Result<()> {
    let xml = concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
        r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>"#,
        r#"</Relationships>"#,
    );
    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("_rels/.rels", opts)?;
    zip.write_all(xml.as_bytes())?;
    Ok(())
}

/// Add xl/workbook.xml
pub fn add_workbook<W: IoWrite + Seek>(
    zip: &mut ZipWriter<W>,
    sheets: &[SheetData],
) -> Result<()> {
    let mut xml = String::with_capacity(512);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#);
    xml.push_str(r#"<workbookPr/>"#);
    xml.push_str(r#"<bookViews><workbookView activeTab="0"/></bookViews>"#);
    xml.push_str(r#"<sheets>"#);
    for (idx, sheet) in sheets.iter().enumerate() {
        xml.push_str(&format!(
            r#"<sheet name="{}" sheetId="{}" r:id="rId{}"/>"#,
            escape_xml(&sheet.name),
            idx + 1,
            idx + 1
        ));
    }
    xml.push_str(r#"</sheets>"#);

    // Print areas as defined names
    let print_areas: Vec<(usize, &str)> = sheets
        .iter()
        .enumerate()
        .filter_map(|(idx, s)| s.print_setup.as_ref()?.print_area.as_ref().map(|a| (idx, a.as_str())))
        .collect();
    if !print_areas.is_empty() {
        xml.push_str(r#"<definedNames>"#);
        for (idx, area) in print_areas {
            let sheet_name = escape_xml(&sheets[idx].name);
            xml.push_str(&format!(
                r#"<definedName name="_xlnm.Print_Area" localSheetId="{}">'{}'!{}</definedName>"#,
                idx, sheet_name, area
            ));
        }
        xml.push_str(r#"</definedNames>"#);
    }

    xml.push_str(r#"<calcPr calcId="124519" fullCalcOnLoad="1"/>"#);
    xml.push_str(r#"</workbook>"#);

    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("xl/workbook.xml", opts)?;
    zip.write_all(xml.as_bytes())?;
    Ok(())
}

/// Add xl/_rels/workbook.xml.rels
pub fn add_workbook_rels<W: IoWrite + Seek>(
    zip: &mut ZipWriter<W>,
    sheet_count: usize,
) -> Result<()> {
    let mut xml = String::with_capacity(512);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#);
    for idx in 0..sheet_count {
        xml.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{}.xml"/>"#,
            idx + 1, idx + 1
        ));
    }
    xml.push_str(&format!(
        r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>"#,
        sheet_count + 1
    ));
    xml.push_str(&format!(
        r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>"#,
        sheet_count + 2
    ));
    xml.push_str(r#"</Relationships>"#);

    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("xl/_rels/workbook.xml.rels", opts)?;
    zip.write_all(xml.as_bytes())?;
    Ok(())
}

/// Add xl/styles.xml
///
/// When the registry only contains the default cellXf (no user
/// styles registered), this emits the same minimal styles.xml that
/// earlier versions of the writer emitted. When the registry holds
/// additional fonts/fills/borders/numFmts/cellXfs, those tables are
/// emitted instead. Either way, output is well-formed OOXML that
/// Excel, Numbers, and LibreOffice accept.
pub fn add_styles<W: IoWrite + Seek>(zip: &mut ZipWriter<W>) -> Result<()> {
    let registry = StyleRegistry::new();
    add_styles_with_registry(zip, &registry)
}

/// Same as `add_styles`, but emits whatever the given registry holds.
/// This is the path used by `XlsxWriter::save`.
pub fn add_styles_with_registry<W: IoWrite + Seek>(
    zip: &mut ZipWriter<W>,
    registry: &StyleRegistry,
) -> Result<()> {
    let xml = styles_xml(registry);
    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("xl/styles.xml", opts)?;
    zip.write_all(xml.as_bytes())?;
    Ok(())
}

fn fonts_xml(reg: &StyleRegistry) -> String {
    let mut xml = String::with_capacity(reg.fonts.len() * 80);
    let _ = write!(xml, "<fonts count=\"{}\">", reg.fonts.len());
    for f in &reg.fonts {
        xml.push_str("<font>");
        let _ = write!(xml, "<sz val=\"{}\"/>", f.size_half as f64 / 100.0);
        let _ = write!(xml, "<name val=\"{}\"/>", escape_xml(&f.name));
        if f.bold { xml.push_str("<b/>"); }
        if f.italic { xml.push_str("<i/>"); }
        if f.underline { xml.push_str("<u val=\"single\"/>"); }
        if let Some(c) = &f.color_argb {
            let _ = write!(xml, "<color rgb=\"{}\"/>", c);
        } else {
            xml.push_str("<color theme=\"1\"/>");
        }
        xml.push_str("</font>");
    }
    xml.push_str("</fonts>");
    xml
}

fn fills_xml(reg: &StyleRegistry) -> String {
    let mut xml = String::with_capacity(reg.fills.len() * 80);
    let _ = write!(xml, "<fills count=\"{}\">", reg.fills.len());
    for f in &reg.fills {
        if let Some(c) = &f.color_argb {
            let _ = write!(
                xml,
                "<fill><patternFill patternType=\"solid\"><fgColor rgb=\"{}\"/><bgColor rgb=\"{}\"/></patternFill></fill>",
                c, c
            );
        } else {
            xml.push_str("<fill><patternFill/></fill>");
        }
    }
    xml.push_str("</fills>");
    xml
}

fn borders_xml(reg: &StyleRegistry) -> String {
    let mut xml = String::with_capacity(reg.borders.len() * 100);
    let _ = write!(xml, "<borders count=\"{}\">", reg.borders.len());
    for b in &reg.borders {
        xml.push_str("<border>");
        xml.push_str(&border_side_xml("left", &b.left));
        xml.push_str(&border_side_xml("right", &b.right));
        xml.push_str(&border_side_xml("top", &b.top));
        xml.push_str(&border_side_xml("bottom", &b.bottom));
        xml.push_str("<diagonal/>");
        xml.push_str("</border>");
    }
    xml.push_str("</borders>");
    xml
}

fn cell_xfs_xml(reg: &StyleRegistry) -> String {
    let mut xml = String::with_capacity(reg.cell_xfs.len() * 120);
    let _ = write!(xml, "<cellXfs count=\"{}\">", reg.cell_xfs.len());
    for xf in &reg.cell_xfs {
        let _ = write!(
            xml,
            "<xf numFmtId=\"{}\" fontId=\"{}\" fillId=\"{}\" borderId=\"{}\" xfId=\"0\"",
            xf.num_fmt_id, xf.font_id, xf.fill_id, xf.border_id
        );
        if xf.apply_number_format { xml.push_str(" applyNumberFormat=\"1\""); }
        if xf.apply_font { xml.push_str(" applyFont=\"1\""); }
        if xf.apply_fill { xml.push_str(" applyFill=\"1\""); }
        if xf.apply_border { xml.push_str(" applyBorder=\"1\""); }
        if xf.apply_alignment { xml.push_str(" applyAlignment=\"1\""); }
        let has_align = xf.alignment.horizontal.is_some()
            || xf.alignment.vertical.is_some()
            || xf.alignment.wrap_text;
        if has_align {
            xml.push('>');
            xml.push_str("<alignment");
            if let Some(h) = &xf.alignment.horizontal {
                let _ = write!(xml, " horizontal=\"{}\"", escape_xml(h));
            }
            if let Some(v) = &xf.alignment.vertical {
                let _ = write!(xml, " vertical=\"{}\"", escape_xml(v));
            }
            if xf.alignment.wrap_text {
                xml.push_str(" wrapText=\"1\"");
            }
            xml.push_str("/></xf>");
        } else {
            xml.push_str("/>");
        }
    }
    xml.push_str("</cellXfs>");
    xml
}

/// Build the styles.xml XML string for a registry.
fn styles_xml(reg: &StyleRegistry) -> String {
    let mut xml = String::with_capacity(2048);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str("<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">");

    // numFmts (custom only; built-ins are referenced by reserved id).
    if !reg.num_fmts.is_empty() {
        let _ = write!(xml, "<numFmts count=\"{}\">", reg.num_fmts.len());
        for nf in &reg.num_fmts {
            let _ = write!(
                xml,
                "<numFmt numFmtId=\"{}\" formatCode=\"{}\"/>",
                nf.id, escape_xml(&nf.code)
            );
        }
        xml.push_str("</numFmts>");
    } else {
        xml.push_str("<numFmts count=\"0\"/>");
    }

    xml.push_str(&fonts_xml(reg));
    xml.push_str(&fills_xml(reg));
    xml.push_str(&borders_xml(reg));

    xml.push_str("<cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs>");
    xml.push_str(&cell_xfs_xml(reg));
    xml.push_str("<cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles>");

    xml.push_str("</styleSheet>");
    xml
}

fn border_side_xml(name: &str, side: &super::style_registry::BorderSide) -> String {
    if side.style.is_empty() {
        return format!("<{}/>", name);
    }
    let color = side
        .color_argb
        .as_deref()
        .unwrap_or("FF000000");
    format!("<{} style=\"{}\" color=\"{}\"/>", name, escape_xml(&side.style), color)
}

/// Add xl/theme/theme1.xml
pub fn add_theme<W: IoWrite + Seek>(zip: &mut ZipWriter<W>) -> Result<()> {
    // Minimal but complete Office theme that Excel/Numbers accept
    let xml = concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">"#,
        r#"<a:themeElements>"#,
        r#"<a:clrScheme name="Office">"#,
        r#"<a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>"#,
        r#"<a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>"#,
        r#"<a:dk2><a:srgbClr val="1F497D"/></a:dk2>"#,
        r#"<a:lt2><a:srgbClr val="EEECE1"/></a:lt2>"#,
        r#"<a:accent1><a:srgbClr val="4F81BD"/></a:accent1>"#,
        r#"<a:accent2><a:srgbClr val="C0504D"/></a:accent2>"#,
        r#"<a:accent3><a:srgbClr val="9BBB59"/></a:accent3>"#,
        r#"<a:accent4><a:srgbClr val="8064A2"/></a:accent4>"#,
        r#"<a:accent5><a:srgbClr val="4BACC6"/></a:accent5>"#,
        r#"<a:accent6><a:srgbClr val="F79646"/></a:accent6>"#,
        r#"<a:hlink><a:srgbClr val="0000FF"/></a:hlink>"#,
        r#"<a:folHlink><a:srgbClr val="800080"/></a:folHlink>"#,
        r#"</a:clrScheme>"#,
        r#"<a:fontScheme name="Office">"#,
        r#"<a:majorFont><a:latin typeface="Cambria"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont>"#,
        r#"<a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont>"#,
        r#"</a:fontScheme>"#,
        r#"<a:fmtScheme name="Office">"#,
        r#"<a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst>"#,
        r#"<a:lnStyleLst><a:ln w="9525"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="25400"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="38100"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst>"#,
        r#"<a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst>"#,
        r#"<a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst>"#,
        r#"</a:fmtScheme>"#,
        r#"</a:themeElements>"#,
        r#"</a:theme>"#,
    );

    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("xl/theme/theme1.xml", opts)?;
    zip.write_all(xml.as_bytes())?;
    Ok(())
}

/// Build outline level and collapsed lookup vectors from a list of groups.
/// Each group covers a range `[start, end]` (inclusive) and contributes
/// its `level` (max wins) and `collapsed` flag (OR'd).
fn build_outline_lookup(
    count: usize,
    groups: &[(usize, usize, u8, bool)],
) -> (Vec<u8>, Vec<bool>) {
    let mut levels = vec![0u8; count];
    let mut collapsed = vec![false; count];
    for &(start, end, level, coll) in groups {
        for i in start..=end.min(count.saturating_sub(1)) {
            levels[i] = levels[i].max(level);
            collapsed[i] = collapsed[i] || coll;
        }
    }
    (levels, collapsed)
}

fn sheet_views_xml(options: &WriteOptions) -> String {
    if options.freeze_header {
        let mut s = String::with_capacity(128);
        s.push_str("<sheetViews>");
        s.push_str("<sheetView workbookViewId=\"0\">");
        s.push_str("<pane ySplit=\"1\" topLeftCell=\"A2\" activePane=\"bottomLeft\" state=\"frozen\"/>");
        s.push_str("<selection pane=\"bottomLeft\" activeCell=\"A2\" sqref=\"A2\"/>");
        s.push_str("</sheetView>");
        s.push_str("</sheetViews>");
        s
    } else {
        let mut s = String::with_capacity(64);
        s.push_str("<sheetViews>");
        s.push_str("<sheetView workbookViewId=\"0\">");
        s.push_str("<selection activeCell=\"A1\" sqref=\"A1\"/>");
        s.push_str("</sheetView>");
        s.push_str("</sheetViews>");
        s
    }
}

fn cols_xml(
    sheet: &SheetData,
    col_outline: &[u8],
    col_collapsed: &[bool],
) -> String {
    if sheet.column_widths.is_empty() {
        return String::new();
    }
    let mut xml = String::with_capacity(sheet.column_widths.len() * 80);
    xml.push_str(r#"<cols>"#);
    for (col_idx, &width) in sheet.column_widths.iter().enumerate() {
        let outline = if col_idx < col_outline.len() && col_outline[col_idx] > 0 {
            format!(r#" outlineLevel="{}""#, col_outline[col_idx])
        } else {
            String::new()
        };
        let collapsed = if col_idx < col_collapsed.len() && col_collapsed[col_idx] {
            r#" collapsed="1""#
        } else {
            ""
        };
        let _ = write!(
            xml,
            r#"<col min="{}" max="{}"{}{} width="{}" customWidth="1"/>"#,
            col_idx + 1, col_idx + 1, outline, collapsed, width
        );
    }
    xml.push_str(r#"</cols>"#);
    xml
}

fn sheet_data_xml(
    sheet: &SheetData,
    row_outline: &[u8],
    row_collapsed: &[bool],
) -> String {
    let mut xml = String::with_capacity(sheet.rows.len() * 64);
    xml.push_str(r#"<sheetData>"#);
    let mut col_buf = String::with_capacity(3);
    for (row_idx, row) in sheet.rows.iter().enumerate() {
        let outline_attr = if row_outline[row_idx] > 0 {
            format!(r#" outlineLevel="{}""#, row_outline[row_idx])
        } else {
            String::new()
        };
        let collapsed_attr = if row_collapsed[row_idx] {
            r#" collapsed="1" hidden="1""#
        } else {
            ""
        };
        let _ = write!(xml, r#"<row r="{}{}{}">"#, row_idx + 1, outline_attr, collapsed_attr);
        for (col_idx, cell) in row.cells.iter().enumerate() {
            if matches!(cell, CellData::Empty) {
                continue;
            }
            let style_attr = row
                .cell_styles
                .get(col_idx)
                .and_then(|s| *s)
                .filter(|&i| i != 0)
                .map(|i| format!(r#" s="{}""#, i))
                .unwrap_or_default();
            col_num_to_letter_into(col_idx + 1, &mut col_buf);
            let cell_ref = format!("{}{}", col_buf, row_idx + 1);
            match cell {
                CellData::String(s) => {
                    let _ = write!(
                        xml,
                        r#"<c r="{}"{} t="inlineStr"><is><t>"#,
                        cell_ref, style_attr
                    );
                    escape_xml_into(s, &mut xml);
                    xml.push_str(r#"</t></is></c>"#);
                }
                CellData::Number(n) => {
                    let _ = write!(xml, r#"<c r="{}"{}><v>{}</v></c>"#, cell_ref, style_attr, n);
                }
                CellData::Bool(b) => {
                    let _ = write!(
                        xml,
                        r#"<c r="{}"{} t="b"><v>{}</v></c>"#,
                        cell_ref, style_attr, if *b { 1 } else { 0 }
                    );
                }
                CellData::Formula(f) => {
                    let formula = f.strip_prefix('=').unwrap_or(f);
                    let _ = write!(xml, r#"<c r="{}"{}><f>"#, cell_ref, style_attr);
                    escape_xml_into(formula, &mut xml);
                    xml.push_str(r#"</f></c>"#);
                }
                CellData::Empty => unreachable!(),
            }
        }
        xml.push_str(r#"</row>"#);
    }
    xml.push_str(r#"</sheetData>"#);
    xml
}

fn merge_cells_xml(sheet: &SheetData) -> String {
    if sheet.merge_cells.is_empty() {
        return String::new();
    }
    let mut xml = String::with_capacity(sheet.merge_cells.len() * 60);
    let _ = write!(xml, r#"<mergeCells count="{}">"#, sheet.merge_cells.len());
    for mc in &sheet.merge_cells {
        let start_ref = format!("{}{}", col_num_to_letter(mc.start_col + 1), mc.start_row + 1);
        let end_ref = format!("{}{}", col_num_to_letter(mc.end_col + 1), mc.end_row + 1);
        let _ = write!(xml, r#"<mergeCell ref="{}:{}"/>"#, start_ref, end_ref);
    }
    xml.push_str(r#"</mergeCells>"#);
    xml
}

fn data_validations_xml(sheet: &SheetData) -> String {
    if sheet.data_validations.is_empty() {
        return String::new();
    }
    let mut xml = String::with_capacity(sheet.data_validations.len() * 100);
    let _ = write!(xml, r#"<dataValidations count="{}">"#, sheet.data_validations.len());
    for dv in &sheet.data_validations {
        xml.push_str(&generate_data_validation_xml(dv));
    }
    xml.push_str(r#"</dataValidations>"#);
    xml
}

fn hyperlinks_xml(sheet: &SheetData, has_chart: bool) -> String {
    if sheet.hyperlinks.is_empty() {
        return String::new();
    }
    let mut xml = String::with_capacity(sheet.hyperlinks.len() * 80);
    xml.push_str(r#"<hyperlinks>"#);
    let mut rel_id = if has_chart { 2 } else { 1 };
    for hl in &sheet.hyperlinks {
        let tooltip_attr = hl.tooltip.as_ref().map(|t| format!(r#" tooltip="{}""#, escape_xml(t))).unwrap_or_default();
        let _ = write!(xml, r#"<hyperlink ref="{}" r:id="rId{}"{}/>"#, hl.cell_ref, rel_id, tooltip_attr);
        rel_id += 1;
    }
    xml.push_str(r#"</hyperlinks>"#);
    xml
}

/// Add worksheet XML
pub fn add_worksheet<W: IoWrite + Seek>(
    zip: &mut ZipWriter<W>,
    idx: usize,
    sheet: &SheetData,
    options: &WriteOptions,
    has_chart: bool,
) -> Result<()> {
    let max_row = sheet.rows.len();
    let max_col = sheet.rows.iter().map(|r| r.cells.len()).max().unwrap_or(0);
    let needs_r_namespace = has_chart || !sheet.hyperlinks.is_empty() || !sheet.comments.is_empty();

    // Build outline lookups
    let col_groups: Vec<(usize, usize, u8, bool)> = sheet.col_groups.iter()
        .map(|g| (g.start_col, g.end_col, g.level, g.collapsed))
        .collect();
    let max_col_for_outline = sheet.column_widths.len().max(max_col);
    let (col_outline, col_collapsed) = build_outline_lookup(max_col_for_outline, &col_groups);

    let row_groups: Vec<(usize, usize, u8, bool)> = sheet.row_groups.iter()
        .map(|g| (g.start_row, g.end_row, g.level, g.collapsed))
        .collect();
    let (row_outline, row_collapsed) = build_outline_lookup(max_row, &row_groups);

    // Assemble worksheet XML
    let mut xml = String::with_capacity(max_row * max_col * 40 + 1024);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    if needs_r_namespace {
        xml.push_str(r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#);
    } else {
        xml.push_str(r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#);
    }

    xml.push_str(r#"<sheetPr><outlinePr summaryBelow="1" summaryRight="1"/><pageSetUpPr/></sheetPr>"#);

    if max_row > 0 && max_col > 0 {
        let _ = write!(xml, r#"<dimension ref="A1:{}{}"/>"#, col_num_to_letter(max_col), max_row);
    } else {
        xml.push_str(r#"<dimension ref="A1"/>"#);
    }

    xml.push_str(&sheet_views_xml(options));
    xml.push_str(r#"<sheetFormatPr baseColWidth="8" defaultRowHeight="15"/>"#);
    xml.push_str(&cols_xml(sheet, &col_outline, &col_collapsed));
    xml.push_str(&sheet_data_xml(sheet, &row_outline, &row_collapsed));

    if options.auto_filter && max_row > 0 && max_col > 0 {
        let _ = write!(xml, r#"<autoFilter ref="A1:{}{}"/>"#, col_num_to_letter(max_col), max_row);
    }

    if !sheet.conditional_formats.is_empty() {
        let (cf_xml, _) = super::cond_fmt_xml::generate_conditional_formatting_xml(&sheet.conditional_formats, 0);
        xml.push_str(&cf_xml);
    }

    xml.push_str(&merge_cells_xml(sheet));
    xml.push_str(&data_validations_xml(sheet));
    xml.push_str(&hyperlinks_xml(sheet, has_chart));

    let margins = sheet.print_setup.as_ref().and_then(|ps| ps.margins).unwrap_or_default();
    let _ = write!(
        xml,
        r#"<pageMargins left="{}" right="{}" top="{}" bottom="{}" header="{}" footer="{}"/>"#,
        margins.left, margins.right, margins.top, margins.bottom, margins.header, margins.footer
    );

    if let Some(ref ps) = sheet.print_setup {
        xml.push_str(&generate_page_setup_xml(ps));
    }

    if has_chart {
        xml.push_str(r#"<drawing r:id="rId1"/>"#);
    }

    if !sheet.sparkline_groups.is_empty() {
        let sparkline_xml = super::sparkline_xml::generate_sparkline_ext_xml(&sheet.sparkline_groups, &sheet.name);
        xml.push_str(&sparkline_xml);
    }

    xml.push_str(r#"</worksheet>"#);

    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(format!("xl/worksheets/sheet{}.xml", idx + 1), opts)?;
    zip.write_all(xml.as_bytes())?;

    if needs_r_namespace {
        add_worksheet_rels(zip, idx, has_chart, &sheet.hyperlinks, &sheet.comments)?;
    }
    if !sheet.comments.is_empty() {
        add_comments_xml(zip, idx, &sheet.comments)?;
    }

    Ok(())
}

/// Add content types for chart/drawing parts
pub fn add_chart_content_types(xml: &mut String, _sheet_count: usize, charts: &[bool]) {
    for (idx, has_chart) in charts.iter().enumerate() {
        if *has_chart {
            let n = idx + 1;
            xml.push_str(&format!(
                r#"<Override PartName="/xl/charts/chart{}.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>"#,
                n
            ));
            xml.push_str(&format!(
                r#"<Override PartName="/xl/drawings/drawing{}.xml" ContentType="application/vnd.openxmlformats-officedocument.drawing+xml"/>"#,
                n
            ));
        }
    }
}

/// Add content types for comments
pub fn add_comment_content_types(xml: &mut String, comment_flags: &[bool]) {
    for (idx, has_comments) in comment_flags.iter().enumerate() {
        if *has_comments {
            let n = idx + 1;
            xml.push_str(&format!(
                r#"<Override PartName="/xl/comments{}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml"/>"#,
                n
            ));
        }
    }
}

fn generate_data_validation_xml(dv: &super::types::DataValidation) -> String {
    use super::types::ValidationType;

    let type_str = match &dv.validation_type {
        ValidationType::List { .. } => "list",
        ValidationType::Whole { .. } => "whole",
        ValidationType::Decimal { .. } => "decimal",
        ValidationType::Date { .. } => "date",
        ValidationType::TextLength { .. } => "textLength",
        ValidationType::Custom { .. } => "custom",
    };

    let operator_str = match &dv.validation_type {
        ValidationType::List { .. } => None,
        ValidationType::Custom { .. } => None,
        ValidationType::Whole { operator, .. }
        | ValidationType::Decimal { operator, .. }
        | ValidationType::Date { operator, .. }
        | ValidationType::TextLength { operator, .. } => Some(operator_to_str(*operator)),
    };

    let allow_blank = if dv.allow_blank { "1" } else { "0" };
    let show_dropdown = if dv.show_dropdown { "0" } else { "1" };

    let mut xml = format!(
        r#"<dataValidation type="{}" allowBlank="{}" showDropDown="{}" sqref="{}""#,
        type_str, allow_blank, show_dropdown, dv.range
    );

    if let Some(op) = operator_str {
        xml.push_str(&format!(r#" operator="{}""#, op));
    }

    xml.push('>');

    match &dv.validation_type {
        ValidationType::List { source } => {
            xml.push_str(&format!(r#"<formula1>"{}"</formula1>"#, escape_xml(source)));
        }
        ValidationType::Whole { formula1, formula2, .. }
        | ValidationType::Decimal { formula1, formula2, .. }
        | ValidationType::Date { formula1, formula2, .. } => {
            xml.push_str(&format!(r#"<formula1>{}</formula1>"#, escape_xml(formula1)));
            if let Some(f2) = formula2 {
                xml.push_str(&format!(r#"<formula2>{}</formula2>"#, escape_xml(f2)));
            }
        }
        ValidationType::TextLength { formula1, .. } => {
            xml.push_str(&format!(r#"<formula1>{}</formula1>"#, escape_xml(formula1)));
        }
        ValidationType::Custom { formula } => {
            xml.push_str(&format!(r#"<formula1>{}</formula1>"#, escape_xml(formula)));
        }
    }

    xml.push_str(r#"</dataValidation>"#);
    xml
}

fn operator_to_str(op: super::types::Operator) -> &'static str {
    use super::types::Operator;
    match op {
        Operator::Between => "between",
        Operator::NotBetween => "notBetween",
        Operator::Equal => "equal",
        Operator::NotEqual => "notEqual",
        Operator::GreaterThan => "greaterThan",
        Operator::LessThan => "lessThan",
        Operator::GreaterThanOrEqual => "greaterThanOrEqual",
        Operator::LessThanOrEqual => "lessThanOrEqual",
    }
}

fn generate_page_setup_xml(ps: &super::types::PrintSetup) -> String {
    use super::types::PageOrientation;
    let mut attrs = String::new();
    if let Some(orientation) = ps.orientation {
        attrs.push_str(&format!(
            r#" orientation="{}""#,
            match orientation {
                PageOrientation::Portrait => "portrait",
                PageOrientation::Landscape => "landscape",
            }
        ));
    }
    if let Some(paper_size) = ps.paper_size {
        attrs.push_str(&format!(r#" paperSize="{}""#, paper_size));
    }
    if let Some(scale) = ps.scale {
        attrs.push_str(&format!(r#" scale="{}""#, scale));
    }
    if let Some(fit_to_width) = ps.fit_to_width {
        attrs.push_str(&format!(r#" fitToWidth="{}""#, fit_to_width));
    }
    if let Some(fit_to_height) = ps.fit_to_height {
        attrs.push_str(&format!(r#" fitToHeight="{}""#, fit_to_height));
    }
    format!(r#"<pageSetup{} />"#, attrs)
}

fn add_worksheet_rels<W: IoWrite + Seek>(
    zip: &mut ZipWriter<W>,
    idx: usize,
    has_chart: bool,
    hyperlinks: &[super::types::Hyperlink],
    comments: &[super::types::CellComment],
) -> Result<()> {
    let sheet_idx = idx + 1;
    let mut xml = String::with_capacity(512);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#);

    let mut rel_id = 1;

    if has_chart {
        xml.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing{}.xml"/>"#,
            rel_id, sheet_idx
        ));
        rel_id += 1;
    }

    for hl in hyperlinks {
        xml.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="{}" TargetMode="External"/>"#,
            rel_id, escape_xml(&hl.url)
        ));
        rel_id += 1;
    }

    if !comments.is_empty() {
        xml.push_str(&format!(
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="../comments{}.xml"/>"#,
            rel_id, sheet_idx
        ));
    }

    xml.push_str(r#"</Relationships>"#);

    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(
        format!("xl/worksheets/_rels/sheet{}.xml.rels", sheet_idx),
        opts,
    )?;
    zip.write_all(xml.as_bytes())?;
    Ok(())
}

fn add_comments_xml<W: IoWrite + Seek>(
    zip: &mut ZipWriter<W>,
    idx: usize,
    comments: &[super::types::CellComment],
) -> Result<()> {
    let sheet_idx = idx + 1;
    let mut xml = String::with_capacity(512);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(r#"<comments xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#);

    xml.push_str(r#"<authors>"#);
    for comment in comments {
        let author = comment.author.as_deref().unwrap_or("Author");
        xml.push_str(&format!(r#"<author>{}</author>"#, escape_xml(author)));
    }
    xml.push_str(r#"</authors>"#);

    xml.push_str(r#"<commentList>"#);
    for (i, comment) in comments.iter().enumerate() {
        xml.push_str(&format!(
            r#"<comment ref="{}" authorId="{}">"#,
            comment.cell_ref, i
        ));
        xml.push_str(r#"<text>"#);
        xml.push_str(&format!(
            r#"<r><rPr><b/><sz val="9"/><color indexed="81"/><rFont val="Calibri"/></rPr><t>{}</t></r>"#,
            escape_xml(&comment.text)
        ));
        xml.push_str(r#"</text>"#);
        xml.push_str(r#"</comment>"#);
    }
    xml.push_str(r#"</commentList>"#);
    xml.push_str(r#"</comments>"#);

    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(format!("xl/comments{}.xml", sheet_idx), opts)?;
    zip.write_all(xml.as_bytes())?;
    Ok(())
}
