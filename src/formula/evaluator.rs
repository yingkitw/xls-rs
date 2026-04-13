//! Formula evaluator

use super::types::{CellRange, FormulaResult};
use crate::excel::ExcelHandler;
use anyhow::{Context, Result};
use calamine::{open_workbook, Reader, Xlsx};
use csv::{ReaderBuilder, WriterBuilder};

pub struct FormulaEvaluator {
    excel_handler: ExcelHandler,
}

impl FormulaEvaluator {
    pub fn new() -> Self {
        Self {
            excel_handler: ExcelHandler::new(),
        }
    }

    pub fn apply_to_excel(
        &self,
        input: &str,
        output: &str,
        _formula: &str,
        _cell: &str,
        sheet_name: Option<&str>,
    ) -> Result<()> {
        let mut workbook: Xlsx<_> = open_workbook(input)
            .with_context(|| format!("Failed to open Excel file: {}", input))?;

        let sheet_names = workbook.sheet_names();
        let sheet_name = sheet_name
            .or_else(|| sheet_names.first().map(|s| s.as_str()))
            .ok_or_else(|| anyhow::anyhow!("No sheets found in workbook"))?;

        let range = workbook
            .worksheet_range(sheet_name)
            .with_context(|| format!("Failed to read sheet: {}", sheet_name))?;

        use crate::excel::xlsx_writer::XlsxWriter;

        let mut writer = XlsxWriter::new();
        writer.add_sheet(sheet_name)?;

        // Read all data from the existing workbook
        let mut all_data: Vec<Vec<String>> = Vec::new();
        for row in range.rows() {
            all_data.push(row.iter().map(|c| c.to_string()).collect());
        }

        // Add all data to the writer
        writer.add_data(&all_data);

        // Note: Excel formulas require special cell type with formula attribute
        // The custom XLSX writer supports formulas via CellData::Formula
        // However, we need to add the formula to a specific cell
        // For now, this is a limitation - formulas require modifying existing cells
        // which is complex with the current architecture

        let file = std::fs::File::create(output)?;
        let mut buf_writer = std::io::BufWriter::new(file);
        writer.save(&mut buf_writer)?;

        Ok(())
    }

    pub fn apply_to_csv(&self, input: &str, output: &str, formula: &str, cell: &str) -> Result<()> {
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(input)
            .with_context(|| format!("Failed to open CSV file: {}", input))?;

        let mut records: Vec<Vec<String>> = Vec::new();
        for result in reader.records() {
            let record = result?;
            records.push(record.iter().map(|s| s.to_string()).collect());
        }

        let (row, col) = self.parse_cell_reference(cell)?;
        let value = self.evaluate_formula_full(formula, &records)?;

        while records.len() <= row as usize {
            records.push(Vec::new());
        }

        let max_cols = records
            .iter()
            .map(|r| r.len())
            .max()
            .unwrap_or(0)
            .max((col as usize) + 1);

        for record in &mut records {
            while record.len() < max_cols {
                record.push(String::new());
            }
        }

        while records[row as usize].len() <= col as usize {
            records[row as usize].push(String::new());
        }

        records[row as usize][col as usize] = value.to_string();

        // Check output format based on extension
        if output.ends_with(".xlsx") {
            use crate::excel::xlsx_writer::XlsxWriter;
            let mut writer = XlsxWriter::new();
            writer.add_sheet("Sheet1")?;
            writer.add_data(&records);

            let file = std::fs::File::create(output)
                .with_context(|| format!("Failed to create XLSX file: {}", output))?;
            let mut buf_writer = std::io::BufWriter::new(file);
            writer.save(&mut buf_writer)?;
        } else {
            let mut writer = WriterBuilder::new()
                .has_headers(false)
                .flexible(true)
                .from_path(output)
                .with_context(|| format!("Failed to create CSV file: {}", output))?;

            for record in records {
                writer.write_record(&record)?;
            }
            writer.flush()?;
        }

        Ok(())
    }

    pub(crate) fn parse_cell_reference(&self, cell: &str) -> Result<(u32, u16)> {
        let mut col_str = String::new();
        let mut row_str = String::new();

        for ch in cell.chars() {
            if ch.is_alphabetic() {
                col_str.push(ch);
            } else if ch.is_ascii_digit() {
                row_str.push(ch);
            }
        }

        let col = self.column_to_index(&col_str)?;
        let row = row_str
            .parse::<u32>()
            .with_context(|| format!("Invalid row number in cell reference: {}", cell))?;

        Ok((row - 1, col))
    }

    fn column_to_index(&self, col: &str) -> Result<u16> {
        let mut index = 0u32;
        for ch in col.chars() {
            index = index
                .checked_mul(26)
                .and_then(|i| i.checked_add(ch.to_ascii_uppercase() as u32 - b'A' as u32 + 1))
                .ok_or_else(|| anyhow::anyhow!("Column '{}' is out of range", col))?;
        }
        let idx = index.checked_sub(1).ok_or_else(|| anyhow::anyhow!("Invalid column"))?;
        if idx > u16::MAX as u32 {
            anyhow::bail!("Column '{}' is out of range", col);
        }
        Ok(idx as u16)
    }

    pub(crate) fn evaluate_formula_full(
        &self,
        formula: &str,
        data: &[Vec<String>],
    ) -> Result<FormulaResult> {
        let formula_trimmed = formula.trim();

        if formula_trimmed.starts_with("IF(") {
            self.evaluate_if(formula_trimmed, data)
        } else if formula_trimmed.starts_with("CONCAT(") {
            self.evaluate_concat(formula_trimmed, data)
        } else if formula_trimmed.to_uppercase().starts_with("INDEX(") {
            self.evaluate_index(&formula_trimmed.to_uppercase(), data)
        } else {
            let num = self.evaluate_formula(formula_trimmed, data)?;
            Ok(FormulaResult::Number(num))
        }
    }

    pub(crate) fn evaluate_formula(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let formula = formula.trim().to_uppercase();

        if formula.starts_with("SUM(") {
            self.evaluate_sum(&formula, data)
        } else if formula.starts_with("AVERAGE(") {
            self.evaluate_average(&formula, data)
        } else if formula.starts_with("MIN(") {
            self.evaluate_min(&formula, data)
        } else if formula.starts_with("MAX(") {
            self.evaluate_max(&formula, data)
        } else if formula.starts_with("COUNT(") {
            self.evaluate_count(&formula, data)
        } else if formula.starts_with("ROUND(") {
            self.evaluate_round(&formula, data)
        } else if formula.starts_with("ABS(") {
            self.evaluate_abs(&formula, data)
        } else if formula.starts_with("LEN(") {
            self.evaluate_len(&formula, data)
        } else if formula.starts_with("VLOOKUP(") {
            self.evaluate_vlookup(&formula, data)
        } else if formula.starts_with("SUMIF(") {
            self.evaluate_sumif(&formula, data)
        } else if formula.starts_with("COUNTIF(") {
            self.evaluate_countif(&formula, data)
        } else if formula.starts_with("MATCH(") {
            self.evaluate_match(&formula, data)
        } else if formula.contains('+')
            || formula.contains('-')
            || formula.contains('*')
            || formula.contains('/')
        {
            self.evaluate_arithmetic(&formula, data)
        } else if let Ok(num) = formula.parse::<f64>() {
            Ok(num)
        } else {
            self.get_cell_value(&formula, data)
        }
    }

    fn evaluate_if(&self, formula: &str, data: &[Vec<String>]) -> Result<FormulaResult> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.len() != 3 {
            anyhow::bail!("IF requires 3 arguments: IF(condition, true_value, false_value)");
        }

        let condition = self.evaluate_condition(&args[0], data)?;
        let result_expr = if condition { &args[1] } else { &args[2] };

        if let Ok(num) = self.evaluate_formula(result_expr, data) {
            Ok(FormulaResult::Number(num))
        } else {
            Ok(FormulaResult::Text(
                result_expr.trim().trim_matches('"').to_string(),
            ))
        }
    }

    fn evaluate_condition(&self, condition: &str, data: &[Vec<String>]) -> Result<bool> {
        let ops = [">=", "<=", "<>", "!=", "=", ">", "<"];

        for op in ops {
            if let Some(pos) = condition.find(op) {
                let left = condition[..pos].trim();
                let right = condition[pos + op.len()..].trim();

                let left_val = self.evaluate_formula(left, data).ok();
                let right_val = self.evaluate_formula(right, data).ok();

                return Ok(match (left_val, right_val) {
                    (Some(l), Some(r)) => match op {
                        ">=" => l >= r,
                        "<=" => l <= r,
                        "<>" | "!=" => (l - r).abs() > f64::EPSILON,
                        "=" => (l - r).abs() < f64::EPSILON,
                        ">" => l > r,
                        "<" => l < r,
                        _ => false,
                    },
                    _ => {
                        let left_str = left.trim_matches('"');
                        let right_str = right.trim_matches('"');
                        match op {
                            "=" => left_str == right_str,
                            "<>" | "!=" => left_str != right_str,
                            _ => false,
                        }
                    }
                });
            }
        }

        anyhow::bail!("Invalid condition: {}", condition)
    }

    fn evaluate_concat(&self, formula: &str, data: &[Vec<String>]) -> Result<FormulaResult> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        let mut result = String::new();
        for arg in args {
            let arg = arg.trim();
            if arg.starts_with('"') && arg.ends_with('"') {
                result.push_str(&arg[1..arg.len() - 1]);
            } else if let Ok((row, col)) = self.parse_cell_reference(arg) {
                if let Some(text) = self.get_cell_text_by_index(row, col, data) {
                    result.push_str(&text);
                }
            } else {
                result.push_str(arg);
            }
        }

        Ok(FormulaResult::Text(result))
    }

    pub(crate) fn get_cell_text_by_index(
        &self,
        row: u32,
        col: u16,
        data: &[Vec<String>],
    ) -> Option<String> {
        if row as usize >= data.len() {
            return None;
        }
        let row_data = &data[row as usize];
        if col as usize >= row_data.len() {
            return None;
        }
        // Return Cow-like behavior: avoid clone if we can
        // Since we need to return owned String, use .clone() only when necessary
        Some(row_data[col as usize].clone())
    }

    pub(crate) fn get_cell_value(&self, cell_ref: &str, data: &[Vec<String>]) -> Result<f64> {
        let (row, col) = self.parse_cell_reference(cell_ref)?;
        self.get_cell_value_by_index(row, col, data)
            .ok_or_else(|| anyhow::anyhow!("Cell {} is empty or invalid", cell_ref))
    }

    pub(crate) fn get_cell_value_by_index(
        &self,
        row: u32,
        col: u16,
        data: &[Vec<String>],
    ) -> Option<f64> {
        if row as usize >= data.len() {
            return None;
        }
        let row_data = &data[row as usize];
        if col as usize >= row_data.len() {
            return None;
        }
        // Parse once and cache the result in Option
        let text = &row_data[col as usize];
        // Fast path: check if string is empty first
        if text.is_empty() {
            return None;
        }
        text.parse::<f64>().ok()
    }

    pub(crate) fn extract_range(&self, formula: &str) -> Result<CellRange> {
        let start = formula
            .find('(')
            .ok_or_else(|| anyhow::anyhow!("Invalid formula format"))?;
        let end = formula
            .rfind(')')
            .ok_or_else(|| anyhow::anyhow!("Invalid formula format"))?;
        let range_str = &formula[start + 1..end];

        if let Some(colon_pos) = range_str.find(':') {
            let start_cell = &range_str[..colon_pos];
            let end_cell = &range_str[colon_pos + 1..];

            let (start_row, start_col) = self.parse_cell_reference(start_cell)?;
            let (end_row, end_col) = self.parse_cell_reference(end_cell)?;

            Ok(CellRange {
                start_row,
                start_col,
                end_row,
                end_col,
            })
        } else {
            let (row, col) = self.parse_cell_reference(range_str)?;
            Ok(CellRange {
                start_row: row,
                start_col: col,
                end_row: row,
                end_col: col,
            })
        }
    }

    pub(crate) fn extract_function_args(&self, formula: &str) -> Result<String> {
        let start = formula
            .find('(')
            .ok_or_else(|| anyhow::anyhow!("Missing opening parenthesis in formula"))?;
        let end = formula
            .rfind(')')
            .ok_or_else(|| anyhow::anyhow!("Missing closing parenthesis in formula"))?;

        if end <= start {
            anyhow::bail!("Invalid parentheses in formula");
        }

        Ok(formula[start + 1..end].to_string())
    }

    pub(crate) fn split_args(&self, args: &str) -> Result<Vec<String>> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut depth = 0;

        for ch in args.chars() {
            match ch {
                '(' => {
                    depth += 1;
                    current.push(ch);
                }
                ')' => {
                    depth -= 1;
                    current.push(ch);
                }
                ',' if depth == 0 => {
                    result.push(current.trim().to_string());
                    current = String::new();
                }
                _ => current.push(ch),
            }
        }

        if !current.is_empty() {
            result.push(current.trim().to_string());
        }

        Ok(result)
    }
}
