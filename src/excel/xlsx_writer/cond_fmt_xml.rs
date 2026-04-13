//! Conditional formatting XML generation for XLSX files
//!
//! Supports: color scales, data bars, icon sets, and formula-based conditions.

use super::xml_gen::escape_xml;

/// Conditional formatting rule type
#[derive(Debug, Clone)]
pub enum ConditionalRule {
    /// Two-color scale (min color → max color)
    ColorScale {
        min_color: String,
        max_color: String,
    },
    /// Three-color scale (min → mid → max)
    ThreeColorScale {
        min_color: String,
        mid_color: String,
        max_color: String,
    },
    /// Data bar visualization
    DataBar {
        color: String,
    },
    /// Icon set (3Icons, 4Arrows, 5Quarters, etc.)
    IconSet {
        icon_style: String,
    },
    /// Formula-based: highlight cells where formula is true
    Formula {
        formula: String,
        bg_color: Option<String>,
        font_color: Option<String>,
        bold: bool,
    },
    /// Cell value condition (greaterThan, lessThan, equal, between, etc.)
    CellValue {
        operator: String,
        value: String,
        bg_color: Option<String>,
    },
}

/// A conditional formatting entry for a range
#[derive(Debug, Clone)]
pub struct ConditionalFormat {
    pub range: String,
    pub rules: Vec<ConditionalRule>,
}

/// Generate conditional formatting XML fragment to insert into worksheet XML.
/// Returns (cf_xml, dxf_styles) where dxf_styles are differential format entries
/// that must be added to styles.xml.
pub fn generate_conditional_formatting_xml(
    formats: &[ConditionalFormat],
    dxf_start_id: usize,
) -> (String, Vec<String>) {
    let mut xml = String::new();
    let mut dxf_entries = Vec::new();
    let mut dxf_id = dxf_start_id;

    for cf in formats {
        xml.push_str(&format!(
            r#"<conditionalFormatting sqref="{}">"#,
            escape_xml(&cf.range)
        ));

        for (rule_idx, rule) in cf.rules.iter().enumerate() {
            let priority = dxf_id + rule_idx + 1;
            match rule {
                ConditionalRule::ColorScale { min_color, max_color } => {
                    xml.push_str(&format!(
                        r#"<cfRule type="colorScale" priority="{}">"#,
                        priority
                    ));
                    xml.push_str(r#"<colorScale>"#);
                    xml.push_str(r#"<cfvo type="min"/>"#);
                    xml.push_str(r#"<cfvo type="max"/>"#);
                    xml.push_str(&format!(r#"<color rgb="FF{}"/>"#, min_color));
                    xml.push_str(&format!(r#"<color rgb="FF{}"/>"#, max_color));
                    xml.push_str(r#"</colorScale>"#);
                    xml.push_str(r#"</cfRule>"#);
                }
                ConditionalRule::ThreeColorScale {
                    min_color,
                    mid_color,
                    max_color,
                } => {
                    xml.push_str(&format!(
                        r#"<cfRule type="colorScale" priority="{}">"#,
                        priority
                    ));
                    xml.push_str(r#"<colorScale>"#);
                    xml.push_str(r#"<cfvo type="min"/>"#);
                    xml.push_str(r#"<cfvo type="percentile" val="50"/>"#);
                    xml.push_str(r#"<cfvo type="max"/>"#);
                    xml.push_str(&format!(r#"<color rgb="FF{}"/>"#, min_color));
                    xml.push_str(&format!(r#"<color rgb="FF{}"/>"#, mid_color));
                    xml.push_str(&format!(r#"<color rgb="FF{}"/>"#, max_color));
                    xml.push_str(r#"</colorScale>"#);
                    xml.push_str(r#"</cfRule>"#);
                }
                ConditionalRule::DataBar { color } => {
                    xml.push_str(&format!(
                        r#"<cfRule type="dataBar" priority="{}">"#,
                        priority
                    ));
                    xml.push_str(r#"<dataBar>"#);
                    xml.push_str(r#"<cfvo type="min"/>"#);
                    xml.push_str(r#"<cfvo type="max"/>"#);
                    xml.push_str(&format!(r#"<color rgb="FF{}"/>"#, color));
                    xml.push_str(r#"</dataBar>"#);
                    xml.push_str(r#"</cfRule>"#);
                }
                ConditionalRule::IconSet { icon_style } => {
                    xml.push_str(&format!(
                        r#"<cfRule type="iconSet" priority="{}">"#,
                        priority
                    ));
                    xml.push_str(&format!(r#"<iconSet iconSet="{}">"#, escape_xml(icon_style)));
                    xml.push_str(r#"<cfvo type="percent" val="0"/>"#);
                    xml.push_str(r#"<cfvo type="percent" val="33"/>"#);
                    xml.push_str(r#"<cfvo type="percent" val="67"/>"#);
                    xml.push_str(r#"</iconSet>"#);
                    xml.push_str(r#"</cfRule>"#);
                }
                ConditionalRule::Formula {
                    formula,
                    bg_color,
                    font_color,
                    bold,
                } => {
                    xml.push_str(&format!(
                        r#"<cfRule type="expression" dxfId="{}" priority="{}">"#,
                        dxf_id, priority
                    ));
                    xml.push_str(&format!(r#"<formula>{}</formula>"#, escape_xml(formula)));
                    xml.push_str(r#"</cfRule>"#);

                    // Build differential format
                    let mut dxf = String::from("<dxf>");
                    if *bold {
                        dxf.push_str("<font><b/></font>");
                    }
                    if let Some(fc) = font_color {
                        if !*bold {
                            dxf.push_str(&format!(r#"<font><color rgb="FF{}"/></font>"#, fc));
                        } else {
                            dxf.clear();
                            dxf.push_str("<dxf>");
                            dxf.push_str(&format!(r#"<font><b/><color rgb="FF{}"/></font>"#, fc));
                        }
                    }
                    if let Some(bg) = bg_color {
                        dxf.push_str(&format!(
                            r#"<fill><patternFill><bgColor rgb="FF{}"/></patternFill></fill>"#,
                            bg
                        ));
                    }
                    dxf.push_str("</dxf>");
                    dxf_entries.push(dxf);
                    dxf_id += 1;
                }
                ConditionalRule::CellValue {
                    operator,
                    value,
                    bg_color,
                } => {
                    xml.push_str(&format!(
                        r#"<cfRule type="cellIs" dxfId="{}" priority="{}" operator="{}">"#,
                        dxf_id,
                        priority,
                        escape_xml(operator)
                    ));
                    xml.push_str(&format!(r#"<formula>{}</formula>"#, escape_xml(value)));
                    xml.push_str(r#"</cfRule>"#);

                    let mut dxf = String::from("<dxf>");
                    if let Some(bg) = bg_color {
                        dxf.push_str(&format!(
                            r#"<fill><patternFill><bgColor rgb="FF{}"/></patternFill></fill>"#,
                            bg
                        ));
                    }
                    dxf.push_str("</dxf>");
                    dxf_entries.push(dxf);
                    dxf_id += 1;
                }
            }
        }

        xml.push_str(r#"</conditionalFormatting>"#);
    }

    (xml, dxf_entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_scale() {
        let fmts = vec![ConditionalFormat {
            range: "B2:B10".to_string(),
            rules: vec![ConditionalRule::ColorScale {
                min_color: "F8696B".to_string(),
                max_color: "63BE7B".to_string(),
            }],
        }];
        let (xml, dxfs) = generate_conditional_formatting_xml(&fmts, 0);
        assert!(xml.contains("colorScale"));
        assert!(xml.contains("F8696B"));
        assert!(dxfs.is_empty());
    }

    #[test]
    fn test_data_bar() {
        let fmts = vec![ConditionalFormat {
            range: "C2:C10".to_string(),
            rules: vec![ConditionalRule::DataBar {
                color: "638EC6".to_string(),
            }],
        }];
        let (xml, dxfs) = generate_conditional_formatting_xml(&fmts, 0);
        assert!(xml.contains("dataBar"));
        assert!(xml.contains("638EC6"));
        assert!(dxfs.is_empty());
    }

    #[test]
    fn test_icon_set() {
        let fmts = vec![ConditionalFormat {
            range: "D2:D10".to_string(),
            rules: vec![ConditionalRule::IconSet {
                icon_style: "3TrafficLights1".to_string(),
            }],
        }];
        let (xml, _) = generate_conditional_formatting_xml(&fmts, 0);
        assert!(xml.contains("iconSet"));
        assert!(xml.contains("3TrafficLights1"));
    }

    #[test]
    fn test_formula_rule() {
        let fmts = vec![ConditionalFormat {
            range: "A2:A10".to_string(),
            rules: vec![ConditionalRule::Formula {
                formula: "A2>100".to_string(),
                bg_color: Some("00FF00".to_string()),
                font_color: None,
                bold: true,
            }],
        }];
        let (xml, dxfs) = generate_conditional_formatting_xml(&fmts, 0);
        assert!(xml.contains("expression"));
        assert!(xml.contains("A2&gt;100"));
        assert_eq!(dxfs.len(), 1);
        assert!(dxfs[0].contains("<b/>"));
    }

    #[test]
    fn test_cell_value_rule() {
        let fmts = vec![ConditionalFormat {
            range: "B2:B10".to_string(),
            rules: vec![ConditionalRule::CellValue {
                operator: "greaterThan".to_string(),
                value: "50".to_string(),
                bg_color: Some("FFFF00".to_string()),
            }],
        }];
        let (xml, dxfs) = generate_conditional_formatting_xml(&fmts, 0);
        assert!(xml.contains("cellIs"));
        assert!(xml.contains("greaterThan"));
        assert_eq!(dxfs.len(), 1);
    }

    #[test]
    fn test_three_color_scale() {
        let fmts = vec![ConditionalFormat {
            range: "A1:A10".to_string(),
            rules: vec![ConditionalRule::ThreeColorScale {
                min_color: "FF0000".to_string(),
                mid_color: "FFFF00".to_string(),
                max_color: "00FF00".to_string(),
            }],
        }];
        let (xml, dxfs) = generate_conditional_formatting_xml(&fmts, 0);
        assert!(xml.contains("colorScale"));
        assert!(xml.contains("FF0000"));
        assert!(xml.contains("FFFF00"));
        assert!(xml.contains("00FF00"));
        assert!(xml.contains(r#"type="percentile" val="50""#));
        assert!(dxfs.is_empty());
    }

    #[test]
    fn test_empty_formats() {
        let fmts: Vec<ConditionalFormat> = vec![];
        let (xml, dxfs) = generate_conditional_formatting_xml(&fmts, 0);
        assert!(xml.is_empty());
        assert!(dxfs.is_empty());
    }

    #[test]
    fn test_multiple_formats_on_different_ranges() {
        let fmts = vec![
            ConditionalFormat {
                range: "A1:A10".to_string(),
                rules: vec![ConditionalRule::DataBar {
                    color: "4472C4".to_string(),
                }],
            },
            ConditionalFormat {
                range: "B1:B10".to_string(),
                rules: vec![ConditionalRule::ColorScale {
                    min_color: "FF0000".to_string(),
                    max_color: "00FF00".to_string(),
                }],
            },
        ];
        let (xml, _) = generate_conditional_formatting_xml(&fmts, 0);
        assert!(xml.contains(r#"sqref="A1:A10""#));
        assert!(xml.contains(r#"sqref="B1:B10""#));
        assert!(xml.contains("dataBar"));
        assert!(xml.contains("colorScale"));
    }

    #[test]
    fn test_formula_with_font_color_no_bold() {
        let fmts = vec![ConditionalFormat {
            range: "C1:C5".to_string(),
            rules: vec![ConditionalRule::Formula {
                formula: "C1<0".to_string(),
                bg_color: None,
                font_color: Some("FF0000".to_string()),
                bold: false,
            }],
        }];
        let (xml, dxfs) = generate_conditional_formatting_xml(&fmts, 0);
        assert!(xml.contains("expression"));
        assert_eq!(dxfs.len(), 1);
        assert!(dxfs[0].contains("FF0000"));
        assert!(!dxfs[0].contains("<b/>"));
    }

    #[test]
    fn test_formula_bold_with_font_color() {
        let fmts = vec![ConditionalFormat {
            range: "D1:D5".to_string(),
            rules: vec![ConditionalRule::Formula {
                formula: "D1>100".to_string(),
                bg_color: Some("C6EFCE".to_string()),
                font_color: Some("006100".to_string()),
                bold: true,
            }],
        }];
        let (_, dxfs) = generate_conditional_formatting_xml(&fmts, 0);
        assert_eq!(dxfs.len(), 1);
        assert!(dxfs[0].contains("<b/>"));
        assert!(dxfs[0].contains("006100"));
        assert!(dxfs[0].contains("C6EFCE"));
    }

    #[test]
    fn test_cell_value_no_bg_color() {
        let fmts = vec![ConditionalFormat {
            range: "E1:E5".to_string(),
            rules: vec![ConditionalRule::CellValue {
                operator: "lessThan".to_string(),
                value: "0".to_string(),
                bg_color: None,
            }],
        }];
        let (xml, dxfs) = generate_conditional_formatting_xml(&fmts, 0);
        assert!(xml.contains("lessThan"));
        assert_eq!(dxfs.len(), 1);
        assert!(!dxfs[0].contains("bgColor"));
    }

    #[test]
    fn test_dxf_start_id_offset() {
        let fmts = vec![ConditionalFormat {
            range: "A1:A5".to_string(),
            rules: vec![ConditionalRule::Formula {
                formula: "A1>0".to_string(),
                bg_color: None,
                font_color: None,
                bold: false,
            }],
        }];
        let (xml, dxfs) = generate_conditional_formatting_xml(&fmts, 5);
        // dxfId should start at 5
        assert!(xml.contains(r#"dxfId="5""#));
        assert_eq!(dxfs.len(), 1);
    }

    #[test]
    fn test_special_chars_in_range() {
        let fmts = vec![ConditionalFormat {
            range: "Sheet1!A1:B10".to_string(),
            rules: vec![ConditionalRule::DataBar {
                color: "4472C4".to_string(),
            }],
        }];
        let (xml, _) = generate_conditional_formatting_xml(&fmts, 0);
        assert!(xml.contains("Sheet1!A1:B10"));
    }
}
