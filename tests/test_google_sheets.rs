//! Tests for Google Sheets functionality

use xls_rs::google_sheets::GoogleSheetsHandler;
use xls_rs::traits::{DataReader, DataWriter, FileHandler};

#[test]
fn test_parse_spreadsheet_id() {
    let handler = GoogleSheetsHandler::new();

    // Test gsheet:// protocol
    let result =
        handler.parse_spreadsheet_id("gsheet://1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        "1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms"
    );

    // Test full URL
    let result = handler.parse_spreadsheet_id(
        "https://docs.google.com/spreadsheets/d/1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms/edit",
    );
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        "1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms"
    );

    // Test plain ID
    let result = handler.parse_spreadsheet_id("1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        "1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms"
    );

    // Test invalid ID
    let result = handler.parse_spreadsheet_id("invalid-id");
    assert!(result.is_err());
}

#[test]
fn test_parse_sheet_name() {
    let handler = GoogleSheetsHandler::new();

    // Test gsheet:// with sheet name
    let result =
        handler.parse_sheet_name("gsheet://1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms/Sheet1");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), "Sheet1");

    // Test gsheet:// without sheet name
    let result = handler.parse_sheet_name("gsheet://1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms");
    assert!(result.is_none());

    // Test full URL (currently not implemented)
    let result = handler.parse_sheet_name(
        "https://docs.google.com/spreadsheets/d/1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms/edit#gid=0"
    );
    assert!(result.is_none());
}

#[test]
fn test_a1_to_row_col() {
    let handler = GoogleSheetsHandler::new();

    // Test basic A1 notation
    let result = handler.a1_to_row_col("A1");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), (0, 0));

    // Test other cells
    let result = handler.a1_to_row_col("B2");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), (1, 1));

    let result = handler.a1_to_row_col("Z10");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), (9, 25));

    let result = handler.a1_to_row_col("AA1");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), (0, 26));

    // Test invalid A1 notation
    let result = handler.a1_to_row_col("invalid");
    assert!(result.is_err());
}

#[test]
fn test_row_col_to_a1() {
    let handler = GoogleSheetsHandler::new();

    // Test basic conversion
    let result = handler.row_col_to_a1(0, 0);
    assert_eq!(result, "A1");

    let result = handler.row_col_to_a1(1, 1);
    assert_eq!(result, "B2");

    let result = handler.row_col_to_a1(9, 25);
    assert_eq!(result, "Z10");

    let result = handler.row_col_to_a1(0, 26);
    assert_eq!(result, "AA1");

    let result = handler.row_col_to_a1(26, 26);
    assert_eq!(result, "AA27");
}

#[test]
fn test_supports_format() {
    let handler = GoogleSheetsHandler::new();

    // Test gsheet:// protocol
    assert!(DataReader::supports_format(&handler, "gsheet://1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms"));

    // Test full URL
    assert!(DataReader::supports_format(
        &handler,
        "https://docs.google.com/spreadsheets/d/1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms/edit"
    ));

    // Test plain ID
    assert!(DataReader::supports_format(&handler, "1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms"));

    // Test non-supported format
    assert!(!DataReader::supports_format(&handler, "test.csv"));
    assert!(!DataReader::supports_format(&handler, "test.xlsx"));
}

#[test]
fn test_format_name() {
    let handler = GoogleSheetsHandler::new();
    assert_eq!(handler.format_name(), "gsheet");
}

#[test]
fn test_supported_extensions() {
    let handler = GoogleSheetsHandler::new();
    let extensions = handler.supported_extensions();
    assert!(extensions.contains(&"gsheet"));
}

// Test reading data (placeholder implementation)
#[test]
fn test_read() {
    let handler = GoogleSheetsHandler::new();
    let result = handler.read("gsheet://1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms");
    assert!(result.is_ok());

    let data = result.unwrap();
    assert!(!data.is_empty());
    assert_eq!(data[0], vec!["Column1", "Column2"]);
    assert_eq!(data[1], vec!["Value1", "Value2"]);
}

// Test writing data (placeholder implementation)
#[test]
fn test_write() {
    let handler = GoogleSheetsHandler::new();
    let data = vec![
        vec!["Header1".to_string(), "Header2".to_string()],
        vec!["Row1Col1".to_string(), "Row1Col2".to_string()],
        vec!["Row2Col1".to_string(), "Row2Col2".to_string()],
    ];

    let options = xls_rs::traits::DataWriteOptions {
        sheet_name: Some("TestSheet".to_string()),
        column_names: None,
        include_headers: true,
    };

    let result = handler.write(
        "gsheet://1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms",
        &data,
        options,
    );
    assert!(result.is_ok());
}
