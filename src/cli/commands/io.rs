//! File I/O command handlers
//!
//! Implements read, write, convert, and related I/O operations.

use crate::cli::OutputFormat;
use xls_rs::{
    config::Config,
    converter::Converter,
    excel::ExcelHandler,
    formula::FormulaEvaluator,
    google_sheets::GoogleSheetsHandler,
    handler_registry::HandlerRegistry,
    helpers::filter_by_range,
    CellRange,
};
use anyhow::{Context, Result};

/// I/O command handler
#[derive(Default)]
pub struct IoCommandHandler;

impl IoCommandHandler {
    /// Create a new I/O command handler
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle the read command
    ///
    /// Reads data from a file and displays it in the specified format.
    pub fn handle_read(
        &self,
        input: String,
        sheet: Option<String>,
        range: Option<String>,
        format: OutputFormat,
    ) -> Result<()> {
        crate::cli::runtime::ensure_safe_input(&input)?;
        let converter = Converter::new();

        // Read data
        let mut data = if let Some(sheet_name) = sheet {
            converter.read_any_data(&input, Some(&sheet_name))?
        } else {
            converter.read_any_data(&input, None)?
        };

        // Apply range filter if specified
        if let Some(range_str) = range {
            data = self.apply_range(&data, &range_str)?;
        }

        // Output in requested format
        match format {
            OutputFormat::Csv => self.print_csv(&data),
            OutputFormat::Json => self.print_json(&data)?,
            OutputFormat::Jsonl => self.print_jsonl(&data)?,
            OutputFormat::Markdown => self.print_markdown(&data),
        }

        Ok(())
    }

    /// Handle the write command
    ///
    /// Writes data to a file in the appropriate format.
    pub fn handle_write(
        &self,
        output: String,
        csv: Option<String>,
        sheet: Option<String>,
    ) -> Result<()> {
        crate::cli::runtime::ensure_can_write(&output)?;
        let converter = Converter::new();

        // Read from CSV if provided, otherwise stdin
        let data = if let Some(csv_path) = csv {
            crate::cli::runtime::ensure_safe_input(&csv_path)?;
            converter.read_any_data(&csv_path, None)?
        } else {
            // Read from stdin
            let mut input = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)
                .context("Failed to read from stdin")?;
            input
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.split(',').map(|s| s.trim().to_string()).collect())
                .collect()
        };

        // Write to output
        converter.write_any_data(&output, &data, sheet.as_deref())?;
        if output != "-" {
            crate::cli::runtime::log(format!("Wrote {output}"));
        }

        Ok(())
    }

    /// Handle the convert command
    ///
    /// Converts a file from one format to another.
    pub fn handle_convert(
        &self,
        input: String,
        output: String,
        sheet: Option<String>,
    ) -> Result<()> {
        crate::cli::runtime::ensure_safe_input(&input)?;
        crate::cli::runtime::ensure_can_write(&output)?;
        let converter = Converter::new();
        converter.convert(&input, &output, sheet.as_deref())?;
        if output != "-" {
            crate::cli::runtime::log(format!("Converted {input} to {output}"));
        }
        Ok(())
    }

    /// Handle the formula command
    ///
    /// Applies a formula to a specific cell in a spreadsheet.
    pub fn handle_formula(
        &self,
        input: String,
        output: String,
        formula: String,
        cell: String,
        sheet: Option<String>,
    ) -> Result<()> {
        crate::cli::runtime::ensure_safe_input(&input)?;
        crate::cli::runtime::ensure_can_write(&output)?;
        let evaluator = FormulaEvaluator::new();

        if input.ends_with(".csv") {
            evaluator.apply_to_csv(&input, &output, &formula, &cell)?
        } else if input.ends_with(".xls") || input.ends_with(".xlsx") {
            evaluator.apply_to_excel(&input, &output, &formula, &cell, sheet.as_deref())?
        } else {
            anyhow::bail!("Unsupported file format for formula. Use .csv, .xls, or .xlsx");
        };

        Ok(())
    }

    /// Handle the serve command
    ///
    /// Starts the MCP server for model context protocol.
    pub fn handle_serve(&self) -> Result<()> {
        // MCP server requires async runtime
        // For now, provide instructions
        println!("MCP server requires running with tokio async runtime.");
        println!("Please use the MCP integration directly or run with appropriate runtime.");
        println!("The xls-rs library provides XlsRsMcpServer for MCP protocol support.");
        Ok(())
    }

    /// Handle the sheets command
    ///
    /// Lists all sheets in an Excel file.
    pub fn handle_sheets(&self, input: String) -> Result<()> {
        let handler = ExcelHandler::new();
        let sheets = handler.list_sheets(&input)?;

        println!("Sheets in {input}:");
        for (i, sheet) in sheets.iter().enumerate() {
            println!("  {}. {}", i + 1, sheet);
        }

        Ok(())
    }

    /// Handle the read_all command
    ///
    /// Reads all sheets from an Excel file.
    pub fn handle_read_all(&self, input: String, format: OutputFormat) -> Result<()> {
        crate::cli::runtime::ensure_safe_input(&input)?;
        let handler = ExcelHandler::new();
        let sheets = handler.list_sheets(&input)?;

        for sheet in &sheets {
            println!("=== Sheet: {sheet} ===");
            self.handle_read(input.clone(), Some(sheet.clone()), None, format)?;
            println!();
        }

        Ok(())
    }

    /// Handle the write_range command
    ///
    /// Writes data starting at a specific cell.
    pub fn handle_write_range(&self, input: String, output: String, start: String) -> Result<()> {
        crate::cli::runtime::ensure_can_write(&output)?;
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;

        // Parse start cell
        let (start_row, start_col) = self.parse_cell_ref(&start)?;

        // Create new data structure with offset
        let mut offset_data = vec![vec![String::new(); start_col]; start_row];
        for row in data {
            offset_data.push(row);
        }

        converter.write_any_data(&output, &offset_data, None)?;
        crate::cli::runtime::log(format!("Wrote data starting at {start} in {output}"));

        Ok(())
    }

    /// Handle the append command
    pub fn handle_append(&self, source: String, target: String) -> Result<()> {
        crate::cli::runtime::ensure_safe_input(&source)?;
        crate::cli::runtime::ensure_safe_input(&target)?;
        let converter = Converter::new();

        // Read source data
        let data = converter.read_any_data(&source, None)?;

        // Append to target using registry
        let registry = HandlerRegistry::new();
        let writer = registry.get_writer(&target)?;
        writer.append(&target, &data)?;

        println!("Successfully appended {} rows to {}", data.len(), target);
        Ok(())
    }

    /// Handle the GSheetsList command
    pub fn handle_gsheets_list(&self, spreadsheet: String) -> Result<()> {
        let config_path = crate::cli::runtime::get()
            .config_path
            .clone()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|p| p.join(".xls-rs.toml"))
                    .unwrap_or_else(|| ".xls-rs.toml".into())
            });

        let config = if config_path.exists() {
            Config::load_from(&config_path.to_string_lossy())?
        } else {
            Config::default()
        };

        let handler = GoogleSheetsHandler::with_config(config);
        let titles = handler.list_sheet_titles(&spreadsheet)?;
        for title in titles {
            println!("{title}");
        }

        Ok(())
    }

    /// Handle the GSheetsAuth command
    pub fn handle_gsheets_auth(&self) -> Result<()> {
        let config_path = crate::cli::runtime::get()
            .config_path
            .clone()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|p| p.join(".xls-rs.toml"))
                    .unwrap_or_else(|| ".xls-rs.toml".into())
            });
        
        let config = if config_path.exists() {
            Config::load_from(&config_path.to_string_lossy())?
        } else {
            Config::default()
        };
        
        crate::cli::runtime::log("Google Sheets Authentication Setup");
        crate::cli::runtime::log("=================================");
        crate::cli::runtime::log("");
        crate::cli::runtime::log("To authenticate with Google Sheets, you have several options:");
        crate::cli::runtime::log("");
        crate::cli::runtime::log("1. Service Account (Recommended for server applications):");
        crate::cli::runtime::log("   - Create a service account at https://console.cloud.google.com/");
        crate::cli::runtime::log("   - Download the JSON key file");
        crate::cli::runtime::log("   - Add this to your config file:");
        crate::cli::runtime::log("     [google_sheets]");
        crate::cli::runtime::log("     service_account_file = \"/path/to/service-account.json\"");
        crate::cli::runtime::log("");
        crate::cli::runtime::log("2. OAuth2 Flow (Recommended for personal use):");
        crate::cli::runtime::log("   - Create OAuth2 credentials at Google Cloud Console");
        crate::cli::runtime::log("   - Download the client secrets JSON file");
        crate::cli::runtime::log("   - Add this to your config file:");
        crate::cli::runtime::log("     [google_sheets]");
        crate::cli::runtime::log("     client_secrets_file = \"/path/to/client-secrets.json\"");
        crate::cli::runtime::log("     token_file = \"/path/to/token.json\"");
        crate::cli::runtime::log("");
        crate::cli::runtime::log("3. API Key (Read-only access to public sheets):");
        crate::cli::runtime::log("   - Create an API key at Google Cloud Console");
        crate::cli::runtime::log("   - Add this to your config file:");
        crate::cli::runtime::log("     [google_sheets]");
        crate::cli::runtime::log("     api_key = \"your-api-key\"");
        crate::cli::runtime::log("");
        crate::cli::runtime::log(format!("Current config file: {}", config_path.display()));
        
        if !config.google_sheets.service_account_file.is_some() 
            && !config.google_sheets.client_secrets_file.is_some() 
            && !config.google_sheets.api_key.is_some() {
            crate::cli::runtime::log("No Google Sheets credentials configured.");
        }
        
        Ok(())
    }

    /// Handle the GSheetsSetDefault command
    pub fn handle_gsheets_set_default(&self, spreadsheet: String) -> Result<()> {
        let config_path = crate::cli::runtime::get()
            .config_path
            .clone()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|p| p.join(".xls-rs.toml"))
                    .unwrap_or_else(|| ".xls-rs.toml".into())
            });
        
        let mut config = if config_path.exists() {
            Config::load_from(&config_path.to_string_lossy())?
        } else {
            Config::default()
        };
        
        config.google_sheets.default_spreadsheet_id = Some(spreadsheet.clone());
        config.save(&config_path.to_string_lossy())?;
        
        crate::cli::runtime::log(format!("Set default Google Sheets to: {}", spreadsheet));
        crate::cli::runtime::log(format!("Config updated at: {}", config_path.display()));
        
        Ok(())
    }

    /// Parse Excel-style cell reference (e.g., "A1" -> row=0, col=0)
    fn parse_cell_ref(&self, cell: &str) -> Result<(usize, usize)> {
        let cell = cell.to_uppercase();
        let (col_part, row_part): (String, String) = cell.chars().partition(|c| c.is_alphabetic());

        // Parse column (base-26)
        let mut col_idx = 0;
        for c in col_part.chars() {
            col_idx = col_idx * 26 + (c as usize - 'A' as usize + 1);
        }
        col_idx -= 1;

        // Parse row
        let row_idx: usize = row_part
            .parse()
            .context(format!("Invalid row number in cell reference: {cell}"))?;
        let row_idx = row_idx - 1; // Convert to 0-indexed

        Ok((row_idx, col_idx))
    }

    /// Apply a cell range filter to data
    fn apply_range(&self, data: &[Vec<String>], range: &str) -> Result<Vec<Vec<String>>> {
        let cell = CellRange::parse(range)?;
        Ok(filter_by_range(data, &cell))
    }

    /// Print data as CSV
    fn print_csv(&self, data: &[Vec<String>]) {
        for row in data {
            println!("{}", row.join(","));
        }
    }

    /// Print data as JSON
    fn print_json(&self, data: &[Vec<String>]) -> Result<()> {
        if data.is_empty() {
            println!("[]");
            return Ok(());
        }

        let headers = &data[0];
        let rows: Vec<serde_json::Value> = data[1..]
            .iter()
            .map(|row| {
                let mut obj = serde_json::Map::new();
                for (i, header) in headers.iter().enumerate() {
                    let value = row.get(i).map(|s| s.as_str()).unwrap_or("");
                    obj.insert(header.clone(), serde_json::json!(value));
                }
                serde_json::Value::Object(obj)
            })
            .collect();

        println!("{}", serde_json::to_string_pretty(&rows)?);
        Ok(())
    }

    /// Print data as JSON Lines (NDJSON) - one JSON object per line
    fn print_jsonl(&self, data: &[Vec<String>]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        let headers = &data[0];
        for row in &data[1..] {
            let mut obj = serde_json::Map::new();
            for (i, header) in headers.iter().enumerate() {
                let value = row.get(i).map(|s| s.as_str()).unwrap_or("");
                obj.insert(header.clone(), serde_json::json!(value));
            }
            println!("{}", serde_json::to_string(&serde_json::Value::Object(obj))?);
        }
        Ok(())
    }

    /// Print data as Markdown table
    fn print_markdown(&self, data: &[Vec<String>]) {
        if data.is_empty() {
            return;
        }

        // Calculate column widths
        let num_cols = data.iter().map(|r| r.len()).max().unwrap_or(0);
        let mut col_widths = vec![0; num_cols];

        for row in data {
            for (i, cell) in row.iter().enumerate() {
                if i < col_widths.len() {
                    col_widths[i] = col_widths[i].max(cell.len());
                }
            }
        }

        // Print header
        if let Some(header) = data.first() {
            for (i, cell) in header.iter().enumerate() {
                print!("| {:<width$} ", cell, width = col_widths[i]);
            }
            println!("|");

            // Print separator
            for width in &col_widths {
                print!("|-{:<width$}-", "", width = width);
            }
            println!("|");
        }

        // Print data rows
        for row in &data[1..] {
            for (i, cell) in row.iter().enumerate() {
                if i < col_widths.len() {
                    print!("| {:<width$} ", cell, width = col_widths[i]);
                }
            }
            println!("|");
        }
    }
}
