//! Custom XLSX writer implementation
//!
//! This module provides a lightweight Excel XLSX writer that creates
//! XLSX files (ZIP archives containing XML files) without external dependencies.
//!
//! # Supported Features
//! - Multiple sheets with validation (max 31 char name, invalid characters)
//! - Cell data types: String, Number, Formula, Empty
//! - Column width configuration (auto-fit and manual)
//! - Freeze headers (freeze top row)
//! - Auto-filter for tables
//! - Basic styling (bold, alignment, borders, fills)
//! - XML escaping for special characters
//!
//! # Current Limitations
//! - **Chart generation**: Not implemented - requires complex XML drawing markup
//!   - Needs: xl/drawings/, xl/charts/, worksheet relationships
//! - **Sparklines**: Not implemented - requires additional chart XML
//! - **Conditional formatting**: Not implemented - requires conditional formatting XML
//! - **Advanced Excel features**: Some features require additional XML namespaces
//! - **Merged cells**: Not implemented
//! - **Data validation**: Not implemented
//! - **Pivot tables**: Not implemented

use anyhow::Result;
use std::io::{Seek, Write};
use zip::ZipWriter;

mod types;
mod xml_gen;
pub mod chart_xml;
pub mod cond_fmt_xml;
pub mod sparkline_xml;
pub mod streaming;

pub use types::{CellData, RowData};
pub use cond_fmt_xml::{ConditionalFormat, ConditionalRule};
pub use sparkline_xml::{Sparkline, SparklineGroup, SparklineType};

use super::types::WriteOptions;
use types::SheetData;
use xml_gen::*;

use super::chart::{ChartConfig};

/// XLSX workbook writer
pub struct XlsxWriter {
    pub sheets: Vec<SheetData>,
    options: WriteOptions,
    /// Chart config per sheet index (None = no chart for that sheet)
    chart_configs: Vec<Option<(ChartConfig, Vec<Vec<String>>)>>,
}

impl XlsxWriter {
    pub fn new() -> Self {
        Self {
            sheets: Vec::new(),
            options: WriteOptions::default(),
            chart_configs: Vec::new(),
        }
    }

    pub fn with_options(options: WriteOptions) -> Self {
        Self {
            sheets: Vec::new(),
            options,
            chart_configs: Vec::new(),
        }
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
            for cell in row {
                if let Ok(num) = cell.parse::<f64>() {
                    row_data.add_number(num);
                } else if !cell.is_empty() {
                    row_data.add_string(cell);
                } else {
                    row_data.add_empty();
                }
            }
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

    /// Save the workbook to a writer
    pub fn save<W: Write + Seek>(&self, mut writer: W) -> Result<()> {
        let mut zip = ZipWriter::new(&mut writer);

        // Determine which sheets have charts
        let chart_flags: Vec<bool> = (0..self.sheets.len())
            .map(|i| self.chart_configs.get(i).and_then(|c| c.as_ref()).is_some())
            .collect();
        let _has_any_chart = chart_flags.iter().any(|&f| f);

        // Add [Content_Types].xml (with chart content types if needed)
        add_content_types_ext(&mut zip, self.sheets.len(), &chart_flags)?;

        // Add _rels/.rels
        add_rels(&mut zip)?;

        // Add xl/workbook.xml
        add_workbook(&mut zip, &self.sheets)?;

        // Add xl/_rels/workbook.xml.rels
        add_workbook_rels(&mut zip, self.sheets.len())?;

        // Add xl/styles.xml
        add_styles(&mut zip)?;

        // Add worksheets
        for (idx, sheet) in self.sheets.iter().enumerate() {
            add_worksheet(&mut zip, idx, sheet, &self.options, chart_flags[idx])?;
        }

        // Add chart files for sheets that have charts
        for (idx, sheet) in self.sheets.iter().enumerate() {
            if let Some(Some((config, data))) = self.chart_configs.get(idx) {
                chart_xml::add_chart_to_zip(&mut zip, idx, config, data, &sheet.name)?;
            }
        }

        // Add xl/theme/theme1.xml
        add_theme(&mut zip)?;

        zip.finish()?;
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
        assert!(output.len() > 0);
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
        assert!(output.len() > 0);
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
        assert!(output.len() > 0);
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
        assert!(output.len() > 0);
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
        assert!(output.len() > 0);
        assert_eq!(&output[0..4], b"PK\x03\x04");

        assert_eq!(writer.sheets[0].rows[0].cells.len(), 3);
        assert!(matches!(writer.sheets[0].rows[0].cells[0], CellData::String(_)));
        assert!(matches!(writer.sheets[0].rows[0].cells[1], CellData::Empty));
        assert!(matches!(writer.sheets[0].rows[0].cells[2], CellData::String(_)));
    }
}
