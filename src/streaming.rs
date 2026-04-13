//! Real-time data streaming support
//!
//! Provides streaming capabilities for processing large datasets incrementally.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tokio::sync::broadcast;

/// Streaming data chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataChunk {
    pub sequence: usize,
    pub data: Vec<Vec<String>>,
    pub metadata: ChunkMetadata,
}

/// Chunk metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub timestamp: String,
    pub source: Option<String>,
    pub row_count: usize,
    pub column_count: usize,
}

/// Streaming reader trait
pub trait StreamingDataReader: Send + Sync {
    fn read_chunk(&mut self, chunk_size: usize) -> Result<Option<DataChunk>>;
    fn has_more(&self) -> bool;
    fn reset(&mut self) -> Result<()>;
}

/// Streaming writer trait
pub trait StreamingDataWriter: Send + Sync {
    fn write_chunk(&mut self, chunk: &DataChunk) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
}

/// Streaming processor
pub struct StreamingProcessor {
    buffer_size: usize,
    chunk_size: usize,
}

impl StreamingProcessor {
    pub fn new(chunk_size: usize, buffer_size: usize) -> Self {
        Self {
            chunk_size,
            buffer_size,
        }
    }

    /// Process data in streaming fashion
    pub fn process_streaming<R, W, F>(
        &self,
        reader: &mut R,
        writer: &mut W,
        processor: F,
    ) -> Result<usize>
    where
        R: StreamingDataReader,
        W: StreamingDataWriter,
        F: Fn(&DataChunk) -> Result<DataChunk>,
    {
        let mut total_chunks = 0;
        let mut buffer = VecDeque::new();

        while reader.has_more() {
            if let Some(chunk) = reader.read_chunk(self.chunk_size)? {
                let processed = processor(&chunk)?;

                // Buffer chunks if needed
                buffer.push_back(processed);

                // Write when buffer is full
                if buffer.len() >= self.buffer_size {
                    if let Some(buffered) = buffer.pop_front() {
                        writer.write_chunk(&buffered)?;
                        total_chunks += 1;
                    }
                }
            }
        }

        // Flush remaining chunks
        while let Some(chunk) = buffer.pop_front() {
            writer.write_chunk(&chunk)?;
            total_chunks += 1;
        }

        writer.flush()?;
        Ok(total_chunks)
    }

    /// Stream data with callback
    pub fn stream_with_callback<R, F>(&self, reader: &mut R, callback: F) -> Result<usize>
    where
        R: StreamingDataReader,
        F: Fn(&DataChunk) -> Result<()>,
    {
        let mut total_chunks = 0;

        while reader.has_more() {
            if let Some(chunk) = reader.read_chunk(self.chunk_size)? {
                callback(&chunk)?;
                total_chunks += 1;
            }
        }

        Ok(total_chunks)
    }
}

/// Broadcast-based streaming channel
pub struct StreamingChannel {
    sender: broadcast::Sender<DataChunk>,
    receiver: broadcast::Receiver<DataChunk>,
}

impl StreamingChannel {
    pub fn new(buffer: usize) -> Self {
        let (sender, receiver) = broadcast::channel(buffer);
        Self { sender, receiver }
    }

    pub fn send(&self, chunk: DataChunk) -> Result<usize> {
        self.sender
            .send(chunk)
            .map_err(|e| anyhow::anyhow!("Failed to send chunk: {}", e))
    }

    pub async fn receive(&mut self) -> Result<DataChunk> {
        self.receiver
            .recv()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to receive chunk: {}", e))
    }
}

/// CSV streaming reader implementation
pub struct CsvStreamingReader {
    path: String,
    current_row: usize,
    total_rows: Option<usize>,
    reader: Option<csv::Reader<std::fs::File>>,
}

impl CsvStreamingReader {
    pub fn new(path: &str) -> Result<Self> {
        // Create reader on initialization
        let reader = csv::Reader::from_path(path)
            .map_err(|e| anyhow::anyhow!("Failed to open CSV: {}", e))?;

        Ok(Self {
            path: path.to_string(),
            current_row: 0,
            total_rows: None,
            reader: Some(reader),
        })
    }

    fn ensure_reader(&mut self) -> Result<&mut csv::Reader<std::fs::File>> {
        if self.reader.is_none() {
            self.reader = Some(
                csv::Reader::from_path(&self.path)
                    .map_err(|e| anyhow::anyhow!("Failed to open CSV: {}", e))?,
            );
        }
        self.reader
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Failed to get reader reference"))
    }
}

impl StreamingDataReader for CsvStreamingReader {
    fn read_chunk(&mut self, chunk_size: usize) -> Result<Option<DataChunk>> {
        let start_row = self.current_row;
        let reader = self.ensure_reader()?;

        let mut chunk_data: Vec<Vec<String>> = Vec::new();
        let mut rows_read = 0;

        for result in reader.records().take(chunk_size) {
            let record = result?;
            chunk_data.push(record.iter().map(|s| s.to_string()).collect());
            rows_read += 1;
        }

        // Update current_row after reading
        self.current_row = start_row + rows_read;

        if chunk_data.is_empty() {
            return Ok(None);
        }

        let column_count = chunk_data.first().map(|r| r.len()).unwrap_or(0);

        let sequence = if chunk_size > 0 {
            start_row / chunk_size
        } else {
            0
        };

        Ok(Some(DataChunk {
            sequence,
            data: chunk_data,
            metadata: ChunkMetadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                source: Some(self.path.clone()),
                row_count: rows_read,
                column_count,
            },
        }))
    }

    fn has_more(&self) -> bool {
        // Simplified - in real implementation, would check file position
        self.reader.is_some()
    }

    fn reset(&mut self) -> Result<()> {
        self.reader = Some(csv::Reader::from_path(&self.path)?);
        self.current_row = 0;
        Ok(())
    }
}
