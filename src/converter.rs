use crate::csv_handler::CsvHandler;
use crate::excel::ExcelHandler;
use crate::format_detector::DefaultFormatDetector;
use crate::handler_registry::HandlerRegistry;
use crate::traits::{DataWriteOptions, FormatDetector};
use anyhow::{Context, Result};
use csv::{ReaderBuilder, WriterBuilder};
use std::io::Cursor;

pub struct Converter {
    registry: HandlerRegistry,
    excel_handler: ExcelHandler,
    csv_handler: CsvHandler,
    format_detector: DefaultFormatDetector,
}

impl Converter {
    pub fn new() -> Self {
        Self {
            registry: HandlerRegistry::new(),
            excel_handler: ExcelHandler::new(),
            csv_handler: CsvHandler::new(),
            format_detector: DefaultFormatDetector,
        }
    }

    pub fn read_any_data(&self, path: &str, sheet_name: Option<&str>) -> Result<Vec<Vec<String>>> {
        self.read_any(path, sheet_name)
    }

    pub fn write_any_data(
        &self,
        path: &str,
        data: &[Vec<String>],
        sheet_name: Option<&str>,
    ) -> Result<()> {
        self.write_any(path, data, sheet_name)
    }

    /// Convert between any supported formats
    /// Supported: csv, xlsx, xls, ods, parquet, avro
    pub fn convert(&self, input: &str, output: &str, sheet_name: Option<&str>) -> Result<()> {
        // Validate input format is supported
        let input_format = self.format_detector.detect_format(input)?;
        if !self.format_detector.is_supported(&input_format) {
            anyhow::bail!("Unsupported input format: {}", input_format);
        }

        // Output "-" means stdout (CSV)
        if output != "-" {
            let output_format = self.format_detector.detect_format(output)?;
            if !self.format_detector.is_supported(&output_format) {
                anyhow::bail!("Unsupported output format: {}", output_format);
            }
        }

        // Read input data
        let data = self.read_any(input, sheet_name)?;

        // Write to output format
        self.write_any(output, &data, sheet_name)?;

        Ok(())
    }

    /// Read data from any supported format
    fn read_any(&self, path: &str, sheet_name: Option<&str>) -> Result<Vec<Vec<String>>> {
        // Stdin support: "-" reads CSV from stdin
        if path == "-" {
            let mut input = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)
                .context("Failed to read from stdin")?;
            return Ok(self.parse_csv_data(&input));
        }

        let format = self.format_detector.detect_format(path)?;

        match format.as_str() {
            "ods" => self.excel_handler.read_ods_data(path, sheet_name),
            "xlsx" | "xls" => {
                let content = self.excel_handler.read_with_sheet(path, sheet_name)?;
                Ok(self.parse_csv_data(&content))
            }
            "parquet" => {
                use crate::columnar::ParquetHandler;
                let handler = ParquetHandler::new();
                handler.read_with_headers(path)
            }
            "avro" => {
                use crate::columnar::AvroHandler;
                let handler = AvroHandler::new();
                handler.read_with_headers(path)
            }
            _ => self.registry.read(path),
        }
    }

    /// Write data to any supported format
    fn write_any(&self, path: &str, data: &[Vec<String>], sheet_name: Option<&str>) -> Result<()> {
        // Stdout support: "-" writes CSV to stdout
        if path == "-" {
            let mut writer = WriterBuilder::new()
                .has_headers(false)
                .flexible(true)
                .from_writer(std::io::stdout());
            for record in data {
                writer.write_record(record)?;
            }
            writer.flush().context("Failed to flush stdout")?;
            return Ok(());
        }

        let format = self.format_detector.detect_format(path)?;

        match format.as_str() {
            "xlsx" | "xls" => {
                // Write to temp CSV then convert
                let temp_csv = format!("{}.tmp.csv", path);

                // Ensure temp file is cleaned up even if an error occurs
                let cleanup_temp = |temp_path: &str| {
                    if let Err(e) = std::fs::remove_file(temp_path) {
                        eprintln!("Warning: Failed to remove temp file {}: {}", temp_path, e);
                    }
                };

                // Write to temp CSV
                self.csv_handler.write_records(&temp_csv, data.to_vec())
                    .with_context(|| format!("Failed to write temp CSV file: {}", temp_csv))?;

                // Convert to Excel
                match self.excel_handler.write_from_csv(&temp_csv, path, sheet_name) {
                    Ok(_) => {
                        cleanup_temp(&temp_csv);
                        Ok(())
                    }
                    Err(e) => {
                        cleanup_temp(&temp_csv);
                        Err(e).context(format!("Failed to convert CSV to Excel: {}", path))
                    }
                }
            }
            _ => {
                // Use registry for other formats
                let options = DataWriteOptions {
                    sheet_name: sheet_name.map(|s| s.to_string()),
                    ..Default::default()
                };
                self.registry.write(path, data, options)
            }
        }
    }

    fn parse_csv_data(&self, data: &str) -> Vec<Vec<String>> {
        let cursor = Cursor::new(data);
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(cursor);

        let mut result = Vec::with_capacity(128); // Pre-allocate for performance

        for record in reader.records() {
            match record {
                Ok(r) => {
                    let row: Vec<String> = r.iter().map(|s| s.to_string()).collect();
                    result.push(row);
                }
                Err(_) => {
                    // Skip malformed rows
                    continue;
                }
            }
        }

        result
    }
}
