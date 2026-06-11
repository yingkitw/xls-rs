//! Google Sheets API handler for reading and writing Google Sheets

use crate::config::Config;
use crate::csv_handler::CellRange;
use crate::traits::{DataReader, DataWriteOptions, DataWriter, FileHandler};
use anyhow::{anyhow, Context, Result};
use tokio::runtime::Runtime;

/// Handler for Google Sheets operations
pub struct GoogleSheetsHandler {
    config: Config,
    rt: Runtime,
}

impl GoogleSheetsHandler {
    /// Create a new Google Sheets handler
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            rt: Runtime::new().expect("Failed to create tokio runtime"),
        }
    }

    /// Create a new Google Sheets handler with custom config
    pub fn with_config(config: Config) -> Self {
        Self {
            config,
            rt: Runtime::new().expect("Failed to create tokio runtime"),
        }
    }

    /// Parse Google Sheets URL or ID to extract spreadsheet ID
    pub fn parse_spreadsheet_id(&self, path: &str) -> Result<String> {
        // If it's a gsheet:// protocol URL
        if path.starts_with("gsheet://") {
            let id = path
                .strip_prefix("gsheet://")
                .ok_or_else(|| anyhow!("Invalid gsheet URL"))?;
            return Ok(id.split('/').next().unwrap_or(id).to_string());
        }

        // If it's a full Google Sheets URL, extract the ID
        if path.starts_with("https://docs.google.com/spreadsheets/") {
            if let Some(start) = path.find("/d/") {
                let start = start + 3;
                if let Some(end) = path[start..].find('/') {
                    return Ok(path[start..start + end].to_string());
                } else {
                    return Ok(path[start..].to_string());
                }
            }
        }

        // Check if it's just the ID
        if path.len() >= 44
            && path
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Ok(path.to_string());
        }

        Err(anyhow!("Invalid Google Sheets URL or ID: {}", path))
    }

    /// Parse sheet name from path
    pub fn parse_sheet_name(&self, path: &str) -> Option<String> {
        // Extract from gsheets://id/sheet_name
        if path.starts_with("gsheet://") {
            let parts: Vec<&str> = path[9..].split('/').collect();
            if parts.len() > 1 {
                return Some(parts[1].to_string());
            }
        }

        None
    }

    /// Convert A1 notation to row/column indices
    pub fn a1_to_row_col(&self, a1: &str) -> Result<(usize, usize)> {
        let mut col_start = 0;
        let mut row_start = 0;

        // Find where letters end and numbers begin
        for (i, c) in a1.chars().enumerate() {
            if c.is_alphabetic() {
                col_start += 1;
            } else if c.is_numeric() {
                row_start = i;
                break;
            }
        }

        if col_start == 0 || row_start == 0 {
            return Err(anyhow!("Invalid A1 notation: {}", a1));
        }

        // Parse column (base-26)
        let col_str = &a1[..col_start];
        let mut col = 0;
        for c in col_str.chars() {
            col = col * 26 + (c.to_ascii_uppercase() as u8 - b'A' + 1) as usize;
        }
        col -= 1; // Convert to 0-based

        // Parse row
        let row = a1[row_start..].parse::<usize>()? - 1; // Convert to 0-based

        Ok((row, col))
    }

    /// Convert row/column indices to A1 notation
    pub fn row_col_to_a1(&self, row: usize, col: usize) -> String {
        let mut col = col + 1;
        let mut col_str = String::new();

        while col > 0 {
            col -= 1;
            col_str.insert(0, ((col % 26) as u8 + b'A') as char);
            col /= 26;
        }

        format!("{}{}", col_str, row + 1)
    }

    /// Convert CellRange to A1 notation range
    pub fn cell_range_to_a1(&self, range: &CellRange, sheet_name: Option<&str>) -> String {
        let start = self.row_col_to_a1(range.start_row, range.start_col);
        let end = self.row_col_to_a1(range.end_row, range.end_col);

        let range_str = if start == end {
            start
        } else {
            format!("{}:{}", start, end)
        };

        if let Some(name) = sheet_name {
            format!("'{}'!{}", name, range_str)
        } else {
            range_str
        }
    }

    /// List sheet titles for a spreadsheet using the Google Sheets API.
    ///
    /// Requires `google_sheets.api_key` in [`Config`] (suitable for public spreadsheets when the
    /// Sheets API is enabled for the key).
    pub fn list_sheet_titles(&self, spreadsheet_ref: &str) -> Result<Vec<String>> {
        let api_key = self.config.google_sheets.api_key.as_deref().ok_or_else(|| {
            anyhow!(
                "google_sheets.api_key is not set in config; required to list sheet titles via the API"
            )
        })?;

        let id = self.parse_spreadsheet_id(spreadsheet_ref)?;
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{id}?fields=sheets.properties.title&key={api_key}"
        );

        let resp = ureq::get(&url)
            .call()
            .map_err(|e| anyhow!("Google Sheets request failed: {}", e))?;

        let status = resp.status();
        let body = resp
            .into_string()
            .map_err(|e| anyhow!("Failed to read Sheets API response: {}", e))?;

        if status != 200 {
            anyhow::bail!("Google Sheets API returned HTTP {}: {}", status, body);
        }

        let v: serde_json::Value =
            serde_json::from_str(&body).with_context(|| "Invalid JSON from Sheets API")?;

        let sheets = v
            .get("sheets")
            .and_then(|s| s.as_array())
            .ok_or_else(|| anyhow!("Sheets API response missing 'sheets' array"))?;

        let mut titles = Vec::new();
        for sheet in sheets {
            if let Some(title) = sheet
                .get("properties")
                .and_then(|p| p.get("title"))
                .and_then(|t| t.as_str())
            {
                titles.push(title.to_string());
            }
        }

        Ok(titles)
    }

    /// Obtain authorization header for Google Sheets API calls.
    fn auth_header(&self) -> Result<String> {
        if let Some(token) = &self.config.google_sheets.access_token {
            Ok(format!("Bearer {}", token))
        } else if let Some(_api_key) = &self.config.google_sheets.api_key {
            // API key is query-param based; return empty for header auth
            Ok(String::new())
        } else {
            Err(anyhow!(
                "No Google Sheets credentials configured. \
                 Set google_sheets.access_token or google_sheets.api_key in config."
            ))
        }
    }

    /// Read values from a range via the Google Sheets API.
    fn read_values(&self, spreadsheet_id: &str, range: &str) -> Result<Vec<Vec<String>>> {
        let auth = self.auth_header()?;
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}/values/{range}"
        );

        let req = ureq::get(&url);
        let req = if auth.is_empty() {
            if let Some(api_key) = &self.config.google_sheets.api_key {
                req.query("key", api_key)
            } else {
                req
            }
        } else {
            req.set("Authorization", &auth)
        };

        let resp = req
            .call()
            .map_err(|e| anyhow!("Google Sheets read failed: {}", e))?;

        let status = resp.status();
        let body = resp
            .into_string()
            .map_err(|e| anyhow!("Failed to read Sheets API response: {}", e))?;

        if status != 200 {
            anyhow::bail!("Google Sheets API returned HTTP {}: {}", status, body);
        }

        let v: serde_json::Value = serde_json::from_str(&body)
            .with_context(|| "Invalid JSON from Sheets API")?;

        let values = v
            .get("values")
            .and_then(|s| s.as_array())
            .cloned()
            .unwrap_or_default();

        let mut result = Vec::new();
        for row in values {
            if let Some(cols) = row.as_array() {
                let row_strs: Vec<String> = cols
                    .iter()
                    .map(|c| c.as_str().unwrap_or("").to_string())
                    .collect();
                result.push(row_strs);
            }
        }
        Ok(result)
    }

    /// Write values to a range via the Google Sheets API.
    fn write_values(
        &self,
        spreadsheet_id: &str,
        range: &str,
        data: &[Vec<String>],
    ) -> Result<()> {
        let auth = self.auth_header()?;
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}/values/{range}?valueInputOption=USER_ENTERED"
        );

        let payload = serde_json::json!({
            "range": range,
            "majorDimension": "ROWS",
            "values": data,
        });

        let resp = ureq::put(&url)
            .set("Authorization", &auth)
            .set("Content-Type", "application/json")
            .send_string(&payload.to_string())
            .map_err(|e| anyhow!("Google Sheets write failed: {}", e))?;

        let status = resp.status();
        let body = resp
            .into_string()
            .map_err(|e| anyhow!("Failed to read Sheets API response: {}", e))?;

        if status != 200 {
            anyhow::bail!("Google Sheets API returned HTTP {}: {}", status, body);
        }

        Ok(())
    }

    /// Append values to a sheet via the Google Sheets API.
    fn append_values(
        &self,
        spreadsheet_id: &str,
        range: &str,
        data: &[Vec<String>],
    ) -> Result<()> {
        let auth = self.auth_header()?;
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}/values/{range}:append?valueInputOption=USER_ENTERED&insertDataOption=INSERT_ROWS"
        );

        let payload = serde_json::json!({
            "range": range,
            "majorDimension": "ROWS",
            "values": data,
        });

        let resp = ureq::post(&url)
            .set("Authorization", &auth)
            .set("Content-Type", "application/json")
            .send_string(&payload.to_string())
            .map_err(|e| anyhow!("Google Sheets append failed: {}", e))?;

        let status = resp.status();
        let body = resp
            .into_string()
            .map_err(|e| anyhow!("Failed to read Sheets API response: {}", e))?;

        if status != 200 {
            anyhow::bail!("Google Sheets API returned HTTP {}: {}", status, body);
        }

        Ok(())
    }
}

impl Default for GoogleSheetsHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DataReader for GoogleSheetsHandler {
    fn read(&self, path: &str) -> Result<Vec<Vec<String>>> {
        let spreadsheet_id = self.parse_spreadsheet_id(path)?;
        let sheet_name = self.parse_sheet_name(path);
        let range = if let Some(name) = sheet_name {
            format!("{}!A1:ZZ1000000", name)
        } else {
            "Sheet1!A1:ZZ1000000".to_string()
        };
        self.read_values(&spreadsheet_id, &range)
    }

    fn read_with_headers(&self, path: &str) -> Result<Vec<Vec<String>>> {
        self.read(path)
    }

    fn read_range(&self, path: &str, range: &CellRange) -> Result<Vec<Vec<String>>> {
        let spreadsheet_id = self.parse_spreadsheet_id(path)?;
        let sheet_name = self.parse_sheet_name(path);
        let range_str = self.cell_range_to_a1(range, sheet_name.as_deref());
        self.read_values(&spreadsheet_id, &range_str)
    }

    fn read_as_json(&self, path: &str) -> Result<String> {
        let data = self.read(path)?;
        serde_json::to_string_pretty(&data).map_err(Into::into)
    }

    fn supports_format(&self, path: &str) -> bool {
        path.starts_with("gsheet://")
            || path.starts_with("https://docs.google.com/spreadsheets/")
            || (path.len() >= 44
                && path
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_'))
    }
}

impl DataWriter for GoogleSheetsHandler {
    fn write(&self, path: &str, data: &[Vec<String>], options: DataWriteOptions) -> Result<()> {
        let spreadsheet_id = self.parse_spreadsheet_id(path)?;
        let sheet_name = options.sheet_name.or_else(|| self.parse_sheet_name(path));
        let range = if let Some(name) = sheet_name {
            format!("{}!A1", name)
        } else {
            "Sheet1!A1".to_string()
        };
        self.write_values(&spreadsheet_id, &range, data)
    }

    fn write_range(
        &self,
        path: &str,
        data: &[Vec<String>],
        start_row: usize,
        start_col: usize,
    ) -> Result<()> {
        let spreadsheet_id = self.parse_spreadsheet_id(path)?;
        let sheet_name = self.parse_sheet_name(path);
        let start_a1 = self.row_col_to_a1(start_row, start_col);
        let range = if let Some(name) = sheet_name {
            format!("{}!{}", name, start_a1)
        } else {
            format!("Sheet1!{}", start_a1)
        };
        self.write_values(&spreadsheet_id, &range, data)
    }

    fn append(&self, path: &str, data: &[Vec<String>]) -> Result<()> {
        let spreadsheet_id = self.parse_spreadsheet_id(path)?;
        let sheet_name = self.parse_sheet_name(path);
        let range = if let Some(name) = sheet_name {
            format!("{}!A1", name)
        } else {
            "Sheet1!A1".to_string()
        };
        self.append_values(&spreadsheet_id, &range, data)
    }

    fn supports_format(&self, path: &str) -> bool {
        path.starts_with("gsheet://")
            || path.starts_with("https://docs.google.com/spreadsheets/")
            || (path.len() >= 44
                && path
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_'))
    }
}

impl FileHandler for GoogleSheetsHandler {
    fn format_name(&self) -> &'static str {
        "gsheet"
    }

    fn supported_extensions(&self) -> &'static [&'static str] {
        &["gsheet"]
    }
}

impl Clone for GoogleSheetsHandler {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            rt: Runtime::new().expect("Failed to create tokio runtime"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_header_returns_bearer_when_access_token_set() {
        let mut config = Config::default();
        config.google_sheets.access_token = Some("my_token".to_string());
        let handler = GoogleSheetsHandler::with_config(config);
        assert_eq!(handler.auth_header().unwrap(), "Bearer my_token");
    }

    #[test]
    fn auth_header_returns_empty_when_only_api_key() {
        let mut config = Config::default();
        config.google_sheets.api_key = Some("my_key".to_string());
        let handler = GoogleSheetsHandler::with_config(config);
        assert_eq!(handler.auth_header().unwrap(), "");
    }

    #[test]
    fn auth_header_errors_when_no_credentials() {
        let handler = GoogleSheetsHandler::new();
        let err = handler.auth_header().unwrap_err().to_string();
        assert!(err.contains("No Google Sheets credentials configured"));
    }

    #[test]
    fn a1_to_row_col_basic() {
        let handler = GoogleSheetsHandler::new();
        assert_eq!(handler.a1_to_row_col("A1").unwrap(), (0, 0));
        assert_eq!(handler.a1_to_row_col("B2").unwrap(), (1, 1));
        assert_eq!(handler.a1_to_row_col("Z26").unwrap(), (25, 25));
    }

    #[test]
    fn row_col_to_a1_basic() {
        let handler = GoogleSheetsHandler::new();
        assert_eq!(handler.row_col_to_a1(0, 0), "A1");
        assert_eq!(handler.row_col_to_a1(1, 1), "B2");
        assert_eq!(handler.row_col_to_a1(25, 25), "Z26");
    }
}
