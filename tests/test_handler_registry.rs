//! Tests for handler registry module

use xls_rs::handler_registry::HandlerRegistry;

#[test]
fn test_handler_registry_creation() {
    let _registry = HandlerRegistry::new();
}

#[test]
fn test_get_reader_csv() {
    let registry = HandlerRegistry::new();
    let reader = registry.get_reader("test.csv");
    assert!(reader.is_ok());
}

#[test]
fn test_get_reader_xlsx() {
    let registry = HandlerRegistry::new();
    let reader = registry.get_reader("test.xlsx");
    assert!(reader.is_ok());
}

#[test]
fn test_get_reader_parquet() {
    let registry = HandlerRegistry::new();
    let reader = registry.get_reader("test.parquet");
    assert!(reader.is_ok());
}

#[test]
fn test_get_reader_avro() {
    let registry = HandlerRegistry::new();
    let reader = registry.get_reader("test.avro");
    assert!(reader.is_ok());
}

#[test]
fn test_get_reader_unknown() {
    let registry = HandlerRegistry::new();
    let reader = registry.get_reader("test.unknown");
    assert!(reader.is_err());
}

#[test]
fn test_get_writer_csv() {
    let registry = HandlerRegistry::new();
    let writer = registry.get_writer("test.csv");
    assert!(writer.is_ok());
}

#[test]
fn test_get_writer_parquet() {
    let registry = HandlerRegistry::new();
    let writer = registry.get_writer("test.parquet");
    assert!(writer.is_ok());
}

#[test]
fn test_get_writer_avro() {
    let registry = HandlerRegistry::new();
    let writer = registry.get_writer("test.avro");
    assert!(writer.is_ok());
}

#[test]
fn test_get_handler_csv() {
    let registry = HandlerRegistry::new();
    let handler = registry.get_handler("test.csv");
    assert!(handler.is_ok());
}

#[test]
fn test_get_handler_parquet() {
    let registry = HandlerRegistry::new();
    let handler = registry.get_handler("test.parquet");
    assert!(handler.is_ok());
}

#[test]
fn test_get_handler_avro() {
    let registry = HandlerRegistry::new();
    let handler = registry.get_handler("test.avro");
    assert!(handler.is_ok());
}

#[test]
fn test_get_handler_unsupported() {
    let registry = HandlerRegistry::new();
    let handler = registry.get_handler("test.xlsx");
    assert!(handler.is_err());
}
