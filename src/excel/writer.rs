use anyhow::{Context, Result};
use calamine::{open_workbook, Reader, Xlsx};
use std::fs::File;
use std::io::BufWriter;

use super::reader::ExcelHandler;
use super::types::WriteOptions;
use super::xlsx_writer::{CellData, RowData, XlsxWriter};
use crate::traits::{DataWriteOptions, DataWriter};

/// Write mode for range operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    /// Expand sheet bounds as needed
    Expand,
    /// Preserve existing cells outside the range
    Preserve,
    /// Overwrite all cells with new data
    Overwrite,
}

impl Default for WriteMode {
    fn default() -> Self {
        Self::Expand
    }
}

impl ExcelHandler {
    pub fn write_from_csv(
        &self,
        csv_path: &str,
        excel_path: &str,
        sheet_name: Option<&str>,
    ) -> Result<()> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(csv_path)
            .with_context(|| format!("Failed to open CSV file: {csv_path}"))?;

        let mut writer = XlsxWriter::new();
        let name = sheet_name.unwrap_or("Sheet1");
        writer.add_sheet(name)?;

        for result in reader.records() {
            let record = result?;
            let mut row = RowData::new();
            for field in record.iter() {
                if let Ok(num) = field.parse::<f64>() {
                    row.add_number(num);
                } else if !field.is_empty() {
                    row.add_string(field);
                } else {
                    row.add_empty();
                }
            }
            writer.add_row(row);
        }

        // Auto-fit columns
        let column_widths: Vec<f64> = if let Some(sheet) = writer.sheets.last() {
            (0..sheet.column_widths.len())
                .map(|col_idx| {
                    let max_width = sheet
                        .rows
                        .iter()
                        .map(|row| {
                            row.cells
                                .get(col_idx)
                                .map(|c| match c {
                                    CellData::String(s) => s.len(),
                                    CellData::Number(_) => 10,
                                    _ => 0,
                                })
                                .unwrap_or(0)
                        })
                        .max()
                        .unwrap_or(10);
                    (max_width + 2) as f64
                })
                .collect()
        } else {
            Vec::new()
        };

        for (col_idx, width) in column_widths.iter().enumerate() {
            writer.set_column_width(col_idx, *width);
        }

        let file = File::create(excel_path)?;
        let mut buf_writer = BufWriter::new(file);
        writer.save(&mut buf_writer)?;

        Ok(())
    }

    /// Write data to Excel with styling options
    pub fn write_styled(
        &self,
        path: &str,
        data: &[Vec<String>],
        options: &WriteOptions,
    ) -> Result<()> {
        let mut writer = XlsxWriter::with_options(options.clone());
        let sheet_name = options.sheet_name.as_deref().unwrap_or("Sheet1");
        writer.add_sheet(sheet_name)?;

        for (row_idx, row) in data.iter().enumerate() {
            let is_header = row_idx == 0 && options.style_header;

            let mut row_data = RowData::new();
            for cell in row {
                // Determine cell format
                let _use_header_style = is_header;

                if let Ok(num) = cell.parse::<f64>() {
                    row_data.add_number(num);
                } else if !cell.is_empty() {
                    row_data.add_string(cell);
                } else {
                    row_data.add_empty();
                }
            }
            writer.add_row(row_data);
        }

        let file = File::create(path)?;
        let mut buf_writer = BufWriter::new(file);
        writer.save(&mut buf_writer)?;

        Ok(())
    }

    /// Add a sparkline to an Excel file
    pub fn add_sparkline_formula(
        &self,
        excel_path: &str,
        data_range: &str,
        sparkline_cell: &str,
        sheet_name: Option<&str>,
    ) -> Result<()> {
        use super::xlsx_writer::{Sparkline, SparklineGroup, SparklineType};

        let mut writer = XlsxWriter::new();
        let name = sheet_name.unwrap_or("Sheet1");
        writer.add_sheet(name)?;
        writer.add_sparkline_group(SparklineGroup {
            sparkline_type: SparklineType::Line,
            sparklines: vec![Sparkline {
                location: sparkline_cell.to_string(),
                data_range: data_range.to_string(),
            }],
            color: "4472C4".to_string(),
            show_markers: false,
        });

        let file = File::create(excel_path)?;
        writer.save(BufWriter::new(file))?;
        Ok(())
    }

    /// Apply conditional formatting to an Excel file
    pub fn apply_conditional_format_formula(
        &self,
        excel_path: &str,
        range: &str,
        condition: &str,
        true_format: &super::types::CellStyle,
        _false_format: Option<&super::types::CellStyle>,
        sheet_name: Option<&str>,
    ) -> Result<()> {
        use super::xlsx_writer::{ConditionalFormat, ConditionalRule};

        let mut writer = XlsxWriter::new();
        let name = sheet_name.unwrap_or("Sheet1");
        writer.add_sheet(name)?;
        writer.add_conditional_format(ConditionalFormat {
            range: range.to_string(),
            rules: vec![ConditionalRule::Formula {
                formula: condition.to_string(),
                bg_color: true_format.bg_color.clone(),
                font_color: true_format.font_color.clone(),
                bold: true_format.bold,
            }],
        });

        let file = File::create(excel_path)?;
        writer.save(BufWriter::new(file))?;
        Ok(())
    }

    /// Write data to a specific range in Excel starting at the given row and column
    pub fn write_range(
        &self,
        path: &str,
        data: &[Vec<String>],
        start_row: u32,
        start_col: u16,
        sheet_name: Option<&str>,
    ) -> Result<()> {
        self.write_range_with_mode(path, data, start_row, start_col, sheet_name, WriteMode::Expand)
    }

    /// Write data to a specific range with specified write mode
    pub fn write_range_with_mode(
        &self,
        path: &str,
        data: &[Vec<String>],
        start_row: u32,
        start_col: u16,
        sheet_name: Option<&str>,
        mode: WriteMode,
    ) -> Result<()> {
        match mode {
            WriteMode::Expand => self.write_range_expand(path, data, start_row, start_col, sheet_name),
            WriteMode::Preserve => self.write_range_preserve(path, data, start_row, start_col, sheet_name),
            WriteMode::Overwrite => self.write_range_overwrite(path, data, start_row, start_col, sheet_name),
        }
    }

    fn write_range_expand(
        &self,
        path: &str,
        data: &[Vec<String>],
        start_row: u32,
        start_col: u16,
        sheet_name: Option<&str>,
    ) -> Result<()> {
        let mut writer = XlsxWriter::new();
        let name = sheet_name.unwrap_or("Sheet1");
        writer.add_sheet(name)?;

        // Add empty rows for offset
        for _ in 0..start_row {
            writer.add_row(RowData::new());
        }

        // Add empty cells for column offset
        for row in data {
            let mut row_data = RowData::new();

            // Add empty cells for column offset
            for _ in 0..start_col {
                row_data.add_empty();
            }

            for cell in row {
                if let Ok(num) = cell.parse::<f64>() {
                    row_data.add_number(num);
                } else if !cell.is_empty() {
                    row_data.add_string(cell);
                } else {
                    row_data.add_empty();
                }
            }
            writer.add_row(row_data);
        }

        let file = File::create(path)?;
        let mut buf_writer = BufWriter::new(file);
        writer.save(&mut buf_writer)?;

        Ok(())
    }

    fn write_range_preserve(
        &self,
        path: &str,
        data: &[Vec<String>],
        start_row: u32,
        start_col: u16,
        sheet_name: Option<&str>,
    ) -> Result<()> {
        let name = sheet_name.unwrap_or("Sheet1");

        // Read existing data if file exists
        let mut existing_data: Vec<Vec<String>> = Vec::new();

        if std::path::Path::new(path).exists() {
            let mut workbook: Xlsx<_> = open_workbook(path)
                .with_context(|| format!("Failed to open Excel file: {path}"))?;

            let sheet_names = workbook.sheet_names();
            let sheet_name = sheet_names
                .first()
                .map(|s| s.as_str())
                .unwrap_or("Sheet1");

            if let Ok(range) = workbook.worksheet_range(sheet_name) {
                for row in range.rows() {
                    existing_data.push(row.iter().map(|c| c.to_string()).collect());
                }
            }
        }

        // Ensure existing data is large enough
        let required_rows = (start_row as usize + data.len()).max(existing_data.len());
        let max_cols = existing_data
            .iter()
            .map(|r| r.len())
            .max()
            .unwrap_or(0)
            .max((start_col as usize) + data.iter().map(|r| r.len()).max().unwrap_or(0));

        // Expand existing data if needed
        while existing_data.len() < required_rows {
            existing_data.push(vec![String::new(); max_cols]);
        }
        for row in &mut existing_data {
            while row.len() < max_cols {
                row.push(String::new());
            }
        }

        // Write new data, preserving existing cells outside the range
        for (data_row_idx, data_row) in data.iter().enumerate() {
            let target_row_idx = start_row as usize + data_row_idx;
            for (data_col_idx, cell) in data_row.iter().enumerate() {
                let target_col_idx = start_col as usize + data_col_idx;
                if target_row_idx < existing_data.len()
                    && target_col_idx < existing_data[target_row_idx].len()
                {
                    existing_data[target_row_idx][target_col_idx] = cell.clone();
                }
            }
        }

        // Write everything back
        let mut writer = XlsxWriter::new();
        writer.add_sheet(name)?;
        writer.add_data(&existing_data);

        let file = File::create(path)?;
        let mut buf_writer = BufWriter::new(file);
        writer.save(&mut buf_writer)?;

        Ok(())
    }

    fn write_range_overwrite(
        &self,
        path: &str,
        data: &[Vec<String>],
        start_row: u32,
        start_col: u16,
        sheet_name: Option<&str>,
    ) -> Result<()> {
        let mut writer = XlsxWriter::new();
        let name = sheet_name.unwrap_or("Sheet1");
        writer.add_sheet(name)?;

        // Add empty rows for offset
        for _ in 0..start_row {
            writer.add_row(RowData::new());
        }

        for row in data {
            let mut row_data = RowData::new();

            // Add empty cells for column offset
            for _ in 0..start_col {
                row_data.add_empty();
            }

            for cell in row {
                if let Ok(num) = cell.parse::<f64>() {
                    row_data.add_number(num);
                } else if !cell.is_empty() {
                    row_data.add_string(cell);
                } else {
                    row_data.add_empty();
                }
            }
            writer.add_row(row_data);
        }

        let file = File::create(path)?;
        let mut buf_writer = BufWriter::new(file);
        writer.save(&mut buf_writer)?;

        Ok(())
    }
}

impl DataWriter for ExcelHandler {
    fn write(&self, path: &str, data: &[Vec<String>], options: DataWriteOptions) -> Result<()> {
        let mut writer = XlsxWriter::new();
        let sheet_name = options.sheet_name.as_deref().unwrap_or("Sheet1");
        writer.add_sheet(sheet_name)?;

        writer.add_data(data);

        // Auto-fit columns
        for col_idx in 0..data.get(0).map(|r| r.len()).unwrap_or(0) {
            let max_width = data
                .iter()
                .map(|row| {
                    row.get(col_idx)
                        .map(|s| s.len())
                        .unwrap_or(0)
                })
                .max()
                .unwrap_or(10);
            writer.set_column_width(col_idx, (max_width + 2) as f64);
        }

        let file = File::create(path)?;
        let mut buf_writer = BufWriter::new(file);
        writer.save(&mut buf_writer)?;

        Ok(())
    }

    fn write_range(
        &self,
        path: &str,
        data: &[Vec<String>],
        start_row: usize,
        start_col: usize,
    ) -> Result<()> {
        self.write_range(path, data, start_row as u32, start_col as u16, None)
    }

    fn append(&self, path: &str, data: &[Vec<String>]) -> Result<()> {
        use calamine::{open_workbook, Reader, Xlsx};

        // Check if file exists
        if !std::path::Path::new(path).exists() {
            // File doesn't exist, just write the data
            return self.write(path, data, DataWriteOptions::default());
        }

        // Read existing data from the file
        let mut existing_data: Vec<Vec<String>> = Vec::new();

        // Try to open as Excel file first
        if let Ok(mut workbook) = open_workbook::<Xlsx<_>, _>(path) {
            let sheet_names = workbook.sheet_names();
            let sheet_name = sheet_names
                .first()
                .map(|s| s.as_str())
                .unwrap_or("Sheet1");

            if let Ok(range) = workbook.worksheet_range(sheet_name) {
                for row in range.rows() {
                    let row_data: Vec<String> = row.iter().map(|cell| cell.to_string()).collect();
                    existing_data.push(row_data);
                }
            }
        }

        // Append new data
        existing_data.extend(data.iter().cloned());

        // Write everything back
        let mut writer = XlsxWriter::new();
        writer.add_sheet("Sheet1")?;
        writer.add_data(&existing_data);

        let file = File::create(path)?;
        let mut buf_writer = BufWriter::new(file);
        writer.save(&mut buf_writer)?;

        Ok(())
    }

    fn supports_format(&self, path: &str) -> bool {
        let path_lower = path.to_lowercase();
        path_lower.ends_with(".xlsx")
    }
}
