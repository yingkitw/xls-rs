//! Generate comprehensive XLSX files demonstrating all writer features:
//! formulas, merged cells, freeze panes, auto-filter, styled cells,
//! conditional formatting, data bars, color scales, icon sets,
//! sparklines, data validation, hyperlinks, cell comments, print setup,
//! row/column outlines, and embedded charts.
//!
//! Run with: `cargo run --example write_rich_xls`

use xls_rs::excel::chart::{ChartConfig, DataChartType};
use xls_rs::excel::{
    ConditionalFormat, ConditionalRule, DataValidation, Operator, PageMargins,
    PageOrientation, PrintSetup, RowData, Sparkline, SparklineGroup, SparklineType,
    ValidationType, WriteOptions, XlsxCellStyle, XlsxWriter,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("output");
    std::fs::create_dir_all(&out_dir)?;

    // ── Example 1: Sales report with formulas, freeze panes, auto-filter ──
    {
        let options = WriteOptions {
            freeze_header: true,
            auto_filter: true,
            ..Default::default()
        };
        let mut w = XlsxWriter::with_options(options);
        w.add_sheet("Sales")?;

        let mut hdr = RowData::new();
        hdr.add_string("Product");
        hdr.add_string("Category");
        hdr.add_string("Price");
        hdr.add_string("Quantity");
        hdr.add_string("Revenue");
        w.add_row(hdr);

        let products: &[(&str, &str, f64, f64)] = &[
            ("Laptop", "Electronics", 1200.0, 1.0),
            ("Mouse", "Electronics", 25.0, 2.0),
            ("Desk", "Furniture", 300.0, 1.0),
            ("Chair", "Furniture", 150.0, 4.0),
            ("Pen", "Stationery", 2.0, 10.0),
            ("Lamp", "Home", 45.0, 1.0),
        ];

        for (i, (name, cat, price, qty)) in products.iter().enumerate() {
            let mut row = RowData::new();
            row.add_string(name);
            row.add_string(cat);
            row.add_number(*price);
            row.add_number(*qty);
            let row_num = i + 2;
            row.add_formula(format!("C{}*D{}", row_num, row_num));
            w.add_row(row);
        }

        let mut totals = RowData::new();
        totals.add_string("Total");
        totals.add_empty();
        totals.add_empty();
        totals.add_empty();
        totals.add_formula("SUM(E2:E7)");
        w.add_row(totals);

        w.set_column_width(0, 12.0);
        w.set_column_width(1, 14.0);

        let path = out_dir.join("sales_rich.xlsx");
        w.save(std::fs::File::create(&path)?)?;
        println!("Created {}", path.display());
    }

    // ── Example 2: Employee report with merged cells ──
    {
        let mut w = XlsxWriter::new();
        w.add_sheet("Employees")?;

        let mut title = RowData::new();
        title.add_string("Employee Directory");
        w.add_row(title);
        w.add_merge_cell(0, 0, 0, 3);

        let mut hdr = RowData::new();
        hdr.add_string("ID");
        hdr.add_string("Name");
        hdr.add_string("Department");
        hdr.add_string("Salary");
        w.add_row(hdr);

        let employees: &[(f64, &str, &str, f64)] = &[
            (1.0, "Alice Johnson", "Engineering", 85000.0),
            (2.0, "Bob Smith", "Sales", 65000.0),
            (3.0, "Carol Davis", "Engineering", 92000.0),
            (4.0, "Dan Miller", "Marketing", 72000.0),
            (6.0, "Grace Anderson", "Engineering", 81000.0),
            (7.0, "Henry Wilson", "Engineering", 95000.0),
        ];

        for (id, name, dept, salary) in employees {
            let mut row = RowData::new();
            row.add_number(*id);
            row.add_string(name);
            row.add_string(dept);
            row.add_number(*salary);
            w.add_row(row);
        }

        let mut summary = RowData::new();
        summary.add_empty();
        summary.add_string("Average Salary");
        summary.add_empty();
        summary.add_formula("AVERAGE(D3:D8)");
        w.add_row(summary);

        let mut err_row = RowData::new();
        err_row.add_empty();
        err_row.add_string("Missing Entry");
        err_row.add_string("#N/A");
        err_row.add_string("#VALUE!");
        w.add_row(err_row);

        w.set_column_width(1, 18.0);
        w.set_column_width(2, 14.0);

        let path = out_dir.join("employees_rich.xlsx");
        w.save(std::fs::File::create(&path)?)?;
        println!("Created {}", path.display());
    }

    // ── Example 3: Multi-sheet workbook with formulas and booleans ──
    {
        let mut w = XlsxWriter::new();

        w.add_sheet("Budget")?;
        let mut hdr = RowData::new();
        hdr.add_string("Item");
        hdr.add_string("Budgeted");
        hdr.add_string("Actual");
        hdr.add_string("Status");
        w.add_row(hdr);

        let budget: &[(&str, f64, f64)] = &[
            ("Rent", 2000.0, 2000.0),
            ("Food", 500.0, 620.0),
            ("Transport", 300.0, 280.0),
            ("Entertainment", 200.0, 350.0),
        ];

        for (i, (item, budgeted, actual)) in budget.iter().enumerate() {
            let mut row = RowData::new();
            row.add_string(item);
            row.add_number(*budgeted);
            row.add_number(*actual);
            let row_num = i + 2;
            row.add_formula(format!("IF(C{}<=B{},\"OK\",\"OVER\")", row_num, row_num));
            w.add_row(row);
        }

        w.add_sheet("Summary")?;
        let mut r = RowData::new();
        r.add_string("Total Budget");
        r.add_formula("SUM(Budget!B2:B5)");
        w.add_row(r);

        let mut r2 = RowData::new();
        r2.add_string("Total Actual");
        r2.add_formula("SUM(Budget!C2:C5)");
        w.add_row(r2);

        let mut r3 = RowData::new();
        r3.add_string("Over Budget?");
        r3.add_bool(true);
        w.add_row(r3);

        let path = out_dir.join("budget_rich.xlsx");
        w.save(std::fs::File::create(&path)?)?;
        println!("Created {}", path.display());
    }

    // ── Example 4: Data with VLOOKUP formula ──
    {
        let options = WriteOptions {
            freeze_header: true,
            auto_filter: true,
            ..Default::default()
        };
        let mut w = XlsxWriter::with_options(options);
        w.add_sheet("Lookup")?;

        let mut hdr = RowData::new();
        hdr.add_string("Code");
        hdr.add_string("Name");
        hdr.add_string("Price");
        w.add_row(hdr);

        let items: &[(&str, &str, f64)] = &[
            ("W", "Widget", 10.0),
            ("G", "Gadget", 25.0),
            ("S", "Sprocket", 15.0),
            ("D", "Doohickey", 50.0),
        ];

        for (code, name, price) in items {
            let mut row = RowData::new();
            row.add_string(code);
            row.add_string(name);
            row.add_number(*price);
            w.add_row(row);
        }

        let queries = ["W", "G", "X"];
        for (i, q) in queries.iter().enumerate() {
            let mut row = RowData::new();
            row.add_string(q);
            let row_num = i + 6;
            row.add_formula(format!("VLOOKUP(A{},A2:C5,2,FALSE)", row_num));
            row.add_formula(format!("VLOOKUP(A{},A2:C5,3,FALSE)", row_num));
            w.add_row(row);
        }

        let path = out_dir.join("lookup_rich.xlsx");
        w.save(std::fs::File::create(&path)?)?;
        println!("Created {}", path.display());
    }

    // ── Example 5: Styled cells with number formats, colors, borders ──
    {
        let mut w = XlsxWriter::new();
        w.add_sheet("StyledReport")?;

        // Register styles
        let header_style = XlsxCellStyle {
            bold: Some(true),
            font_color: Some("FFFFFF".to_string()),
            fill_color: Some("4472C4".to_string()),
            align: Some("center".to_string()),
            border: Some("thin".to_string()),
            border_color: Some("000000".to_string()),
            ..Default::default()
        };
        let header_idx = w.register_cell_style(&header_style);

        let money_style = XlsxCellStyle {
            number_format: Some("#,##0.00".to_string()),
            align: Some("right".to_string()),
            ..Default::default()
        };
        let money_idx = w.register_cell_style(&money_style);

        let percent_style = XlsxCellStyle {
            number_format: Some("0.00%".to_string()),
            align: Some("right".to_string()),
            ..Default::default()
        };
        let percent_idx = w.register_cell_style(&percent_style);

        let date_style = XlsxCellStyle {
            number_format: Some("yyyy-mm-dd".to_string()),
            ..Default::default()
        };
        let date_idx = w.register_cell_style(&date_style);

        let warning_style = XlsxCellStyle {
            bold: Some(true),
            font_color: Some("FF0000".to_string()),
            fill_color: Some("FFF2CC".to_string()),
            ..Default::default()
        };
        let warning_idx = w.register_cell_style(&warning_style);

        // Header row with style
        let mut hdr = RowData::new();
        hdr.add_string("Date");
        hdr.add_string("Description");
        hdr.add_string("Amount");
        hdr.add_string("Tax Rate");
        hdr.add_string("Status");
        for c in 0..5 {
            hdr.set_cell_style(c, header_idx);
        }
        w.add_row(hdr);

        // Data rows with per-cell styles
        let transactions: &[(&str, &str, f64, f64, &str)] = &[
            ("2024-01-15", "Office supplies", 1250.50, 0.0875, "Approved"),
            ("2024-02-03", "Software license", 4800.00, 0.0875, "Approved"),
            ("2024-02-20", "Travel expense", 3200.75, 0.0875, "Pending"),
            ("2024-03-01", "Consulting fee", 15000.00, 0.0, "Approved"),
            ("2024-03-10", "Equipment", 8750.25, 0.0875, "Rejected"),
        ];

        for (date, desc, amount, tax, status) in transactions {
            let mut row = RowData::new();
            row.add_string(date);
            row.style_last(date_idx);
            row.add_string(desc);
            row.add_number(*amount);
            row.style_last(money_idx);
            row.add_number(*tax);
            row.style_last(percent_idx);
            row.add_string(status);
            if *status == "Pending" || *status == "Rejected" {
                row.style_last(warning_idx);
            }
            w.add_row(row);
        }

        // Totals
        let mut total = RowData::new();
        total.add_empty();
        total.add_string("Total");
        total.add_formula("SUM(C2:C6)");
        total.style_last(money_idx);
        total.add_empty();
        total.add_empty();
        w.add_row(total);

        w.set_column_width(0, 12.0);
        w.set_column_width(1, 20.0);
        w.set_column_width(2, 14.0);
        w.set_column_width(3, 12.0);
        w.set_column_width(4, 12.0);

        let path = out_dir.join("styled_report.xlsx");
        w.save(std::fs::File::create(&path)?)?;
        println!("Created {}", path.display());
    }

    // ── Example 6: Conditional formatting (color scale, data bars, icon set) ──
    {
        let mut w = XlsxWriter::new();
        w.add_sheet("CondFormat")?;

        let mut hdr = RowData::new();
        hdr.add_string("Salesperson");
        hdr.add_string("Q1");
        hdr.add_string("Q2");
        hdr.add_string("Q3");
        hdr.add_string("Q4");
        hdr.add_string("Total");
        w.add_row(hdr);

        let sales: &[(&str, f64, f64, f64, f64)] = &[
            ("Alice", 45.0, 52.0, 38.0, 61.0),
            ("Bob", 28.0, 35.0, 42.0, 30.0),
            ("Carol", 55.0, 48.0, 62.0, 58.0),
            ("Dan", 18.0, 22.0, 15.0, 25.0),
            ("Eve", 40.0, 45.0, 50.0, 55.0),
            ("Frank", 33.0, 28.0, 35.0, 42.0),
        ];

        for (name, q1, q2, q3, q4) in sales {
            let mut row = RowData::new();
            row.add_string(name);
            row.add_number(*q1);
            row.add_number(*q2);
            row.add_number(*q3);
            row.add_number(*q4);
            row.add_formula(format!("SUM(B{}:E{})", sales.len() - sales.iter().position(|s| s.0 == *name).unwrap() + 1, sales.len() - sales.iter().position(|s| s.0 == *name).unwrap() + 1));
            w.add_row(row);
        }

        // Color scale on Q1-Q4 data (B2:E7)
        w.add_conditional_format(ConditionalFormat {
            range: "B2:E7".to_string(),
            rules: vec![ConditionalRule::ThreeColorScale {
                min_color: "F8696B".to_string(),
                mid_color: "FFEB84".to_string(),
                max_color: "63BE7B".to_string(),
            }],
        });

        // Data bar on Total column (F2:F7)
        w.add_conditional_format(ConditionalFormat {
            range: "F2:F7".to_string(),
            rules: vec![ConditionalRule::DataBar {
                color: "5B9BD5".to_string(),
            }],
        });

        // Icon set on Q1 column
        w.add_conditional_format(ConditionalFormat {
            range: "B2:B7".to_string(),
            rules: vec![ConditionalRule::IconSet {
                icon_style: "3Arrows".to_string(),
            }],
        });

        // Formula-based: highlight rows where total > 150
        w.add_conditional_format(ConditionalFormat {
            range: "A2:F7".to_string(),
            rules: vec![ConditionalRule::Formula {
                formula: "$F2>150".to_string(),
                bg_color: Some("C6EFCE".to_string()),
                font_color: Some("006100".to_string()),
                bold: true,
            }],
        });

        w.set_column_width(0, 14.0);

        let path = out_dir.join("conditional_format.xlsx");
        w.save(std::fs::File::create(&path)?)?;
        println!("Created {}", path.display());
    }

    // ── Example 7: Sparklines, data validation, hyperlinks, comments ──
    {
        let mut w = XlsxWriter::new();
        w.add_sheet("Dashboard")?;

        let mut hdr = RowData::new();
        hdr.add_string("Region");
        hdr.add_string("Jan");
        hdr.add_string("Feb");
        hdr.add_string("Mar");
        hdr.add_string("Apr");
        hdr.add_string("May");
        hdr.add_string("Jun");
        hdr.add_string("Trend");
        hdr.add_string("Link");
        w.add_row(hdr);

        let regions: &[(&str, f64, f64, f64, f64, f64, f64)] = &[
            ("North", 120.0, 135.0, 140.0, 155.0, 160.0, 170.0),
            ("South", 90.0, 85.0, 95.0, 100.0, 110.0, 105.0),
            ("East", 200.0, 195.0, 210.0, 220.0, 230.0, 240.0),
            ("West", 75.0, 80.0, 85.0, 90.0, 88.0, 95.0),
        ];

        let mut sparklines = Vec::new();
        for (i, (name, j, f, m, a, my, ju)) in regions.iter().enumerate() {
            let row_num = i + 2; // 1-based, header is row 1
            let mut row = RowData::new();
            row.add_string(name);
            row.add_number(*j);
            row.add_number(*f);
            row.add_number(*m);
            row.add_number(*a);
            row.add_number(*my);
            row.add_number(*ju);
            row.add_empty(); // sparkline cell
            row.add_string("Details");
            w.add_row(row);

            sparklines.push(Sparkline {
                location: format!("H{row_num}"),
                data_range: format!("B{row_num}:G{row_num}"),
            });
        }

        // Add sparkline group (line type with markers)
        w.add_sparkline_group(SparklineGroup {
            sparkline_type: SparklineType::Line,
            sparklines,
            color: "4472C4".to_string(),
            show_markers: true,
        });

        // Data validation: restrict region names
        w.add_data_validation(DataValidation {
            range: "A2:A10".to_string(),
            validation_type: ValidationType::List {
                source: "North,South,East,West,Central".to_string(),
            },
            allow_blank: true,
            show_dropdown: true,
            error_title: Some("Invalid region".to_string()),
            error_message: Some("Please select a valid region from the dropdown.".to_string()),
        });

        // Data validation: numbers must be between 0 and 500
        w.add_data_validation(DataValidation {
            range: "B2:G10".to_string(),
            validation_type: ValidationType::Decimal {
                operator: Operator::Between,
                formula1: "0".to_string(),
                formula2: Some("500".to_string()),
            },
            allow_blank: true,
            show_dropdown: true,
            error_title: Some("Out of range".to_string()),
            error_message: Some("Values must be between 0 and 500.".to_string()),
        });

        // Hyperlinks
        w.add_hyperlink("I2", "https://example.com/north", Some("North region details"));
        w.add_hyperlink("I3", "https://example.com/south", Some("South region details"));
        w.add_hyperlink("I4", "https://example.com/east", Some("East region details"));
        w.add_hyperlink("I5", "https://example.com/west", Some("West region details"));

        // Cell comments
        w.add_comment("B2", "Strong start to the year", Some("Analyst"));
        w.add_comment("F4", "Peak season boost", Some("Manager"));

        w.set_column_width(0, 10.0);
        w.set_column_width(7, 12.0);
        w.set_column_width(8, 10.0);

        let path = out_dir.join("dashboard.xlsx");
        w.save(std::fs::File::create(&path)?)?;
        println!("Created {}", path.display());
    }

    // ── Example 8: Row/column outlines + print setup ──
    {
        let mut w = XlsxWriter::new();
        w.add_sheet("Financials")?;

        // Title
        let mut title = RowData::new();
        title.add_string("Q1 2024 Financial Summary");
        w.add_row(title);
        w.add_merge_cell(0, 0, 0, 5);

        // Header
        let mut hdr = RowData::new();
        hdr.add_string("Category");
        hdr.add_string("Sub-category");
        hdr.add_string("Budget");
        hdr.add_string("Actual");
        hdr.add_string("Variance");
        hdr.add_string("Var %");
        w.add_row(hdr);

        // Revenue section (rows 3-5, outline level 1)
        let revenue: &[(&str, &str, f64, f64)] = &[
            ("Revenue", "Product sales", 50000.0, 52000.0),
            ("Revenue", "Services", 30000.0, 28000.0),
            ("Revenue", "Licensing", 15000.0, 16000.0),
        ];
        for (cat, sub, budget, actual) in revenue {
            let row_num = w.sheets[0].rows.len() + 1;
            let mut row = RowData::new();
            row.add_string(cat);
            row.add_string(sub);
            row.add_number(*budget);
            row.add_number(*actual);
            row.add_formula(format!("D{row_num}-C{row_num}"));
            row.add_formula(format!("IF(C{row_num}=0,0,(D{row_num}-C{row_num})/C{row_num})"));
            w.add_row(row);
        }

        // Expenses section (rows 6-9, outline level 1)
        let expenses: &[(&str, &str, f64, f64)] = &[
            ("Expenses", "Salaries", 40000.0, 42000.0),
            ("Expenses", "Rent", 8000.0, 8000.0),
            ("Expenses", "Utilities", 2000.0, 1800.0),
            ("Expenses", "Marketing", 5000.0, 6200.0),
        ];
        for (cat, sub, budget, actual) in expenses {
            let row_num = w.sheets[0].rows.len() + 1;
            let mut row = RowData::new();
            row.add_string(cat);
            row.add_string(sub);
            row.add_number(*budget);
            row.add_number(*actual);
            row.add_formula(format!("D{row_num}-C{row_num}"));
            row.add_formula(format!("IF(C{row_num}=0,0,(D{row_num}-C{row_num})/C{row_num})"));
            w.add_row(row);
        }

        // Summary row
        let summary_row = w.sheets[0].rows.len() + 1;
        let mut summary = RowData::new();
        summary.add_string("Net");
        summary.add_empty();
        summary.add_formula("SUM(C3:C5)-SUM(C6:C9)");
        summary.add_formula("SUM(D3:D5)-SUM(D6:D9)");
        summary.add_formula(format!("D{summary_row}-C{summary_row}"));
        summary.add_formula(format!("IF(C{summary_row}=0,0,(D{summary_row}-C{summary_row})/C{summary_row})"));
        w.add_row(summary);

        // Row outlines: group revenue rows (2-4, 0-based) and expense rows (5-8)
        w.add_row_group(2, 4, 1, false);
        w.add_row_group(5, 8, 1, false);

        // Column outline: group Budget and Actual (cols 2-3) under Variance
        w.add_col_group(2, 3, 1, false);

        // Print setup
        w.set_print_setup(PrintSetup {
            orientation: Some(PageOrientation::Landscape),
            paper_size: Some(9), // A4
            scale: Some(90),
            fit_to_width: Some(1),
            fit_to_height: Some(0),
            print_area: Some("A1:F10".to_string()),
            margins: Some(PageMargins {
                left: 0.5,
                right: 0.5,
                top: 0.75,
                bottom: 0.75,
                header: 0.3,
                footer: 0.3,
            }),
        });

        w.set_column_width(0, 12.0);
        w.set_column_width(1, 16.0);
        w.set_column_width(2, 12.0);
        w.set_column_width(3, 12.0);
        w.set_column_width(4, 12.0);
        w.set_column_width(5, 10.0);

        let path = out_dir.join("financials.xlsx");
        w.save(std::fs::File::create(&path)?)?;
        println!("Created {}", path.display());
    }

    // ── Example 9: Embedded chart ──
    {
        let mut w = XlsxWriter::new();
        w.add_sheet("ChartData")?;

        let mut hdr = RowData::new();
        hdr.add_string("Month");
        hdr.add_string("Revenue");
        hdr.add_string("Expenses");
        hdr.add_string("Profit");
        w.add_row(hdr);

        let months = [
            ("Jan", 45000.0, 32000.0),
            ("Feb", 52000.0, 35000.0),
            ("Mar", 48000.0, 30000.0),
            ("Apr", 61000.0, 38000.0),
            ("May", 55000.0, 36000.0),
            ("Jun", 67000.0, 40000.0),
        ];

        for (i, (month, rev, exp)) in months.iter().enumerate() {
            let row_num = i + 2;
            let mut row = RowData::new();
            row.add_string(month);
            row.add_number(*rev);
            row.add_number(*exp);
            row.add_formula(format!("B{row_num}-C{row_num}"));
            w.add_row(row);
        }

        // Add a column chart
        let chart_data: Vec<Vec<String>> = months
            .iter()
            .map(|(m, r, e)| {
                vec![m.to_string(), r.to_string(), e.to_string()]
            })
            .collect();

        w.set_chart(
            ChartConfig {
                chart_type: DataChartType::Column,
                title: Some("Revenue vs Expenses (H1 2024)".to_string()),
                x_axis_title: Some("Month".to_string()),
                y_axis_title: Some("Amount ($)".to_string()),
                category_column: 0,
                value_columns: vec![1, 2],
                width: 720,
                height: 432,
                show_legend: true,
                colors: Some(vec!["4472C4".to_string(), "ED7D31".to_string()]),
            },
            chart_data,
        );

        w.set_column_width(0, 10.0);
        w.set_column_width(1, 12.0);
        w.set_column_width(2, 12.0);
        w.set_column_width(3, 12.0);

        let path = out_dir.join("chart_report.xlsx");
        w.save(std::fs::File::create(&path)?)?;
        println!("Created {}", path.display());
    }

    // ── Example 10: Large dataset with mixed types and freeze panes ──
    {
        let options = WriteOptions {
            freeze_header: true,
            auto_filter: true,
            style_header: true,
            auto_fit: true,
            ..Default::default()
        };
        let mut w = XlsxWriter::with_options(options);
        w.add_sheet("LargeDataset")?;

        let mut hdr = RowData::new();
        for col_name in &["ID", "Name", "Score", "Grade", "Active", "Notes"] {
            hdr.add_string(col_name);
        }
        w.add_row(hdr);

        for i in 0..500 {
            let mut row = RowData::new();
            row.add_number(i as f64 + 1.0);
            row.add_string(&format!("Item-{i:04}"));
            row.add_number((i as f64 * 1.7) % 100.0);
            let grade = match (i * 7) % 5 {
                0 => "A",
                1 => "B",
                2 => "C",
                3 => "D",
                _ => "F",
            };
            row.add_string(grade);
            row.add_bool(i % 3 != 0);
            if i % 10 == 0 {
                row.add_string("checkpoint");
            } else {
                row.add_empty();
            }
            w.add_row(row);
        }

        let path = out_dir.join("large_dataset.xlsx");
        w.save(std::fs::File::create(&path)?)?;
        println!("Created {}", path.display());
    }

    println!("\nAll XLSX files written to {}", out_dir.display());
    Ok(())
}
