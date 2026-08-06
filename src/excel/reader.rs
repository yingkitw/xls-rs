use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::limits::{METADATA_CACHE_SIZE, DEFAULT_ESTIMATED_ROWS};

use crate::csv_handler::CellRange;
use crate::traits::DataReader;
use crate::excel::xls_reader::XlsReader as NativeXlsReader;
use crate::excel::xlsx_reader::XlsxReader as NativeXlsxReader;
use crate::excel::ods_reader::OdsReader as NativeOdsReader;

/// Helper function to execute operations on sheet data regardless of format
fn with_sheet_data<R, F>(path: &str, sheet_name: &str, f: F) -> Result<R>
where
    F: FnOnce(&[Vec<String>]) -> Result<R>,
{
    if is_xls(path) {
        let reader = NativeXlsReader::from_path(path)
            .with_context(|| format!("Failed to open XLS file: {path}"))?;
        let sheet = reader
            .get_sheet_by_name(sheet_name)
            .with_context(|| format!("Failed to find sheet: {sheet_name}"))?;
        f(&sheet.to_string_vec())
    } else {
        let workbook = NativeXlsxReader::from_path(path)
            .with_context(|| format!("Failed to open Excel file: {path}"))?;
        let sheet = workbook
            .get_sheet_by_name(sheet_name)
            .with_context(|| format!("Failed to read sheet: {sheet_name}"))?;
        f(&sheet.to_string_vec())
    }
}

/// Excel metadata cache entry
#[derive(Debug, Clone)]
struct ExcelMetadata {
    sheet_names: Vec<String>,
    modified_time: Option<std::time::SystemTime>,
}

/// Thread-safe metadata cache for Excel files
struct ExcelMetadataCache {
    cache: Arc<RwLock<HashMap<String, ExcelMetadata>>>,
}

impl ExcelMetadataCache {
    fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn get(&self, path: &str) -> Option<ExcelMetadata> {
        let cache = self.cache.read().ok()?;
        if let Some(metadata) = cache.get(path) {
            // Check if file is still valid
            if let Ok(current_modified) = std::fs::metadata(path).and_then(|m| m.modified())
                && let Some(cached_modified) = metadata.modified_time
                    && current_modified == cached_modified {
                        return Some(metadata.clone());
                    }
        }
        None
    }

    fn insert(&self, path: String, metadata: ExcelMetadata) {
        if let Ok(mut cache) = self.cache.write() {
            // Simple cache eviction: remove oldest entries if cache gets too large
            if cache.len() > METADATA_CACHE_SIZE {
                cache.clear();
            }
            cache.insert(path, metadata);
        }
    }

    fn invalidate(&self, path: &str) {
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(path);
        }
    }
}

/// Pick the right reader type based on file extension.
fn is_xls(path: &str) -> bool {
    path.to_lowercase().ends_with(".xls")
}

/// Excel file handler
pub struct ExcelHandler {
    metadata_cache: ExcelMetadataCache,
}

impl Default for ExcelHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ExcelHandler {
    pub fn new() -> Self {
        Self {
            metadata_cache: ExcelMetadataCache::new(),
        }
    }

    /// Resolve sheet name: use the first sheet if `requested` is `None`, otherwise require an exact
    /// match so users get a clear error instead of a low-level failure.
    fn resolve_sheet_selection(requested: Option<&str>, available: &[String]) -> Result<String> {
        match requested {
            Some(name) => {
                if available.iter().any(|s| s == name) {
                    Ok(name.to_string())
                } else {
                    let list = if available.is_empty() {
                        "(none)".to_string()
                    } else {
                        available.join(", ")
                    };
                    anyhow::bail!(
                        "Sheet '{name}' not found in workbook. Available sheets: {list}"
                    );
                }
            }
            None => available
                .first()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("No sheets found in workbook")),
        }
    }

    /// Get or load Excel metadata with caching. Dispatches to native XLS reader for .xls
    /// and native XLSX reader for .xlsx files.
    fn get_metadata(&self, path: &str) -> Result<ExcelMetadata> {
        if let Some(metadata) = self.metadata_cache.get(path) {
            return Ok(metadata);
        }

        let modified_time = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        let sheet_names = if is_xls(path) {
            let reader = NativeXlsReader::from_path(path)
                .with_context(|| format!("Failed to open XLS file: {path}"))?;
            reader.sheet_names()
        } else {
            let workbook = NativeXlsxReader::from_path(path)
                .with_context(|| format!("Failed to open Excel file: {path}"))?;
            workbook.sheet_names()
        };

        let metadata = ExcelMetadata { sheet_names, modified_time };
        self.metadata_cache.insert(path.to_string(), metadata.clone());
        Ok(metadata)
    }

    pub fn read(&self, path: &str) -> Result<String> {
        self.read_with_sheet(path, None)
    }

    pub fn read_with_sheet(&self, path: &str, sheet_name: Option<&str>) -> Result<String> {
        let metadata = self.get_metadata(path)?;
        let sheet_name = Self::resolve_sheet_selection(sheet_name, &metadata.sheet_names)?;

        with_sheet_data(path, &sheet_name, |rows| {
            let mut output = String::with_capacity(rows.len() * 10);
            for row in rows {
                output.push_str(&row.join(","));
                output.push('\n');
            }
            Ok(output)
        })
    }

    pub fn parse_cell_reference(&self, cell: &str) -> Result<(u32, u16)> {
        let mut col_str = String::new();
        let mut row_str = String::new();

        for ch in cell.chars() {
            if ch.is_alphabetic() {
                col_str.push(ch);
            } else if ch.is_ascii_digit() {
                row_str.push(ch);
            }
        }

        let col = self.column_to_index(&col_str)?;
        let row = row_str
            .parse::<u32>()
            .with_context(|| format!("Invalid row number in cell reference: {cell}"))?;

        Ok((row - 1, col))
    }

    fn column_to_index(&self, col: &str) -> Result<u16> {
        let mut index = 0u16;
        for ch in col.chars() {
            index = index * 26 + (ch.to_ascii_uppercase() as u16 - b'A' as u16 + 1);
        }
        Ok(index - 1)
    }

    /// Read a sheet into structured data without CSV serialization
    pub fn read_sheet_data(&self, path: &str, sheet_name: Option<&str>) -> Result<Vec<Vec<String>>> {
        let metadata = self.get_metadata(path)?;
        let sheet_name = Self::resolve_sheet_selection(sheet_name, &metadata.sheet_names)?;

        with_sheet_data(path, &sheet_name, |rows| Ok(rows.to_vec()))
    }

    /// Read a specific range from Excel file
    pub fn read_range(
        &self,
        path: &str,
        range: &CellRange,
        sheet_name: Option<&str>,
    ) -> Result<Vec<Vec<String>>> {
        let metadata = self.get_metadata(path)?;
        let sheet_name = Self::resolve_sheet_selection(sheet_name, &metadata.sheet_names)?;

        let result = if is_xls(path) {
            let reader = NativeXlsReader::from_path(path)
                .with_context(|| format!("Failed to open XLS file: {path}"))?;
            let sheet = reader.get_sheet_by_name(&sheet_name)
                .with_context(|| format!("Failed to find sheet: {sheet_name}"))?;

            let estimated_rows = range.end_row.saturating_sub(range.start_row) + 1;
            let estimated_cols = range.end_col.saturating_sub(range.start_col) + 1;
            let mut result = Vec::with_capacity(estimated_rows.min(DEFAULT_ESTIMATED_ROWS));

            for row_idx in range.start_row..=range.end_row {
                let mut row_data = Vec::with_capacity(estimated_cols);
                for col_idx in range.start_col..=range.end_col {
                    let cell_value = sheet.get_cell(row_idx, col_idx);
                    row_data.push(cell_value.to_string());
                }
                result.push(row_data);
            }
            result
        } else {
            let workbook = NativeXlsxReader::from_path(path)
                .with_context(|| format!("Failed to open Excel file: {path}"))?;
            let sheet = workbook.get_sheet_by_name(&sheet_name)
                .with_context(|| format!("Failed to read sheet: {sheet_name}"))?;

            let estimated_rows = range.end_row.saturating_sub(range.start_row) + 1;
            let estimated_cols = range.end_col.saturating_sub(range.start_col) + 1;
            let mut result = Vec::with_capacity(estimated_rows.min(DEFAULT_ESTIMATED_ROWS));

            for row_idx in range.start_row..=range.end_row {
                let mut row_data = Vec::with_capacity(estimated_cols);
                for col_idx in range.start_col..=range.end_col {
                    let cell_value = sheet.get_cell(row_idx, col_idx);
                    row_data.push(cell_value.to_string());
                }
                result.push(row_data);
            }
            result
        };

        Ok(result)
    }

    /// Read Excel and return as JSON array
    pub fn read_as_json(&self, path: &str, sheet_name: Option<&str>) -> Result<String> {
        let rows = self.read_sheet_data(path, sheet_name)?;
        serde_json::to_string_pretty(&rows).with_context(|| "Failed to serialize to JSON")
    }

    /// Get list of sheet names in workbook (cached)
    pub fn list_sheets(&self, path: &str) -> Result<Vec<String>> {
        let metadata = self.get_metadata(path)?;
        Ok(metadata.sheet_names)
    }

    /// Read all sheets at once, returns map of sheet_name -> data
    pub fn read_all_sheets(
        &self,
        path: &str,
    ) -> Result<std::collections::HashMap<String, Vec<Vec<String>>>> {
        let workbook = NativeXlsxReader::from_path(path)
            .with_context(|| format!("Failed to open Excel file: {path}"))?;
        Ok(workbook.read_all_to_string_vec())
    }

    /// Read ODS as CSV-like string
    pub fn read_ods(&self, path: &str, sheet_name: Option<&str>) -> Result<String> {
        let workbook = NativeOdsReader::from_path(path)
            .with_context(|| format!("Failed to open ODS file: {path}"))?;

        let sheet_names = workbook.sheet_names();
        let sheet_name = Self::resolve_sheet_selection(sheet_name, &sheet_names)?;

        let sheet = workbook.get_sheet_by_name(&sheet_name)
            .with_context(|| format!("Failed to read sheet: {sheet_name}"))?;

        let mut output = String::new();
        for row in &sheet.cells {
            let row_str: Vec<String> = row.iter().map(|cell| cell.to_string()).collect();
            output.push_str(&row_str.join(","));
            output.push('\n');
        }

        Ok(output)
    }

    /// Read ODS into `Vec<Vec<String>>`
    pub fn read_ods_data(&self, path: &str, sheet_name: Option<&str>) -> Result<Vec<Vec<String>>> {
        let workbook = NativeOdsReader::from_path(path)
            .with_context(|| format!("Failed to open ODS file: {path}"))?;

        let sheet_names = workbook.sheet_names();
        let sheet_name = Self::resolve_sheet_selection(sheet_name, &sheet_names)?;

        let sheet = workbook.get_sheet_by_name(&sheet_name)
            .with_context(|| format!("Failed to read sheet: {sheet_name}"))?;

        Ok(sheet.to_string_vec())
    }

    /// List sheets in ODS file
    pub fn list_ods_sheets(&self, path: &str) -> Result<Vec<String>> {
        let workbook = NativeOdsReader::from_path(path)
            .with_context(|| format!("Failed to open ODS file: {path}"))?;
        Ok(workbook.sheet_names())
    }

    /// Auto-detect format (XLSX/XLS/ODS) and read into `Vec<Vec<String>>`
    pub fn read_auto(&self, path: &str, sheet_or_range: Option<&str>) -> Result<Vec<Vec<String>>> {
        let path_lower = path.to_lowercase();

        if path_lower.ends_with(".ods") {
            return self.read_ods_data(path, sheet_or_range);
        }

        if path_lower.ends_with(".xlsx") || path_lower.ends_with(".xls") {
            if let Some(range_str) = sheet_or_range {
                let cell_range = CellRange::parse(range_str)?;
                return self.read_range(path, &cell_range, None);
            } else {
                return self.read_sheet_data(path, None);
            }
        }

        anyhow::bail!("Unsupported file format: {path}")
    }
}

impl DataReader for ExcelHandler {
    fn read(&self, path: &str) -> Result<Vec<Vec<String>>> {
        self.read_sheet_data(path, None)
    }

    fn read_with_headers(&self, path: &str) -> Result<Vec<Vec<String>>> {
        self.read_sheet_data(path, None)
    }

    fn read_range(&self, path: &str, range: &CellRange) -> Result<Vec<Vec<String>>> {
        self.read_range(path, range, None)
    }

    fn read_as_json(&self, path: &str) -> Result<String> {
        self.read_as_json(path, None)
    }

    fn supports_format(&self, path: &str) -> bool {
        let path_lower = path.to_lowercase();
        path_lower.ends_with(".xlsx") || path_lower.ends_with(".xls") || path_lower.ends_with(".ods")
    }
}
