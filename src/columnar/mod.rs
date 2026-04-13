//! Parquet and Avro file handling module

mod avro;
mod parquet;

pub use avro::AvroHandler;
pub use parquet::ParquetHandler;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_parquet_write_read() {
        let handler = ParquetHandler::new();
        let data = vec![
            vec!["a".to_string(), "1".to_string()],
            vec!["b".to_string(), "2".to_string()],
        ];

        let path = "/tmp/test_xls_rs.parquet";
        handler
            .write(
                path,
                &data,
                Some(&["name".to_string(), "value".to_string()]),
            )
            .unwrap();

        let read_data = handler.read(path).unwrap();
        assert_eq!(read_data.len(), 2);
        assert_eq!(read_data[0][0], "a");

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_avro_write_read() {
        let handler = AvroHandler::new();
        let data = vec![
            vec!["x".to_string(), "10".to_string()],
            vec!["y".to_string(), "20".to_string()],
        ];

        let path = "/tmp/test_xls_rs.avro";
        handler
            .write(
                path,
                &data,
                Some(&["name".to_string(), "value".to_string()]),
            )
            .unwrap();

        let read_data = handler.read(path).unwrap();
        assert_eq!(read_data.len(), 2);
        assert_eq!(read_data[0][0], "x");

        fs::remove_file(path).ok();
    }
}
