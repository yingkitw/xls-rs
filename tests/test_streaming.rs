//! Tests for streaming module

use std::io::Write;
use xls_rs::streaming::{ChunkMetadata, CsvStreamingReader, DataChunk, StreamingDataReader, StreamingProcessor};

#[test]
fn test_data_chunk_creation() {
    let data = vec![
        vec!["Name".to_string(), "Age".to_string()],
        vec!["Alice".to_string(), "30".to_string()],
    ];

    let metadata = ChunkMetadata {
        timestamp: "2026-01-26T10:00:00Z".to_string(),
        source: Some("test.csv".to_string()),
        row_count: 2,
        column_count: 2,
    };

    let chunk = DataChunk {
        sequence: 1,
        data: data.clone(),
        metadata: metadata.clone(),
    };

    assert_eq!(chunk.sequence, 1);
    assert_eq!(chunk.data.len(), 2);
    assert_eq!(chunk.metadata.row_count, 2);
    assert_eq!(chunk.metadata.column_count, 2);
}

#[test]
fn test_streaming_processor_creation() {
    let _processor = StreamingProcessor::new(1000, 10);
}

#[test]
fn test_chunk_metadata_serialization() {
    let metadata = ChunkMetadata {
        timestamp: "2026-01-26T10:00:00Z".to_string(),
        source: Some("test.csv".to_string()),
        row_count: 100,
        column_count: 5,
    };

    let json = serde_json::to_string(&metadata).unwrap();
    assert!(json.contains("timestamp"));
    assert!(json.contains("test.csv"));
    assert!(json.contains("100"));
}

#[test]
fn test_chunk_metadata_deserialization() {
    let json = r#"{
        "timestamp": "2026-01-26T10:00:00Z",
        "source": "test.csv",
        "row_count": 100,
        "column_count": 5
    }"#;

    let metadata: ChunkMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(metadata.timestamp, "2026-01-26T10:00:00Z");
    assert_eq!(metadata.source, Some("test.csv".to_string()));
    assert_eq!(metadata.row_count, 100);
    assert_eq!(metadata.column_count, 5);
}

#[test]
fn test_data_chunk_clone() {
    let data = vec![vec!["test".to_string()]];
    let metadata = ChunkMetadata {
        timestamp: "2026-01-26T10:00:00Z".to_string(),
        source: None,
        row_count: 1,
        column_count: 1,
    };

    let chunk = DataChunk {
        sequence: 1,
        data: data.clone(),
        metadata: metadata.clone(),
    };

    let cloned = chunk.clone();
    assert_eq!(cloned.sequence, chunk.sequence);
    assert_eq!(cloned.data, chunk.data);
}

#[test]
fn test_chunk_with_large_data() {
    let data: Vec<Vec<String>> = (0..1000)
        .map(|i| vec![format!("row_{}", i), format!("value_{}", i)])
        .collect();

    let metadata = ChunkMetadata {
        timestamp: "2026-01-26T10:00:00Z".to_string(),
        source: Some("large.csv".to_string()),
        row_count: 1000,
        column_count: 2,
    };

    let chunk = DataChunk {
        sequence: 1,
        data,
        metadata,
    };

    assert_eq!(chunk.data.len(), 1000);
    assert_eq!(chunk.metadata.row_count, 1000);
}

#[test]
fn test_chunk_with_empty_data() {
    let data: Vec<Vec<String>> = vec![];
    let metadata = ChunkMetadata {
        timestamp: "2026-01-26T10:00:00Z".to_string(),
        source: None,
        row_count: 0,
        column_count: 0,
    };

    let chunk = DataChunk {
        sequence: 0,
        data,
        metadata,
    };

    assert_eq!(chunk.data.len(), 0);
    assert_eq!(chunk.metadata.row_count, 0);
}

#[test]
fn test_multiple_chunks_sequence() {
    let metadata = ChunkMetadata {
        timestamp: "2026-01-26T10:00:00Z".to_string(),
        source: Some("test.csv".to_string()),
        row_count: 10,
        column_count: 2,
    };

    let chunks: Vec<DataChunk> = (0..5)
        .map(|i| DataChunk {
            sequence: i,
            data: vec![vec![format!("row_{}", i)]],
            metadata: metadata.clone(),
        })
        .collect();

    assert_eq!(chunks.len(), 5);
    assert_eq!(chunks[0].sequence, 0);
    assert_eq!(chunks[4].sequence, 4);
}

#[test]
fn test_csv_streaming_reader_reads_chunks_and_stops() {
    let mut temp = tempfile::NamedTempFile::with_suffix(".csv").unwrap();
    // Write 5 data rows plus header = 6 total rows
    writeln!(temp, "col_a,col_b").unwrap();
    for i in 0..5 {
        writeln!(temp, "{},{}", i, i * 10).unwrap();
    }
    temp.flush().unwrap();

    let mut reader = CsvStreamingReader::new(temp.path().to_str().unwrap()).unwrap();

    // csv::Reader treats first line as header by default, so records() yields 5 data rows
    let chunk1 = reader.read_chunk(2).unwrap();
    assert!(chunk1.is_some());
    let chunk1 = chunk1.unwrap();
    assert_eq!(chunk1.data.len(), 2);
    assert!(reader.has_more());

    let chunk2 = reader.read_chunk(2).unwrap();
    assert!(chunk2.is_some());
    let chunk2 = chunk2.unwrap();
    assert_eq!(chunk2.data.len(), 2);
    assert!(reader.has_more());

    let chunk3 = reader.read_chunk(2).unwrap();
    assert!(chunk3.is_some());
    let chunk3 = chunk3.unwrap();
    assert_eq!(chunk3.data.len(), 1); // last remaining data row
    // has_more stays true until an empty read actually occurs
    assert!(reader.has_more());

    let chunk4 = reader.read_chunk(2).unwrap();
    assert!(chunk4.is_none());
    assert!(!reader.has_more());
}

#[test]
fn test_csv_streaming_reader_empty_file() {
    let temp = tempfile::NamedTempFile::with_suffix(".csv").unwrap();
    let mut reader = CsvStreamingReader::new(temp.path().to_str().unwrap()).unwrap();
    let chunk = reader.read_chunk(10).unwrap();
    assert!(chunk.is_none());
    assert!(!reader.has_more());
}
