//! XML generation for XLSX files
//!
//! Generates proper Office Open XML (OOXML) that is compatible with
//! Microsoft Excel, Apple Numbers, and LibreOffice Calc.

use anyhow::Result;
use std::io::{Seek, Write};
use zip::ZipWriter;
use zip::write::FileOptions;

use super::types::{CellData, SheetData};
use super::WriteOptions;

/// Escape special XML characters
pub fn escape_xml(s: &str) -> String {
    let extra = s.bytes().filter(|&b| matches!(b, b'&' | b'<' | b'>' | b'"' | b'\'')).count();
    let mut out = String::with_capacity(s.len() + extra * 4);
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
    out
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

/// Add [Content_Types].xml
pub fn add_content_types<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    sheet_count: usize,
) -> Result<()> {
    let no_flags = vec![false; sheet_count];
    add_content_types_ext(zip, sheet_count, &no_flags, &no_flags)
}

/// Add [Content_Types].xml with optional chart/drawing/comment content types
pub fn add_content_types_ext<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    sheet_count: usize,
    chart_flags: &[bool],
    comment_flags: &[bool],
) -> Result<()> {
    let mut xml = String::with_capacity(1024);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#);
    xml.push_str(r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#);
    xml.push_str(r#"<Default Extension="xml" ContentType="application/xml"/>"#);
    xml.push_str(r#"<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#);
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
pub fn add_rels<W: Write + Seek>(zip: &mut ZipWriter<W>) -> Result<()> {
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
pub fn add_workbook<W: Write + Seek>(
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
pub fn add_workbook_rels<W: Write + Seek>(
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
pub fn add_styles<W: Write + Seek>(zip: &mut ZipWriter<W>) -> Result<()> {
    let xml = concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
        r#"<numFmts count="0"/>"#,
        // Font 0: normal, Font 1: bold
        r#"<fonts count="2">"#,
        r#"<font><name val="Calibri"/><family val="2"/><color theme="1"/><sz val="11"/><scheme val="minor"/></font>"#,
        r#"<font><b/><name val="Calibri"/><family val="2"/><color theme="1"/><sz val="11"/><scheme val="minor"/></font>"#,
        r#"</fonts>"#,
        // Fill 0: none, Fill 1: gray125 (required), Fill 2: header blue
        r#"<fills count="3">"#,
        r#"<fill><patternFill/></fill>"#,
        r#"<fill><patternFill patternType="gray125"/></fill>"#,
        r#"<fill><patternFill patternType="solid"><fgColor rgb="FF4472C4"/><bgColor indexed="64"/></patternFill></fill>"#,
        r#"</fills>"#,
        // Border 0: none, Border 1: thin all sides
        r#"<borders count="2">"#,
        r#"<border><left/><right/><top/><bottom/><diagonal/></border>"#,
        r#"<border><left style="thin"><color auto="1"/></left><right style="thin"><color auto="1"/></right><top style="thin"><color auto="1"/></top><bottom style="thin"><color auto="1"/></bottom><diagonal/></border>"#,
        r#"</borders>"#,
        r#"<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>"#,
        // xf 0: normal, xf 1: bold+fill+border (header), xf 2: centered
        r#"<cellXfs count="3">"#,
        r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>"#,
        r#"<xf numFmtId="0" fontId="1" fillId="2" borderId="1" xfId="0" applyFont="1" applyFill="1" applyBorder="1"><alignment horizontal="center"/></xf>"#,
        r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"><alignment horizontal="center"/></xf>"#,
        r#"</cellXfs>"#,
        r#"<cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>"#,
        r#"<tableStyles count="0" defaultTableStyle="TableStyleMedium9" defaultPivotStyle="PivotStyleLight16"/>"#,
        r#"</styleSheet>"#,
    );

    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("xl/styles.xml", opts)?;
    zip.write_all(xml.as_bytes())?;
    Ok(())
}

/// Add xl/theme/theme1.xml
pub fn add_theme<W: Write + Seek>(zip: &mut ZipWriter<W>) -> Result<()> {
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

/// Add worksheet XML
pub fn add_worksheet<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    idx: usize,
    sheet: &SheetData,
    options: &WriteOptions,
    has_chart: bool,
) -> Result<()> {
    let max_row = sheet.rows.len();
    let max_col = sheet.rows.iter().map(|r| r.cells.len()).max().unwrap_or(0);

    let needs_r_namespace = has_chart || !sheet.hyperlinks.is_empty() || !sheet.comments.is_empty();

    let mut xml = String::with_capacity(max_row * max_col * 40 + 1024);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    if needs_r_namespace {
        xml.push_str(r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#);
    } else {
        xml.push_str(r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#);
    }

    // Sheet properties
    xml.push_str(r#"<sheetPr><outlinePr summaryBelow="1" summaryRight="1"/><pageSetUpPr/></sheetPr>"#);

    // Dimension
    if max_row > 0 && max_col > 0 {
        xml.push_str(&format!(
            r#"<dimension ref="A1:{}{}"/>"#,
            col_num_to_letter(max_col),
            max_row
        ));
    } else {
        xml.push_str(r#"<dimension ref="A1"/>"#);
    }

    // Sheet views
    xml.push_str(r#"<sheetViews>"#);
    if options.freeze_header {
        xml.push_str(r#"<sheetView workbookViewId="0">"#);
        xml.push_str(r#"<pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/>"#);
        xml.push_str(r#"<selection pane="bottomLeft" activeCell="A2" sqref="A2"/>"#);
        xml.push_str(r#"</sheetView>"#);
    } else {
        xml.push_str(r#"<sheetView workbookViewId="0">"#);
        xml.push_str(r#"<selection activeCell="A1" sqref="A1"/>"#);
        xml.push_str(r#"</sheetView>"#);
    }
    xml.push_str(r#"</sheetViews>"#);

    // Sheet format properties (required by Excel/Numbers)
    xml.push_str(r#"<sheetFormatPr baseColWidth="8" defaultRowHeight="15"/>"#);

    // Build column outline level lookup
    let mut col_outline: Vec<u8> = Vec::new();
    let mut col_collapsed: Vec<bool> = Vec::new();
    for cg in &sheet.col_groups {
        let end = cg.end_col.max(col_outline.len().saturating_sub(1));
        if col_outline.len() <= end {
            col_outline.resize(end + 1, 0);
            col_collapsed.resize(end + 1, false);
        }
        for c in cg.start_col..=cg.end_col {
            if c < col_outline.len() {
                col_outline[c] = col_outline[c].max(cg.level);
                col_collapsed[c] = col_collapsed[c] || cg.collapsed;
            }
        }
    }

    // Column widths
    if !sheet.column_widths.is_empty() {
        xml.push_str(r#"<cols>"#);
        for (col_idx, &width) in sheet.column_widths.iter().enumerate() {
            let outline = if col_idx < col_outline.len() && col_outline[col_idx] > 0 {
                format!(r#" outlineLevel="{}""#, col_outline[col_idx])
            } else {
                String::new()
            };
            let collapsed = if col_idx < col_collapsed.len() && col_collapsed[col_idx] {
                r#" collapsed="1""#.to_string()
            } else {
                String::new()
            };
            xml.push_str(&format!(
                r#"<col min="{}" max="{}"{}{} width="{}" customWidth="1"/>"#,
                col_idx + 1,
                col_idx + 1,
                outline,
                collapsed,
                width
            ));
        }
        xml.push_str(r#"</cols>"#);
    }

    // Build row outline level lookup
    let mut row_outline: Vec<u8> = vec![0; sheet.rows.len()];
    let mut row_collapsed: Vec<bool> = vec![false; sheet.rows.len()];
    for rg in &sheet.row_groups {
        for r in rg.start_row..=rg.end_row {
            if r < row_outline.len() {
                row_outline[r] = row_outline[r].max(rg.level);
                row_collapsed[r] = row_collapsed[r] || rg.collapsed;
            }
        }
    }

    // Sheet data
    xml.push_str(r#"<sheetData>"#);
    for (row_idx, row) in sheet.rows.iter().enumerate() {
        let outline_attr = if row_outline[row_idx] > 0 {
            format!(r#" outlineLevel="{}""#, row_outline[row_idx])
        } else {
            String::new()
        };
        let collapsed_attr = if row_collapsed[row_idx] {
            r#" collapsed="1" hidden="1""#.to_string()
        } else {
            String::new()
        };
        xml.push_str(&format!(r#"<row r="{}{}{}">"#, row_idx + 1, outline_attr, collapsed_attr));
        for (col_idx, cell) in row.cells.iter().enumerate() {
            let col_ref = col_num_to_letter(col_idx + 1);
            let cell_ref = format!("{}{}", col_ref, row_idx + 1);
            match cell {
                CellData::String(s) => {
                    xml.push_str(&format!(
                        r#"<c r="{}" t="inlineStr"><is><t>{}</t></is></c>"#,
                        cell_ref,
                        escape_xml(s)
                    ));
                }
                CellData::Number(n) => {
                    xml.push_str(&format!(
                        r#"<c r="{}" t="n"><v>{}</v></c>"#,
                        cell_ref, n
                    ));
                }
                CellData::Formula(f) => {
                    let formula = f.strip_prefix('=').unwrap_or(f);
                    xml.push_str(&format!(
                        r#"<c r="{}"><f>{}</f></c>"#,
                        cell_ref,
                        escape_xml(formula)
                    ));
                }
                CellData::Empty => {}
            }
        }
        xml.push_str(r#"</row>"#);
    }
    xml.push_str(r#"</sheetData>"#);

    // AutoFilter
    if options.auto_filter && max_row > 0 && max_col > 0 {
        xml.push_str(&format!(
            r#"<autoFilter ref="A1:{}{}"/>"#,
            col_num_to_letter(max_col),
            max_row
        ));
    }

    // Conditional formatting
    if !sheet.conditional_formats.is_empty() {
        let (cf_xml, _dxf_entries) =
            super::cond_fmt_xml::generate_conditional_formatting_xml(&sheet.conditional_formats, 0);
        xml.push_str(&cf_xml);
    }

    // Merge cells
    if !sheet.merge_cells.is_empty() {
        xml.push_str(&format!(
            r#"<mergeCells count="{}">"#,
            sheet.merge_cells.len()
        ));
        for mc in &sheet.merge_cells {
            let start_ref = format!("{}{}", col_num_to_letter(mc.start_col + 1), mc.start_row + 1);
            let end_ref = format!("{}{}", col_num_to_letter(mc.end_col + 1), mc.end_row + 1);
            xml.push_str(&format!(r#"<mergeCell ref="{}:{}"/>"#, start_ref, end_ref));
        }
        xml.push_str(r#"</mergeCells>"#);
    }

    // Data validations
    if !sheet.data_validations.is_empty() {
        xml.push_str(&format!(
            r#"<dataValidations count="{}">"#,
            sheet.data_validations.len()
        ));
        for dv in &sheet.data_validations {
            xml.push_str(&generate_data_validation_xml(dv));
        }
        xml.push_str(r#"</dataValidations>"#);
    }

    // Hyperlinks
    if !sheet.hyperlinks.is_empty() {
        xml.push_str(r#"<hyperlinks>"#);
        let mut rel_id = if has_chart { 2 } else { 1 };
        for hl in &sheet.hyperlinks {
            let tooltip_attr = hl.tooltip.as_ref().map(|t| format!(r#" tooltip="{}""#, escape_xml(t))).unwrap_or_default();
            xml.push_str(&format!(
                r#"<hyperlink ref="{}" r:id="rId{}"{}/>"#,
                hl.cell_ref, rel_id, tooltip_attr
            ));
            rel_id += 1;
        }
        xml.push_str(r#"</hyperlinks>"#);
    }

    // Page margins (use PrintSetup if provided, else defaults)
    let margins = sheet.print_setup.as_ref().and_then(|ps| ps.margins).unwrap_or_default();
    xml.push_str(&format!(
        r#"<pageMargins left="{}" right="{}" top="{}" bottom="{}" header="{}" footer="{}"/>"#,
        margins.left, margins.right, margins.top, margins.bottom, margins.header, margins.footer
    ));

    // Page setup
    if let Some(ref ps) = sheet.print_setup {
        xml.push_str(&generate_page_setup_xml(ps));
    }

    // Drawing reference (for charts)
    if has_chart {
        xml.push_str(r#"<drawing r:id="rId1"/>"#);
    }

    // Sparklines (must come after pageMargins, before closing worksheet)
    if !sheet.sparkline_groups.is_empty() {
        let sparkline_xml =
            super::sparkline_xml::generate_sparkline_ext_xml(&sheet.sparkline_groups, &sheet.name);
        xml.push_str(&sparkline_xml);
    }

    xml.push_str(r#"</worksheet>"#);

    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(&format!("xl/worksheets/sheet{}.xml", idx + 1), opts)?;
    zip.write_all(xml.as_bytes())?;

    // Worksheet rels (hyperlinks, comments, charts)
    if needs_r_namespace {
        add_worksheet_rels(zip, idx, has_chart, &sheet.hyperlinks, &sheet.comments)?;
    }

    // Comments XML
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

    xml.push_str(">");

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

fn add_worksheet_rels<W: Write + Seek>(
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
        &format!("xl/worksheets/_rels/sheet{}.xml.rels", sheet_idx),
        opts,
    )?;
    zip.write_all(xml.as_bytes())?;
    Ok(())
}

fn add_comments_xml<W: Write + Seek>(
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
    zip.start_file(&format!("xl/comments{}.xml", sheet_idx), opts)?;
    zip.write_all(xml.as_bytes())?;
    Ok(())
}
