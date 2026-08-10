use crate::excel::ExcelHandler;
use crate::format_detector::DefaultFormatDetector;
use crate::traits::FormatDetector;
use anyhow::{Context, Result};

pub struct Converter {
    excel_handler: ExcelHandler,
    format_detector: DefaultFormatDetector,
}

impl Default for Converter {
    fn default() -> Self {
        Self::new()
    }
}

impl Converter {
    pub fn new() -> Self {
        Self {
            excel_handler: ExcelHandler::new(),
            format_detector: DefaultFormatDetector,
        }
    }

    pub fn read_any_data(&self, path: &str, sheet_name: Option<&str>) -> Result<Vec<Vec<String>>> {
        let format = self.format_detector.detect_format(path)?;
        match format.as_str() {
            "xlsx" => self.excel_handler.read_sheet_data(path, sheet_name),
            _ => anyhow::bail!("Unsupported format: {}. Only XLSX is supported.", format),
        }
    }

    pub fn write_any_data(
        &self,
        path: &str,
        data: &[Vec<String>],
        sheet_name: Option<&str>,
    ) -> Result<()> {
        let format = self.format_detector.detect_format(path)?;
        match format.as_str() {
            "xlsx" => {
                let temp = tempfile::NamedTempFile::new()
                    .context("Failed to create temp file for XLSX conversion")?;
                let temp_path = temp.path().to_str()
                    .ok_or_else(|| anyhow::anyhow!("Temp file path is not valid UTF-8"))?
                    .to_string();

                // Write data to temp CSV for the Excel handler to consume
                {
                    use std::io::Write;
                    let mut f = std::fs::File::create(&temp_path)
                        .with_context(|| format!("Failed to create temp file: {}", temp_path))?;
                    for row in data {
                        let escaped: Vec<String> = row.iter().map(|cell| {
                            if cell.contains(',') || cell.contains('"') || cell.contains('\n') {
                                format!("\"{}\"", cell.replace('"', "\"\""))
                            } else {
                                cell.clone()
                            }
                        }).collect();
                        writeln!(f, "{}", escaped.join(","))?;
                    }
                    f.flush()?;
                }

                let result = self.excel_handler
                    .write_from_csv(&temp_path, path, sheet_name)
                    .context(format!("Failed to write XLSX: {}", path));
                drop(temp);
                result
            }
            _ => anyhow::bail!("Unsupported format: {}. Only XLSX is supported.", format),
        }
    }

    /// Convert between supported formats (XLSX only)
    pub fn convert(&self, input: &str, output: &str, sheet_name: Option<&str>) -> Result<()> {
        let input_format = self.format_detector.detect_format(input)?;
        if input_format != "xlsx" {
            anyhow::bail!("Unsupported input format: {}. Only XLSX is supported.", input_format);
        }
        let output_format = self.format_detector.detect_format(output)?;
        if output_format != "xlsx" {
            anyhow::bail!("Unsupported output format: {}. Only XLSX is supported.", output_format);
        }

        let data = self.read_any_data(input, sheet_name)?;
        self.write_any_data(output, &data, sheet_name)?;
        Ok(())
    }
}
