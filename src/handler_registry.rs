//! Handler registry for unified file format handling

use crate::excel::ExcelHandler;
use crate::format_detector::DefaultFormatDetector;
use crate::traits::FormatDetector;
use crate::traits::{DataReader, DataWriteOptions, DataWriter, FileHandler};
use anyhow::Result;

/// Registry that manages file handlers by format
pub struct HandlerRegistry {
    format_detector: DefaultFormatDetector,
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self {
            format_detector: DefaultFormatDetector::new(),
        }
    }

    pub fn get_reader(&self, path: &str) -> Result<Box<dyn DataReader>> {
        let format = self.format_detector.detect_format(path)?;
        match format.as_str() {
            "xlsx" => Ok(Box::new(ExcelHandler::new())),
            _ => anyhow::bail!("Unsupported format: {format}. Only XLSX is supported."),
        }
    }

    pub fn get_writer(&self, path: &str) -> Result<Box<dyn DataWriter>> {
        let format = self.format_detector.detect_format(path)?;
        match format.as_str() {
            "xlsx" => Ok(Box::new(ExcelHandler::new())),
            _ => anyhow::bail!("Unsupported format: {format}. Only XLSX is supported."),
        }
    }

    pub fn get_handler(&self, path: &str) -> Result<Box<dyn FileHandler>> {
        let format = self.format_detector.detect_format(path)?;
        match format.as_str() {
            "xlsx" => Ok(Box::new(ExcelHandler::new())),
            _ => anyhow::bail!("Unsupported format: {format}. Only XLSX is supported."),
        }
    }

    pub fn read(&self, path: &str) -> Result<Vec<Vec<String>>> {
        let reader = self.get_reader(path)?;
        reader.read(path)
    }

    pub fn write(&self, path: &str, data: &[Vec<String>], options: DataWriteOptions) -> Result<()> {
        let writer = self.get_writer(path)?;
        writer.write(path, data, options)
    }
}
