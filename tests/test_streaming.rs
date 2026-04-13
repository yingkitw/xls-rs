//! Tests for streaming module

use xls_rs::streaming::{ChunkMetadata, DataChunk, StreamingProcessor};

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
