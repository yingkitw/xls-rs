//! Template filler for XLSX files
//!
//! Fills placeholder cells in template data with actual values and writes to XLSX.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;

use crate::excel::template::template_reader::TemplateData;
use crate::excel::xlsx_writer::{RowData, XlsxWriter};

/// Template filler that replaces placeholders with actual data
pub struct TemplateFiller;

impl TemplateFiller {
    /// Fill placeholders in template data with provided values
    ///
    /// # Arguments
    /// * `template_data` - The template data read from a file
    /// * `values` - Map of placeholder names to replacement values
    ///
    /// # Returns
    /// A new XlsxWriter with all template data and filled values
    pub fn fill_template(
        template_data: &TemplateData,
        values: &HashMap<String, String>,
    ) -> Result<XlsxWriter> {
        let mut writer = XlsxWriter::new();
        
        writer.add_sheet(&template_data.sheet_name)
            .with_context(|| format!("Failed to add sheet: {}", template_data.sheet_name))?;

        // Find the maximum dimensions of the template
        let max_row = template_data.cells
            .keys()
            .map(|(r, _)| *r)
            .max()
            .unwrap_or(0);
        let max_col = template_data.cells
            .keys()
            .map(|(_, c)| *c)
            .max()
            .unwrap_or(0);

        // Write all cells row by row
        for row in 0..=max_row {
            let mut row_data = RowData::new();
            
            for col in 0..=max_col {
                let cell_key = (row, col);

                if let Some(cell_value) = template_data.cells.get(&cell_key) {
                    if cell_value.contains("{{") && cell_value.contains("}}") {
                        // Cell contains placeholder syntax — interpolate all matches
                        let filled_value = Self::replace_placeholders_in_string(cell_value, values);
                        row_data.add_string(&filled_value);
                    } else {
                        // Regular cell, preserve as-is
                        row_data.add_string(cell_value);
                    }
                } else {
                    // Empty cell
                    row_data.add_empty();
                }
            }
            
            writer.add_row(row_data);
        }

        Ok(writer)
    }

    /// Fill multiple sheets from template data
    pub fn fill_multi_sheet_template(
        all_sheets_data: &HashMap<String, TemplateData>,
        values: &HashMap<String, String>,
    ) -> Result<XlsxWriter> {
        let mut writer = XlsxWriter::new();

        for (_sheet_name, template_data) in all_sheets_data {
            let sheet_writer = Self::fill_template(template_data, values)?;
            // Merge sheets from sheet_writer into writer
            for sheet in sheet_writer.sheets {
                writer.sheets.push(sheet);
            }
        }

        Ok(writer)
    }

    /// Replace all {{placeholder}} patterns in a string with values
    fn replace_placeholders_in_string(s: &str, values: &HashMap<String, String>) -> String {
        let mut result = s.to_string();
        
        for (key, value) in values {
            let pattern = format!("{{{{{}}}}}", key);
            result = result.replace(&pattern, value);
        }
        
        result
    }

    /// Fill template from file and save to output path
    ///
    /// # Arguments
    /// * `template_path` - Path to the template XLSX file
    /// * `output_path` - Path where the filled XLSX will be saved
    /// * `values` - Map of placeholder names to replacement values
    /// * `sheet_name` - Optional sheet name (uses first sheet if None)
    pub fn fill_from_file(
        template_path: &str,
        output_path: &str,
        values: &HashMap<String, String>,
        sheet_name: Option<&str>,
    ) -> Result<()> {
        let reader = crate::excel::template::TemplateReader::new()?;
        let template_data = reader.read_template(template_path, sheet_name)?;
        
        let writer = Self::fill_template(&template_data, values)?;
        
        let file = File::create(output_path)
            .with_context(|| format!("Failed to create output file: {}", output_path))?;
        let buffered = BufWriter::new(file);
        
        writer.save(buffered)
            .with_context(|| format!("Failed to write XLSX file: {}", output_path))?;
        
        Ok(())
    }

    /// Get list of required placeholders from a template file
    pub fn get_required_placeholders(
        template_path: &str,
        sheet_name: Option<&str>,
    ) -> Result<Vec<String>> {
        let reader = crate::excel::template::TemplateReader::new()?;
        let template_data = reader.read_template(template_path, sheet_name)?;
        Ok(template_data.placeholder_names())
    }

    /// Validate that all required placeholders have values provided
    pub fn validate_placeholders(
        template_data: &TemplateData,
        values: &HashMap<String, String>,
    ) -> Result<()> {
        let required = template_data.placeholder_names();
        let missing: Vec<&String> = required
            .iter()
            .filter(|name| !values.contains_key(*name))
            .collect();

        if !missing.is_empty() {
            let missing_str: Vec<String> = missing.iter().map(|s| (*s).clone()).collect();
            anyhow::bail!(
                "Missing values for placeholders: {}",
                missing_str.join(", ")
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::template::template_reader::PlaceholderInfo;

    #[test]
    fn test_replace_placeholders_in_string() {
        let mut values = HashMap::new();
        values.insert("name".to_string(), "Alice".to_string());
        values.insert("age".to_string(), "30".to_string());

        assert_eq!(
            TemplateFiller::replace_placeholders_in_string("Hello {{name}}", &values),
            "Hello Alice"
        );
        assert_eq!(
            TemplateFiller::replace_placeholders_in_string("{{name}} is {{age}}", &values),
            "Alice is 30"
        );
        assert_eq!(
            TemplateFiller::replace_placeholders_in_string("No placeholder here", &values),
            "No placeholder here"
        );
    }

    #[test]
    fn test_template_data_placeholder_names() {
        let mut template_data = TemplateData::new("Sheet1".to_string());
        
        template_data.placeholders.push(PlaceholderInfo {
            cell_ref: "A1".to_string(),
            row: 0,
            col: 0,
            name: "name".to_string(),
            full_value: "{{name}}".to_string(),
        });
        
        template_data.placeholders.push(PlaceholderInfo {
            cell_ref: "B1".to_string(),
            row: 0,
            col: 1,
            name: "age".to_string(),
            full_value: "{{age}}".to_string(),
        });

        let names = template_data.placeholder_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"name".to_string()));
        assert!(names.contains(&"age".to_string()));
    }

    #[test]
    fn test_validate_placeholders() {
        let mut template_data = TemplateData::new("Sheet1".to_string());
        
        template_data.placeholders.push(PlaceholderInfo {
            cell_ref: "A1".to_string(),
            row: 0,
            col: 0,
            name: "name".to_string(),
            full_value: "{{name}}".to_string(),
        });

        let mut values = HashMap::new();
        values.insert("name".to_string(), "Alice".to_string());

        assert!(TemplateFiller::validate_placeholders(&template_data, &values).is_ok());

        values.clear();
        assert!(TemplateFiller::validate_placeholders(&template_data, &values).is_err());
    }
}