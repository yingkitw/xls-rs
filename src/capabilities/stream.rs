//! Stream capability

use crate::capabilities::{Capability, CapabilityMetadata};
use crate::streaming::{CsvStreamingReader, StreamingDataReader};
use anyhow::{Context, Result};
use serde_json::{json, Value};

pub struct StreamCapability;

impl Capability for StreamCapability {
    fn metadata(&self) -> CapabilityMetadata {
        CapabilityMetadata {
            name: "stream".to_string(),
            description: "Stream-process a large CSV file in chunks to another CSV".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "Input CSV file path" },
                    "output": { "type": "string", "description": "Output CSV file path" },
                    "chunk_size": { "type": "integer", "description": "Rows per chunk (default: 1000)" }
                },
                "required": ["input", "output"]
            }),
        }
    }

    fn execute(&self, args: Value) -> Result<Value> {
        let input = args["input"].as_str().context("Missing input")?;
        let output = args["output"].as_str().context("Missing output")?;
        let chunk_size = args["chunk_size"].as_u64().unwrap_or(1000) as usize;

        if !input.ends_with(".csv") {
            let converter = crate::converter::Converter::new();
            let data = converter.read_any_data(input, None)?;
            converter.write_any_data(output, &data, None)?;
            return Ok(json!({
                "status": "success",
                "mode": "fallback",
                "total_rows": data.len(),
                "total_chunks": 1,
            }));
        }

        let mut reader = CsvStreamingReader::new(input)?;
        let mut writer = csv::WriterBuilder::new()
            .from_path(output)
            .map_err(|e| anyhow::anyhow!("Failed to create output CSV: {}", e))?;

        let chunk_size = if chunk_size == 0 { 1000 } else { chunk_size };
        let mut total_rows = 0usize;
        let mut total_chunks = 0usize;

        while reader.has_more() {
            if let Some(chunk) = reader.read_chunk(chunk_size)? {
                for row in &chunk.data {
                    writer.write_record(row)?;
                }
                total_rows += chunk.data.len();
                total_chunks += 1;
            }
        }

        writer.flush()?;

        Ok(json!({
            "status": "success",
            "mode": "streaming",
            "total_rows": total_rows,
            "total_chunks": total_chunks,
        }))
    }
}
