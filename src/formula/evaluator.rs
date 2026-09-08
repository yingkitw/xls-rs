//! Formula evaluator

use super::types::{CellRange, FormulaResult};
use crate::excel::ExcelHandler;
use anyhow::{Context, Result};
use std::cell::Cell;

use crate::excel::xlsx_reader::XlsxReader as NativeXlsxReader;

thread_local! {
    static FORMULA_DEPTH: Cell<usize> = const { Cell::new(0) };
}

pub struct FormulaEvaluator {
    excel_handler: ExcelHandler,
    /// Workbook defined names (uppercased key → raw reference), enabling
    /// `SUM(MyRange)`-style formulas. Populated via `with_defined_names`.
    defined_names: std::collections::HashMap<String, String>,
}

impl Default for FormulaEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl FormulaEvaluator {
    pub fn new() -> Self {
        Self {
            excel_handler: ExcelHandler::new(),
            defined_names: std::collections::HashMap::new(),
        }
    }

    /// Register workbook defined names (name → reference, e.g.
    /// `"MyData" → "Sheet1!$A$1:$A$9"`) so formulas can use them.
    /// Keys are matched case-insensitively, as in Excel.
    pub fn with_defined_names(mut self, names: std::collections::HashMap<String, String>) -> Self {
        for (k, v) in names {
            self.defined_names.insert(k.to_uppercase(), v);
        }
        self
    }

    pub fn apply_to_excel(
        &self,
        input: &str,
        output: &str,
        formula: &str,
        cell: &str,
        sheet_name: Option<&str>,
    ) -> Result<()> {
        let workbook = NativeXlsxReader::from_path(input)
            .with_context(|| format!("Failed to open Excel file: {}", input))?;

        let sheet_names = workbook.sheet_names();
        let sheet_name = sheet_name
            .or_else(|| sheet_names.first().map(|s| s.as_str()))
            .ok_or_else(|| anyhow::anyhow!("No sheets found in workbook"))?;

        let sheet = workbook
            .get_sheet_by_name(sheet_name)
            .with_context(|| format!("Failed to read sheet: {}", sheet_name))?;

        use crate::excel::xlsx_writer::RowData;
        use crate::excel::xlsx_writer::XlsxWriter;

        let mut writer = XlsxWriter::new();
        writer.add_sheet(sheet_name)?;

        let (target_row, target_col) = self.parse_cell_reference(cell)?;

        // Extend to the target cell even when it lies outside the existing
        // grid (e.g. column F on a 4-column sheet) — previously such targets
        // silently no-op'd.
        let last_row = (sheet.cells.len() as u32).max(target_row + 1);
        for row_idx in 0..last_row {
            let existing_row = sheet.cells.get(row_idx as usize);
            let mut row_len = existing_row.map(|r| r.len()).unwrap_or(0);
            let is_target_row = row_idx == target_row;
            if is_target_row {
                row_len = row_len.max(target_col as usize + 1);
            }

            let mut row_data = RowData::new();
            for col_idx in 0..row_len {
                if is_target_row && col_idx == target_col as usize {
                    row_data.add_formula(formula);
                } else if let Some(cell) = existing_row.and_then(|r| r.get(col_idx)) {
                    let cell_str = cell.to_string();
                    if let Ok(num) = cell_str.parse::<f64>() {
                        row_data.add_number(num);
                    } else if !cell_str.is_empty() {
                        row_data.add_string(&cell_str);
                    } else {
                        row_data.add_empty();
                    }
                } else {
                    row_data.add_empty();
                }
            }
            writer.add_row(row_data);
        }

        let file = std::fs::File::create(output)?;
        let mut buf_writer =
            std::io::BufWriter::with_capacity(crate::limits::BUFFER_CAPACITY, file);
        writer.save(&mut buf_writer)?;

        Ok(())
    }

    pub fn apply_to_range(
        &self,
        input: &str,
        output: &str,
        formula: &str,
        target_range: &crate::excel::reader::CellRange,
        sheet_name: Option<&str>,
    ) -> Result<usize> {
        let workbook = NativeXlsxReader::from_path(input)
            .with_context(|| format!("Failed to open Excel file: {}", input))?;

        let sheet_names = workbook.sheet_names();
        let sheet_name = sheet_name
            .or_else(|| sheet_names.first().map(|s| s.as_str()))
            .ok_or_else(|| anyhow::anyhow!("No sheets found in workbook"))?;

        let sheet = workbook
            .get_sheet_by_name(sheet_name)
            .with_context(|| format!("Failed to read sheet: {}", sheet_name))?;

        use crate::excel::xlsx_writer::RowData;
        use crate::excel::xlsx_writer::XlsxWriter;

        let mut writer = XlsxWriter::new();
        writer.add_sheet(sheet_name)?;

        let mut cells_affected = 0usize;

        for (row_idx, row) in sheet.cells.iter().enumerate() {
            let mut row_data = RowData::new();
            for (col_idx, cell) in row.iter().enumerate() {
                if row_idx >= target_range.start_row
                    && row_idx <= target_range.end_row
                    && col_idx >= target_range.start_col
                    && col_idx <= target_range.end_col
                {
                    row_data.add_formula(formula);
                    cells_affected += 1;
                } else {
                    let cell_str = cell.to_string();
                    if let Ok(num) = cell_str.parse::<f64>() {
                        row_data.add_number(num);
                    } else if !cell_str.is_empty() {
                        row_data.add_string(&cell_str);
                    } else {
                        row_data.add_empty();
                    }
                }
            }
            writer.add_row(row_data);
        }

        let file = std::fs::File::create(output)?;
        let mut buf_writer =
            std::io::BufWriter::with_capacity(crate::limits::BUFFER_CAPACITY, file);
        writer.save(&mut buf_writer)?;

        Ok(cells_affected)
    }

    pub fn apply_to_csv(
        &self,
        _input: &str,
        _output: &str,
        _formula: &str,
        _cell: &str,
    ) -> Result<()> {
        anyhow::bail!("CSV support has been removed. Use apply_to_range for XLSX files.")
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
        let idx = index
            .checked_sub(1)
            .ok_or_else(|| anyhow::anyhow!("Invalid column"))?;
        if idx > u16::MAX as u32 {
            anyhow::bail!("Column '{}' is out of range", col);
        }
        Ok(idx as u16)
    }

    pub fn evaluate_formula_full(
        &self,
        formula: &str,
        data: &[Vec<String>],
    ) -> Result<FormulaResult> {
        let formula_trimmed = formula.trim();
        let upper = formula_trimmed.to_uppercase();

        if formula_trimmed.starts_with("IF(") {
            self.evaluate_if(formula_trimmed, data)
        } else if formula_trimmed.starts_with("CONCAT(") {
            self.evaluate_concat(formula_trimmed, data)
        } else if upper.starts_with("INDEX(") {
            self.evaluate_index(&upper, data)
        } else if upper.starts_with("UPPER(") {
            self.evaluate_upper(formula_trimmed, data)
        } else if upper.starts_with("LOWER(") {
            self.evaluate_lower(formula_trimmed, data)
        } else if upper.starts_with("TRIM(") {
            self.evaluate_trim(formula_trimmed, data)
        } else if upper.starts_with("LEFT(") {
            self.evaluate_left(formula_trimmed, data)
        } else if upper.starts_with("RIGHT(") {
            self.evaluate_right(formula_trimmed, data)
        } else if upper.starts_with("MID(") {
            self.evaluate_mid(formula_trimmed, data)
        } else if upper.starts_with("AND(") {
            Ok(FormulaResult::Bool(
                self.evaluate_and(formula_trimmed, data)?,
            ))
        } else if upper.starts_with("OR(") {
            Ok(FormulaResult::Bool(
                self.evaluate_or(formula_trimmed, data)?,
            ))
        } else if upper.starts_with("NOT(") {
            Ok(FormulaResult::Bool(
                self.evaluate_not(formula_trimmed, data)?,
            ))
        } else if upper.starts_with("IFERROR(") {
            self.evaluate_iferror(formula_trimmed, data)
        } else {
            let num = self.evaluate_formula(formula_trimmed, data)?;
            Ok(FormulaResult::Number(num))
        }
    }

    pub(crate) fn evaluate_formula(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let formula = formula.trim().to_uppercase();

        FORMULA_DEPTH.with(|d| {
            let prev = d.get();
            if prev >= crate::limits::MAX_FORMULA_DEPTH {
                return Err(anyhow::anyhow!(
                    "Formula evaluation exceeded max depth ({})",
                    crate::limits::MAX_FORMULA_DEPTH
                ));
            }
            d.set(prev + 1);
            let result = self.evaluate_formula_inner(&formula, data);
            d.set(prev);
            result
        })
    }

    fn evaluate_formula_inner(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        // Top-level comparison (outside parentheses/quotes): `A1>=B1`,
        // `SUM(MyData)>Threshold`. Checked first — a leading function call
        // would otherwise swallow the whole expression via its own branch.
        if let Some(v) = self.try_eval_comparison(formula, data)? {
            return Ok(v);
        }

        if formula.starts_with("SUM(") {
            self.evaluate_sum(formula, data)
        } else if formula.starts_with("AVERAGE(") {
            self.evaluate_average(formula, data)
        } else if formula.starts_with("MIN(") {
            self.evaluate_min(formula, data)
        } else if formula.starts_with("MAX(") {
            self.evaluate_max(formula, data)
        } else if formula.starts_with("COUNT(") {
            self.evaluate_count(formula, data)
        } else if formula.starts_with("ROUND(") {
            self.evaluate_round(formula, data)
        } else if formula.starts_with("ABS(") {
            self.evaluate_abs(formula, data)
        } else if formula.starts_with("LEN(") {
            self.evaluate_len(formula, data)
        } else if formula.starts_with("VLOOKUP(") {
            self.evaluate_vlookup(formula, data)
        } else if formula.starts_with("SUMIF(") {
            self.evaluate_sumif(formula, data)
        } else if formula.starts_with("COUNTIF(") {
            self.evaluate_countif(formula, data)
        } else if formula.starts_with("MATCH(") {
            self.evaluate_match(formula, data)
        } else if formula.starts_with("COUNTA(") {
            self.evaluate_counta(formula, data)
        } else if formula.starts_with("AVERAGEIF(") {
            self.evaluate_averageif(formula, data)
        } else if formula.starts_with("MOD(") {
            self.evaluate_mod(formula, data)
        } else if formula.starts_with("INT(") {
            self.evaluate_int(formula, data)
        } else if formula.starts_with("POWER(") {
            self.evaluate_power(formula, data)
        } else if formula.starts_with("SQRT(") {
            self.evaluate_sqrt(formula, data)
        } else if formula.starts_with("ROUNDUP(") {
            self.evaluate_roundup(formula, data)
        } else if formula.starts_with("ROUNDDOWN(") {
            self.evaluate_rounddown(formula, data)
        } else if formula.contains('+')
            || formula.contains('-')
            || formula.contains('*')
            || formula.contains('/')
        {
            self.evaluate_arithmetic(formula, data)
        } else if let Ok(num) = formula.parse::<f64>() {
            Ok(num)
        } else if let Some(target) = self.defined_names.get(&formula.to_uppercase()) {
            // Named single cell used in a scalar context (e.g. `Threshold`)
            let cleaned = target.rsplit('!').next().unwrap_or(target).replace('$', "");
            if let Ok(range) = self.parse_simple_range(&cleaned)
                && range.start_row == range.end_row
                && range.start_col == range.end_col
            {
                return self
                    .get_cell_value_by_index(range.start_row, range.start_col, data)
                    .ok_or_else(|| anyhow::anyhow!("Named cell {} is empty", formula));
            }
            anyhow::bail!("Named range '{}' cannot be used as a scalar value", formula)
        } else {
            self.get_cell_value(formula, data)
        }
    }

    /// Detect and evaluate a comparison at paren-depth 0 (`>`, `>=`, `<`, `<=`,
    /// `=`, `<>`, `!=`), returning 1.0/0.0 (Excel numeric coercion of booleans).
    /// Returns `None` when the expression has no top-level comparison.
    fn try_eval_comparison(&self, formula: &str, data: &[Vec<String>]) -> Result<Option<f64>> {
        const OPS: [&str; 7] = [">=", "<=", "<>", "!=", "=", ">", "<"];
        let bytes = formula.as_bytes();
        let mut depth = 0i32;
        let mut i = 0usize;

        while i < bytes.len() {
            match bytes[i] {
                b'(' => depth += 1,
                b')' => depth -= 1,
                b'"' => {
                    i += 1;
                    while i < bytes.len() && bytes[i] != b'"' {
                        i += 1;
                    }
                }
                _ if depth == 0 => {
                    for op in OPS {
                        if i > 0 && formula[i..].starts_with(op) {
                            let left = formula[..i].trim();
                            let right = formula[i + op.len()..].trim();
                            if left.is_empty() || right.is_empty() {
                                continue;
                            }
                            let l = self.evaluate_formula(left, data)?;
                            let r = self.evaluate_formula(right, data)?;
                            let res = match op {
                                ">=" => l >= r,
                                "<=" => l <= r,
                                "<>" | "!=" => (l - r).abs() > f64::EPSILON,
                                "=" => (l - r).abs() < f64::EPSILON,
                                ">" => l > r,
                                "<" => l < r,
                                _ => false,
                            };
                            return Ok(Some(if res { 1.0 } else { 0.0 }));
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
        Ok(None)
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

    pub(crate) fn evaluate_condition(&self, condition: &str, data: &[Vec<String>]) -> Result<bool> {
        // Logical functions compose inside conditions (e.g. inside IF) and
        // must be checked before operator scanning, since their arguments
        // contain comparison operators.
        let trimmed = condition.trim();
        let upper = trimmed.to_uppercase();
        if upper.starts_with("AND(") {
            return self.evaluate_and(trimmed, data);
        } else if upper.starts_with("OR(") {
            return self.evaluate_or(trimmed, data);
        } else if upper.starts_with("NOT(") {
            return self.evaluate_not(trimmed, data);
        }

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
        self.resolve_range_str(range_str, 0)
    }

    /// Resolve a range reference: `A1:B2`, single cell `B2`, sheet-qualified
    /// `Sheet1!$A$1:$B$2`, or a workbook defined name like `MyData`.
    fn resolve_range_str(&self, raw: &str, depth: usize) -> Result<CellRange> {
        if depth > 4 {
            anyhow::bail!("Defined name resolution too deep (cycle?)");
        }

        // Strip a sheet prefix ("Sheet1!A1:B2") and absolute markers ("$")
        let cleaned = raw.trim();
        let after_sheet = cleaned.rsplit('!').next().unwrap_or(cleaned);
        let stripped = after_sheet.replace('$', "");

        if let Ok(range) = self.parse_simple_range(&stripped) {
            return Ok(range);
        }

        // Not a cell reference — try a defined name (case-insensitive)
        if let Some(target) = self.defined_names.get(&stripped.to_uppercase()) {
            return self.resolve_range_str(target, depth + 1);
        }

        anyhow::bail!("Invalid range: {}", raw)
    }

    fn parse_simple_range(&self, stripped: &str) -> Result<CellRange> {
        if let Some(colon_pos) = stripped.find(':') {
            let start_cell = &stripped[..colon_pos];
            let end_cell = &stripped[colon_pos + 1..];

            let (start_row, start_col) = self.parse_cell_reference(start_cell)?;
            let (end_row, end_col) = self.parse_cell_reference(end_cell)?;

            Ok(CellRange {
                start_row,
                start_col,
                end_row,
                end_col,
            })
        } else {
            let (row, col) = self.parse_cell_reference(stripped)?;
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
