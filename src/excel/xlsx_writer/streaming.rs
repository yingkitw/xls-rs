//! Streaming XLSX writer for large file operations.
//!
//! Buffers rows and writes them in a single pass to the ZIP archive,
//! but accepts data incrementally to avoid building the entire dataset in memory upfront.

use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufWriter, Seek, Write};

use super::types::RowData;
use super::XlsxWriter;
use crate::excel::types::WriteOptions;

/// Streaming XLSX writer that accepts rows one at a time.
///
/// Usage:
/// ```no_run
/// use xls_rs::StreamingXlsxWriter;
/// let mut writer = StreamingXlsxWriter::create("output.xlsx", "Sheet1").unwrap();
/// writer.write_row(&["Name".to_string(), "Value".to_string()]).unwrap();
/// writer.write_row(&["Alice".to_string(), "100".to_string()]).unwrap();
/// writer.finish().unwrap();
/// ```
pub struct StreamingXlsxWriter {
    inner: XlsxWriter,
    path: String,
    rows_written: usize,
}

impl StreamingXlsxWriter {
    /// Create a new streaming XLSX writer
    pub fn create(path: &str, sheet_name: &str) -> Result<Self> {
        let mut inner = XlsxWriter::new();
        inner.add_sheet(sheet_name)?;
        Ok(Self {
            inner,
            path: path.to_string(),
            rows_written: 0,
        })
    }

    /// Create with custom write options
    pub fn create_with_options(
        path: &str,
        sheet_name: &str,
        options: WriteOptions,
    ) -> Result<Self> {
        let mut inner = XlsxWriter::with_options(options);
        inner.add_sheet(sheet_name)?;
        Ok(Self {
            inner,
            path: path.to_string(),
            rows_written: 0,
        })
    }

    /// Write a row of string values (auto-detects numbers)
    pub fn write_row(&mut self, values: &[String]) -> Result<()> {
        let mut row = RowData::new();
        for val in values {
            if let Ok(num) = val.parse::<f64>() {
                row.add_number(num);
            } else if !val.is_empty() {
                row.add_string(val);
            } else {
                row.add_empty();
            }
        }
        self.inner.add_row(row);
        self.rows_written += 1;
        Ok(())
    }

    /// Write a pre-built RowData
    pub fn write_row_data(&mut self, row: RowData) -> Result<()> {
        self.inner.add_row(row);
        self.rows_written += 1;
        Ok(())
    }

    /// Get the number of rows written so far
    pub fn rows_written(&self) -> usize {
        self.rows_written
    }

    /// Finalize and write the XLSX file to disk
    pub fn finish(self) -> Result<()> {
        let file = File::create(&self.path)
            .with_context(|| format!("Failed to create XLSX file: {}", self.path))?;
        let buf = BufWriter::with_capacity(64 * 1024, file);
        self.inner.save(buf)?;
        Ok(())
    }

    /// Finalize and write to an arbitrary writer
    pub fn finish_to_writer<W: Write + Seek>(self, writer: W) -> Result<()> {
        self.inner.save(writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn temp_xlsx(name: &str) -> (TempDir, String) {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let path = dir.path().join(name).to_string_lossy().into_owned();
        (dir, path)
    }

    #[test]
    fn test_streaming_xlsx_basic() {
        let (_dir, path) = temp_xlsx("test_streaming_basic.xlsx");
        let mut writer = StreamingXlsxWriter::create(&path, "Data").unwrap();

        writer.write_row(&["Name".to_string(), "Score".to_string()]).unwrap();
        writer.write_row(&["Alice".to_string(), "95".to_string()]).unwrap();
        writer.write_row(&["Bob".to_string(), "87".to_string()]).unwrap();

        assert_eq!(writer.rows_written(), 3);
        writer.finish().unwrap();

        assert!(std::path::Path::new(&path).exists());
        let metadata = fs::metadata(&path).unwrap();
        assert!(metadata.len() > 100);
    }

    #[test]
    fn test_streaming_xlsx_large() {
        let (_dir, path) = temp_xlsx("test_streaming_large.xlsx");
        let mut writer = StreamingXlsxWriter::create(&path, "BigData").unwrap();

        // Write header
        writer.write_row(&["ID".to_string(), "Value".to_string(), "Label".to_string()]).unwrap();

        // Write 10000 rows
        for i in 0..10_000 {
            writer.write_row(&[
                format!("{}", i),
                format!("{:.2}", i as f64 * 1.5),
                format!("Row_{}", i),
            ]).unwrap();
        }

        assert_eq!(writer.rows_written(), 10_001);
        writer.finish().unwrap();

        assert!(std::path::Path::new(&path).exists());
        let metadata = fs::metadata(&path).unwrap();
        assert!(metadata.len() > 10_000); // Should be substantial
    }

    #[test]
    fn test_streaming_xlsx_write_row_data() {
        let (_dir, path) = temp_xlsx("test_streaming_row_data.xlsx");
        let mut writer = StreamingXlsxWriter::create(&path, "Sheet1").unwrap();

        let mut row = RowData::new();
        row.add_string("Hello");
        row.add_number(42.0);
        row.add_empty();
        writer.write_row_data(row).unwrap();

        assert_eq!(writer.rows_written(), 1);
        writer.finish().unwrap();

        assert!(std::path::Path::new(&path).exists());
    }

    #[test]
    fn test_streaming_xlsx_with_options() {
        use crate::excel::types::WriteOptions;

        let (_dir, path) = temp_xlsx("test_streaming_options.xlsx");
        let options = WriteOptions {
            sheet_name: Some("Custom".to_string()),
            freeze_header: true,
            auto_filter: true,
            ..Default::default()
        };
        let mut writer =
            StreamingXlsxWriter::create_with_options(&path, "Custom", options).unwrap();

        writer.write_row(&["Col1".to_string(), "Col2".to_string()]).unwrap();
        writer.write_row(&["a".to_string(), "1".to_string()]).unwrap();

        assert_eq!(writer.rows_written(), 2);
        writer.finish().unwrap();

        assert!(std::path::Path::new(&path).exists());
    }

    #[test]
    fn test_streaming_xlsx_empty() {
        let (_dir, path) = temp_xlsx("test_streaming_empty.xlsx");
        let writer = StreamingXlsxWriter::create(&path, "Empty").unwrap();
        assert_eq!(writer.rows_written(), 0);
        writer.finish().unwrap();

        assert!(std::path::Path::new(&path).exists());
    }

    #[test]
    fn test_streaming_xlsx_finish_to_writer() {
        let mut buf = std::io::Cursor::new(Vec::new());
        let (_dir, path) = temp_xlsx("unused.xlsx");
        let mut writer = StreamingXlsxWriter::create(&path, "Sheet1").unwrap();
        writer.write_row(&["test".to_string()]).unwrap();

        // finish_to_writer writes to the provided writer, not to the file path
        writer.finish_to_writer(&mut buf).unwrap();
        assert!(buf.get_ref().len() > 100);
    }
}
