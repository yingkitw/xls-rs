//! Integration tests for advanced Excel features:
//! - Chart generation (all types)
//! - Conditional formatting
//! - Sparklines
//! - CSV injection protection
//! - XlsxWriter direct API

use xls_rs::{
    CellStyle, ChartConfig, ConditionalFormat, ConditionalRule, DataChartType, ExcelHandler,
    RowData, Sparkline, SparklineGroup, SparklineType, StreamingXlsxWriter, XlsxWriter,
    sanitize_csv_value, sanitize_csv_row, CsvHandler,
};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_path(prefix: &str, ext: &str) -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("test_adv_{prefix}_{id}.{ext}")
}

// ============ Chart Integration Tests ============

#[test]
fn test_chart_scatter() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["X".to_string(), "Y".to_string()],
        vec!["1".to_string(), "2".to_string()],
        vec!["3".to_string(), "6".to_string()],
        vec!["5".to_string(), "10".to_string()],
    ];
    let output_path = unique_path("chart_scatter", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Scatter,
        title: Some("Scatter Plot".to_string()),
        category_column: 0,
        value_columns: vec![1],
        ..Default::default()
    };

    handler.write_with_chart(&output_path, &data, &config).unwrap();
    assert!(Path::new(&output_path).exists());

    let content = handler.read_with_sheet(&output_path, None).unwrap();
    assert!(!content.is_empty());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_chart_doughnut() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["Segment".to_string(), "Value".to_string()],
        vec!["A".to_string(), "40".to_string()],
        vec!["B".to_string(), "35".to_string()],
        vec!["C".to_string(), "25".to_string()],
    ];
    let output_path = unique_path("chart_doughnut", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Doughnut,
        title: Some("Doughnut Chart".to_string()),
        category_column: 0,
        value_columns: vec![1],
        ..Default::default()
    };

    handler.write_with_chart(&output_path, &data, &config).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_chart_area() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["Month".to_string(), "Revenue".to_string()],
        vec!["Jan".to_string(), "100".to_string()],
        vec!["Feb".to_string(), "150".to_string()],
        vec!["Mar".to_string(), "200".to_string()],
    ];
    let output_path = unique_path("chart_area", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Area,
        title: Some("Area Chart".to_string()),
        category_column: 0,
        value_columns: vec![1],
        ..Default::default()
    };

    handler.write_with_chart(&output_path, &data, &config).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_chart_multiple_series() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["Q".to_string(), "Sales".to_string(), "Costs".to_string(), "Profit".to_string()],
        vec!["Q1".to_string(), "100".to_string(), "60".to_string(), "40".to_string()],
        vec!["Q2".to_string(), "120".to_string(), "70".to_string(), "50".to_string()],
        vec!["Q3".to_string(), "140".to_string(), "80".to_string(), "60".to_string()],
        vec!["Q4".to_string(), "160".to_string(), "90".to_string(), "70".to_string()],
    ];
    let output_path = unique_path("chart_multi_series", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Column,
        title: Some("Quarterly Performance".to_string()),
        x_axis_title: Some("Quarter".to_string()),
        y_axis_title: Some("Amount".to_string()),
        category_column: 0,
        value_columns: vec![1, 2, 3],
        colors: Some(vec!["4472C4".to_string(), "ED7D31".to_string(), "70AD47".to_string()]),
        ..Default::default()
    };

    handler.write_with_chart(&output_path, &data, &config).unwrap();
    assert!(Path::new(&output_path).exists());

    let content = handler.read_with_sheet(&output_path, None).unwrap();
    assert!(content.contains("Sales"));
    assert!(content.contains("Profit"));

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_add_chart_to_data() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["X".to_string(), "Y".to_string()],
        vec!["A".to_string(), "10".to_string()],
        vec!["B".to_string(), "20".to_string()],
    ];
    let output_path = unique_path("chart_add_to_data", "xlsx");

    let config = ChartConfig::default();
    handler.add_chart_to_data(&data, &config, &output_path).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

// ============ Conditional Formatting Tests ============

#[test]
fn test_xlsx_writer_conditional_color_scale() {
    let output_path = unique_path("cond_color_scale", "xlsx");
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Data").unwrap();

    // Add header + data
    let data = vec![
        vec!["Value".to_string()],
        vec!["10".to_string()],
        vec!["50".to_string()],
        vec!["90".to_string()],
    ];
    writer.add_data(&data);

    writer.add_conditional_format(ConditionalFormat {
        range: "A2:A4".to_string(),
        rules: vec![ConditionalRule::ColorScale {
            min_color: "F8696B".to_string(),
            max_color: "63BE7B".to_string(),
        }],
    });

    let file = fs::File::create(&output_path).unwrap();
    writer.save(std::io::BufWriter::new(file)).unwrap();
    assert!(Path::new(&output_path).exists());

    // Verify readable
    let handler = ExcelHandler::new();
    let content = handler.read_with_sheet(&output_path, None).unwrap();
    assert!(content.contains("Value"));

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_xlsx_writer_conditional_three_color_scale() {
    let output_path = unique_path("cond_3color", "xlsx");
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let data: Vec<Vec<String>> = (0..10)
        .map(|i| vec![format!("{}", i * 10)])
        .collect();
    writer.add_data(&data);

    writer.add_conditional_format(ConditionalFormat {
        range: "A1:A10".to_string(),
        rules: vec![ConditionalRule::ThreeColorScale {
            min_color: "F8696B".to_string(),
            mid_color: "FFEB84".to_string(),
            max_color: "63BE7B".to_string(),
        }],
    });

    let file = fs::File::create(&output_path).unwrap();
    writer.save(std::io::BufWriter::new(file)).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_xlsx_writer_conditional_data_bar() {
    let output_path = unique_path("cond_databar", "xlsx");
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let data = vec![
        vec!["Score".to_string()],
        vec!["25".to_string()],
        vec!["50".to_string()],
        vec!["75".to_string()],
        vec!["100".to_string()],
    ];
    writer.add_data(&data);

    writer.add_conditional_format(ConditionalFormat {
        range: "A2:A5".to_string(),
        rules: vec![ConditionalRule::DataBar {
            color: "638EC6".to_string(),
        }],
    });

    let file = fs::File::create(&output_path).unwrap();
    writer.save(std::io::BufWriter::new(file)).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_xlsx_writer_conditional_icon_set() {
    let output_path = unique_path("cond_iconset", "xlsx");
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let data = vec![
        vec!["Rating".to_string()],
        vec!["10".to_string()],
        vec!["50".to_string()],
        vec!["90".to_string()],
    ];
    writer.add_data(&data);

    writer.add_conditional_format(ConditionalFormat {
        range: "A2:A4".to_string(),
        rules: vec![ConditionalRule::IconSet {
            icon_style: "3TrafficLights1".to_string(),
        }],
    });

    let file = fs::File::create(&output_path).unwrap();
    writer.save(std::io::BufWriter::new(file)).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_xlsx_writer_conditional_formula() {
    let output_path = unique_path("cond_formula", "xlsx");
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let data = vec![
        vec!["Name".to_string(), "Score".to_string()],
        vec!["Alice".to_string(), "85".to_string()],
        vec!["Bob".to_string(), "42".to_string()],
        vec!["Carol".to_string(), "91".to_string()],
    ];
    writer.add_data(&data);

    writer.add_conditional_format(ConditionalFormat {
        range: "B2:B4".to_string(),
        rules: vec![ConditionalRule::Formula {
            formula: "B2>80".to_string(),
            bg_color: Some("C6EFCE".to_string()),
            font_color: Some("006100".to_string()),
            bold: true,
        }],
    });

    let file = fs::File::create(&output_path).unwrap();
    writer.save(std::io::BufWriter::new(file)).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_xlsx_writer_conditional_cell_value() {
    let output_path = unique_path("cond_cellvalue", "xlsx");
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let data = vec![
        vec!["Value".to_string()],
        vec!["10".to_string()],
        vec!["60".to_string()],
        vec!["30".to_string()],
    ];
    writer.add_data(&data);

    writer.add_conditional_format(ConditionalFormat {
        range: "A2:A4".to_string(),
        rules: vec![ConditionalRule::CellValue {
            operator: "greaterThan".to_string(),
            value: "50".to_string(),
            bg_color: Some("FFFF00".to_string()),
        }],
    });

    let file = fs::File::create(&output_path).unwrap();
    writer.save(std::io::BufWriter::new(file)).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_xlsx_writer_multiple_conditional_rules() {
    let output_path = unique_path("cond_multi", "xlsx");
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let data = vec![
        vec!["Value".to_string()],
        vec!["10".to_string()],
        vec!["50".to_string()],
        vec!["90".to_string()],
    ];
    writer.add_data(&data);

    // Color scale on same range
    writer.add_conditional_format(ConditionalFormat {
        range: "A2:A4".to_string(),
        rules: vec![
            ConditionalRule::ColorScale {
                min_color: "FF0000".to_string(),
                max_color: "00FF00".to_string(),
            },
            ConditionalRule::DataBar {
                color: "4472C4".to_string(),
            },
        ],
    });

    let file = fs::File::create(&output_path).unwrap();
    writer.save(std::io::BufWriter::new(file)).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

// ============ Sparkline Tests ============

#[test]
fn test_xlsx_writer_sparkline_line() {
    let output_path = unique_path("sparkline_line", "xlsx");
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let data = vec![
        vec!["Q1".to_string(), "Q2".to_string(), "Q3".to_string(), "Q4".to_string(), "Trend".to_string()],
        vec!["10".to_string(), "20".to_string(), "15".to_string(), "25".to_string(), "".to_string()],
        vec!["30".to_string(), "25".to_string(), "35".to_string(), "40".to_string(), "".to_string()],
    ];
    writer.add_data(&data);

    writer.add_sparkline_group(SparklineGroup {
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
        color: "4472C4".to_string(),
        show_markers: false,
    });

    let file = fs::File::create(&output_path).unwrap();
    writer.save(std::io::BufWriter::new(file)).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_xlsx_writer_sparkline_column() {
    let output_path = unique_path("sparkline_col", "xlsx");
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Data").unwrap();

    let data = vec![
        vec!["A".to_string(), "B".to_string(), "C".to_string(), "Spark".to_string()],
        vec!["5".to_string(), "10".to_string(), "15".to_string(), "".to_string()],
    ];
    writer.add_data(&data);

    writer.add_sparkline_group(SparklineGroup {
        sparkline_type: SparklineType::Column,
        sparklines: vec![Sparkline {
            location: "D2".to_string(),
            data_range: "A2:C2".to_string(),
        }],
        color: "ED7D31".to_string(),
        show_markers: false,
    });

    let file = fs::File::create(&output_path).unwrap();
    writer.save(std::io::BufWriter::new(file)).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_xlsx_writer_sparkline_with_markers() {
    let output_path = unique_path("sparkline_markers", "xlsx");
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let data = vec![
        vec!["1".to_string(), "3".to_string(), "2".to_string(), "5".to_string(), "".to_string()],
    ];
    writer.add_data(&data);

    writer.add_sparkline_group(SparklineGroup {
        sparkline_type: SparklineType::Line,
        sparklines: vec![Sparkline {
            location: "E1".to_string(),
            data_range: "A1:D1".to_string(),
        }],
        color: "4472C4".to_string(),
        show_markers: true,
    });

    let file = fs::File::create(&output_path).unwrap();
    writer.save(std::io::BufWriter::new(file)).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

// ============ CSV Injection Protection Tests ============

#[test]
fn test_sanitize_csv_value_formula_injection() {
    assert_eq!(sanitize_csv_value("=CMD()"), "'=CMD()");
    assert_eq!(sanitize_csv_value("+CMD()"), "'+CMD()");
    assert_eq!(sanitize_csv_value("-CMD()"), "'-CMD()");
    assert_eq!(sanitize_csv_value("@SUM(A1)"), "'@SUM(A1)");
}

#[test]
fn test_sanitize_csv_value_safe_values() {
    assert_eq!(sanitize_csv_value("Hello"), "Hello");
    assert_eq!(sanitize_csv_value("123"), "123");
    assert_eq!(sanitize_csv_value(""), "");
    assert_eq!(sanitize_csv_value("Normal text"), "Normal text");
}

#[test]
fn test_sanitize_csv_value_tab_newline() {
    assert_eq!(sanitize_csv_value("\tdata"), "'\tdata");
    assert_eq!(sanitize_csv_value("\rdata"), "'\rdata");
    assert_eq!(sanitize_csv_value("\ndata"), "'\ndata");
}

#[test]
fn test_sanitize_csv_row() {
    let row = vec![
        "Name".to_string(),
        "=HYPERLINK(\"evil\")".to_string(),
        "100".to_string(),
    ];
    let sanitized = sanitize_csv_row(&row);
    assert_eq!(sanitized[0], "Name");
    assert_eq!(sanitized[1], "'=HYPERLINK(\"evil\")");
    assert_eq!(sanitized[2], "100");
}

#[test]
fn test_csv_write_records_safe() {
    let handler = CsvHandler;
    let output_path = unique_path("csv_safe", "csv");

    let records = vec![
        vec!["Name".to_string(), "Formula".to_string()],
        vec!["Alice".to_string(), "=1+1".to_string()],
        vec!["Bob".to_string(), "+cmd".to_string()],
    ];

    handler.write_records_safe(&output_path, records).unwrap();
    assert!(Path::new(&output_path).exists());

    // Read back and verify sanitization
    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("'=1+1"));
    assert!(content.contains("'+cmd"));
    assert!(!content.contains(",=1+1")); // Should be sanitized

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_csv_append_records_safe() {
    let handler = CsvHandler;
    let output_path = unique_path("csv_append_safe", "csv");

    // Write initial data
    handler.write_records(&output_path, vec![
        vec!["Header".to_string()],
    ]).unwrap();

    // Append with injection protection
    handler.append_records_safe(&output_path, &[
        vec!["=EVIL()".to_string()],
        vec!["Safe".to_string()],
    ]).unwrap();

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("'=EVIL()"));
    assert!(content.contains("Safe"));

    fs::remove_file(&output_path).ok();
}

// ============ Combined Features Test ============

#[test]
fn test_xlsx_writer_chart_with_conditional_formatting() {
    let output_path = unique_path("chart_cond_fmt", "xlsx");
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Dashboard").unwrap();

    let data = vec![
        vec!["Product".to_string(), "Sales".to_string()],
        vec!["Widget A".to_string(), "150".to_string()],
        vec!["Widget B".to_string(), "80".to_string()],
        vec!["Widget C".to_string(), "200".to_string()],
    ];
    writer.add_data(&data);

    // Add conditional formatting
    writer.add_conditional_format(ConditionalFormat {
        range: "B2:B4".to_string(),
        rules: vec![ConditionalRule::DataBar {
            color: "5B9BD5".to_string(),
        }],
    });

    // Add chart
    let chart_config = ChartConfig {
        chart_type: DataChartType::Column,
        title: Some("Sales Dashboard".to_string()),
        category_column: 0,
        value_columns: vec![1],
        ..Default::default()
    };
    writer.set_chart(chart_config, data.clone());

    let file = fs::File::create(&output_path).unwrap();
    writer.save(std::io::BufWriter::new(file)).unwrap();
    assert!(Path::new(&output_path).exists());

    // Verify data readable
    let handler = ExcelHandler::new();
    let content = handler.read_with_sheet(&output_path, None).unwrap();
    assert!(content.contains("Product"));
    assert!(content.contains("Widget A"));

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_xlsx_writer_all_features_combined() {
    let output_path = unique_path("all_features", "xlsx");
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Report").unwrap();

    let data = vec![
        vec!["Month".to_string(), "Revenue".to_string(), "Cost".to_string(), "Profit".to_string(), "Trend".to_string()],
        vec!["Jan".to_string(), "100".to_string(), "60".to_string(), "40".to_string(), "".to_string()],
        vec!["Feb".to_string(), "120".to_string(), "70".to_string(), "50".to_string(), "".to_string()],
        vec!["Mar".to_string(), "140".to_string(), "80".to_string(), "60".to_string(), "".to_string()],
        vec!["Apr".to_string(), "160".to_string(), "90".to_string(), "70".to_string(), "".to_string()],
    ];
    writer.add_data(&data);

    // Conditional formatting on Revenue
    writer.add_conditional_format(ConditionalFormat {
        range: "B2:B5".to_string(),
        rules: vec![ConditionalRule::ColorScale {
            min_color: "F8696B".to_string(),
            max_color: "63BE7B".to_string(),
        }],
    });

    // Data bars on Profit
    writer.add_conditional_format(ConditionalFormat {
        range: "D2:D5".to_string(),
        rules: vec![ConditionalRule::DataBar {
            color: "70AD47".to_string(),
        }],
    });

    // Sparklines for trend
    writer.add_sparkline_group(SparklineGroup {
        sparkline_type: SparklineType::Line,
        sparklines: vec![
            Sparkline { location: "E2".to_string(), data_range: "B2:D2".to_string() },
            Sparkline { location: "E3".to_string(), data_range: "B3:D3".to_string() },
            Sparkline { location: "E4".to_string(), data_range: "B4:D4".to_string() },
            Sparkline { location: "E5".to_string(), data_range: "B5:D5".to_string() },
        ],
        color: "4472C4".to_string(),
        show_markers: true,
    });

    // Chart
    let chart_config = ChartConfig {
        chart_type: DataChartType::Line,
        title: Some("Monthly Report".to_string()),
        x_axis_title: Some("Month".to_string()),
        y_axis_title: Some("Amount".to_string()),
        category_column: 0,
        value_columns: vec![1, 2, 3],
        colors: Some(vec!["4472C4".to_string(), "ED7D31".to_string(), "70AD47".to_string()]),
        show_legend: true,
        ..Default::default()
    };
    writer.set_chart(chart_config, data.clone());

    let file = fs::File::create(&output_path).unwrap();
    writer.save(std::io::BufWriter::new(file)).unwrap();
    assert!(Path::new(&output_path).exists());

    // Verify file size is reasonable (should be > 1KB with all features)
    let metadata = fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 1000, "File too small: {} bytes", metadata.len());

    // Verify data readable
    let handler = ExcelHandler::new();
    let content = handler.read_with_sheet(&output_path, None).unwrap();
    assert!(content.contains("Month"));
    assert!(content.contains("Revenue"));

    fs::remove_file(&output_path).ok();
}

// ============ Streaming XLSX Integration Tests ============

#[test]
fn test_streaming_xlsx_readback() {
    let output_path = unique_path("streaming_readback", "xlsx");
    let mut writer = StreamingXlsxWriter::create(&output_path, "Data").unwrap();

    writer.write_row(&["Name".to_string(), "Score".to_string()]).unwrap();
    writer.write_row(&["Alice".to_string(), "95".to_string()]).unwrap();
    writer.write_row(&["Bob".to_string(), "87".to_string()]).unwrap();
    writer.finish().unwrap();

    // Read back and verify content
    let handler = ExcelHandler::new();
    let content = handler.read_with_sheet(&output_path, None).unwrap();
    assert!(content.contains("Name"));
    assert!(content.contains("Alice"));
    assert!(content.contains("Bob"));

    // Verify sheet name
    let sheets = handler.list_sheets(&output_path).unwrap();
    assert!(sheets.contains(&"Data".to_string()));

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_streaming_xlsx_numbers_detected() {
    let output_path = unique_path("streaming_numbers", "xlsx");
    let mut writer = StreamingXlsxWriter::create(&output_path, "Sheet1").unwrap();

    writer.write_row(&["ID".to_string(), "Value".to_string()]).unwrap();
    writer.write_row(&["1".to_string(), "99.5".to_string()]).unwrap();
    writer.write_row(&["2".to_string(), "0".to_string()]).unwrap();
    writer.finish().unwrap();

    let handler = ExcelHandler::new();
    let content = handler.read_with_sheet(&output_path, None).unwrap();
    assert!(content.contains("99.5"));

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_streaming_xlsx_row_data_api() {
    let output_path = unique_path("streaming_rowdata", "xlsx");
    let mut writer = StreamingXlsxWriter::create(&output_path, "Sheet1").unwrap();

    let mut row = RowData::new();
    row.add_string("Header");
    row.add_number(42.0);
    writer.write_row_data(row).unwrap();

    assert_eq!(writer.rows_written(), 1);
    writer.finish().unwrap();

    assert!(Path::new(&output_path).exists());
    fs::remove_file(&output_path).ok();
}

// ============ ExcelHandler Sparkline/CondFmt Method Tests ============

#[test]
fn test_excel_handler_add_sparkline_formula() {
    let handler = ExcelHandler::new();
    let output_path = unique_path("handler_sparkline", "xlsx");

    handler
        .add_sparkline_formula(&output_path, "A2:D2", "E2", None)
        .unwrap();

    assert!(Path::new(&output_path).exists());
    fs::remove_file(&output_path).ok();
}

#[test]
fn test_excel_handler_apply_conditional_format() {
    let handler = ExcelHandler::new();
    let output_path = unique_path("handler_condfmt", "xlsx");

    let style = CellStyle {
        bold: true,
        bg_color: Some("C6EFCE".to_string()),
        font_color: Some("006100".to_string()),
        ..Default::default()
    };

    handler
        .apply_conditional_format_formula(&output_path, "B2:B10", "B2>50", &style, None, None)
        .unwrap();

    assert!(Path::new(&output_path).exists());
    fs::remove_file(&output_path).ok();
}

// ============ Edge Case Tests ============

#[test]
fn test_chart_with_header_only_data() {
    let handler = ExcelHandler::new();
    let data: Vec<Vec<String>> = vec![
        vec!["X".to_string(), "Y".to_string()],
    ];
    let output_path = unique_path("chart_header_only", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Column,
        title: Some("Empty".to_string()),
        ..Default::default()
    };

    handler.write_with_chart(&output_path, &data, &config).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_chart_with_single_data_row() {
    let handler = ExcelHandler::new();
    let data = vec![
        vec!["Cat".to_string(), "Val".to_string()],
        vec!["Only".to_string(), "42".to_string()],
    ];
    let output_path = unique_path("chart_single_row", "xlsx");

    let config = ChartConfig {
        chart_type: DataChartType::Pie,
        ..Default::default()
    };

    handler.write_with_chart(&output_path, &data, &config).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_sparkline_winloss_type() {
    let output_path = unique_path("sparkline_winloss", "xlsx");
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    let data = vec![
        vec!["1".to_string(), "-1".to_string(), "1".to_string(), "-1".to_string(), "".to_string()],
    ];
    writer.add_data(&data);

    writer.add_sparkline_group(SparklineGroup {
        sparkline_type: SparklineType::WinLoss,
        sparklines: vec![Sparkline {
            location: "E1".to_string(),
            data_range: "A1:D1".to_string(),
        }],
        color: "70AD47".to_string(),
        show_markers: false,
    });

    let file = fs::File::create(&output_path).unwrap();
    writer.save(std::io::BufWriter::new(file)).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_conditional_format_on_empty_sheet() {
    let output_path = unique_path("cond_empty_sheet", "xlsx");
    let mut writer = XlsxWriter::new();
    writer.add_sheet("Sheet1").unwrap();

    writer.add_conditional_format(ConditionalFormat {
        range: "A1:A10".to_string(),
        rules: vec![ConditionalRule::DataBar {
            color: "4472C4".to_string(),
        }],
    });

    let file = fs::File::create(&output_path).unwrap();
    writer.save(std::io::BufWriter::new(file)).unwrap();
    assert!(Path::new(&output_path).exists());

    fs::remove_file(&output_path).ok();
}

#[test]
fn test_csv_sanitize_negative_numbers() {
    // Negative numbers start with '-' which is a dangerous char
    let result = sanitize_csv_value("-42");
    assert_eq!(result, "'-42");

    // Positive numbers are safe
    assert_eq!(sanitize_csv_value("42"), "42");
}

#[test]
fn test_csv_sanitize_mixed_row_comprehensive() {
    let row = vec![
        "Safe".to_string(),
        "=HYPERLINK(\"http://evil.com\")".to_string(),
        "+1234567890".to_string(),
        "@import".to_string(),
        "100".to_string(),
        "".to_string(),
    ];
    let sanitized = sanitize_csv_row(&row);
    assert_eq!(sanitized[0], "Safe");
    assert!(sanitized[1].starts_with("'="));
    assert!(sanitized[2].starts_with("'+"));
    assert!(sanitized[3].starts_with("'@"));
    assert_eq!(sanitized[4], "100");
    assert_eq!(sanitized[5], "");
}
