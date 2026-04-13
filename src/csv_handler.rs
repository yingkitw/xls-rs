use crate::traits::{
    CellRangeProvider, DataReader, DataWriteOptions, DataWriter, FileHandler, SchemaProvider,
};
use anyhow::{Context, Result};
use csv::{ReaderBuilder, WriterBuilder};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};

/// Represents a cell range like A1:B3
#[derive(Debug, Clone)]
pub struct CellRange {
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
}

impl CellRange {
    /// Parse a range string like "A1:B3" or "A1"
    pub fn parse(range_str: &str) -> Result<Self> {
        let range_str = range_str.trim().to_uppercase();

        if let Some(colon_pos) = range_str.find(':') {
            let start = &range_str[..colon_pos];
            let end = &range_str[colon_pos + 1..];
            let (start_row, start_col) = Self::parse_cell(start)?;
            let (end_row, end_col) = Self::parse_cell(end)?;
            Ok(Self {
                start_row,
                start_col,
                end_row,
                end_col,
            })
        } else {
            let (row, col) = Self::parse_cell(&range_str)?;
            Ok(Self {
                start_row: row,
                start_col: col,
                end_row: row,
                end_col: col,
            })
        }
    }

    fn parse_cell(cell: &str) -> Result<(usize, usize)> {
        let mut col_str = String::new();
        let mut row_str = String::new();

        for ch in cell.chars() {
            if ch.is_alphabetic() {
                col_str.push(ch);
            } else if ch.is_ascii_digit() {
                row_str.push(ch);
            }
        }

        let col = Self::column_to_index(&col_str)?;
        let row = row_str
            .parse::<usize>()
            .with_context(|| format!("Invalid row in cell: {cell}"))?;

        Ok((row.saturating_sub(1), col)) // Convert to 0-indexed
    }

    fn column_to_index(col: &str) -> Result<usize> {
        if col.is_empty() {
            anyhow::bail!("Empty column reference");
        }
        let mut index = 0usize;
        for ch in col.chars() {
            index = index * 26 + (ch.to_ascii_uppercase() as usize - b'A' as usize + 1);
        }
        Ok(index - 1)
    }
}

pub struct CsvHandler;

impl CsvHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn read(&self, path: &str) -> Result<String> {
        let mut file =
            File::open(path).with_context(|| format!("Failed to open CSV file: {path}"))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        Ok(contents)
    }

    pub fn write_from_csv(&self, input_path: &str, output_path: &str) -> Result<()> {
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .from_path(input_path)
            .with_context(|| format!("Failed to open CSV file: {input_path}"))?;

        let mut writer = WriterBuilder::new()
            .has_headers(false)
            .from_path(output_path)
            .with_context(|| format!("Failed to create CSV file: {}", output_path))?;

        for result in reader.records() {
            let record = result?;
            writer.write_record(&record)?;
        }

        writer.flush()?;
        Ok(())
    }

    pub fn write_records(&self, path: &str, records: Vec<Vec<String>>) -> Result<()> {
        let mut writer = WriterBuilder::new()
            .has_headers(false)
            .from_path(path)
            .with_context(|| format!("Failed to create CSV file: {path}"))?;

        for record in records {
            writer.write_record(&record)?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Read a specific range from CSV file
    pub fn read_range(&self, path: &str, range: &CellRange) -> Result<Vec<Vec<String>>> {
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(path)
            .with_context(|| format!("Failed to open CSV file: {path}"))?;

        let estimated_rows = range.end_row.saturating_sub(range.start_row) + 1;
        let _estimated_cols = range.end_col.saturating_sub(range.start_col) + 1;
        let mut result = Vec::with_capacity(estimated_rows.min(1024));

        for (row_idx, record) in reader.records().enumerate() {
            if row_idx < range.start_row {
                continue;
            }
            if row_idx > range.end_row {
                break;
            }

            let record = record?;
            // Pre-allocate with exact capacity to avoid reallocations
            let num_cols = (range.end_col.saturating_sub(range.start_col) + 1)
                .min(record.len().saturating_sub(range.start_col));
            let mut row = Vec::with_capacity(num_cols);
            for col_idx in range.start_col..=range.end_col {
                if let Some(val) = record.get(col_idx) {
                    row.push(String::from(val));
                }
            }
            result.push(row);
        }

        Ok(result)
    }

    /// Read CSV and return as JSON array
    pub fn read_as_json(&self, path: &str) -> Result<String> {
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(path)
            .with_context(|| format!("Failed to open CSV file: {path}"))?;

        let mut rows: Vec<Vec<String>> = Vec::with_capacity(1024);
        for record in reader.records() {
            let record = record?;
            // Pre-allocate based on record length
            let mut row = Vec::with_capacity(record.len());
            for val in record.iter() {
                row.push(String::from(val));
            }
            rows.push(row);
        }

        serde_json::to_string_pretty(&rows).with_context(|| "Failed to serialize to JSON")
    }

    /// Append records to an existing CSV file (or create if doesn't exist)
    pub fn append_records(&self, path: &str, records: &[Vec<String>]) -> Result<()> {
        use std::fs::OpenOptions;

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("Failed to open CSV file for append: {path}"))?;

        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(file);

        for record in records {
            writer.write_record(record)?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Write data to a specific cell range in CSV
    pub fn write_range(
        &self,
        path: &str,
        data: &[Vec<String>],
        start_row: usize,
        start_col: usize,
    ) -> Result<()> {
        // Read existing data if file exists
        let mut existing: Vec<Vec<String>> = if std::path::Path::new(path).exists() {
            let mut reader = ReaderBuilder::new()
                .has_headers(false)
                .flexible(true)
                .from_path(path)?;
            reader
                .records()
                .map(|r| {
                    let record =
                        r.with_context(|| format!("Failed to read CSV record from: {path}"))?;
                    Ok(record.iter().map(|s| s.to_string()).collect())
                })
                .collect::<Result<Vec<Vec<String>>>>()?
        } else {
            Vec::with_capacity(start_row + data.len())
        };

        // Expand existing data if needed
        let needed_rows = start_row + data.len();
        if existing.len() < needed_rows {
            existing.resize_with(needed_rows, Vec::new);
        }

        // Write data to range
        for (row_idx, row) in data.iter().enumerate() {
            let target_row = start_row + row_idx;
            let needed_cols = start_col + row.len();

            if existing[target_row].len() < needed_cols {
                existing[target_row].resize(needed_cols, String::new());
            }

            // Direct assignment instead of clone when possible
            for (col_idx, value) in row.iter().enumerate() {
                existing[target_row][start_col + col_idx] = value.clone();
            }
        }

        self.write_records(path, existing)
    }
}

/// Characters that can trigger formula injection in spreadsheet applications
const CSV_INJECTION_CHARS: &[char] = &['=', '+', '-', '@', '\t', '\r', '\n'];

/// Sanitize a cell value to prevent CSV delimiter injection.
/// Prefixes dangerous leading characters with a single quote to neutralize them.
/// Also handles embedded newlines by quoting the value.
pub fn sanitize_csv_value(value: &str) -> String {
    if value.is_empty() {
        return value.to_string();
    }
    let first = value.chars().next().unwrap();
    if CSV_INJECTION_CHARS.contains(&first) {
        format!("'{}", value)
    } else {
        value.to_string()
    }
}

/// Sanitize an entire row of CSV values
pub fn sanitize_csv_row(row: &[String]) -> Vec<String> {
    row.iter().map(|v| sanitize_csv_value(v)).collect()
}

impl CsvHandler {
    /// Write records with CSV injection protection
    pub fn write_records_safe(&self, path: &str, records: Vec<Vec<String>>) -> Result<()> {
        let sanitized: Vec<Vec<String>> = records.iter().map(|row| sanitize_csv_row(row)).collect();
        self.write_records(path, sanitized)
    }

    /// Append records with CSV injection protection
    pub fn append_records_safe(&self, path: &str, records: &[Vec<String>]) -> Result<()> {
        let sanitized: Vec<Vec<String>> = records.iter().map(|row| sanitize_csv_row(row)).collect();
        self.append_records(path, &sanitized)
    }
}

/// Streaming CSV reader for large files - processes rows one at a time
pub struct StreamingCsvReader {
    reader: csv::Reader<BufReader<File>>,
    current_row: usize,
}

impl StreamingCsvReader {
    pub fn open(path: &str) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("Failed to open CSV file: {path}"))?;
        let buf_reader = BufReader::with_capacity(64 * 1024, file); // 64KB buffer

        let reader = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(buf_reader);

        Ok(Self {
            reader,
            current_row: 0,
        })
    }

    pub fn current_row(&self) -> usize {
        self.current_row
    }
}

impl Iterator for StreamingCsvReader {
    type Item = Result<Vec<String>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.records().next() {
            Some(Ok(record)) => {
                self.current_row += 1;
                // Pre-allocate capacity to avoid reallocations
                let mut row = Vec::with_capacity(record.len());
                for val in record.iter() {
                    row.push(String::from(val));
                }
                Some(Ok(row))
            }
            Some(Err(e)) => Some(Err(anyhow::anyhow!("CSV read error: {}", e))),
            None => None,
        }
    }
}

/// Streaming CSV writer for large files
pub struct StreamingCsvWriter {
    writer: csv::Writer<BufWriter<File>>,
    rows_written: usize,
}

impl StreamingCsvWriter {
    pub fn create(path: &str) -> Result<Self> {
        let file =
            File::create(path).with_context(|| format!("Failed to create CSV file: {path}"))?;
        let buf_writer = BufWriter::with_capacity(64 * 1024, file);

        let writer = WriterBuilder::new()
            .has_headers(false)
            .from_writer(buf_writer);

        Ok(Self {
            writer,
            rows_written: 0,
        })
    }

    pub fn write_row(&mut self, row: &[String]) -> Result<()> {
        self.writer.write_record(row)?;
        self.rows_written += 1;
        Ok(())
    }

    pub fn rows_written(&self) -> usize {
        self.rows_written
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}

impl Drop for StreamingCsvWriter {
    fn drop(&mut self) {
        let _ = self.writer.flush();
    }
}

// Trait implementations for CsvHandler

impl DataReader for CsvHandler {
    fn read(&self, path: &str) -> Result<Vec<Vec<String>>> {
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(path)
            .with_context(|| format!("Failed to open CSV file: {path}"))?;

        // Pre-allocate with capacity hint for better performance
        let mut rows = Vec::with_capacity(1024);
        for record in reader.records() {
            let record = record?;
            let mut row = Vec::with_capacity(record.len());
            for val in record.iter() {
                row.push(String::from(val));
            }
            rows.push(row);
        }
        Ok(rows)
    }

    fn read_with_headers(&self, path: &str) -> Result<Vec<Vec<String>>> {
        // For CSV, headers are just the first row
        <Self as DataReader>::read(self, path)
    }

    fn read_range(&self, path: &str, range: &CellRange) -> Result<Vec<Vec<String>>> {
        CsvHandler::read_range(self, path, range)
    }

    fn read_as_json(&self, path: &str) -> Result<String> {
        CsvHandler::read_as_json(self, path)
    }

    fn supports_format(&self, path: &str) -> bool {
        path.to_lowercase().ends_with(".csv")
    }
}

impl DataWriter for CsvHandler {
    fn write(&self, path: &str, data: &[Vec<String>], _options: DataWriteOptions) -> Result<()> {
        self.write_records(path, data.to_vec())
    }

    fn write_range(
        &self,
        path: &str,
        data: &[Vec<String>],
        start_row: usize,
        start_col: usize,
    ) -> Result<()> {
        self.write_range(path, data, start_row, start_col)
    }

    fn append(&self, path: &str, data: &[Vec<String>]) -> Result<()> {
        self.append_records(path, data)
    }

    fn supports_format(&self, path: &str) -> bool {
        path.to_lowercase().ends_with(".csv")
    }
}

impl FileHandler for CsvHandler {
    fn format_name(&self) -> &'static str {
        "csv"
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &["csv"]
    }
}

impl SchemaProvider for CsvHandler {
    fn get_schema(&self, path: &str) -> Result<Vec<(String, String)>> {
        let data = <Self as DataReader>::read(self, path)?;
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let num_cols = data[0].len();
        Ok((0..num_cols)
            .map(|i| (format!("col_{}", i), "string".to_string()))
            .collect())
    }

    fn get_column_names(&self, path: &str) -> Result<Vec<String>> {
        let data = <Self as DataReader>::read(self, path)?;
        if data.is_empty() {
            return Ok(Vec::new());
        }

        Ok(data[0].clone())
    }

    fn get_row_count(&self, path: &str) -> Result<usize> {
        let data = <Self as DataReader>::read(self, path)?;
        Ok(data.len())
    }

    fn get_column_count(&self, path: &str) -> Result<usize> {
        let data = <Self as DataReader>::read(self, path)?;
        Ok(data.first().map(|r| r.len()).unwrap_or(0))
    }
}

/// Helper struct for CellRangeProvider implementation
pub struct CellRangeHelper;

impl CellRangeProvider for CellRangeHelper {
    fn parse_range(&self, range_str: &str) -> Result<CellRange> {
        CellRange::parse(range_str)
    }

    fn to_cell_reference(&self, row: usize, col: usize) -> String {
        CellRange::index_to_column(col, row)
    }

    fn from_cell_reference(&self, cell: &str) -> Result<(usize, usize)> {
        CellRange::parse_cell(cell)
    }
}

impl CellRange {
    /// Convert column index and row to cell reference (e.g., (0, 0) -> "A1")
    pub fn index_to_column(col: usize, row: usize) -> String {
        let mut index = col;
        index += 1; // Convert to 1-based
        let mut result = String::new();
        while index > 0 {
            index -= 1;
            result.push((b'A' + (index % 26) as u8) as char);
            index /= 26;
        }
        let col_str: String = result.chars().rev().collect();
        format!("{}{}", col_str, row + 1)
    }
}
