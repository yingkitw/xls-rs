//! Mock implementations of traits for testing

use crate::csv_handler::{CellRange, CellRangeHelper};
use crate::traits::{
    DataOperator, DataReader, DataWriteOptions, DataWriter, FileHandler,
    FilterCondition, FilterOperator, SortOperator, TransformOperation,
    TransformOperator,
};
use anyhow::Result;

/// Mock data reader for testing
pub struct MockDataReader {
    pub data: Vec<Vec<String>>,
}

impl MockDataReader {
    pub fn new(data: Vec<Vec<String>>) -> Self {
        Self { data }
    }
}

impl DataReader for MockDataReader {
    fn read(&self, _path: &str) -> Result<Vec<Vec<String>>> {
        Ok(self.data.clone())
    }

    fn read_with_headers(&self, _path: &str) -> Result<Vec<Vec<String>>> {
        Ok(self.data.clone())
    }

    fn read_range(&self, _path: &str, range: &CellRange) -> Result<Vec<Vec<String>>> {
        use crate::helpers::filter_by_range;
        Ok(filter_by_range(&self.data, range))
    }

    fn read_as_json(&self, _path: &str) -> Result<String> {
        serde_json::to_string_pretty(&self.data)
            .map_err(|e| anyhow::anyhow!("JSON serialization error: {}", e))
    }

    fn supports_format(&self, path: &str) -> bool {
        path.ends_with(".mock")
    }
}

/// Mock data writer for testing
pub struct MockDataWriter {
    pub written_data: std::sync::Arc<std::sync::Mutex<Vec<Vec<String>>>>,
}

impl MockDataWriter {
    pub fn new() -> Self {
        Self {
            written_data: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn get_written(&self) -> Vec<Vec<String>> {
        self.written_data.lock().unwrap().clone()
    }
}

impl DataWriter for MockDataWriter {
    fn write(&self, _path: &str, data: &[Vec<String>], _options: DataWriteOptions) -> Result<()> {
        *self.written_data.lock().unwrap() = data.to_vec();
        Ok(())
    }

    fn write_range(
        &self,
        _path: &str,
        data: &[Vec<String>],
        _start_row: usize,
        _start_col: usize,
    ) -> Result<()> {
        *self.written_data.lock().unwrap() = data.to_vec();
        Ok(())
    }

    fn append(&self, _path: &str, data: &[Vec<String>]) -> Result<()> {
        self.written_data.lock().unwrap().extend_from_slice(data);
        Ok(())
    }

    fn supports_format(&self, path: &str) -> bool {
        path.ends_with(".mock")
    }
}

/// Mock file handler combining reader and writer
pub struct MockFileHandler {
    reader: MockDataReader,
    writer: MockDataWriter,
}

impl MockFileHandler {
    pub fn new(data: Vec<Vec<String>>) -> Self {
        Self {
            reader: MockDataReader::new(data),
            writer: MockDataWriter::new(),
        }
    }

    pub fn get_written(&self) -> Vec<Vec<String>> {
        self.writer.get_written()
    }
}

impl FileHandler for MockFileHandler {
    fn format_name(&self) -> &'static str {
        "mock"
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &["mock"]
    }
}

impl DataReader for MockFileHandler {
    fn read(&self, path: &str) -> Result<Vec<Vec<String>>> {
        self.reader.read(path)
    }

    fn read_with_headers(&self, path: &str) -> Result<Vec<Vec<String>>> {
        self.reader.read_with_headers(path)
    }

    fn read_range(&self, path: &str, range: &CellRange) -> Result<Vec<Vec<String>>> {
        self.reader.read_range(path, range)
    }

    fn read_as_json(&self, path: &str) -> Result<String> {
        self.reader.read_as_json(path)
    }

    fn supports_format(&self, path: &str) -> bool {
        self.reader.supports_format(path)
    }
}

impl DataWriter for MockFileHandler {
    fn write(&self, path: &str, data: &[Vec<String>], options: DataWriteOptions) -> Result<()> {
        self.writer.write(path, data, options)
    }

    fn write_range(
        &self,
        path: &str,
        data: &[Vec<String>],
        start_row: usize,
        start_col: usize,
    ) -> Result<()> {
        self.writer.write_range(path, data, start_row, start_col)
    }

    fn append(&self, path: &str, data: &[Vec<String>]) -> Result<()> {
        self.writer.append(path, data)
    }

    fn supports_format(&self, path: &str) -> bool {
        self.writer.supports_format(path)
    }
}

/// Mock sort operator for testing
pub struct MockSortOperator;

impl SortOperator for MockSortOperator {
    fn sort(&self, data: &mut Vec<Vec<String>>, column: usize, ascending: bool) -> Result<()> {
        data.sort_by(|a, b| {
            let val_a = a.get(column).map(|s| s.as_str()).unwrap_or("");
            let val_b = b.get(column).map(|s| s.as_str()).unwrap_or("");
            let cmp = val_a.cmp(val_b);
            if ascending { cmp } else { cmp.reverse() }
        });
        Ok(())
    }
}

/// Mock filter operator for testing
pub struct MockFilterOperator;

impl FilterOperator for MockFilterOperator {
    fn filter(
        &self,
        data: &[Vec<String>],
        column: usize,
        condition: FilterCondition,
    ) -> Result<Vec<Vec<String>>> {
        let mut result = Vec::new();
        for row in data {
            let cell_value = row.get(column).map(|s| s.as_str()).unwrap_or("");
            let matches = match &condition {
                FilterCondition::Equals(v) => cell_value == v,
                FilterCondition::NotEquals(v) => cell_value != v,
                FilterCondition::Contains(v) => cell_value.contains(v),
                _ => false, // Simplified for mock
            };
            if matches {
                result.push(row.clone());
            }
        }
        Ok(result)
    }
}

/// Mock transform operator for testing
pub struct MockTransformOperator;

impl TransformOperator for MockTransformOperator {
    fn transform(&self, data: &mut Vec<Vec<String>>, operation: TransformOperation) -> Result<()> {
        match operation {
            TransformOperation::RenameColumn { from, to } => {
                if let Some(row) = data.first_mut() {
                    if from < row.len() {
                        row[from] = to;
                    }
                }
            }
            TransformOperation::DropColumn(col_idx) => {
                for row in data.iter_mut() {
                    if col_idx < row.len() {
                        row.remove(col_idx);
                    }
                }
            }
            _ => {} // Simplified for mock
        }
        Ok(())
    }
}

// Note: Individual mock operators don't implement DataOperator
// because DataOperator requires all three traits (SortOperator, FilterOperator, TransformOperator)
// For testing, use the individual traits directly or create a combined mock

/// Combined mock operator that implements all three traits
pub struct MockDataOperator;

impl SortOperator for MockDataOperator {
    fn sort(&self, data: &mut Vec<Vec<String>>, column: usize, ascending: bool) -> Result<()> {
        MockSortOperator.sort(data, column, ascending)
    }
}

impl FilterOperator for MockDataOperator {
    fn filter(
        &self,
        data: &[Vec<String>],
        column: usize,
        condition: FilterCondition,
    ) -> Result<Vec<Vec<String>>> {
        MockFilterOperator.filter(data, column, condition)
    }
}

impl TransformOperator for MockDataOperator {
    fn transform(&self, data: &mut Vec<Vec<String>>, operation: TransformOperation) -> Result<()> {
        MockTransformOperator.transform(data, operation)
    }
}

impl DataOperator for MockDataOperator {}

/// Re-export CellRangeHelper as mock cell range provider
pub type MockCellRangeProvider = CellRangeHelper;
