//! Shared paths for integration tests.
use std::path::PathBuf;
use std::sync::OnceLock;

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn examples_dir() -> PathBuf {
    let dir = workspace_root().join("examples");
    // Many integration tests write fixtures here.
    std::fs::create_dir_all(&dir).expect("Failed to create examples directory");
    dir
}

#[allow(dead_code)]
pub fn example_path(rel: &str) -> String {
    examples_dir().join(rel).to_string_lossy().into()
}

/// Create canonical example fixtures under `examples/` for integration tests.
///
/// A number of test suites rely on specific example filenames and content.
#[allow(dead_code)]
pub fn ensure_example_fixtures() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let dir = examples_dir();

        // CSV fixtures
        std::fs::write(dir.join("numbers.csv"), "A,B\n1,2\n4,3\n")
            .expect("Failed to write examples/numbers.csv");

        std::fs::write(
            dir.join("employees.csv"),
            "ID,Name,Department,Salary\n\
1,Alice Johnson,Engineering,85000\n\
2,Bob Smith,Sales,65000\n\
3,Carol Davis,Engineering,92000\n\
4,Dan Miller,Marketing,72000\n\
6,Grace Anderson,Engineering,81000\n\
7,Henry Wilson,Engineering,95000\n",
        )
        .expect("Failed to write examples/employees.csv");

        std::fs::write(
            dir.join("sales.csv"),
            "Product,Category,Price,Quantity,Date\n\
Laptop,Electronics,1200,1,2026-01-01\n\
Mouse,Electronics,25,2,2026-01-02\n\
Desk,Furniture,300,1,2026-01-03\n\
Chair,Furniture,150,4,2026-01-04\n\
Pen,Stationery,2,10,2026-01-05\n\
Lamp,Home,45,1,2026-01-06\n",
        )
        .expect("Failed to write examples/sales.csv");

        std::fs::write(
            dir.join("duplicates.csv"),
            "Product,Value\n\
Apple,100\n\
Banana,200\n\
Apple,100\n\
Cherry,300\n\
Date,400\n\
Banana,200\n\
Cherry,300\n",
        )
        .expect("Failed to write examples/duplicates.csv");

        std::fs::write(dir.join("lookup.csv"), "Code,Name\nW,Widget\nG,Gadget\n")
            .expect("Failed to write examples/lookup.csv");

        // Columnar + Excel fixtures (create if missing; CSVs above are authoritative).
        let parquet = xls_rs::ParquetHandler::new();
        let avro = xls_rs::AvroHandler::new();
        let excel = xls_rs::ExcelHandler::new();
        let options = xls_rs::WriteOptions::default();

    let sales_header = vec![
        "Product".to_string(),
        "Category".to_string(),
        "Price".to_string(),
        "Quantity".to_string(),
        "Date".to_string(),
    ];
    let sales_rows = vec![
        vec![
            "Laptop".to_string(),
            "Electronics".to_string(),
            "1200".to_string(),
            "1".to_string(),
            "2026-01-01".to_string(),
        ],
        vec![
            "Mouse".to_string(),
            "Electronics".to_string(),
            "25".to_string(),
            "2".to_string(),
            "2026-01-02".to_string(),
        ],
        vec![
            "Desk".to_string(),
            "Furniture".to_string(),
            "300".to_string(),
            "1".to_string(),
            "2026-01-03".to_string(),
        ],
        vec![
            "Chair".to_string(),
            "Furniture".to_string(),
            "150".to_string(),
            "4".to_string(),
            "2026-01-04".to_string(),
        ],
        vec![
            "Pen".to_string(),
            "Stationery".to_string(),
            "2".to_string(),
            "10".to_string(),
            "2026-01-05".to_string(),
        ],
        vec![
            "Lamp".to_string(),
            "Home".to_string(),
            "45".to_string(),
            "1".to_string(),
            "2026-01-06".to_string(),
        ],
    ];

    let employees_header = vec![
        "ID".to_string(),
        "Name".to_string(),
        "Department".to_string(),
        "Salary".to_string(),
    ];
    let employees_rows = vec![
        vec![
            "1".to_string(),
            "Alice Johnson".to_string(),
            "Engineering".to_string(),
            "85000".to_string(),
        ],
        vec![
            "2".to_string(),
            "Bob Smith".to_string(),
            "Sales".to_string(),
            "65000".to_string(),
        ],
        vec![
            "3".to_string(),
            "Carol Davis".to_string(),
            "Engineering".to_string(),
            "92000".to_string(),
        ],
        vec![
            "6".to_string(),
            "Grace Anderson".to_string(),
            "Engineering".to_string(),
            "81000".to_string(),
        ],
        vec![
            "7".to_string(),
            "Henry Wilson".to_string(),
            "Engineering".to_string(),
            "95000".to_string(),
        ],
    ];

    let numbers_header = vec!["A".to_string(), "B".to_string()];
    let numbers_rows = vec![vec!["1".to_string(), "2".to_string()], vec!["4".to_string(), "3".to_string()]];

    let lookup_header = vec!["Code".to_string(), "Name".to_string()];
    let lookup_rows = vec![
        vec!["W".to_string(), "Widget".to_string()],
        vec!["G".to_string(), "Gadget".to_string()],
    ];

        let sales_parquet = dir.join("sales.parquet");
        if !sales_parquet.exists() {
            parquet
                .write(
                    sales_parquet.to_string_lossy().as_ref(),
                    &sales_rows,
                    Some(&sales_header),
                )
                .expect("Failed to write examples/sales.parquet");
        }

        let numbers_parquet = dir.join("numbers.parquet");
        if !numbers_parquet.exists() {
            parquet
                .write(
                    numbers_parquet.to_string_lossy().as_ref(),
                    &numbers_rows,
                    Some(&numbers_header),
                )
                .expect("Failed to write examples/numbers.parquet");
        }

        let employees_parquet = dir.join("employees.parquet");
        if !employees_parquet.exists() {
            parquet
                .write(
                    employees_parquet.to_string_lossy().as_ref(),
                    &employees_rows,
                    Some(&employees_header),
                )
                .expect("Failed to write examples/employees.parquet");
        }

        let employees_avro = dir.join("employees.avro");
        if !employees_avro.exists() {
            avro.write(
                employees_avro.to_string_lossy().as_ref(),
                &employees_rows,
                Some(&employees_header),
            )
            .expect("Failed to write examples/employees.avro");
        }

        let sales_avro = dir.join("sales.avro");
        if !sales_avro.exists() {
            avro.write(
                sales_avro.to_string_lossy().as_ref(),
                &sales_rows,
                Some(&sales_header),
            )
            .expect("Failed to write examples/sales.avro");
        }

        let lookup_avro = dir.join("lookup.avro");
        if !lookup_avro.exists() {
            avro.write(
                lookup_avro.to_string_lossy().as_ref(),
                &lookup_rows,
                Some(&lookup_header),
            )
            .expect("Failed to write examples/lookup.avro");
        }

        let sales_xlsx = dir.join("sales.xlsx");
        if !sales_xlsx.exists() {
            let mut table = vec![sales_header.clone()];
            table.extend(sales_rows.clone());
            excel
                .write_styled(sales_xlsx.to_string_lossy().as_ref(), &table, &options)
                .expect("Failed to write examples/sales.xlsx");
        }

        let employees_xlsx = dir.join("employees.xlsx");
        if !employees_xlsx.exists() {
            let mut table = vec![employees_header.clone()];
            table.extend(employees_rows.clone());
            excel
                .write_styled(
                    employees_xlsx.to_string_lossy().as_ref(),
                    &table,
                    &options,
                )
                .expect("Failed to write examples/employees.xlsx");
        }
    });
}
