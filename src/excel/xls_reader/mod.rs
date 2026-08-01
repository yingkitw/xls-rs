//! XLS reader (BIFF8 format, OLE2 / CFB container).
//!
//! Native implementation for reading XLS files without external dependencies.
//! Parses CFB/BIFF8 format and returns structured cell data.

use anyhow::{Context, Result};

mod cfb_reader;
mod biff_reader;

use cfb_reader::CfbReader;
use biff_reader::{BiffRecord, RecordId};

/// Cell data type for reading.
#[derive(Debug, Clone)]
pub enum CellValue {
    String(String),
    Number(f64),
    Bool(bool),
    Error(String),
    Empty,
}

impl CellValue {
    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match self {
            CellValue::String(s) => s.clone(),
            CellValue::Number(n) => {
                // Format numbers as integers when they have no fractional part
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            CellValue::Bool(b) => if *b { "TRUE".to_string() } else { "FALSE".to_string() },
            CellValue::Error(e) => format!("#{}", e),
            CellValue::Empty => String::new(),
        }
    }
}

/// Sheet data structure.
#[derive(Debug, Clone)]
pub struct SheetData {
    pub name: String,
    pub cells: Vec<Vec<CellValue>>,
}

impl SheetData {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cells: Vec::new(),
        }
    }

    /// Get cell value at position (row, col), returns Empty if out of bounds
    pub fn get_cell(&self, row: usize, col: usize) -> &CellValue {
        self.cells.get(row)
            .and_then(|r| r.get(col))
            .unwrap_or(&CellValue::Empty)
    }

    /// Get number of rows
    pub fn row_count(&self) -> usize {
        self.cells.len()
    }

    /// Get number of columns (maximum row length)
    pub fn col_count(&self) -> usize {
        self.cells.iter().map(|r| r.len()).max().unwrap_or(0)
    }

    /// Convert to `Vec<Vec<String>>` for compatibility with existing API
    pub fn to_string_vec(&self) -> Vec<Vec<String>> {
        self.cells.iter()
            .map(|row| row.iter().map(|cell| cell.to_string()).collect())
            .collect()
    }
}

/// XLS workbook reader.
pub struct XlsReader {
    sheets: Vec<SheetData>,
    shared_strings: Vec<String>,
}

impl XlsReader {
    /// Create a new reader (empty, for API compatibility)
    pub fn new() -> Self {
        Self {
            sheets: Vec::new(),
            shared_strings: Vec::new(),
        }
    }

    /// Read XLS file from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Self::from_owned_bytes(bytes.to_vec())
    }

    /// Read XLS file from owned bytes (avoids an extra full-file copy when possible)
    pub fn from_owned_bytes(bytes: Vec<u8>) -> Result<Self> {
        // Parse CFB container (takes ownership — no sector double-buffer)
        let cfb = CfbReader::parse(bytes)
            .context("Failed to parse CFB container")?;

        // Extract workbook stream
        let workbook_data = cfb.get_stream("Workbook")
            .or_else(|| cfb.get_stream("Book"))
            .context("Workbook stream not found in CFB")?;

        // Parse BIFF8 records
        Self::parse_workbook(&workbook_data)
    }

    /// Read XLS file from path
    pub fn from_path(path: &str) -> Result<Self> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("Failed to read file: {}", path))?;
        Self::from_owned_bytes(bytes)
    }

    /// Parse workbook BIFF8 records
    fn parse_workbook(data: &[u8]) -> Result<Self> {
        let mut reader = Self {
            sheets: Vec::new(),
            shared_strings: Vec::new(),
        };

        let records = BiffRecord::parse_stream(data);
        let mut sheet_offsets: Vec<(u32, String)> = Vec::new();

        // First pass: read workbook globals
        for record in records {
            match record.id {
                RecordId::BoundSheet => {
                    let (offset, name) = BiffRecord::parse_bound_sheet(&record.data)?;
                    sheet_offsets.push((offset, name));
                }
                RecordId::Sst => {
                    reader.shared_strings = BiffRecord::parse_sst(&record.data)?;
                }
                RecordId::Eof => break,
                _ => {}
            }
        }

        // Second pass: parse each sheet
        for (offset, name) in sheet_offsets {
            if let Some(sheet_data) = reader.parse_sheet_at_offset(data, offset as usize) {
                let mut sheet = SheetData::new(name);
                sheet.cells = sheet_data;
                reader.sheets.push(sheet);
            }
        }

        Ok(reader)
    }

    /// Parse sheet at given offset
    fn parse_sheet_at_offset(&self, data: &[u8], offset: usize) -> Option<Vec<Vec<CellValue>>> {
        let records = BiffRecord::parse_stream(&data[offset..]);
        let mut cells: std::collections::HashMap<(u16, u16), CellValue> = std::collections::HashMap::new();
        let mut max_row: u16 = 0;
        let mut max_col: u16 = 0;

        for record in records {
            match record.id {
                RecordId::Eof => break,
                RecordId::LabelSst => {
                    let (row, col, _xf, sst_index) = BiffRecord::parse_labelsst(&record.data).ok()?;
                    let value = self.shared_strings.get(sst_index as usize)
                        .cloned()
                        .unwrap_or_else(String::new);
                    cells.insert((row, col), CellValue::String(value));
                    max_row = max_row.max(row);
                    max_col = max_col.max(col);
                }
                RecordId::Number => {
                    let (row, col, _xf, value) = BiffRecord::parse_number(&record.data).ok()?;
                    cells.insert((row, col), CellValue::Number(value));
                    max_row = max_row.max(row);
                    max_col = max_col.max(col);
                }
                RecordId::BoolErr => {
                    let (row, col, _xf, value, is_error) = BiffRecord::parse_boolerr(&record.data).ok()?;
                    if is_error {
                        cells.insert((row, col), CellValue::Error(value.to_string()));
                    } else {
                        cells.insert((row, col), CellValue::Bool(value != 0));
                    }
                    max_row = max_row.max(row);
                    max_col = max_col.max(col);
                }
                RecordId::Formula => {
                    let (row, col, _xf, _result) = BiffRecord::parse_formula(&record.data).ok()?;
                    // Formulas store cached result; for now, mark as empty
                    // A full implementation would evaluate or use the cached value
                    cells.insert((row, col), CellValue::Empty);
                    max_row = max_row.max(row);
                    max_col = max_col.max(col);
                }
                RecordId::Blank => {
                    let (row, col, _xf) = BiffRecord::parse_blank(&record.data).ok()?;
                    cells.insert((row, col), CellValue::Empty);
                    max_row = max_row.max(row);
                    max_col = max_col.max(col);
                }
                RecordId::Rk => {
                    if let Some((row, col, _xf, value)) = BiffRecord::parse_rk(&record.data) {
                        cells.insert((row, col), CellValue::Number(value));
                        max_row = max_row.max(row);
                        max_col = max_col.max(col);
                    }
                }
                _ => {}
            }
        }

        // Convert sparse cell map to dense Vec<Vec> (clamped to avoid OOM)
        if max_row == 0 && max_col == 0 && cells.is_empty() {
            return Some(Vec::new());
        }

        let (n_rows, n_cols) =
            crate::limits::clamp_dense_dims(max_row as usize, max_col as usize);
        if n_rows == 0 || n_cols == 0 {
            return Some(Vec::new());
        }

        let mut result = vec![vec![CellValue::Empty; n_cols]; n_rows];
        for ((row, col), value) in cells {
            let r = row as usize;
            let c = col as usize;
            if r < n_rows && c < n_cols {
                result[r][c] = value;
            }
        }

        Some(result)
    }

    /// Get list of sheet names
    pub fn sheet_names(&self) -> Vec<String> {
        self.sheets.iter().map(|s| s.name.clone()).collect()
    }

    /// Get sheet by index
    pub fn get_sheet(&self, index: usize) -> Option<&SheetData> {
        self.sheets.get(index)
    }

    /// Get sheet by name
    pub fn get_sheet_by_name(&self, name: &str) -> Option<&SheetData> {
        self.sheets.iter().find(|s| s.name == name)
    }

    /// Get number of sheets
    pub fn sheet_count(&self) -> usize {
        self.sheets.len()
    }
}

impl Default for XlsReader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_value_string_conversion() {
        assert_eq!(CellValue::String("hello".to_string()).to_string(), "hello");
        assert_eq!(CellValue::Number(42.0).to_string(), "42");
        assert_eq!(CellValue::Number(std::f64::consts::PI).to_string(), "3.141592653589793");
        assert_eq!(CellValue::Bool(true).to_string(), "TRUE");
        assert_eq!(CellValue::Bool(false).to_string(), "FALSE");
        assert_eq!(CellValue::Error("VALUE".to_string()).to_string(), "#VALUE");
        assert_eq!(CellValue::Empty.to_string(), "");
    }

    #[test]
    fn test_empty_reader() {
        let reader = XlsReader::new();
        assert_eq!(reader.sheet_count(), 0);
        assert!(reader.sheet_names().is_empty());
    }

    #[test]
    fn test_round_trip_with_writer() {
        use crate::excel::xls_writer::{XlsWriter, RowData};

        // Create a simple workbook
        let mut writer = XlsWriter::new();
        writer.add_sheet("Test").unwrap();
        let mut row = RowData::new();
        row.add_string("Hello");
        row.add_number(42.5);
        row.add_bool(true);
        writer.add_row(row);

        let bytes = writer.to_bytes().unwrap();

        // Read it back
        let reader = XlsReader::from_bytes(&bytes).unwrap();
        assert_eq!(reader.sheet_count(), 1);
        assert_eq!(reader.sheet_names(), vec!["Test"]);

        let sheet = reader.get_sheet(0).unwrap();
        assert_eq!(sheet.row_count(), 1);
        assert_eq!(sheet.col_count(), 3);

        assert_eq!(sheet.get_cell(0, 0).to_string(), "Hello");
        assert_eq!(sheet.get_cell(0, 1).to_string(), "42.5");
        assert_eq!(sheet.get_cell(0, 2).to_string(), "TRUE");
    }

    #[test]
    fn test_multiple_sheets() {
        use crate::excel::xls_writer::{XlsWriter, RowData};

        let mut writer = XlsWriter::new();
        writer.add_sheet("Sheet1").unwrap();
        let mut row = RowData::new();
        row.add_string("Data1");
        writer.add_row(row);

        writer.add_sheet("Sheet2").unwrap();
        let mut row2 = RowData::new();
        row2.add_string("Data2");
        writer.add_row(row2);

        let bytes = writer.to_bytes().unwrap();
        let reader = XlsReader::from_bytes(&bytes).unwrap();

        assert_eq!(reader.sheet_count(), 2);
        assert_eq!(reader.sheet_names(), vec!["Sheet1", "Sheet2"]);

        let sheet1 = reader.get_sheet_by_name("Sheet1").unwrap();
        assert_eq!(sheet1.get_cell(0, 0).to_string(), "Data1");

        let sheet2 = reader.get_sheet_by_name("Sheet2").unwrap();
        assert_eq!(sheet2.get_cell(0, 0).to_string(), "Data2");
    }
}