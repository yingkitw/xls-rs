//! Handler registry for unified file format handling (DRY, KISS, SOC)

use crate::columnar::{AvroHandler, ParquetHandler};
use crate::csv_handler::CsvHandler;
use crate::excel::ExcelHandler;
use crate::format_detector::DefaultFormatDetector;
use crate::google_sheets::GoogleSheetsHandler;
use crate::traits::FormatDetector;
use crate::traits::{DataReader, DataWriteOptions, DataWriter, FileHandler};
use anyhow::Result;

/// Registry that manages file handlers by format
pub struct HandlerRegistry {
    format_detector: DefaultFormatDetector,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self {
            format_detector: DefaultFormatDetector::new(),
        }
    }

    /// Get a handler for reading a file based on its format
    pub fn get_reader(&self, path: &str) -> Result<Box<dyn DataReader>> {
        let format = self.format_detector.detect_format(path)?;

        match format.as_str() {
            "csv" => Ok(Box::new(CsvHandler::new())),
            "xlsx" | "xls" | "ods" => Ok(Box::new(ExcelHandler::new())),
            "parquet" => Ok(Box::new(ParquetHandler::new())),
            "avro" => Ok(Box::new(AvroHandler::new())),
            "gsheet" => Ok(Box::new(GoogleSheetsHandler::new())),
            _ => anyhow::bail!("Unsupported format: {format}"),
        }
    }

    /// Get a handler for writing a file based on its format
    pub fn get_writer(&self, path: &str) -> Result<Box<dyn DataWriter>> {
        let format = self.format_detector.detect_format(path)?;

        match format.as_str() {
            "csv" => Ok(Box::new(CsvHandler::new())),
            "xlsx" | "xls" | "ods" => Ok(Box::new(ExcelHandler::new())),
            "parquet" => Ok(Box::new(ParquetHandler::new())),
            "avro" => Ok(Box::new(AvroHandler::new())),
            "gsheet" => Ok(Box::new(GoogleSheetsHandler::new())),
            _ => anyhow::bail!("Unsupported format: {format}"),
        }
    }

    /// Get a file handler (both read and write)
    pub fn get_handler(&self, path: &str) -> Result<Box<dyn FileHandler>> {
        let format = self.format_detector.detect_format(path)?;

        match format.as_str() {
            "csv" => Ok(Box::new(CsvHandler::new())),
            "parquet" => Ok(Box::new(ParquetHandler::new())),
            "avro" => Ok(Box::new(AvroHandler::new())),
            "gsheet" => Ok(Box::new(GoogleSheetsHandler::new())),
            _ => anyhow::bail!("Unsupported format: {format}"),
        }
    }

    /// Read data from any supported format
    pub fn read(&self, path: &str) -> Result<Vec<Vec<String>>> {
        let reader = self.get_reader(path)?;
        reader.read(path)
    }

    /// Write data to any supported format
    pub fn write(&self, path: &str, data: &[Vec<String>], options: DataWriteOptions) -> Result<()> {
        let writer = self.get_writer(path)?;
        writer.write(path, data, options)
    }
}
