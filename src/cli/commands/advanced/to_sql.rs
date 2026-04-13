//! SQL export command handler
//!
//! Generates INSERT statements from tabular data.

use xls_rs::converter::Converter;
use anyhow::Result;

/// Escape a string value for SQL (single quotes doubled)
fn escape_sql_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "''")
}

/// Format a cell value for SQL
fn format_sql_value(value: &str) -> String {
    if value.is_empty() {
        return "NULL".to_string();
    }
    if value.parse::<f64>().is_ok() {
        return value.to_string();
    }
    format!("'{}'", escape_sql_string(value))
}

/// Handle the to-sql command
///
/// Generates INSERT statements from the input data.
pub fn handle_to_sql(
    input: String,
    table: String,
    output: Option<String>,
    batch_size: Option<usize>,
) -> Result<()> {
    let converter = Converter::new();
    let data = converter.read_any_data(&input, None)?;

    if data.len() < 2 {
        anyhow::bail!("Data must have at least a header row and one data row");
    }

    let header = &data[0];
    let columns: Vec<&str> = header.iter().map(|s| s.as_str()).collect();
    let col_list = columns
        .iter()
        .map(|c| format!(r#""{}""#, c.replace('"', r#""""#)))
        .collect::<Vec<_>>()
        .join(", ");
    let table_quoted = format!(r#""{}""#, table.replace('"', r#""""#));

    let batch = batch_size.unwrap_or(100);
    let mut statements = Vec::new();
    let mut values_batch = Vec::with_capacity(batch);

    for row in data.iter().skip(1) {
        let vals: Vec<String> = columns
            .iter()
            .enumerate()
            .map(|(i, _)| format_sql_value(row.get(i).map(|s| s.as_str()).unwrap_or("")))
            .collect();
        values_batch.push(format!("({})", vals.join(", ")));

        if values_batch.len() >= batch {
            statements.push(format!(
                "INSERT INTO {} ({}) VALUES\n{};",
                table_quoted,
                col_list,
                values_batch.join(",\n")
            ));
            values_batch.clear();
        }
    }

    if !values_batch.is_empty() {
        statements.push(format!(
            "INSERT INTO {} ({}) VALUES\n{};",
            table_quoted,
            col_list,
            values_batch.join(",\n")
        ));
    }

    let sql = statements.join("\n\n");

    if let Some(output_path) = &output {
        std::fs::write(output_path, &sql)?;
        println!("SQL saved to {} ({} statements)", output_path, statements.len());
    } else {
        println!("{}", sql);
    }

    Ok(())
}
