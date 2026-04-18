//! Performance benchmarks for key operations
//!
//! This module provides benchmarks for measuring the performance of
//! critical operations like reading/writing Excel files, converting formats, etc.

#[cfg(test)]
mod benches {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    use tempfile::TempDir;

    fn bench_read_xlsx(c: &mut Criterion) {
        let dir = TempDir::new().unwrap();
        let xlsx_path = dir.path().join("test.xlsx");

        // Create test data
        let data = create_test_data(1000, 10);
        write_test_xlsx(&xlsx_path, &data);

        c.bench_function("read_xlsx_1000x10", |b| {
            b.iter(|| {
                let handler = ExcelHandler::new();
                black_box(handler.read(black_box(xlsx_path.to_str().unwrap())).unwrap())
            })
        });
    }

    fn bench_write_xlsx(c: &mut Criterion) {
        let dir = TempDir::new().unwrap();
        let xlsx_path = dir.path().join("test.xlsx");

        let data = create_test_data(1000, 10);

        c.bench_function("write_xlsx_1000x10", |b| {
            b.iter(|| {
                let handler = ExcelHandler::new();
                let options = WriteOptions::default();
                black_box(
                    handler
                        .write_styled(
                            black_box(xlsx_path.to_str().unwrap()),
                            black_box(&data),
                            black_box(&options),
                        )
                        .unwrap()
                )
            })
        });
    }

    fn bench_convert_to_parquet(c: &mut Criterion) {
        let dir = TempDir::new().unwrap();
        let csv_path = dir.path().join("test.csv");
        let parquet_path = dir.path().join("test.parquet");

        // Create test CSV
        let data = create_test_data(1000, 10);
        write_test_csv(&csv_path, &data);

        c.bench_function("convert_csv_to_parquet_1000x10", |b| {
            b.iter(|| {
                let converter = Converter::new();
                black_box(
                    converter
                        .convert(
                            black_box(csv_path.to_str().unwrap()),
                            black_box(parquet_path.to_str().unwrap()),
                            None,
                        )
                        .unwrap()
                )
            })
        });
    }

    fn bench_range_read_xlsx(c: &mut Criterion) {
        let dir = TempDir::new().unwrap();
        let xlsx_path = dir.path().join("test.xlsx");

        let data = create_test_data(1000, 10);
        write_test_xlsx(&xlsx_path, &data);

        c.bench_function("range_read_xlsx_1000x10", |b| {
            b.iter(|| {
                let handler = ExcelHandler::new();
                let range = CellRange {
                    start_row: 0,
                    start_col: 0,
                    end_row: 99,
                    end_col: 9,
                };
                black_box(
                    handler
                        .read_range(
                            black_box(xlsx_path.to_str().unwrap()),
                            black_box(&range),
                        )
                        .unwrap()
                )
            })
        });
    }

    criterion_group!(
        benches,
        bench_read_xlsx,
        bench_write_xlsx,
        bench_convert_to_parquet,
        bench_range_read_xlsx
    );
    criterion_main!(benches);
}

// Helper functions for benchmark setup
fn create_test_data(rows: usize, cols: usize) -> Vec<Vec<String>> {
    (0..rows)
        .map(|row_idx| {
            (0..cols)
                .map(|col_idx| format!("R{}C{}", row_idx, col_idx))
                .collect()
        })
        .collect()
}

fn write_test_csv(path: &std::path::Path, data: &[Vec<String>]) {
    use std::io::Write;
    let mut file = std::fs::File::create(path).unwrap();
    for row in data {
        writeln!(file, "{}", row.join(",")).unwrap();
    }
}

fn write_test_xlsx(path: &std::path::Path, data: &[Vec<String>]) {
    let handler = ExcelHandler::new();
    let options = WriteOptions::default();
    handler.write_styled(path.to_str().unwrap(), data, &options).unwrap();
}
