//! Chart XML generation for XLSX files
//!
//! Generates OOXML DrawingML chart markup for embedding charts in worksheets.
//! Supports: bar, column, line, pie, area, scatter, doughnut charts.

use anyhow::Result;
use std::io::{Seek, Write};
use zip::ZipWriter;
use zip::write::FileOptions;

use super::super::chart::{ChartConfig, DataChartType};
use super::xml_gen::{col_num_to_letter, escape_xml};

/// Default chart colors (Office theme palette)
const DEFAULT_COLORS: &[&str] = &[
    "4472C4", "ED7D31", "A5A5A5", "FFC000", "5B9BD5", "70AD47", "264478", "9B57A0",
];

/// Get color for a series index, using custom colors if provided
fn series_color(config: &ChartConfig, idx: usize) -> String {
    if let Some(ref colors) = config.colors {
        if let Some(c) = colors.get(idx) {
            return c.clone();
        }
    }
    DEFAULT_COLORS[idx % DEFAULT_COLORS.len()].to_string()
}

/// Generate the chart XML (xl/charts/chart{n}.xml)
pub fn generate_chart_xml(
    config: &ChartConfig,
    data: &[Vec<String>],
    sheet_name: &str,
) -> String {
    let mut xml = String::with_capacity(4096);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#);
    xml.push_str(r#"<c:chart>"#);

    // Title
    if let Some(ref title) = config.title {
        xml.push_str(r#"<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>"#);
        xml.push_str(&escape_xml(title));
        xml.push_str(r#"</a:t></a:r></a:p></c:rich></c:tx><c:overlay val="0"/></c:title>"#);
    } else {
        xml.push_str(r#"<c:autoTitleDeleted val="1"/>"#);
    }

    xml.push_str(r#"<c:plotArea><c:layout/>"#);

    // Chart type-specific plot
    let cat_col = config.category_column;
    match config.chart_type {
        DataChartType::Pie | DataChartType::Doughnut => {
            generate_pie_chart(&mut xml, config, data, sheet_name, cat_col);
        }
        DataChartType::Scatter => {
            generate_scatter_chart(&mut xml, config, data, sheet_name, cat_col);
        }
        _ => {
            generate_axis_chart(&mut xml, config, data, sheet_name, cat_col);
        }
    }

    xml.push_str(r#"</c:plotArea>"#);

    // Legend
    if config.show_legend {
        xml.push_str(r#"<c:legend><c:legendPos val="r"/><c:overlay val="0"/></c:legend>"#);
    }

    xml.push_str(r#"<c:plotVisOnly val="1"/></c:chart>"#);
    xml.push_str(r#"</c:chartSpace>"#);
    xml
}

/// Generate bar/column/line/area chart XML
fn generate_axis_chart(
    xml: &mut String,
    config: &ChartConfig,
    data: &[Vec<String>],
    sheet_name: &str,
    cat_col: usize,
) {
    let tag = match config.chart_type {
        DataChartType::Bar => "c:barChart",
        DataChartType::Column => "c:barChart",
        DataChartType::Line => "c:lineChart",
        DataChartType::Area => "c:areaChart",
        _ => "c:barChart",
    };

    xml.push_str(&format!("<{}>", tag));

    // Bar direction
    if matches!(config.chart_type, DataChartType::Bar | DataChartType::Column) {
        let dir = if config.chart_type == DataChartType::Bar {
            "bar"
        } else {
            "col"
        };
        xml.push_str(&format!(r#"<c:barDir val="{}"/>"#, dir));
        xml.push_str(r#"<c:grouping val="clustered"/>"#);
    }
    if config.chart_type == DataChartType::Line {
        xml.push_str(r#"<c:grouping val="standard"/>"#);
    }

    let data_rows = if data.len() > 1 { data.len() - 1 } else { 0 };

    for (ser_idx, &val_col) in config.value_columns.iter().enumerate() {
        let color = series_color(config, ser_idx);
        xml.push_str(&format!(r#"<c:ser><c:idx val="{}"/><c:order val="{}"/>"#, ser_idx, ser_idx));
        xml.push_str(&format!(r#"<c:tx><c:strRef><c:f>'{}'!{}{}</c:f></c:strRef></c:tx>"#,
            escape_xml(sheet_name),
            col_num_to_letter(val_col + 1),
            1
        ));

        // Series color
        xml.push_str(&format!(
            r#"<c:spPr><a:solidFill><a:srgbClr val="{}"/></a:solidFill></c:spPr>"#,
            color
        ));

        // Category reference
        generate_cat_ref(xml, data, sheet_name, cat_col, data_rows);

        // Value reference
        generate_val_ref(xml, data, sheet_name, val_col, data_rows);

        xml.push_str(r#"</c:ser>"#);
    }

    if matches!(config.chart_type, DataChartType::Line) {
        xml.push_str(r#"<c:marker><c:symbol val="none"/></c:marker>"#);
    }

    xml.push_str(r#"<c:axId val="1"/><c:axId val="2"/>"#);
    xml.push_str(&format!("</{}>", tag));

    // Category axis
    xml.push_str(r#"<c:catAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/>"#);
    if let Some(ref t) = config.x_axis_title {
        xml.push_str(&format!(
            r#"<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{}</a:t></a:r></a:p></c:rich></c:tx></c:title>"#,
            escape_xml(t)
        ));
    }
    xml.push_str(r#"<c:crossAx val="2"/></c:catAx>"#);

    // Value axis
    xml.push_str(r#"<c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/>"#);
    if let Some(ref t) = config.y_axis_title {
        xml.push_str(&format!(
            r#"<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{}</a:t></a:r></a:p></c:rich></c:tx></c:title>"#,
            escape_xml(t)
        ));
    }
    xml.push_str(r#"<c:crossAx val="1"/></c:valAx>"#);
}

/// Generate pie/doughnut chart XML
fn generate_pie_chart(
    xml: &mut String,
    config: &ChartConfig,
    data: &[Vec<String>],
    sheet_name: &str,
    cat_col: usize,
) {
    let tag = if config.chart_type == DataChartType::Doughnut {
        "c:doughnutChart"
    } else {
        "c:pieChart"
    };

    xml.push_str(&format!("<{}>", tag));

    let data_rows = if data.len() > 1 { data.len() - 1 } else { 0 };

    // Pie charts typically have one value column
    let val_col = config.value_columns.first().copied().unwrap_or(1);
    xml.push_str(r#"<c:ser><c:idx val="0"/><c:order val="0"/>"#);
    xml.push_str(&format!(r#"<c:tx><c:strRef><c:f>'{}'!{}{}</c:f></c:strRef></c:tx>"#,
        escape_xml(sheet_name),
        col_num_to_letter(val_col + 1),
        1
    ));

    // Per-point colors for pie
    for (pt_idx, _) in data.iter().skip(1).enumerate() {
        let color = series_color(config, pt_idx);
        xml.push_str(&format!(
            r#"<c:dPt><c:idx val="{}"/><c:spPr><a:solidFill><a:srgbClr val="{}"/></a:solidFill></c:spPr></c:dPt>"#,
            pt_idx, color
        ));
    }

    generate_cat_ref(xml, data, sheet_name, cat_col, data_rows);
    generate_val_ref(xml, data, sheet_name, val_col, data_rows);

    xml.push_str(r#"</c:ser>"#);

    if config.chart_type == DataChartType::Doughnut {
        xml.push_str(r#"<c:holeSize val="50"/>"#);
    }

    xml.push_str(&format!("</{}>", tag));
}

/// Generate scatter chart XML
fn generate_scatter_chart(
    xml: &mut String,
    config: &ChartConfig,
    data: &[Vec<String>],
    sheet_name: &str,
    cat_col: usize,
) {
    xml.push_str(r#"<c:scatterChart><c:scatterStyle val="lineMarker"/>"#);

    let data_rows = if data.len() > 1 { data.len() - 1 } else { 0 };

    for (ser_idx, &val_col) in config.value_columns.iter().enumerate() {
        let color = series_color(config, ser_idx);

        xml.push_str(&format!(r#"<c:ser><c:idx val="{}"/><c:order val="{}"/>"#, ser_idx, ser_idx));
        xml.push_str(&format!(
            r#"<c:spPr><a:ln><a:solidFill><a:srgbClr val="{}"/></a:solidFill></a:ln></c:spPr>"#,
            color
        ));

        // X values
        xml.push_str(r#"<c:xVal>"#);
        generate_num_ref_inner(xml, data, sheet_name, cat_col, data_rows);
        xml.push_str(r#"</c:xVal>"#);

        // Y values
        xml.push_str(r#"<c:yVal>"#);
        generate_num_ref_inner(xml, data, sheet_name, val_col, data_rows);
        xml.push_str(r#"</c:yVal>"#);

        xml.push_str(r#"</c:ser>"#);
    }

    xml.push_str(r#"<c:axId val="1"/><c:axId val="2"/></c:scatterChart>"#);

    // X axis
    xml.push_str(r#"<c:valAx><c:axId val="1"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:crossAx val="2"/></c:valAx>"#);
    // Y axis
    xml.push_str(r#"<c:valAx><c:axId val="2"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:crossAx val="1"/></c:valAx>"#);
}

/// Generate category reference XML
fn generate_cat_ref(
    xml: &mut String,
    data: &[Vec<String>],
    sheet_name: &str,
    cat_col: usize,
    data_rows: usize,
) {
    if data_rows == 0 {
        return;
    }
    let col_letter = col_num_to_letter(cat_col + 1);
    let sheet_esc = escape_xml(sheet_name);
    xml.push_str(r#"<c:cat><c:strRef>"#);
    xml.push_str(&format!(
        r#"<c:f>'{}'!${}$2:${}${}</c:f>"#,
        sheet_esc, col_letter, col_letter, data_rows + 1
    ));
    xml.push_str(r#"<c:strCache>"#);
    xml.push_str(&format!(r#"<c:ptCount val="{}"/>"#, data_rows));
    for (i, row) in data.iter().skip(1).enumerate() {
        if let Some(val) = row.get(cat_col) {
            xml.push_str(&format!(r#"<c:pt idx="{}"><c:v>{}</c:v></c:pt>"#, i, escape_xml(val)));
        }
    }
    xml.push_str(r#"</c:strCache></c:strRef></c:cat>"#);
}

/// Generate value reference XML
fn generate_val_ref(
    xml: &mut String,
    data: &[Vec<String>],
    sheet_name: &str,
    val_col: usize,
    data_rows: usize,
) {
    if data_rows == 0 {
        return;
    }
    let col_letter = col_num_to_letter(val_col + 1);
    let sheet_esc = escape_xml(sheet_name);
    xml.push_str(r#"<c:val><c:numRef>"#);
    xml.push_str(&format!(
        r#"<c:f>'{}'!${}$2:${}${}</c:f>"#,
        sheet_esc, col_letter, col_letter, data_rows + 1
    ));
    xml.push_str(r#"<c:numCache>"#);
    xml.push_str(&format!(r#"<c:ptCount val="{}"/>"#, data_rows));
    for (i, row) in data.iter().skip(1).enumerate() {
        if let Some(val) = row.get(val_col) {
            xml.push_str(&format!(r#"<c:pt idx="{}"><c:v>{}</c:v></c:pt>"#, i, escape_xml(val)));
        }
    }
    xml.push_str(r#"</c:numCache></c:numRef></c:val>"#);
}

/// Generate numeric reference (for scatter X/Y values)
fn generate_num_ref_inner(
    xml: &mut String,
    data: &[Vec<String>],
    sheet_name: &str,
    col: usize,
    data_rows: usize,
) {
    if data_rows == 0 {
        return;
    }
    let col_letter = col_num_to_letter(col + 1);
    let sheet_esc = escape_xml(sheet_name);
    xml.push_str(r#"<c:numRef>"#);
    xml.push_str(&format!(
        r#"<c:f>'{}'!${}$2:${}${}</c:f>"#,
        sheet_esc, col_letter, col_letter, data_rows + 1
    ));
    xml.push_str(r#"<c:numCache>"#);
    xml.push_str(&format!(r#"<c:ptCount val="{}"/>"#, data_rows));
    for (i, row) in data.iter().skip(1).enumerate() {
        if let Some(val) = row.get(col) {
            xml.push_str(&format!(r#"<c:pt idx="{}"><c:v>{}</c:v></c:pt>"#, i, escape_xml(val)));
        }
    }
    xml.push_str(r#"</c:numCache></c:numRef>"#);
}

/// Generate the drawing XML (xl/drawings/drawing{n}.xml)
pub fn generate_drawing_xml(chart_rid: &str, width_emu: u64, height_emu: u64) -> String {
    let mut xml = String::with_capacity(1024);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push_str(r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#);
    xml.push_str(r#"<xdr:twoCellAnchor>"#);
    // Position: start at E2, end based on size
    xml.push_str(r#"<xdr:from><xdr:col>4</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>"#);
    xml.push_str(r#"<xdr:to><xdr:col>14</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>20</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>"#);
    xml.push_str(r#"<xdr:graphicFrame macro="">"#);
    xml.push_str(r#"<xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/><xdr:cNvGraphicFramePr/></xdr:nvGraphicFramePr>"#);
    xml.push_str(r#"<xdr:xfrm><a:off x="0" y="0"/>"#);
    xml.push_str(&format!(r#"<a:ext cx="{}" cy="{}"/>"#, width_emu, height_emu));
    xml.push_str(r#"</xdr:xfrm>"#);
    xml.push_str(r#"<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">"#);
    xml.push_str(&format!(r#"<c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="{}"/>"#, chart_rid));
    xml.push_str(r#"</a:graphicData></a:graphic>"#);
    xml.push_str(r#"</xdr:graphicFrame>"#);
    xml.push_str(r#"<xdr:clientData/>"#);
    xml.push_str(r#"</xdr:twoCellAnchor>"#);
    xml.push_str(r#"</xdr:wsDr>"#);
    xml
}

/// Add chart-related files to the ZIP archive for a specific sheet
pub fn add_chart_to_zip<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    sheet_idx: usize,
    config: &ChartConfig,
    data: &[Vec<String>],
    sheet_name: &str,
) -> Result<()> {
    let chart_idx = sheet_idx + 1;
    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Pixel to EMU conversion (1 pixel = 9525 EMU)
    let width_emu = config.width as u64 * 9525;
    let height_emu = config.height as u64 * 9525;

    // 1. xl/charts/chart{n}.xml
    let chart_xml = generate_chart_xml(config, data, sheet_name);
    zip.start_file(format!("xl/charts/chart{}.xml", chart_idx), opts)?;
    zip.write_all(chart_xml.as_bytes())?;

    // 2. xl/drawings/drawing{n}.xml
    let drawing_xml = generate_drawing_xml("rId1", width_emu, height_emu);
    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(format!("xl/drawings/drawing{}.xml", chart_idx), opts)?;
    zip.write_all(drawing_xml.as_bytes())?;

    // 3. xl/drawings/_rels/drawing{n}.xml.rels
    let drawing_rels = format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
            r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart{}.xml"/>"#,
            r#"</Relationships>"#,
        ),
        chart_idx
    );
    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(format!("xl/drawings/_rels/drawing{}.xml.rels", chart_idx), opts)?;
    zip.write_all(drawing_rels.as_bytes())?;

    // 4. xl/worksheets/_rels/sheet{n}.xml.rels
    let sheet_rels = format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
            r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing{}.xml"/>"#,
            r#"</Relationships>"#,
        ),
        chart_idx
    );
    let opts = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(format!("xl/worksheets/_rels/sheet{}.xml.rels", chart_idx), opts)?;
    zip.write_all(sheet_rels.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data() -> Vec<Vec<String>> {
        vec![
            vec!["Category".into(), "Value".into()],
            vec!["A".into(), "10".into()],
            vec!["B".into(), "20".into()],
            vec!["C".into(), "30".into()],
        ]
    }

    fn multi_series_data() -> Vec<Vec<String>> {
        vec![
            vec!["Month".into(), "Sales".into(), "Costs".into()],
            vec!["Jan".into(), "100".into(), "60".into()],
            vec!["Feb".into(), "120".into(), "70".into()],
        ]
    }

    #[test]
    fn test_series_color_default() {
        let config = ChartConfig::default();
        assert_eq!(series_color(&config, 0), "4472C4");
        assert_eq!(series_color(&config, 1), "ED7D31");
        // Wraps around
        assert_eq!(series_color(&config, 8), "4472C4");
    }

    #[test]
    fn test_series_color_custom() {
        let config = ChartConfig {
            colors: Some(vec!["FF0000".into(), "00FF00".into()]),
            ..Default::default()
        };
        assert_eq!(series_color(&config, 0), "FF0000");
        assert_eq!(series_color(&config, 1), "00FF00");
        // Falls back to default when custom exhausted
        assert_eq!(series_color(&config, 2), "A5A5A5");
    }

    #[test]
    fn test_generate_column_chart_xml() {
        let config = ChartConfig {
            chart_type: DataChartType::Column,
            title: Some("Test Chart".into()),
            x_axis_title: Some("X".into()),
            y_axis_title: Some("Y".into()),
            ..Default::default()
        };
        let xml = generate_chart_xml(&config, &sample_data(), "Sheet1");
        assert!(xml.contains("c:chartSpace"));
        assert!(xml.contains("c:barChart"));
        assert!(xml.contains(r#"c:barDir val="col""#));
        assert!(xml.contains("Test Chart"));
        assert!(xml.contains("<a:t>X</a:t>"));
        assert!(xml.contains("<a:t>Y</a:t>"));
        assert!(xml.contains("c:legend"));
    }

    #[test]
    fn test_generate_bar_chart_xml() {
        let config = ChartConfig {
            chart_type: DataChartType::Bar,
            ..Default::default()
        };
        let xml = generate_chart_xml(&config, &sample_data(), "Sheet1");
        assert!(xml.contains(r#"c:barDir val="bar""#));
        assert!(xml.contains("c:grouping"));
    }

    #[test]
    fn test_generate_line_chart_xml() {
        let config = ChartConfig {
            chart_type: DataChartType::Line,
            ..Default::default()
        };
        let xml = generate_chart_xml(&config, &sample_data(), "Sheet1");
        assert!(xml.contains("c:lineChart"));
        assert!(xml.contains(r#"c:grouping val="standard""#));
        assert!(xml.contains(r#"c:symbol val="none""#));
    }

    #[test]
    fn test_generate_area_chart_xml() {
        let config = ChartConfig {
            chart_type: DataChartType::Area,
            ..Default::default()
        };
        let xml = generate_chart_xml(&config, &sample_data(), "Sheet1");
        assert!(xml.contains("c:areaChart"));
    }

    #[test]
    fn test_generate_pie_chart_xml() {
        let config = ChartConfig {
            chart_type: DataChartType::Pie,
            title: Some("Pie".into()),
            ..Default::default()
        };
        let xml = generate_chart_xml(&config, &sample_data(), "Sheet1");
        assert!(xml.contains("c:pieChart"));
        // Per-point colors
        assert!(xml.contains("c:dPt"));
        assert!(xml.contains("Pie"));
        // No axes for pie
        assert!(!xml.contains("c:catAx"));
    }

    #[test]
    fn test_generate_doughnut_chart_xml() {
        let config = ChartConfig {
            chart_type: DataChartType::Doughnut,
            ..Default::default()
        };
        let xml = generate_chart_xml(&config, &sample_data(), "Sheet1");
        assert!(xml.contains("c:doughnutChart"));
        assert!(xml.contains(r#"c:holeSize val="50""#));
    }

    #[test]
    fn test_generate_scatter_chart_xml() {
        let config = ChartConfig {
            chart_type: DataChartType::Scatter,
            ..Default::default()
        };
        let xml = generate_chart_xml(&config, &sample_data(), "Sheet1");
        assert!(xml.contains("c:scatterChart"));
        assert!(xml.contains("c:xVal"));
        assert!(xml.contains("c:yVal"));
    }

    #[test]
    fn test_chart_no_title() {
        let config = ChartConfig {
            title: None,
            ..Default::default()
        };
        let xml = generate_chart_xml(&config, &sample_data(), "Sheet1");
        assert!(xml.contains(r#"c:autoTitleDeleted val="1""#));
    }

    #[test]
    fn test_chart_no_legend() {
        let config = ChartConfig {
            show_legend: false,
            ..Default::default()
        };
        let xml = generate_chart_xml(&config, &sample_data(), "Sheet1");
        assert!(!xml.contains("c:legend"));
    }

    #[test]
    fn test_chart_multi_series() {
        let config = ChartConfig {
            chart_type: DataChartType::Column,
            value_columns: vec![1, 2],
            ..Default::default()
        };
        let xml = generate_chart_xml(&config, &multi_series_data(), "Sheet1");
        // Two series
        assert!(xml.contains(r#"c:idx val="0""#));
        assert!(xml.contains(r#"c:idx val="1""#));
    }

    #[test]
    fn test_chart_empty_data() {
        let config = ChartConfig::default();
        let data: Vec<Vec<String>> = vec![];
        let xml = generate_chart_xml(&config, &data, "Sheet1");
        // Should still produce valid XML structure
        assert!(xml.contains("c:chartSpace"));
        assert!(xml.contains("c:barChart"));
    }

    #[test]
    fn test_chart_single_row_header_only() {
        let config = ChartConfig::default();
        let data = vec![vec!["Header".into(), "Value".into()]];
        let xml = generate_chart_xml(&config, &data, "Sheet1");
        assert!(xml.contains("c:chartSpace"));
        assert!(xml.contains("c:barChart"));
        // No data points generated when only header row exists
        assert!(!xml.contains("<c:pt idx="));
    }

    #[test]
    fn test_chart_special_chars_in_sheet_name() {
        let config = ChartConfig::default();
        let xml = generate_chart_xml(&config, &sample_data(), "Sales & Revenue");
        assert!(xml.contains("Sales &amp; Revenue"));
    }

    #[test]
    fn test_chart_data_references() {
        let config = ChartConfig {
            category_column: 0,
            value_columns: vec![1],
            ..Default::default()
        };
        let xml = generate_chart_xml(&config, &sample_data(), "Sheet1");
        // Category ref should point to column A rows 2-4
        assert!(xml.contains("$A$2:$A$4"));
        // Value ref should point to column B rows 2-4
        assert!(xml.contains("$B$2:$B$4"));
    }

    #[test]
    fn test_generate_drawing_xml() {
        let xml = generate_drawing_xml("rId1", 5715000, 3810000);
        assert!(xml.contains("xdr:wsDr"));
        assert!(xml.contains("xdr:twoCellAnchor"));
        assert!(xml.contains(r#"r:id="rId1""#));
        assert!(xml.contains(r#"cx="5715000""#));
        assert!(xml.contains(r#"cy="3810000""#));
    }

    #[test]
    fn test_drawing_xml_structure() {
        let xml = generate_drawing_xml("rId1", 100, 200);
        assert!(xml.starts_with(r#"<?xml version="1.0""#));
        assert!(xml.contains("xdr:graphicFrame"));
        assert!(xml.contains("xdr:clientData"));
        assert!(xml.ends_with("</xdr:wsDr>"));
    }
}
