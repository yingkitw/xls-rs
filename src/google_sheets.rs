//! Google Sheets API handler for reading and writing Google Sheets

use crate::config::Config;
use crate::csv_handler::CellRange;
use crate::traits::{DataReader, DataWriteOptions, DataWriter, FileHandler};
use anyhow::{anyhow, Result};
use tokio::runtime::Runtime;

/// Handler for Google Sheets operations
pub struct GoogleSheetsHandler {
    config: Config,
    rt: Runtime,
}

impl GoogleSheetsHandler {
    /// Create a new Google Sheets handler
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            rt: Runtime::new().expect("Failed to create tokio runtime"),
        }
    }

    /// Create a new Google Sheets handler with custom config
    pub fn with_config(config: Config) -> Self {
        Self {
            config,
            rt: Runtime::new().expect("Failed to create tokio runtime"),
        }
    }

    /// Parse Google Sheets URL or ID to extract spreadsheet ID
    pub fn parse_spreadsheet_id(&self, path: &str) -> Result<String> {
        // If it's a gsheet:// protocol URL
        if path.starts_with("gsheet://") {
            let id = path
                .strip_prefix("gsheet://")
                .ok_or_else(|| anyhow!("Invalid gsheet URL"))?;
            return Ok(id.split('/').next().unwrap_or(id).to_string());
        }

        // If it's a full Google Sheets URL, extract the ID
        if path.starts_with("https://docs.google.com/spreadsheets/") {
            if let Some(start) = path.find("/d/") {
                let start = start + 3;
                if let Some(end) = path[start..].find('/') {
                    return Ok(path[start..start + end].to_string());
                } else {
                    return Ok(path[start..].to_string());
                }
            }
        }

        // Check if it's just the ID
        if path.len() >= 44
            && path
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Ok(path.to_string());
        }

        Err(anyhow!("Invalid Google Sheets URL or ID: {}", path))
    }

    /// Parse sheet name from path
    pub fn parse_sheet_name(&self, path: &str) -> Option<String> {
        // Extract from gsheets://id/sheet_name
        if path.starts_with("gsheet://") {
            let parts: Vec<&str> = path[9..].split('/').collect();
            if parts.len() > 1 {
                return Some(parts[1].to_string());
            }
        }

        None
    }

    /// Convert A1 notation to row/column indices
    pub fn a1_to_row_col(&self, a1: &str) -> Result<(usize, usize)> {
        let mut col_start = 0;
        let mut row_start = 0;

        // Find where letters end and numbers begin
        for (i, c) in a1.chars().enumerate() {
            if c.is_alphabetic() {
                col_start += 1;
            } else if c.is_numeric() {
                row_start = i;
                break;
            }
        }

        if col_start == 0 || row_start == 0 {
            return Err(anyhow!("Invalid A1 notation: {}", a1));
        }

        // Parse column (base-26)
        let col_str = &a1[..col_start];
        let mut col = 0;
        for c in col_str.chars() {
            col = col * 26 + (c.to_ascii_uppercase() as u8 - b'A' + 1) as usize;
        }
        col -= 1; // Convert to 0-based

        // Parse row
        let row = a1[row_start..].parse::<usize>()? - 1; // Convert to 0-based

        Ok((row, col))
    }

    /// Convert row/column indices to A1 notation
    pub fn row_col_to_a1(&self, row: usize, col: usize) -> String {
        let mut col = col + 1;
        let mut col_str = String::new();

        while col > 0 {
            col -= 1;
            col_str.insert(0, ((col % 26) as u8 + b'A') as char);
            col /= 26;
        }

        format!("{}{}", col_str, row + 1)
    }

    /// Convert CellRange to A1 notation range
    pub fn cell_range_to_a1(&self, range: &CellRange, sheet_name: Option<&str>) -> String {
        let start = self.row_col_to_a1(range.start_row, range.start_col);
        let end = self.row_col_to_a1(range.end_row, range.end_col);

        let range_str = if start == end {
            start
        } else {
            format!("{}:{}", start, end)
        };

        if let Some(name) = sheet_name {
            format!("'{}'!{}", name, range_str)
        } else {
            range_str
        }
    }
}

impl Default for GoogleSheetsHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DataReader for GoogleSheetsHandler {
    fn read(&self, path: &str) -> Result<Vec<Vec<String>>> {
        // For now, return a placeholder implementation
        // In a real implementation, this would use the Google Sheets API
        let _spreadsheet_id = self.parse_spreadsheet_id(path)?;
        let _sheet_name = self.parse_sheet_name(path);

        // TODO: Implement actual Google Sheets API call
        // For now, return sample data
        Ok(vec![
            vec!["Column1".to_string(), "Column2".to_string()],
            vec!["Value1".to_string(), "Value2".to_string()],
        ])
    }

    fn read_with_headers(&self, path: &str) -> Result<Vec<Vec<String>>> {
        self.read(path)
    }

    fn read_range(&self, path: &str, range: &CellRange) -> Result<Vec<Vec<String>>> {
        let _spreadsheet_id = self.parse_spreadsheet_id(path)?;
        let _sheet_name = self.parse_sheet_name(path);
        let _range_str = self.cell_range_to_a1(range, _sheet_name.as_deref());

        // TODO: Implement actual Google Sheets API call for range
        Ok(vec![vec![
            "RangeValue1".to_string(),
            "RangeValue2".to_string(),
        ]])
    }

    fn read_as_json(&self, path: &str) -> Result<String> {
        let data = self.read(path)?;
        serde_json::to_string_pretty(&data).map_err(Into::into)
    }

    fn supports_format(&self, path: &str) -> bool {
        path.starts_with("gsheet://")
            || path.starts_with("https://docs.google.com/spreadsheets/")
            || (path.len() >= 44
                && path
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_'))
    }
}

impl DataWriter for GoogleSheetsHandler {
    fn write(&self, path: &str, data: &[Vec<String>], options: DataWriteOptions) -> Result<()> {
        let spreadsheet_id = self.parse_spreadsheet_id(path)?;
        let sheet_name = options.sheet_name.or_else(|| self.parse_sheet_name(path));

        // TODO: Implement actual Google Sheets API call to write data
        println!("Writing to Google Sheets: {}", spreadsheet_id);
        if let Some(name) = &sheet_name {
            println!("Sheet: {}", name);
        }
        println!("Data rows: {}", data.len());

        Ok(())
    }

    fn write_range(
        &self,
        path: &str,
        data: &[Vec<String>],
        start_row: usize,
        start_col: usize,
    ) -> Result<()> {
        let spreadsheet_id = self.parse_spreadsheet_id(path)?;
        let sheet_name = self.parse_sheet_name(path);
        let start_a1 = self.row_col_to_a1(start_row, start_col);

        // TODO: Implement actual Google Sheets API call to write range
        println!("Writing range to Google Sheets: {}", spreadsheet_id);
        if let Some(name) = &sheet_name {
            println!("Sheet: {}", name);
        }
        println!("Start: {}", start_a1);
        println!("Data rows: {}", data.len());

        Ok(())
    }

    fn append(&self, path: &str, data: &[Vec<String>]) -> Result<()> {
        let spreadsheet_id = self.parse_spreadsheet_id(path)?;
        let sheet_name = self.parse_sheet_name(path);

        // TODO: Implement actual Google Sheets API call to append data
        println!("Appending to Google Sheets: {}", spreadsheet_id);
        if let Some(name) = &sheet_name {
            println!("Sheet: {}", name);
        }
        println!("Data rows: {}", data.len());

        Ok(())
    }

    fn supports_format(&self, path: &str) -> bool {
        path.starts_with("gsheet://")
            || path.starts_with("https://docs.google.com/spreadsheets/")
            || (path.len() >= 44
                && path
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_'))
    }
}

impl FileHandler for GoogleSheetsHandler {
    fn format_name(&self) -> &'static str {
        "gsheet"
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &["gsheet"]
    }
}

impl Clone for GoogleSheetsHandler {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            rt: Runtime::new().expect("Failed to create tokio runtime"),
        }
    }
}
