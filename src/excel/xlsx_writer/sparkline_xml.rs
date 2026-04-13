//! Sparkline XML generation for XLSX files
//!
//! Sparklines are mini charts embedded in cells. They use the x14 extension namespace.

use super::xml_gen::escape_xml;

/// Sparkline type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SparklineType {
    Line,
    Column,
    WinLoss,
}

/// A single sparkline definition
#[derive(Debug, Clone)]
pub struct Sparkline {
    /// Cell where the sparkline is displayed (e.g., "E2")
    pub location: String,
    /// Data range for the sparkline (e.g., "A2:D2")
    pub data_range: String,
}

/// A group of sparklines sharing the same style
#[derive(Debug, Clone)]
pub struct SparklineGroup {
    pub sparkline_type: SparklineType,
    pub sparklines: Vec<Sparkline>,
    /// Color for the sparkline line/bars (hex without #)
    pub color: String,
    /// Whether to show markers on line sparklines
    pub show_markers: bool,
}

impl Default for SparklineGroup {
    fn default() -> Self {
        Self {
            sparkline_type: SparklineType::Line,
            sparklines: Vec::new(),
            color: "4472C4".to_string(),
            show_markers: false,
        }
    }
}

/// Generate sparkline XML as an extLst element to append inside <worksheet>.
/// This uses the x14 extension namespace required by Excel for sparklines.
pub fn generate_sparkline_ext_xml(
    groups: &[SparklineGroup],
    sheet_name: &str,
) -> String {
    if groups.is_empty() {
        return String::new();
    }

    let mut xml = String::with_capacity(1024);
    xml.push_str(r#"<extLst>"#);
    xml.push_str(r#"<ext xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main" uri="{05C60535-1F16-4fd2-B633-F4F36F0B64E0}">"#);
    xml.push_str(r#"<x14:sparklineGroups xmlns:xm="http://schemas.microsoft.com/office/excel/2006/main">"#);

    for group in groups {
        let type_str = match group.sparkline_type {
            SparklineType::Line => "line",
            SparklineType::Column => "column",
            SparklineType::WinLoss => "stacked",
        };

        xml.push_str(&format!(r#"<x14:sparklineGroup type="{}">"#, type_str));
        xml.push_str(&format!(
            r#"<x14:colorSeries rgb="FF{}"/>"#,
            group.color
        ));

        if group.show_markers && group.sparkline_type == SparklineType::Line {
            xml.push_str(r#"<x14:colorMarkers rgb="FFD00000"/>"#);
        }

        xml.push_str(r#"<x14:sparklines>"#);
        for sp in &group.sparklines {
            xml.push_str(r#"<x14:sparkline>"#);
            xml.push_str(&format!(
                r#"<xm:f>'{}'!{}</xm:f>"#,
                escape_xml(sheet_name),
                escape_xml(&sp.data_range)
            ));
            xml.push_str(&format!(
                r#"<xm:sqref>{}</xm:sqref>"#,
                escape_xml(&sp.location)
            ));
            xml.push_str(r#"</x14:sparkline>"#);
        }
        xml.push_str(r#"</x14:sparklines>"#);
        xml.push_str(r#"</x14:sparklineGroup>"#);
    }

    xml.push_str(r#"</x14:sparklineGroups>"#);
    xml.push_str(r#"</ext>"#);
    xml.push_str(r#"</extLst>"#);
    xml
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_groups() {
        let xml = generate_sparkline_ext_xml(&[], "Sheet1");
        assert!(xml.is_empty());
    }

    #[test]
    fn test_line_sparkline() {
        let groups = vec![SparklineGroup {
            sparkline_type: SparklineType::Line,
            sparklines: vec![Sparkline {
                location: "E2".to_string(),
                data_range: "A2:D2".to_string(),
            }],
            color: "4472C4".to_string(),
            show_markers: false,
        }];
        let xml = generate_sparkline_ext_xml(&groups, "Sheet1");
        assert!(xml.contains("x14:sparklineGroup"));
        assert!(xml.contains(r#"type="line""#));
        assert!(xml.contains("E2"));
        assert!(xml.contains("A2:D2"));
    }

    #[test]
    fn test_column_sparkline() {
        let groups = vec![SparklineGroup {
            sparkline_type: SparklineType::Column,
            sparklines: vec![Sparkline {
                location: "F3".to_string(),
                data_range: "B3:E3".to_string(),
            }],
            color: "ED7D31".to_string(),
            show_markers: false,
        }];
        let xml = generate_sparkline_ext_xml(&groups, "Data");
        assert!(xml.contains(r#"type="column""#));
        assert!(xml.contains("ED7D31"));
    }

    #[test]
    fn test_markers() {
        let groups = vec![SparklineGroup {
            sparkline_type: SparklineType::Line,
            sparklines: vec![Sparkline {
                location: "G2".to_string(),
                data_range: "A2:F2".to_string(),
            }],
            color: "4472C4".to_string(),
            show_markers: true,
        }];
        let xml = generate_sparkline_ext_xml(&groups, "Sheet1");
        assert!(xml.contains("colorMarkers"));
    }

    #[test]
    fn test_multiple_sparklines_in_group() {
        let groups = vec![SparklineGroup {
            sparkline_type: SparklineType::Line,
            sparklines: vec![
                Sparkline {
                    location: "E2".to_string(),
                    data_range: "A2:D2".to_string(),
                },
                Sparkline {
                    location: "E3".to_string(),
                    data_range: "A3:D3".to_string(),
                },
            ],
            ..Default::default()
        }];
        let xml = generate_sparkline_ext_xml(&groups, "Sheet1");
        assert!(xml.contains("E2"));
        assert!(xml.contains("E3"));
    }

    #[test]
    fn test_winloss_sparkline() {
        let groups = vec![SparklineGroup {
            sparkline_type: SparklineType::WinLoss,
            sparklines: vec![Sparkline {
                location: "F2".to_string(),
                data_range: "A2:E2".to_string(),
            }],
            color: "70AD47".to_string(),
            show_markers: false,
        }];
        let xml = generate_sparkline_ext_xml(&groups, "Sheet1");
        assert!(xml.contains(r#"type="stacked""#));
        assert!(xml.contains("70AD47"));
    }

    #[test]
    fn test_column_markers_ignored() {
        let groups = vec![SparklineGroup {
            sparkline_type: SparklineType::Column,
            sparklines: vec![Sparkline {
                location: "G2".to_string(),
                data_range: "A2:F2".to_string(),
            }],
            color: "4472C4".to_string(),
            show_markers: true, // markers only apply to line type
        }];
        let xml = generate_sparkline_ext_xml(&groups, "Sheet1");
        assert!(!xml.contains("colorMarkers"));
    }

    #[test]
    fn test_special_chars_in_sheet_name() {
        let groups = vec![SparklineGroup {
            sparkline_type: SparklineType::Line,
            sparklines: vec![Sparkline {
                location: "E2".to_string(),
                data_range: "A2:D2".to_string(),
            }],
            ..Default::default()
        }];
        let xml = generate_sparkline_ext_xml(&groups, "Sales & Data");
        assert!(xml.contains("Sales &amp; Data"));
    }

    #[test]
    fn test_multiple_groups() {
        let groups = vec![
            SparklineGroup {
                sparkline_type: SparklineType::Line,
                sparklines: vec![Sparkline {
                    location: "E2".to_string(),
                    data_range: "A2:D2".to_string(),
                }],
                color: "4472C4".to_string(),
                show_markers: false,
            },
            SparklineGroup {
                sparkline_type: SparklineType::Column,
                sparklines: vec![Sparkline {
                    location: "F2".to_string(),
                    data_range: "A2:D2".to_string(),
                }],
                color: "ED7D31".to_string(),
                show_markers: false,
            },
        ];
        let xml = generate_sparkline_ext_xml(&groups, "Sheet1");
        assert!(xml.contains(r#"type="line""#));
        assert!(xml.contains(r#"type="column""#));
        assert!(xml.contains("4472C4"));
        assert!(xml.contains("ED7D31"));
    }

    #[test]
    fn test_default_group() {
        let group = SparklineGroup::default();
        assert_eq!(group.sparkline_type, SparklineType::Line);
        assert_eq!(group.color, "4472C4");
        assert!(!group.show_markers);
        assert!(group.sparklines.is_empty());
    }
}
