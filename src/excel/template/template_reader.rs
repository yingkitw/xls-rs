//! Template reader for XLSX files
//!
//! Reads existing XLSX files and identifies placeholder cells for template-based generation.

use anyhow::{Context, Result};
use calamine::{Reader, Xlsx};
use regex::Regex;
use std::collections::HashMap;

/// Information about a placeholder cell in a template
#[derive(Debug, Clone)]
pub struct PlaceholderInfo {
    /// Cell reference (e.g., "A1", "B5")
    pub cell_ref: String,
    /// Row index (0-based)
    pub row: usize,
    /// Column index (0-based)
    pub col: usize,
    /// Placeholder name extracted from {{placeholder}}
    pub name: String,
    /// Full cell value including braces
    pub full_value: String,
}

/// Template data structure containing all sheets and their placeholders
#[derive(Debug, Clone)]
pub struct TemplateData {
    /// Sheet name
    pub sheet_name: String,
    /// All cell data (row, col) -> value
    pub cells: HashMap<(usize, usize), String>,
    /// Detected placeholders
    pub placeholders: Vec<PlaceholderInfo>,
    /// Named ranges (if any) - name -> range string
    pub named_ranges: HashMap<String, String>,
}

impl TemplateData {
    pub fn new(sheet_name: String) -> Self {
        Self {
            sheet_name,
            cells: HashMap::new(),
            placeholders: Vec::new(),
            named_ranges: HashMap::new(),
        }
    }

    /// Get all unique placeholder names
    pub fn placeholder_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.placeholders
            .iter()
            .map(|p| p.name.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    }
}

/// Template reader for XLSX files
pub struct TemplateReader {
    placeholder_regex: Regex,
}

impl TemplateReader {
    /// Create a new template reader
    pub fn new() -> Result<Self> {
        Ok(Self {
            placeholder_regex: Regex::new(r"\{\{([^}]+)\}\}")?,
        })
    }

    /// Read an XLSX file and extract template data from a specific sheet
    pub fn read_template(&self, path: &str, sheet_name: Option<&str>) -> Result<TemplateData> {
        let mut workbook: Xlsx<_> = calamine::open_workbook(path)
            .with_context(|| format!("Failed to open template file: {}", path))?;

        let sheet_names = workbook.sheet_names().to_vec();
        let sheet_name = if let Some(name) = sheet_name {
            if !sheet_names.contains(&name.to_string()) {
                anyhow::bail!(
                    "Sheet '{}' not found. Available sheets: {}",
                    name,
                    sheet_names.join(", ")
                );
            }
            name.to_string()
        } else {
            sheet_names
                .first()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("No sheets found in template"))?
        };

        let range = workbook
            .worksheet_range(&sheet_name)
            .with_context(|| format!("Failed to read sheet: {}", sheet_name))?;

        let mut template_data = TemplateData::new(sheet_name.clone());

        // Read all cell data
        for (row_idx, row) in range.rows().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                let cell_value = cell.to_string();
                if !cell_value.is_empty() {
                    template_data.cells.insert((row_idx, col_idx), cell_value.clone());

                    // Check if this cell contains a placeholder
                    if let Some(placeholder) = self.detect_placeholder(&cell_value, row_idx, col_idx) {
                        template_data.placeholders.push(placeholder);
                    }
                }
            }
        }

        // Extract named ranges (if calamine provides access)
        // Note: calamine doesn't directly expose named ranges, so this is a placeholder
        // for future enhancement with direct XML parsing

        Ok(template_data)
    }

    /// Read all sheets from a template file
    pub fn read_all_sheets(&self, path: &str) -> Result<HashMap<String, TemplateData>> {
        let workbook: Xlsx<_> = calamine::open_workbook(path)
            .with_context(|| format!("Failed to open template file: {}", path))?;

        let sheet_names = workbook.sheet_names().to_vec();
        let mut result = HashMap::new();

        for sheet_name in sheet_names {
            let template_data = self.read_template(path, Some(&sheet_name))?;
            result.insert(sheet_name, template_data);
        }

        Ok(result)
    }

    /// Detect if a cell value contains a placeholder pattern
    fn detect_placeholder(&self, value: &str, row: usize, col: usize) -> Option<PlaceholderInfo> {
        if let Some(captures) = self.placeholder_regex.captures(value) {
            let name = captures.get(1)?.as_str().to_string();
            let cell_ref = format!("{}{}", self.col_to_letter(col), row + 1);
            
            Some(PlaceholderInfo {
                cell_ref,
                row,
                col,
                name,
                full_value: value.to_string(),
            })
        } else {
            None
        }
    }

    /// Convert column index to Excel column letter (0=A, 1=B, etc.)
    pub fn col_to_letter(&self, col: usize) -> String {
        let mut result = String::new();
        let mut n = col;
        loop {
            result.insert(0, (b'A' + (n % 26) as u8) as char);
            if n < 26 {
                break;
            }
            n = n / 26 - 1;
        }
        result
    }
}

impl Default for TemplateReader {
    fn default() -> Self {
        Self::new().expect("Failed to create placeholder regex")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_reader_creation() {
        let reader = TemplateReader::new();
        assert!(reader.is_ok());
    }

    #[test]
    fn test_placeholder_detection() {
        let reader = TemplateReader::new().unwrap();
        
        // Test valid placeholder
        assert!(reader.placeholder_regex.is_match("{{name}}"));
        assert!(reader.placeholder_regex.is_match("{{ customer_name }}"));
        assert!(reader.placeholder_regex.is_match("Some text {{value}} more text"));
        
        // Test invalid placeholders
        assert!(!reader.placeholder_regex.is_match("name"));
        assert!(!reader.placeholder_regex.is_match("{name}"));
        assert!(!reader.placeholder_regex.is_match("{{name}"));
    }

    #[test]
    fn test_col_to_letter() {
        let reader = TemplateReader::new().unwrap();
        assert_eq!(reader.col_to_letter(0), "A");
        assert_eq!(reader.col_to_letter(1), "B");
        assert_eq!(reader.col_to_letter(25), "Z");
        assert_eq!(reader.col_to_letter(26), "AA");
        assert_eq!(reader.col_to_letter(27), "AB");
    }

    #[test]
    fn test_placeholder_info() {
        let info = PlaceholderInfo {
            cell_ref: "A1".to_string(),
            row: 0,
            col: 0,
            name: "customer_name".to_string(),
            full_value: "{{customer_name}}".to_string(),
        };
        
        assert_eq!(info.cell_ref, "A1");
        assert_eq!(info.name, "customer_name");
    }
}