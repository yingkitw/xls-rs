//! Generate example files in various formats

mod common;

use xls_rs::{AvroHandler, ExcelHandler, ParquetHandler, WriteOptions};
use std::fs;

fn read_csv(name: &str) -> Vec<Vec<String>> {
    let path = common::example_path(&format!("{name}.csv"));
    let content = fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {path}"));
    content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.split(',').map(|s| s.to_string()).collect())
        .collect()
}

#[test]
#[ignore = "generates example artifacts; not part of normal test runs"]
fn generate_parquet_examples() {
    let handler = ParquetHandler::new();

    // Generate sales.parquet
    let sales = read_csv("sales");
    let header = sales[0].clone();
    let data: Vec<Vec<String>> = sales[1..].to_vec();
    handler
        .write(&common::example_path("sales.parquet"), &data, Some(&header))
        .unwrap();
    println!("Created examples/sales.parquet");

    // Generate employees.parquet
    let employees = read_csv("employees");
    let header = employees[0].clone();
    let data: Vec<Vec<String>> = employees[1..].to_vec();
    handler
        .write(&common::example_path("employees.parquet"), &data, Some(&header))
        .unwrap();
    println!("Created examples/employees.parquet");

    // Generate numbers.parquet
    let numbers = read_csv("numbers");
    let header = numbers[0].clone();
    let data: Vec<Vec<String>> = numbers[1..].to_vec();
    handler
        .write(&common::example_path("numbers.parquet"), &data, Some(&header))
        .unwrap();
    println!("Created examples/numbers.parquet");
}

#[test]
#[ignore = "generates example artifacts; not part of normal test runs"]
fn generate_avro_examples() {
    let handler = AvroHandler::new();

    // Generate sales.avro
    let sales = read_csv("sales");
    let header = sales[0].clone();
    let data: Vec<Vec<String>> = sales[1..].to_vec();
    handler
        .write(&common::example_path("sales.avro"), &data, Some(&header))
        .unwrap();
    println!("Created examples/sales.avro");

    // Generate employees.avro
    let employees = read_csv("employees");
    let header = employees[0].clone();
    let data: Vec<Vec<String>> = employees[1..].to_vec();
    handler
        .write(&common::example_path("employees.avro"), &data, Some(&header))
        .unwrap();
    println!("Created examples/employees.avro");

    // Generate lookup.avro
    let lookup = read_csv("lookup");
    let header = lookup[0].clone();
    let data: Vec<Vec<String>> = lookup[1..].to_vec();
    handler
        .write(&common::example_path("lookup.avro"), &data, Some(&header))
        .unwrap();
    println!("Created examples/lookup.avro");
}

#[test]
#[ignore = "generates example artifacts; not part of normal test runs"]
fn generate_excel_examples() {
    let handler = ExcelHandler::new();

    // Generate sales.xlsx with styling
    let sales = read_csv("sales");
    let options = WriteOptions::default();
    handler
        .write_styled(&common::example_path("sales.xlsx"), &sales, &options)
        .unwrap();
    println!("Created examples/sales.xlsx");

    // Generate employees.xlsx
    let employees = read_csv("employees");
    handler
        .write_styled(&common::example_path("employees.xlsx"), &employees, &options)
        .unwrap();
    println!("Created examples/employees.xlsx");
}
