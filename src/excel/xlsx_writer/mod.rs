//! Custom XLSX writer implementation
//!
//! This module provides a lightweight Excel XLSX writer that creates
//! XLSX files (ZIP archives containing XML files) without external dependencies.
//!
//! # Supported Features
//! - Multiple sheets with validation (max 31 char name, invalid characters)
//! - Cell data types: String, Number, Bool, Formula, Empty
//! - Column width configuration (auto-fit and manual)
//! - Freeze headers (freeze top row)
//! - Auto-filter for tables
//! - Cell styling (bold, italic, underline, font size/name/color, fill color,
//!   alignment, wrap, borders, number formats, date formatting)
//! - Merged cells
//! - Conditional formatting (color scales, data bars, icon sets, formula-based)
//! - Sparklines (line, column, win/loss)
//! - Data validation (list, whole, decimal, date, text length, custom)
//! - Hyperlinks with tooltips
//! - Cell comments with authors
//! - Row/column outline grouping
//! - Print setup (orientation, paper size, scale, fit-to-page, print area, margins)
//! - Embedded charts (bar, column, line, area, pie, scatter, doughnut)
//! - XML escaping for special characters

use anyhow::Result;
use std::io::{Seek, Write};
use zip::ZipWriter;
use zip::write::FileOptions;

mod types;
mod xml_gen;
pub mod chart_xml;
pub mod cond_fmt_xml;
pub mod sparkline_xml;
pub mod streaming;
pub mod style_registry;

pub use types::{
    CellComment, CellData, ColGroup, DataValidation, Hyperlink, MergeCell, Operator, PageMargins,
    PageOrientation, PrintSetup, RowData, RowGroup, SheetData, Table, TableStyleInfo, ValidationType,
};
pub use cond_fmt_xml::{ConditionalFormat, ConditionalRule};
pub use sparkline_xml::{Sparkline, SparklineGroup, SparklineType};
pub use style_registry::{SharedStrings, StyleRegistry, XlsxCellStyle};

use super::types::WriteOptions;
use xml_gen::*;

use super::chart::{ChartConfig};

/// XLSX workbook writer
pub struct XlsxWriter {
    pub sheets: Vec<SheetData>,
    options: WriteOptions,
    /// Chart config per sheet index (None = no chart for that sheet)
    chart_configs: Vec<Option<(ChartConfig, Vec<Vec<String>>)>>,
    /// Style registry — defaults to a fresh registry (one cellXf: the
    /// default). Callers register user styles via
    /// `register_cell_style` / `register_named_format`.
    pub styles: StyleRegistry,
    /// Optional named-format registry: name → cellXf index. Lets a
    /// caller register a style under a stable name once and reference
    /// it from cells written anywhere.
    named_formats: std::collections::BTreeMap<String, u32>,
    vba_project: Option<Vec<u8>>,
}

impl XlsxWriter {
    pub fn new() -> Self {
        Self::with_options(WriteOptions::default())
    }

    pub fn with_options(options: WriteOptions) -> Self {
        Self {
            sheets: Vec::new(),
            options,
            chart_configs: Vec::new(),
            styles: StyleRegistry::new(),
            named_formats: std::collections::BTreeMap::new(),
            vba_project: None,
        }
    }

    /// Register an `XlsxCellStyle` and return its `s="N"` index for
    /// use with `RowData::set_cell_style`. Repeated registrations of
    /// the same style return the same index.
    pub fn register_cell_style(&mut self, style: &XlsxCellStyle) -> u32 {
        self.styles.register(style)
    }

    /// Register a named format, e.g. "header", "money". Returns the
    /// cellXf index, or the previously-registered index if `name` was
    /// already used. The index is also returned from
    /// `named_format_index`.
    pub fn register_named_format(&mut self, name: &str, style: &XlsxCellStyle) -> u32 {
        if let Some(&idx) = self.named_formats.get(name) {
            return idx;
        }
        let idx = self.styles.register(style);
        self.named_formats.insert(name.to_string(), idx);
        idx
    }

    /// Look up a previously-registered named format. Returns `None` if
    /// `name` is unknown.
    pub fn named_format_index(&self, name: &str) -> Option<u32> {
        self.named_formats.get(name).copied()
    }

    /// Set a chart for the current (last added) sheet
    pub fn set_chart(&mut self, config: ChartConfig, data: Vec<Vec<String>>) {
        let sheet_idx = if self.sheets.is_empty() { 0 } else { self.sheets.len() - 1 };
        while self.chart_configs.len() <= sheet_idx {
            self.chart_configs.push(None);
        }
        self.chart_configs[sheet_idx] = Some((config, data));
    }

    /// Add a new sheet to the workbook
    pub fn add_sheet(&mut self, name: &str) -> Result<()> {
        // Validate sheet name (max 31 characters)
        if name.len() > 31 {
            anyhow::bail!("Sheet name cannot exceed 31 characters");
        }

        // Check for invalid characters
        let invalid_chars = ['\\', '/', '?', '*', '[', ']'];
        if name.chars().any(|c| invalid_chars.contains(&c)) {
            anyhow::bail!("Sheet name contains invalid characters: \\ / ? * [ ]");
        }

        self.sheets.push(SheetData {
            name: name.to_string(),
            rows: Vec::new(),
            column_widths: Vec::new(),
            conditional_formats: Vec::new(),
            sparkline_groups: Vec::new(),
            merge_cells: Vec::new(),
            data_validations: Vec::new(),
            hyperlinks: Vec::new(),
            print_setup: None,
            comments: Vec::new(),
            row_groups: Vec::new(),
            col_groups: Vec::new(),
            tables: Vec::new(),
        });
        Ok(())
    }

    /// Add conditional formatting to the current sheet
    pub fn add_conditional_format(&mut self, format: ConditionalFormat) {
        if let Some(sheet) = self.sheets.last_mut() {
            sheet.conditional_formats.push(format);
        }
    }

    /// Add a sparkline group to the current sheet
    pub fn add_sparkline_group(&mut self, group: SparklineGroup) {
        if let Some(sheet) = self.sheets.last_mut() {
            sheet.sparkline_groups.push(group);
        }
    }

    /// Add a merged cell range to the current sheet (0-based, inclusive)
    pub fn add_merge_cell(&mut self, start_row: usize, start_col: usize, end_row: usize, end_col: usize) {
        if let Some(sheet) = self.sheets.last_mut() {
            sheet.merge_cells.push(MergeCell {
                start_row,
                start_col,
                end_row,
                end_col,
            });
        }
    }

    /// Add a data validation rule to the current sheet
    pub fn add_data_validation(&mut self, validation: DataValidation) {
        if let Some(sheet) = self.sheets.last_mut() {
            sheet.data_validations.push(validation);
        }
    }

    /// Add a hyperlink to the current sheet
    pub fn add_hyperlink(&mut self, cell_ref: &str, url: &str, tooltip: Option<&str>) {
        if let Some(sheet) = self.sheets.last_mut() {
            sheet.hyperlinks.push(Hyperlink {
                cell_ref: cell_ref.to_string(),
                url: url.to_string(),
                tooltip: tooltip.map(|s| s.to_string()),
            });
        }
    }

    /// Set print setup for the current sheet
    pub fn set_print_setup(&mut self, setup: PrintSetup) {
        if let Some(sheet) = self.sheets.last_mut() {
            sheet.print_setup = Some(setup);
        }
    }

    /// Add a cell comment to the current sheet
    pub fn add_comment(&mut self, cell_ref: &str, text: &str, author: Option<&str>) {
        if let Some(sheet) = self.sheets.last_mut() {
            sheet.comments.push(CellComment {
                cell_ref: cell_ref.to_string(),
                text: text.to_string(),
                author: author.map(|s| s.to_string()),
            });
        }
    }

    /// Add a row group (outline) to the current sheet
    pub fn add_row_group(&mut self, start_row: usize, end_row: usize, level: u8, collapsed: bool) {
        if let Some(sheet) = self.sheets.last_mut() {
            sheet.row_groups.push(RowGroup {
                start_row,
                end_row,
                level,
                collapsed,
            });
        }
    }

    /// Add a column group (outline) to the current sheet
    pub fn add_col_group(&mut self, start_col: usize, end_col: usize, level: u8, collapsed: bool) {
        if let Some(sheet) = self.sheets.last_mut() {
            sheet.col_groups.push(ColGroup {
                start_col,
                end_col,
                level,
                collapsed,
            });
        }
    }

    /// Add a structured table to the current sheet
    pub fn add_table(&mut self, table: Table) {
        if let Some(sheet) = self.sheets.last_mut() {
            sheet.tables.push(table);
        }
    }

    /// Add a row to the current sheet
    pub fn add_row(&mut self, row: RowData) {
        if let Some(sheet) = self.sheets.last_mut() {
            sheet.rows.push(row);
        }
    }

    /// Add data from a 2D vector
    pub fn add_data(&mut self, data: &[Vec<String>]) {
        if self.sheets.is_empty() {
            return;
        }

        let sheet = self.sheets.last_mut().unwrap();

        for row in data {
            let mut row_data = RowData::new();
            super::add_cells_to_row(&mut row_data, row);
            sheet.rows.push(row_data);
        }
    }

    /// Set column width for a specific column
    pub fn set_column_width(&mut self, col: usize, width: f64) {
        if let Some(sheet) = self.sheets.last_mut() {
            // Expand column_widths vector if necessary
            if sheet.column_widths.len() <= col {
                sheet.column_widths.resize(col + 1, 8.43); // Default Excel column width
            }
            sheet.column_widths[col] = width;
        }
    }

    /// Set VBA project bytes (from `xl/vbaProject.bin`). When set,
    /// the output file will be macro-enabled (`.xlsm`) with the
    /// appropriate content types.
    pub fn set_vba_project(&mut self, data: Vec<u8>) {
        self.vba_project = Some(data);
    }

    /// Save the workbook to a writer
    pub fn save<W: Write + Seek>(&self, mut writer: W) -> Result<()> {
        let mut zip = ZipWriter::new(&mut writer);

        // Determine which sheets have charts, comments, or tables
        let chart_flags: Vec<bool> = (0..self.sheets.len())
            .map(|i| self.chart_configs.get(i).and_then(|c| c.as_ref()).is_some())
            .collect();
        let comment_flags: Vec<bool> = self.sheets.iter().map(|s| !s.comments.is_empty()).collect();
        let table_flags: Vec<bool> = self.sheets.iter().map(|s| !s.tables.is_empty()).collect();

        // Add [Content_Types].xml (with chart/comment/table content types if needed)
        add_content_types_ext(&mut zip, self.sheets.len(), &chart_flags, &comment_flags, self.vba_project.is_some(), &table_flags)?;

        // Add _rels/.rels
        add_rels(&mut zip)?;

        // Add xl/workbook.xml
        add_workbook(&mut zip, &self.sheets)?;

        // Add xl/_rels/workbook.xml.rels
        add_workbook_rels(&mut zip, self.sheets.len())?;

        // Add xl/styles.xml
        add_styles_with_registry(&mut zip, &self.styles)?;

        // Add worksheets — assign table rel IDs (global table index)
        let mut table_global_idx = 1usize;
        for (idx, sheet) in self.sheets.iter().enumerate() {
            let table_rel_id = if !sheet.tables.is_empty() {
                Some(table_global_idx as u32)
            } else {
                None
            };
            add_worksheet(&mut zip, idx, sheet, &self.options, chart_flags[idx], table_rel_id)?;
            // Advance global table index by the number of tables on this sheet
            table_global_idx += sheet.tables.len();
        }

        // Add table XML files
        let mut table_global_idx = 1usize;
        for sheet in &self.sheets {
            for table in &sheet.tables {
                add_table_xml(&mut zip, table_global_idx, table)?;
                table_global_idx += 1;
            }
        }

        // Add chart files for sheets that have charts
        for (idx, sheet) in self.sheets.iter().enumerate() {
            if let Some(Some((config, data))) = self.chart_configs.get(idx) {
                chart_xml::add_chart_to_zip(&mut zip, idx, config, data, &sheet.name)?;
            }
        }

        // Add xl/theme/theme1.xml
        add_theme(&mut zip)?;

        // Add VBA project if present (macro-enabled .xlsm)
        if let Some(vba) = &self.vba_project {
            let opts = FileOptions::<()>::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file("xl/vbaProject.bin", opts)?;
            zip.write_all(vba)?;
        }

        zip.finish()?;
        writer.flush()?;
        Ok(())
    }
}

impl Default for XlsxWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::CellStyle;
    use std::io::Cursor;

    #[test]
    fn test_col_num_to_letter() {
        assert_eq!(col_num_to_letter(1), "A");
        assert_eq!(col_num_to_letter(26), "Z");
        assert_eq!(col_num_to_letter(27), "AA");
        assert_eq!(col_num_to_letter(28), "AB");
        assert_eq!(col_num_to_letter(52), "AZ");
        assert_eq!(col_num_to_letter(53), "BA");
        assert_eq!(col_num_to_letter(701), "ZY");
        assert_eq!(col_num_to_letter(702), "ZZ");
        assert_eq!(col_num_to_letter(703), "AAA");
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("hello"), "hello");
        assert_eq!(escape_xml("a&b"), "a&amp;b");
        assert_eq!(escape_xml("a<b"), "a&lt;b");
        assert_eq!(escape_xml("a>b"), "a&gt;b");
        assert_eq!(escape_xml("a\"b"), "a&quot;b");
        assert_eq!(escape_xml("a'b"), "a&apos;b");
        assert_eq!(escape_xml("<>&\"'"), "&lt;&gt;&amp;&quot;&apos;");
    }

    #[test]
    fn test_row_data_new() {
        let row = RowData::new();
        assert_eq!(row.cells.len(), 0);
    }

    #[test]
    fn test_row_data_add_string() {
        let mut row = RowData::new();
        row.add_string("test");
        assert_eq!(row.cells.len(), 1);
        match &row.cells[0] {
            CellData::String(s) => assert_eq!(s, "test"),
            _ => panic!("Expected String cell"),
        }
    }

    #[test]
    fn test_row_data_add_number() {
        let mut row = RowData::new();
        row.add_number(42.5);
        assert_eq!(row.cells.len(), 1);
        match &row.cells[0] {
            CellData::Number(n) => assert_eq!(*n, 42.5),
            _ => panic!("Expected Number cell"),
        }
    }

    #[test]
    fn test_row_data_add_formula() {
        let mut row = RowData::new();
        row.add_formula("=SUM(A1:A10)");
        assert_eq!(row.cells.len(), 1);
        match &row.cells[0] {
            CellData::Formula(f) => assert_eq!(f, "=SUM(A1:A10)"),
            _ => panic!("Expected Formula cell"),
        }
    }

    #[test]
    fn test_row_data_add_empty() {
        let mut row = RowData::new();
        row.add_empty();
        assert_eq!(row.cells.len(), 1);
        match &row.cells[0] {
            CellData::Empty => {}
            _ => panic!("Expected Empty cell"),
        }
    }

    #[test]
    fn test_row_data_mixed() {
        let mut row = RowData::new();
        row.add_string("Name");
        row.add_number(100.0);
        row.add_formula("=B2*2");
        row.add_empty();
        assert_eq!(row.cells.len(), 4);
    }

    #[test]
    fn test_xlsx_writer_new() {
        let writer = XlsxWriter::new();
        assert_eq!(writer.sheets.len(), 0);
    }

    #[test]
    fn test_xlsx_writer_default() {
        let writer = XlsxWriter::default();
        assert_eq!(writer.sheets.len(), 0);
    }

    #[test]
    fn test_xlsx_writer_with_options() {
        let options = WriteOptions {
            sheet_name: Some("TestSheet".to_string()),
            style_header: true,
            header_style: CellStyle::header(),
            column_styles: None,
            freeze_header: true,
            auto_filter: true,
            auto_fit: true,
        };
        let writer = XlsxWriter::with_options(options.clone());
        assert_eq!(writer.sheets.len(), 0);
        let _ = writer;
    }

    #[test]
    fn test_add_sheet_valid_name() {
        let mut writer = XlsxWriter::new();
        assert!(writer.add_sheet("Sheet1").is_ok());
        assert!(writer.add_sheet("Data").is_ok());
        assert_eq!(writer.sheets.len(), 2);
        assert_eq!(writer.sheets[0].name, "Sheet1");
        assert_eq!(writer.sheets[1].name, "Data");
    }

    #[test]
    fn test_add_sheet_too_long() {
        let mut writer = XlsxWriter::new();
        let long_name = "a".repeat(32); // 32 characters > 31 limit
        assert!(writer.add_sheet(&long_name).is_err());
    }

    #[test]
    fn test_add_sheet_invalid_characters() {
        let mut writer = XlsxWriter::new();
        assert!(writer.add_sheet("Sheet\\Test").is_err());
        assert!(writer.add_sheet("Sheet/Test").is_err());
        assert!(writer.add_sheet("Sheet?Test").is_err());
        assert!(writer.add_sheet("Sheet*Test").is_err());
        assert!(writer.add_sheet("Sheet[Test").is_err());
        assert!(writer.add_sheet("Sheet]Test").is_err());
    }

    #[test]
    fn test_add_row() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Sheet1").unwrap();

        let mut row = RowData::new();
        row.add_string("A");
        row.add_number(1.0);
        writer.add_row(row);

        assert_eq!(writer.sheets[0].rows.len(), 1);
        assert_eq!(writer.sheets[0].rows[0].cells.len(), 2);
    }

    #[test]
    fn test_add_data() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Sheet1").unwrap();

        let data = vec![
            vec!["Name".to_string(), "Age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ];
        writer.add_data(&data);

        assert_eq!(writer.sheets[0].rows.len(), 3);
        assert!(matches!(writer.sheets[0].rows[0].cells[0], CellData::String(_)));
        assert!(matches!(writer.sheets[0].rows[1].cells[0], CellData::String(_)));
        assert!(matches!(writer.sheets[0].rows[1].cells[1], CellData::Number(_)));
        assert!(matches!(writer.sheets[0].rows[2].cells[0], CellData::String(_)));
        assert!(matches!(writer.sheets[0].rows[2].cells[1], CellData::Number(_)));
    }

    #[test]
    fn test_set_column_width() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Sheet1").unwrap();

        writer.set_column_width(0, 15.5);
        writer.set_column_width(1, 20.0);

        assert_eq!(writer.sheets[0].column_widths.len(), 2);
        assert_eq!(writer.sheets[0].column_widths[0], 15.5);
        assert_eq!(writer.sheets[0].column_widths[1], 20.0);
    }

    #[test]
    fn test_set_column_width_expands() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Sheet1").unwrap();

        // Setting column 5 should create columns 0-5 with default width
        writer.set_column_width(5, 10.0);

        assert_eq!(writer.sheets[0].column_widths.len(), 6);
        assert_eq!(writer.sheets[0].column_widths[0], 8.43); // default
        assert_eq!(writer.sheets[0].column_widths[5], 10.0);  // set value
    }

    #[test]
    fn test_add_row_without_sheet() {
        let mut writer = XlsxWriter::new();
        let row = RowData::new();
        writer.add_row(row);
        assert_eq!(writer.sheets.len(), 0);
    }

    #[test]
    fn test_add_multiple_sheets() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Sheet1").unwrap();
        writer.add_sheet("Sheet2").unwrap();
        writer.add_sheet("Sheet3").unwrap();

        assert_eq!(writer.sheets.len(), 3);
        assert_eq!(writer.sheets[0].name, "Sheet1");
        assert_eq!(writer.sheets[1].name, "Sheet2");
        assert_eq!(writer.sheets[2].name, "Sheet3");
    }

    #[test]
    fn test_cell_data_clone() {
        let cell = CellData::String("test".to_string());
        let cloned = cell.clone();
        assert!(matches!(cloned, CellData::String(s) if s == "test"));

        let cell = CellData::Number(42.0);
        let cloned = cell.clone();
        assert!(matches!(cloned, CellData::Number(n) if n == 42.0));
    }

    #[test]
    fn test_save_simple_workbook() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Test").unwrap();

        let mut row = RowData::new();
        row.add_string("Header");
        row.add_number(100.0);
        writer.add_row(row);

        let mut buffer = Cursor::new(Vec::new());
        assert!(writer.save(&mut buffer).is_ok());

        let output = buffer.into_inner();
        assert!(!output.is_empty());
        assert_eq!(&output[0..4], b"PK\x03\x04");
    }

    #[test]
    fn test_save_workbook_with_formulas() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Formulas").unwrap();

        let mut row = RowData::new();
        row.add_number(10.0);
        row.add_number(20.0);
        row.add_formula("=A1+B1");
        writer.add_row(row);

        let mut buffer = Cursor::new(Vec::new());
        assert!(writer.save(&mut buffer).is_ok());

        let output = buffer.into_inner();
        assert!(!output.is_empty());
        assert_eq!(&output[0..4], b"PK\x03\x04");
    }

    #[test]
    fn test_save_workbook_with_freeze_header() {
        let options = WriteOptions {
            sheet_name: None,
            style_header: false,
            header_style: CellStyle::default(),
            column_styles: None,
            freeze_header: true,
            auto_filter: false,
            auto_fit: false,
        };
        let mut writer = XlsxWriter::with_options(options);
        writer.add_sheet("Frozen").unwrap();

        let mut row = RowData::new();
        row.add_string("Header");
        writer.add_row(row);

        let mut row = RowData::new();
        row.add_string("Data");
        writer.add_row(row);

        let mut buffer = Cursor::new(Vec::new());
        assert!(writer.save(&mut buffer).is_ok());

        let output = buffer.into_inner();
        assert!(!output.is_empty());
        assert_eq!(&output[0..4], b"PK\x03\x04");
    }

    #[test]
    fn test_save_workbook_with_auto_filter() {
        let options = WriteOptions {
            sheet_name: None,
            style_header: false,
            header_style: CellStyle::default(),
            column_styles: None,
            freeze_header: false,
            auto_filter: true,
            auto_fit: false,
        };
        let mut writer = XlsxWriter::with_options(options);
        writer.add_sheet("Filtered").unwrap();

        let mut row = RowData::new();
        row.add_string("A");
        row.add_string("B");
        writer.add_row(row);

        let mut row = RowData::new();
        row.add_string("1");
        row.add_string("2");
        writer.add_row(row);

        let mut buffer = Cursor::new(Vec::new());
        assert!(writer.save(&mut buffer).is_ok());

        let output = buffer.into_inner();
        assert!(!output.is_empty());
        assert_eq!(&output[0..4], b"PK\x03\x04");
    }

    #[test]
    fn test_empty_cells_handling() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Empty").unwrap();

        let mut row = RowData::new();
        row.add_string("A");
        row.add_empty();
        row.add_string("C");
        writer.add_row(row);

        let mut buffer = Cursor::new(Vec::new());
        assert!(writer.save(&mut buffer).is_ok());

        let output = buffer.into_inner();
        assert!(!output.is_empty());
        assert_eq!(&output[0..4], b"PK\x03\x04");

        assert_eq!(writer.sheets[0].rows[0].cells.len(), 3);
        assert!(matches!(writer.sheets[0].rows[0].cells[0], CellData::String(_)));
        assert!(matches!(writer.sheets[0].rows[0].cells[1], CellData::Empty));
        assert!(matches!(writer.sheets[0].rows[0].cells[2], CellData::String(_)));
    }

    #[test]
    fn test_save_workbook_with_merge_cells() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Merged").unwrap();

        let mut row = RowData::new();
        row.add_string("A");
        row.add_string("B");
        writer.add_row(row);
        writer.add_merge_cell(0, 0, 0, 1);

        let mut buffer = Cursor::new(Vec::new());
        assert!(writer.save(&mut buffer).is_ok());

        let output = buffer.into_inner();
        assert!(!output.is_empty());
        assert_eq!(&output[0..4], b"PK\x03\x04");
        assert_eq!(writer.sheets[0].merge_cells.len(), 1);
    }

    #[test]
    fn test_save_workbook_with_data_validation() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Validated").unwrap();

        let mut row = RowData::new();
        row.add_string("Status");
        writer.add_row(row);

        writer.add_data_validation(DataValidation {
            range: "A2:A10".to_string(),
            validation_type: ValidationType::List {
                source: "Yes,No,Maybe".to_string(),
            },
            allow_blank: true,
            show_dropdown: true,
            error_title: None,
            error_message: None,
        });

        let mut buffer = Cursor::new(Vec::new());
        assert!(writer.save(&mut buffer).is_ok());

        let output = buffer.into_inner();
        assert!(!output.is_empty());
        assert_eq!(&output[0..4], b"PK\x03\x04");
    }

    #[test]
    fn test_save_workbook_with_hyperlink() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Links").unwrap();

        let mut row = RowData::new();
        row.add_string("Click me");
        writer.add_row(row);
        writer.add_hyperlink("A1", "https://example.com", Some("Example"));

        let mut buffer = Cursor::new(Vec::new());
        assert!(writer.save(&mut buffer).is_ok());

        let output = buffer.into_inner();
        assert!(!output.is_empty());
        assert_eq!(&output[0..4], b"PK\x03\x04");
    }

    #[test]
    fn test_save_workbook_with_print_setup() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Print").unwrap();

        let mut row = RowData::new();
        row.add_string("Data");
        writer.add_row(row);

        writer.set_print_setup(PrintSetup {
            orientation: Some(PageOrientation::Landscape),
            paper_size: Some(9),
            scale: Some(85),
            fit_to_width: None,
            fit_to_height: None,
            print_area: Some("$A$1:$B$10".to_string()),
            margins: Some(PageMargins {
                left: 0.5,
                right: 0.5,
                top: 0.75,
                bottom: 0.75,
                header: 0.3,
                footer: 0.3,
            }),
        });

        let mut buffer = Cursor::new(Vec::new());
        assert!(writer.save(&mut buffer).is_ok());

        let output = buffer.into_inner();
        assert!(!output.is_empty());
        assert_eq!(&output[0..4], b"PK\x03\x04");
    }

    #[test]
    fn test_save_workbook_with_comments() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Comments").unwrap();

        let mut row = RowData::new();
        row.add_string("Item");
        writer.add_row(row);
        writer.add_comment("A1", "This is a comment", Some("Author"));

        let mut buffer = Cursor::new(Vec::new());
        assert!(writer.save(&mut buffer).is_ok());

        let output = buffer.into_inner();
        assert!(!output.is_empty());
        assert_eq!(&output[0..4], b"PK\x03\x04");
    }

    #[test]
    fn test_save_workbook_with_row_groups() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Grouped").unwrap();

        let mut row = RowData::new();
        row.add_string("Header");
        writer.add_row(row);

        let mut row = RowData::new();
        row.add_string("Group1-A");
        writer.add_row(row);

        let mut row = RowData::new();
        row.add_string("Group1-B");
        writer.add_row(row);

        writer.add_row_group(1, 2, 1, false);

        let mut buffer = Cursor::new(Vec::new());
        assert!(writer.save(&mut buffer).is_ok());

        let output = buffer.into_inner();
        assert!(!output.is_empty());
        assert_eq!(&output[0..4], b"PK\x03\x04");
        assert_eq!(writer.sheets[0].row_groups.len(), 1);
    }

    #[test]
    fn test_save_workbook_with_col_groups() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("GroupedCols").unwrap();

        let mut row = RowData::new();
        row.add_string("A");
        row.add_string("B");
        row.add_string("C");
        writer.add_row(row);

        writer.set_column_width(0, 10.0);
        writer.set_column_width(1, 10.0);
        writer.set_column_width(2, 10.0);
        writer.add_col_group(0, 1, 1, false);

        let mut buffer = Cursor::new(Vec::new());
        assert!(writer.save(&mut buffer).is_ok());

        let output = buffer.into_inner();
        assert!(!output.is_empty());
        assert_eq!(&output[0..4], b"PK\x03\x04");
        assert_eq!(writer.sheets[0].col_groups.len(), 1);
    }

    fn read_zip_part(zip_bytes: &[u8], name: &str) -> Option<String> {
        let cursor = Cursor::new(zip_bytes);
        let mut za = zip::ZipArchive::new(cursor).unwrap();
        let mut s = String::new();
        let mut f = za.by_name(name).ok()?;
        use std::io::Read;
        f.read_to_string(&mut s).unwrap();
        Some(s)
    }

    #[test]
    fn test_register_cell_style_emits_in_styles_xml() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Styled").unwrap();
        let mut row = RowData::new();
        row.add_string("Hello");
        row.add_number(42.0);
        writer.add_row(row);

        let idx = writer.register_cell_style(&XlsxCellStyle {
            bold: Some(true),
            fill_color: Some("305496".into()),
            font_color: Some("FFFFFF".into()),
            ..Default::default()
        });
        assert!(idx > 0, "custom style should not collide with default index 0");

        let mut buf = Cursor::new(Vec::new());
        writer.save(&mut buf).unwrap();
        let styles = read_zip_part(buf.get_ref(), "xl/styles.xml").unwrap();

        assert!(styles.contains("<numFmts"), "stylesheet has no <numFmts>");
        assert!(styles.contains("<fills"), "stylesheet has no <fills>");
        assert!(styles.contains("<cellXfs"), "stylesheet has no <cellXfs>");
        assert!(
            styles.contains("FF305496"),
            "registered fill color should appear in <fills>"
        );
    }

    #[test]
    fn test_per_cell_style_index_emits_s_attr() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("PerCell").unwrap();

        let header_idx = writer.register_cell_style(&XlsxCellStyle {
            bold: Some(true),
            fill_color: Some("305496".into()),
            font_color: Some("FFFFFF".into()),
            align: Some("center".into()),
            ..Default::default()
        });
        let money_idx = writer.register_cell_style(&XlsxCellStyle {
            number_format: Some("$#,##0.00".into()),
            ..Default::default()
        });

        let mut header = RowData::new();
        header.add_string("Item");
        header.add_string("Amount");
        header.set_cell_style(0, header_idx);
        header.set_cell_style(1, header_idx);
        writer.add_row(header);

        let mut row = RowData::new();
        row.add_string("Widget");
        row.add_number(1500.5);
        row.set_cell_style(1, money_idx);
        writer.add_row(row);

        let mut buf = Cursor::new(Vec::new());
        writer.save(&mut buf).unwrap();
        let sheet = read_zip_part(buf.get_ref(), "xl/worksheets/sheet1.xml").unwrap();

        assert!(
            sheet.contains(&format!(r#"s="{}""#, header_idx)),
            "header cell missing s={}: sheet={}",
            header_idx,
            sheet
        );
        assert!(
            sheet.contains(&format!(r#"s="{}""#, money_idx)),
            "money cell missing s={}: sheet={}",
            money_idx,
            sheet
        );
        let styles = read_zip_part(buf.get_ref(), "xl/styles.xml").unwrap();
        assert!(
            styles.contains("$#,##0.00"),
            "custom numFmt code should appear in styles.xml"
        );
    }

    #[test]
    fn test_named_format_register_and_lookup() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Named").unwrap();
        let first = writer.register_named_format(
            "header",
            &XlsxCellStyle {
                bold: Some(true),
                fill_color: Some("305496".into()),
                ..Default::default()
            },
        );
        let second = writer.register_named_format(
            "header",
            &XlsxCellStyle {
                bold: Some(true),
                fill_color: Some("305496".into()),
                ..Default::default()
            },
        );
        assert_eq!(first, second, "re-registering same name returns same index");
        assert_eq!(writer.named_format_index("header"), Some(first));
        assert_eq!(writer.named_format_index("unknown"), None);
    }

    #[test]
    fn test_empty_writer_minimal_styles() {
        let writer = XlsxWriter::new();
        let mut buf = Cursor::new(Vec::new());
        writer.save(&mut buf).unwrap();
        let styles = read_zip_part(buf.get_ref(), "xl/styles.xml").unwrap();
        // No user styles → minimal styles.xml with one cellXf.
        assert!(styles.contains("<cellXfs count=\"1\">"));
        assert!(styles.contains("<fonts count=\"1\">"));
        assert!(styles.contains("<fills count=\"2\">"));
    }

    #[test]
    fn test_set_cell_style_skips_empty_cells() {
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Empty").unwrap();
        let idx = writer.register_cell_style(&XlsxCellStyle {
            bold: Some(true),
            ..Default::default()
        });

        let mut row = RowData::new();
        row.add_string("A");
        row.add_empty();
        row.add_string("C");
        // This is a no-op (empty cell), should not panic.
        row.set_cell_style(1, idx);
        // But the surrounding cells can be styled.
        row.set_cell_style(0, idx);
        row.set_cell_style(2, idx);
        writer.add_row(row);

        let mut buf = Cursor::new(Vec::new());
        writer.save(&mut buf).unwrap();
        let sheet = read_zip_part(buf.get_ref(), "xl/worksheets/sheet1.xml").unwrap();
        assert!(sheet.contains(&format!(r#"s="{}""#, idx)));
    }

    #[test]
    fn test_style_preset_helpers() {
        let mut reg = StyleRegistry::new();
        let h = XlsxCellStyle::header();
        let n = XlsxCellStyle::note();
        let hi = XlsxCellStyle::highlighted();
        assert!(reg.register(&h) > 0);
        assert!(reg.register(&n) > 0);
        assert!(reg.register(&hi) > 0);
    }
}
