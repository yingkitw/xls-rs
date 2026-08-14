use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::limits::{METADATA_CACHE_SIZE, DEFAULT_ESTIMATED_ROWS};

use crate::traits::DataReader;
use crate::excel::xlsx_reader::XlsxReader as NativeXlsxReader;

/// Helper function to execute operations on sheet data
fn with_sheet_data<R, F>(path: &str, sheet_name: &str, f: F) -> Result<R>
where
    F: FnOnce(&[Vec<String>]) -> Result<R>,
{
    let workbook = NativeXlsxReader::from_path(path)
        .with_context(|| format!("Failed to open Excel file: {path}"))?;
    let sheet = workbook
        .get_sheet_by_name(sheet_name)
        .with_context(|| format!("Failed to read sheet: {sheet_name}"))?;
    f(&sheet.to_string_vec())
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
        if let Some(metadata) = cache.get(path)
            && let Ok(current_modified) = std::fs::metadata(path).and_then(|m| m.modified())
            && let Some(cached_modified) = metadata.modified_time
            && current_modified == cached_modified
        {
            return Some(metadata.clone());
        }
        None
    }

    fn insert(&self, path: String, metadata: ExcelMetadata) {
        if let Ok(mut cache) = self.cache.write() {
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

/// Simple cell range for reading subsets of data
#[derive(Debug, Clone)]
pub struct CellRange {
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
}

impl CellRange {
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        match parts.len() {
            1 => {
                let (row, col) = parse_cell_ref(parts[0])?;
                Ok(Self { start_row: row, start_col: col, end_row: row, end_col: col })
            }
            2 => {
                let (start_row, start_col) = parse_cell_ref(parts[0])?;
                let (end_row, end_col) = parse_cell_ref(parts[1])?;
                Ok(Self { start_row, start_col, end_row, end_col })
            }
            _ => anyhow::bail!("Invalid cell range format: {s}. Expected e.g. A1:C10"),
        }
    }
}

fn parse_cell_ref(s: &str) -> Result<(usize, usize)> {
    let mut col_str = String::new();
    let mut row_str = String::new();
    for ch in s.chars() {
        if ch.is_alphabetic() {
            col_str.push(ch);
        } else if ch.is_ascii_digit() {
            row_str.push(ch);
        }
    }
    let mut col = 0usize;
    for ch in col_str.chars() {
        col = col * 26 + (ch.to_ascii_uppercase() as usize - b'A' as usize + 1);
    }
    let row = row_str.parse::<usize>()
        .with_context(|| format!("Invalid row number in cell reference: {s}"))?;
    Ok((row - 1, col - 1))
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

    fn get_metadata(&self, path: &str) -> Result<ExcelMetadata> {
        if let Some(metadata) = self.metadata_cache.get(path) {
            return Ok(metadata);
        }

        let modified_time = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        let workbook = NativeXlsxReader::from_path(path)
            .with_context(|| format!("Failed to open Excel file: {path}"))?;
        let sheet_names = workbook.sheet_names();

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

    pub fn parse_cell_reference(&self, cell: &str) -> Result<(usize, usize)> {
        parse_cell_ref(cell)
    }

    pub fn read_sheet_data(&self, path: &str, sheet_name: Option<&str>) -> Result<Vec<Vec<String>>> {
        let metadata = self.get_metadata(path)?;
        let sheet_name = Self::resolve_sheet_selection(sheet_name, &metadata.sheet_names)?;
        with_sheet_data(path, &sheet_name, |rows| Ok(rows.to_vec()))
    }

    pub fn read_range(
        &self,
        path: &str,
        range: &CellRange,
        sheet_name: Option<&str>,
    ) -> Result<Vec<Vec<String>>> {
        let metadata = self.get_metadata(path)?;
        let sheet_name = Self::resolve_sheet_selection(sheet_name, &metadata.sheet_names)?;

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

        Ok(result)
    }

    pub fn read_as_json(&self, path: &str, sheet_name: Option<&str>) -> Result<String> {
        let rows = self.read_sheet_data(path, sheet_name)?;
        serde_json::to_string_pretty(&rows).with_context(|| "Failed to serialize to JSON")
    }

    pub fn list_sheets(&self, path: &str) -> Result<Vec<String>> {
        let metadata = self.get_metadata(path)?;
        Ok(metadata.sheet_names)
    }

    pub fn read_all_sheets(
        &self,
        path: &str,
    ) -> Result<std::collections::HashMap<String, Vec<Vec<String>>>> {
        let workbook = NativeXlsxReader::from_path(path)
            .with_context(|| format!("Failed to open Excel file: {path}"))?;
        Ok(workbook.read_all_to_string_vec())
    }

    pub fn read_auto(&self, path: &str, sheet_or_range: Option<&str>) -> Result<Vec<Vec<String>>> {
        if path.to_lowercase().ends_with(".xlsx") {
            if let Some(range_str) = sheet_or_range
                && range_str.contains(':')
            {
                let cell_range = CellRange::parse(range_str)?;
                return self.read_range(path, &cell_range, None);
            }
            return self.read_sheet_data(path, sheet_or_range);
        }
        anyhow::bail!("Unsupported file format: {path}. Only .xlsx is supported.")
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
        path.to_lowercase().ends_with(".xlsx")
    }
}

impl crate::traits::FileHandler for ExcelHandler {
    fn format_name(&self) -> &'static str {
        "xlsx"
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &["xlsx"]
    }
}
