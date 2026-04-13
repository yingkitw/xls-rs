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
    s.chars()
        .flat_map(|c| match c {
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '<' => "&lt;".chars().collect::<Vec<_>>(),
            '>' => "&gt;".chars().collect::<Vec<_>>(),
            '"' => "&quot;".chars().collect::<Vec<_>>(),
            '\'' => "&apos;".chars().collect::<Vec<_>>(),
            _ => vec![c],
        })
        .collect()
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
    let no_charts = vec![false; sheet_count];
    add_content_types_ext(zip, sheet_count, &no_charts)
}

/// Add [Content_Types].xml with optional chart/drawing content types
pub fn add_content_types_ext<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    sheet_count: usize,
    chart_flags: &[bool],
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

    let mut xml = String::with_capacity(max_row * max_col * 40 + 512);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#);

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

    // Column widths
    if !sheet.column_widths.is_empty() {
        xml.push_str(r#"<cols>"#);
        for (col_idx, &width) in sheet.column_widths.iter().enumerate() {
            xml.push_str(&format!(
                r#"<col min="{}" max="{}" width="{}" customWidth="1"/>"#,
                col_idx + 1,
                col_idx + 1,
                width
            ));
        }
        xml.push_str(r#"</cols>"#);
    }

    // Sheet data
    xml.push_str(r#"<sheetData>"#);
    for (row_idx, row) in sheet.rows.iter().enumerate() {
        xml.push_str(&format!(r#"<row r="{}">"#, row_idx + 1));
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
                    let formula = if f.starts_with('=') { &f[1..] } else { f };
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

    // Page margins (required by Excel/Numbers)
    xml.push_str(r#"<pageMargins left="0.75" right="0.75" top="1" bottom="1" header="0.5" footer="0.5"/>"#);

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

    // If we have a drawing reference, we need the r: namespace
    if has_chart {
        // Replace the worksheet opening tag to include the r: namespace
        xml = xml.replacen(
            r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
            r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
            1,
        );
    }

    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(&format!("xl/worksheets/sheet{}.xml", idx + 1), opts)?;
    zip.write_all(xml.as_bytes())?;
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
