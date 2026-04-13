//! Avro file handling

use anyhow::{Context, Result};
use std::fs::File;

use apache_avro::{
    types::Value as AvroValue, Reader as AvroReader, Schema as AvroSchema, Writer as AvroWriter,
};

use crate::csv_handler::CellRange;
use crate::helpers::{default_column_names, filter_by_range, max_column_count};
use crate::traits::{DataReader, DataWriteOptions, DataWriter, FileHandler, SchemaProvider};

/// Handler for Avro files
#[derive(Default)]
pub struct AvroHandler;

impl AvroHandler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read Avro file into `Vec<Vec<String>>`
    pub fn read(&self, path: &str) -> Result<Vec<Vec<String>>> {
        let file = File::open(path).with_context(|| format!("Failed to open Avro file: {path}"))?;

        let reader = AvroReader::new(file)?;
        let mut all_rows: Vec<Vec<String>> = Vec::new();

        for value in reader {
            let value = value?;
            if let AvroValue::Record(fields) = value {
                let row: Vec<String> = fields
                    .iter()
                    .map(|(_, v)| self.avro_value_to_string(v))
                    .collect();
                all_rows.push(row);
            }
        }

        Ok(all_rows)
    }

    /// Read Avro file with field names as first row
    pub fn read_with_headers(&self, path: &str) -> Result<Vec<Vec<String>>> {
        let file = File::open(path).with_context(|| format!("Failed to open Avro file: {path}"))?;

        let reader = AvroReader::new(file)?;
        let mut all_rows: Vec<Vec<String>> = Vec::new();

        // Get field names from schema
        if let AvroSchema::Record(record) = reader.writer_schema() {
            let headers: Vec<String> = record.fields.iter().map(|f| f.name.clone()).collect();
            all_rows.push(headers);
        }

        for value in reader {
            let value = value?;
            if let AvroValue::Record(fields) = value {
                let row: Vec<String> = fields
                    .iter()
                    .map(|(_, v)| self.avro_value_to_string(v))
                    .collect();
                all_rows.push(row);
            }
        }

        Ok(all_rows)
    }

    /// Write data to Avro file (all fields as strings)
    pub fn write(
        &self,
        path: &str,
        data: &[Vec<String>],
        field_names: Option<&[String]>,
    ) -> Result<()> {
        if data.is_empty() {
            anyhow::bail!("Cannot write empty data to Avro");
        }

        let num_cols = max_column_count(data);

        // Generate field names if not provided
        let names: Vec<String> = field_names
            .map(|n| n.to_vec())
            .unwrap_or_else(|| default_column_names(num_cols, "field"));

        // Build Avro schema
        let schema_json = format!(
            r#"{{
                "type": "record",
                "name": "Row",
                "fields": [{}]
            }}"#,
            names
                .iter()
                .map(|n| format!(r#"{{"name": "{}", "type": ["null", "string"]}}"#, n))
                .collect::<Vec<_>>()
                .join(", ")
        );

        let schema = AvroSchema::parse_str(&schema_json)?;

        let file =
            File::create(path).with_context(|| format!("Failed to create Avro file: {path}"))?;

        {
            let mut writer = AvroWriter::new(&schema, file);

            for row in data {
                let mut record: Vec<(String, AvroValue)> = Vec::new();
                for (i, name) in names.iter().enumerate() {
                    let value = row
                        .get(i)
                        .map(|s| AvroValue::Union(1, Box::new(AvroValue::String(s.clone()))))
                        .unwrap_or(AvroValue::Union(0, Box::new(AvroValue::Null)));
                    record.push((name.clone(), value));
                }
                writer.append(AvroValue::Record(record))?;
            }

            // Flush and finalize the writer - this is critical for Avro format
            writer.flush()?;
            // Writer is dropped here, which finalizes the Avro file
        }

        Ok(())
    }

    /// Get schema information from Avro file
    pub fn get_schema(&self, path: &str) -> Result<Vec<(String, String)>> {
        let file = File::open(path).with_context(|| format!("Failed to open Avro file: {path}"))?;

        let reader = AvroReader::new(file)?;

        let fields = if let AvroSchema::Record(record) = reader.writer_schema() {
            record
                .fields
                .iter()
                .map(|f| (f.name.clone(), format!("{:?}", f.schema)))
                .collect()
        } else {
            Vec::new()
        };

        Ok(fields)
    }

    fn avro_value_to_string(&self, value: &AvroValue) -> String {
        match value {
            AvroValue::Null => String::new(),
            AvroValue::Boolean(b) => b.to_string(),
            AvroValue::Int(i) => i.to_string(),
            AvroValue::Long(l) => l.to_string(),
            AvroValue::Float(f) => f.to_string(),
            AvroValue::Double(d) => d.to_string(),
            AvroValue::String(s) => s.clone(),
            AvroValue::Bytes(b) => String::from_utf8_lossy(b).to_string(),
            AvroValue::Union(_, inner) => self.avro_value_to_string(inner),
            AvroValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| self.avro_value_to_string(v)).collect();
                format!("[{}]", items.join(", "))
            }
            AvroValue::Map(map) => {
                let items: Vec<String> = map
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.avro_value_to_string(v)))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            _ => format!("{:?}", value),
        }
    }
}

impl DataReader for AvroHandler {
    fn read(&self, path: &str) -> Result<Vec<Vec<String>>> {
        self.read(path)
    }

    fn read_with_headers(&self, path: &str) -> Result<Vec<Vec<String>>> {
        self.read_with_headers(path)
    }

    fn read_range(&self, path: &str, range: &CellRange) -> Result<Vec<Vec<String>>> {
        let all_data = self.read(path)?;
        Ok(filter_by_range(&all_data, range))
    }

    fn read_as_json(&self, path: &str) -> Result<String> {
        let data = self.read(path)?;
        serde_json::to_string_pretty(&data).with_context(|| "Failed to serialize to JSON")
    }

    fn supports_format(&self, path: &str) -> bool {
        path.to_lowercase().ends_with(".avro")
    }
}

impl DataWriter for AvroHandler {
    fn write(&self, path: &str, data: &[Vec<String>], options: DataWriteOptions) -> Result<()> {
        self.write(path, data, options.column_names.as_deref())
    }

    fn write_range(
        &self,
        path: &str,
        data: &[Vec<String>],
        _start_row: usize,
        _start_col: usize,
    ) -> Result<()> {
        // For Avro, we write the entire dataset
        self.write(path, data, None)
    }

    fn append(&self, _path: &str, _data: &[Vec<String>]) -> Result<()> {
        anyhow::bail!("Append operation not supported for Avro files")
    }

    fn supports_format(&self, path: &str) -> bool {
        path.to_lowercase().ends_with(".avro")
    }
}

impl FileHandler for AvroHandler {
    fn format_name(&self) -> &'static str {
        "avro"
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &["avro"]
    }
}

impl SchemaProvider for AvroHandler {
    fn get_schema(&self, path: &str) -> Result<Vec<(String, String)>> {
        self.get_schema(path)
    }

    fn get_column_names(&self, path: &str) -> Result<Vec<String>> {
        let schema = self.get_schema(path)?;
        Ok(schema.into_iter().map(|(name, _)| name).collect())
    }

    fn get_row_count(&self, path: &str) -> Result<usize> {
        let data = self.read(path)?;
        Ok(data.len())
    }

    fn get_column_count(&self, path: &str) -> Result<usize> {
        let data = self.read(path)?;
        Ok(data.first().map(|r| r.len()).unwrap_or(0))
    }
}
