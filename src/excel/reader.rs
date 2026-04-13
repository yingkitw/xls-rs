use anyhow::{Context, Result};
use calamine::{open_workbook, Ods, Reader, Xlsx};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::csv_handler::CellRange;
use crate::traits::DataReader;

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
            if let Ok(current_modified) = std::fs::metadata(path).and_then(|m| m.modified()) {
                if let Some(cached_modified) = metadata.modified_time {
                    if current_modified == cached_modified {
                        return Some(metadata.clone());
                    }
                }
            }
        }
        None
    }

    fn insert(&self, path: String, metadata: ExcelMetadata) {
        if let Ok(mut cache) = self.cache.write() {
            // Simple cache eviction: remove oldest entries if cache gets too large
            if cache.len() > 100 {
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

/// Excel file handler
pub struct ExcelHandler {
    metadata_cache: ExcelMetadataCache,
}

impl ExcelHandler {
    pub fn new() -> Self {
        Self {
            metadata_cache: ExcelMetadataCache::new(),
        }
    }

    /// Get or load Excel metadata with caching
    fn get_metadata(&self, path: &str) -> Result<ExcelMetadata> {
        // Check cache first
        if let Some(metadata) = self.metadata_cache.get(path) {
            return Ok(metadata);
        }

        // Load from file
        let workbook: Xlsx<_> =
            open_workbook(path).with_context(|| format!("Failed to open Excel file: {path}"))?;

        let modified_time = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok();

        let metadata = ExcelMetadata {
            sheet_names: workbook.sheet_names().to_vec(),
            modified_time,
        };

        // Cache the metadata
        self.metadata_cache.insert(path.to_string(), metadata.clone());

        Ok(metadata)
    }

    pub fn read(&self, path: &str) -> Result<String> {
        self.read_with_sheet(path, None)
    }

    pub fn read_with_sheet(&self, path: &str, sheet_name: Option<&str>) -> Result<String> {
        let mut workbook: Xlsx<_> =
            open_workbook(path).with_context(|| format!("Failed to open Excel file: {path}"))?;

        let metadata = self.get_metadata(path)?;
        let sheet_name = sheet_name
            .or_else(|| metadata.sheet_names.first().map(|s| s.as_str()))
            .ok_or_else(|| anyhow::anyhow!("No sheets found in workbook"))?;

        let range = workbook
            .worksheet_range(sheet_name)
            .with_context(|| format!("Failed to read sheet: {}", sheet_name))?;

        // Pre-allocate string capacity based on estimated size
        let mut output = String::with_capacity(range.height() * range.width() * 10);
        for row in range.rows() {
            let row_str: Vec<String> = row.iter().map(|cell| cell.to_string()).collect();
            output.push_str(&row_str.join(","));
            output.push('\n');
        }

        Ok(output)
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

    /// Read a specific range from Excel file
    pub fn read_range(
        &self,
        path: &str,
        range: &CellRange,
        sheet_name: Option<&str>,
    ) -> Result<Vec<Vec<String>>> {
        let mut workbook: Xlsx<_> =
            open_workbook(path).with_context(|| format!("Failed to open Excel file: {path}"))?;

        let metadata = self.get_metadata(path)?;
        let sheet_name = sheet_name
            .or_else(|| metadata.sheet_names.first().map(|s| s.as_str()))
            .ok_or_else(|| anyhow::anyhow!("No sheets found in workbook"))?;

        let ws_range = workbook
            .worksheet_range(sheet_name)
            .with_context(|| format!("Failed to read sheet: {}", sheet_name))?;

        let estimated_rows = range.end_row.saturating_sub(range.start_row) + 1;
        let estimated_cols = range.end_col.saturating_sub(range.start_col) + 1;
        let mut result = Vec::with_capacity(estimated_rows.min(1024));

        for (row_idx, row) in ws_range.rows().enumerate() {
            if row_idx < range.start_row {
                continue;
            }
            if row_idx > range.end_row {
                break;
            }

            let mut row_data = Vec::with_capacity(estimated_cols);
            for (col_idx, cell) in row.iter().enumerate() {
                if col_idx >= range.start_col && col_idx <= range.end_col {
                    row_data.push(cell.to_string());
                }
            }
            result.push(row_data);
        }

        Ok(result)
    }

    /// Read Excel and return as JSON array
    pub fn read_as_json(&self, path: &str, sheet_name: Option<&str>) -> Result<String> {
        let mut workbook: Xlsx<_> =
            open_workbook(path).with_context(|| format!("Failed to open Excel file: {path}"))?;

        let metadata = self.get_metadata(path)?;
        let sheet_name = sheet_name
            .or_else(|| metadata.sheet_names.first().map(|s| s.as_str()))
            .ok_or_else(|| anyhow::anyhow!("No sheets found in workbook"))?;

        let range = workbook
            .worksheet_range(sheet_name)
            .with_context(|| format!("Failed to read sheet: {}", sheet_name))?;

        let mut rows: Vec<Vec<String>> = Vec::with_capacity(range.height());
        for row in range.rows() {
            let mut row_data = Vec::with_capacity(range.width());
            for cell in row.iter() {
                row_data.push(cell.to_string());
            }
            rows.push(row_data);
        }

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
        let mut workbook: Xlsx<_> =
            open_workbook(path).with_context(|| format!("Failed to open Excel file: {path}"))?;

        let sheet_names = workbook.sheet_names().to_vec();
        let mut result = std::collections::HashMap::new();

        for sheet_name in sheet_names {
            let range = workbook
                .worksheet_range(&sheet_name)
                .with_context(|| format!("Failed to read sheet: {sheet_name}"))?;

            let mut rows: Vec<Vec<String>> = Vec::new();
            for row in range.rows() {
                rows.push(row.iter().map(|cell| cell.to_string()).collect());
            }

            result.insert(sheet_name, rows);
        }

        Ok(result)
    }

    /// Read ODS as CSV-like string
    pub fn read_ods(&self, path: &str, sheet_name: Option<&str>) -> Result<String> {
        let mut workbook: Ods<_> =
            open_workbook(path).with_context(|| format!("Failed to open ODS file: {path}"))?;

        let sheet_names = workbook.sheet_names();
        let sheet_name = sheet_name
            .or_else(|| sheet_names.first().map(|s| s.as_str()))
            .ok_or_else(|| anyhow::anyhow!("No sheets found in workbook"))?;

        let range = workbook
            .worksheet_range(sheet_name)
            .with_context(|| format!("Failed to read sheet: {}", sheet_name))?;

        let mut output = String::new();
        for row in range.rows() {
            let row_str: Vec<String> = row.iter().map(|cell| cell.to_string()).collect();
            output.push_str(&row_str.join(","));
            output.push('\n');
        }

        Ok(output)
    }

    /// Read ODS into `Vec<Vec<String>>`
    pub fn read_ods_data(&self, path: &str, sheet_name: Option<&str>) -> Result<Vec<Vec<String>>> {
        let mut workbook: Ods<_> =
            open_workbook(path).with_context(|| format!("Failed to open ODS file: {path}"))?;

        let sheet_names = workbook.sheet_names();
        let sheet_name = sheet_name
            .or_else(|| sheet_names.first().map(|s| s.as_str()))
            .ok_or_else(|| anyhow::anyhow!("No sheets found in workbook"))?;

        let range = workbook
            .worksheet_range(sheet_name)
            .with_context(|| format!("Failed to read sheet: {}", sheet_name))?;

        let mut rows: Vec<Vec<String>> = Vec::new();
        for row in range.rows() {
            rows.push(row.iter().map(|cell| cell.to_string()).collect());
        }

        Ok(rows)
    }

    /// List sheets in ODS file
    pub fn list_ods_sheets(&self, path: &str) -> Result<Vec<String>> {
        let workbook: Ods<_> =
            open_workbook(path).with_context(|| format!("Failed to open ODS file: {path}"))?;
        Ok(workbook.sheet_names().to_vec())
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
                let csv_str = self.read_with_sheet(path, None)?;
                let data = csv_str
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|l| l.split(',').map(|s| s.to_string()).collect())
                    .collect();
                return Ok(data);
            }
        }

        anyhow::bail!("Unsupported file format: {path}")
    }
}

impl DataReader for ExcelHandler {
    fn read(&self, path: &str) -> Result<Vec<Vec<String>>> {
        let csv_str = self.read_with_sheet(path, None)?;
        let result: Vec<Vec<String>> = csv_str
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.split(',').map(|s| s.to_string()).collect())
            .collect();
        Ok(result)
    }

    fn read_with_headers(&self, path: &str) -> Result<Vec<Vec<String>>> {
        // Call the trait method explicitly to avoid conflict with inherent method
        let csv_str = self.read_with_sheet(path, None)?;
        let result: Vec<Vec<String>> = csv_str
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.split(',').map(|s| s.to_string()).collect())
            .collect();
        Ok(result)
    }

    fn read_range(&self, path: &str, range: &CellRange) -> Result<Vec<Vec<String>>> {
        // Direct implementation to avoid method name conflicts
        let mut workbook: Xlsx<_> =
            open_workbook(path).with_context(|| format!("Failed to open Excel file: {path}"))?;

        let metadata = self.get_metadata(path)?;
        let sheet_name = metadata
            .sheet_names
            .first()
            .map(|s| s.as_str())
            .ok_or_else(|| anyhow::anyhow!("No sheets found in workbook"))?;

        let ws_range = workbook
            .worksheet_range(sheet_name)
            .with_context(|| format!("Failed to read sheet: {sheet_name}"))?;

        let estimated_rows = range.end_row.saturating_sub(range.start_row) + 1;
        let estimated_cols = range.end_col.saturating_sub(range.start_col) + 1;
        let mut result = Vec::with_capacity(estimated_rows.min(1024));

        for (row_idx, row) in ws_range.rows().enumerate() {
            if row_idx < range.start_row {
                continue;
            }
            if row_idx > range.end_row {
                break;
            }

            let mut row_data = Vec::with_capacity(estimated_cols);
            for (col_idx, cell) in row.iter().enumerate() {
                if col_idx >= range.start_col && col_idx <= range.end_col {
                    row_data.push(cell.to_string());
                }
            }
            result.push(row_data);
        }

        Ok(result)
    }

    fn read_as_json(&self, path: &str) -> Result<String> {
        let mut workbook: Xlsx<_> =
            open_workbook(path).with_context(|| format!("Failed to open Excel file: {path}"))?;

        let metadata = self.get_metadata(path)?;
        let sheet_name = metadata
            .sheet_names
            .first()
            .map(|s| s.as_str())
            .ok_or_else(|| anyhow::anyhow!("No sheets found in workbook"))?;

        let range = workbook
            .worksheet_range(sheet_name)
            .with_context(|| format!("Failed to read sheet: {sheet_name}"))?;

        let mut rows: Vec<Vec<String>> = Vec::with_capacity(range.height());
        for row in range.rows() {
            let mut row_data = Vec::with_capacity(range.width());
            for cell in row.iter() {
                row_data.push(cell.to_string());
            }
            rows.push(row_data);
        }

        serde_json::to_string_pretty(&rows).with_context(|| "Failed to serialize to JSON")
    }

    fn supports_format(&self, path: &str) -> bool {
        let path_lower = path.to_lowercase();
        path_lower.ends_with(".xlsx") || path_lower.ends_with(".xls") || path_lower.ends_with(".ods")
    }
}
