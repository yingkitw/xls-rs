//! Examples generation command handler

use anyhow::Result;

/// Generate deterministic example files under `./examples`.
///
/// This is intended for demos, docs, and quick sanity checks.
pub fn handle_examples_generate() -> Result<()> {
    let dir = std::path::PathBuf::from("examples");
    std::fs::create_dir_all(&dir)?;

    let sales_csv = dir.join("sales.csv");
    if !sales_csv.exists() {
        std::fs::write(
            &sales_csv,
            "Product,Category,Price,Quantity,Date\n\
Laptop,Electronics,1200,1,2026-01-01\n\
Mouse,Electronics,25,2,2026-01-02\n\
Desk,Furniture,300,1,2026-01-03\n\
Chair,Furniture,150,4,2026-01-04\n\
Pen,Stationery,2,10,2026-01-05\n\
Lamp,Home,45,1,2026-01-06\n",
        )?;
    }

    let employees_csv = dir.join("employees.csv");
    if !employees_csv.exists() {
        std::fs::write(
            &employees_csv,
            "ID,Name,Department,Salary\n\
1,Alice Johnson,Engineering,85000\n\
2,Bob Smith,Sales,65000\n\
3,Carol Davis,Engineering,92000\n\
4,Dan Miller,Marketing,72000\n\
6,Grace Anderson,Engineering,81000\n\
7,Henry Wilson,Engineering,95000\n",
        )?;
    }

    let numbers_csv = dir.join("numbers.csv");
    if !numbers_csv.exists() {
        std::fs::write(&numbers_csv, "A,B\n1,2\n4,3\n")?;
    }

    let duplicates_csv = dir.join("duplicates.csv");
    if !duplicates_csv.exists() {
        std::fs::write(
            &duplicates_csv,
            "Product,Value\n\
Apple,100\n\
Banana,200\n\
Apple,100\n\
Cherry,300\n\
Date,400\n\
Banana,200\n\
Cherry,300\n",
        )?;
    }

    let lookup_csv = dir.join("lookup.csv");
    if !lookup_csv.exists() {
        std::fs::write(&lookup_csv, "Code,Name\nW,Widget\nG,Gadget\n")?;
    }

    // Generate a couple of non-CSV artifacts from the CSVs.
    let converter = xls_rs::converter::Converter::new();

    let sales_xlsx = dir.join("sales.xlsx");
    if !sales_xlsx.exists() {
        converter.convert(
            sales_csv.to_string_lossy().as_ref(),
            sales_xlsx.to_string_lossy().as_ref(),
            None,
        )?;
    }

    let sales_parquet = dir.join("sales.parquet");
    if !sales_parquet.exists() {
        converter.convert(
            sales_csv.to_string_lossy().as_ref(),
            sales_parquet.to_string_lossy().as_ref(),
            None,
        )?;
    }

    crate::cli::runtime::log(format!("Generated examples under {}", dir.display()));
    Ok(())
}

