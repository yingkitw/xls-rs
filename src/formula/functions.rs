//! Formula function implementations

use super::evaluator::FormulaEvaluator;
use super::types::{CellRange, FormulaResult};
use crate::regex_cache::cell_reference_regex;
use anyhow::Result;

impl FormulaEvaluator {
    /// Helper function to extract numeric values from a range
    /// Pre-parses all values once to avoid repeated allocations
    fn extract_numeric_values(
        &self,
        range: &crate::formula::types::CellRange,
        data: &[Vec<String>],
    ) -> Vec<f64> {
        // Bound capacity by actual data shape + hard cap — never allocate for
        // A1:XFD1048576-style ranges that exceed the workbook.
        let max_row = (data.len() as u32).saturating_sub(1);
        let end_row = range.end_row.min(max_row);
        if range.start_row > end_row {
            return Vec::new();
        }
        let num_rows = (end_row - range.start_row + 1) as usize;
        let num_cols = (range.end_col.saturating_sub(range.start_col) + 1) as usize;
        let est = num_rows
            .saturating_mul(num_cols)
            .min(crate::limits::MAX_FORMULA_RANGE_CELLS);
        let mut values = Vec::with_capacity(est);
        let mut visited = 0usize;

        for row in range.start_row..=end_row {
            let row_data = &data[row as usize];
            for col in range.start_col..=range.end_col {
                visited += 1;
                if visited > crate::limits::MAX_FORMULA_RANGE_CELLS {
                    return values;
                }
                if (col as usize) < row_data.len()
                    && let Ok(num) = row_data[col as usize].parse::<f64>()
                {
                    values.push(num);
                }
            }
        }

        values
    }

    pub(crate) fn evaluate_sum(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        if let Some(values) = self.try_array_expression(&inner, data)? {
            return Ok(values.iter().sum());
        }
        let range = self.extract_range(formula)?;
        let values = self.extract_numeric_values(&range, data);
        Ok(values.iter().sum())
    }

    pub(crate) fn evaluate_average(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        if let Some(values) = self.try_array_expression(&inner, data)? {
            if values.is_empty() {
                return Ok(0.0);
            }
            return Ok(values.iter().sum::<f64>() / values.len() as f64);
        }
        let range = self.extract_range(formula)?;
        let values = self.extract_numeric_values(&range, data);

        if values.is_empty() {
            Ok(0.0)
        } else {
            Ok(values.iter().sum::<f64>() / values.len() as f64)
        }
    }

    pub(crate) fn evaluate_min(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        if let Some(values) = self.try_array_expression(&inner, data)? {
            return Self::array_min_max(values, true);
        }
        let range = self.extract_range(formula)?;
        let values = self.extract_numeric_values(&range, data);

        values
            .into_iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .ok_or_else(|| anyhow::anyhow!("No numeric values found in range"))
    }

    pub(crate) fn evaluate_max(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        if let Some(values) = self.try_array_expression(&inner, data)? {
            return Self::array_min_max(values, false);
        }
        let range = self.extract_range(formula)?;
        let values = self.extract_numeric_values(&range, data);

        values
            .into_iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .ok_or_else(|| anyhow::anyhow!("No numeric values found in range"))
    }

    fn array_min_max(values: Vec<f64>, want_min: bool) -> Result<f64> {
        values
            .into_iter()
            .reduce(|a, b| {
                if want_min {
                    if b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal)
                        == std::cmp::Ordering::Less
                    {
                        b
                    } else {
                        a
                    }
                } else if b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal)
                    == std::cmp::Ordering::Greater
                {
                    b
                } else {
                    a
                }
            })
            .ok_or_else(|| anyhow::anyhow!("No numeric values found in range"))
    }

    pub(crate) fn evaluate_count(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        if let Some(values) = self.try_array_expression(&inner, data)? {
            // Every element of an arithmetic array is numeric
            return Ok(values.len() as f64);
        }
        let range = self.extract_range(formula)?;
        let values = self.extract_numeric_values(&range, data);
        Ok(values.len() as f64)
    }

    pub(crate) fn evaluate_round(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.is_empty() || args.len() > 2 {
            anyhow::bail!("ROUND requires 1-2 arguments: ROUND(value, [decimals])");
        }

        let value = self.evaluate_formula(&args[0], data)?;
        let decimals = if args.len() > 1 {
            self.evaluate_formula(&args[1], data)? as i32
        } else {
            0
        };

        let multiplier = 10f64.powi(decimals);
        Ok((value * multiplier).round() / multiplier)
    }

    pub(crate) fn evaluate_abs(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        let value = self.evaluate_formula(&inner, data)?;
        Ok(value.abs())
    }

    pub(crate) fn evaluate_len(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        let inner = inner.trim().to_uppercase();

        if let Ok((row, col)) = self.parse_cell_reference(&inner)
            && let Some(text) = self.get_cell_text_by_index(row, col, data)
        {
            return Ok(text.len() as f64);
        }

        let text = inner.trim_matches('"');
        Ok(text.len() as f64)
    }

    pub(crate) fn evaluate_vlookup(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.len() < 3 || args.len() > 4 {
            anyhow::bail!(
                "VLOOKUP requires 3-4 arguments: VLOOKUP(lookup_value, range, col_index, [exact_match])"
            );
        }

        let lookup_value = if let Ok(num) = self.evaluate_formula(&args[0], data) {
            num.to_string()
        } else {
            args[0].trim().trim_matches('"').to_string()
        };

        let range = self.extract_range(&format!("X({})", args[1]))?;
        let col_index = self.evaluate_formula(&args[2], data)? as usize;
        if col_index < 1 {
            anyhow::bail!("VLOOKUP col_index must be >= 1");
        }

        for row in range.start_row..=range.end_row {
            if let Some(cell_text) = self.get_cell_text_by_index(row, range.start_col, data) {
                let matches = if let (Ok(cell_num), Ok(lookup_num)) =
                    (cell_text.parse::<f64>(), lookup_value.parse::<f64>())
                {
                    (cell_num - lookup_num).abs() < f64::EPSILON
                } else {
                    cell_text.to_uppercase() == lookup_value.to_uppercase()
                };

                if matches {
                    let result_col = range.start_col + (col_index as u16 - 1);
                    if let Some(value) = self.get_cell_value_by_index(row, result_col, data) {
                        return Ok(value);
                    } else if let Some(text) = self.get_cell_text_by_index(row, result_col, data)
                        && let Ok(num) = text.parse::<f64>()
                    {
                        return Ok(num);
                    }
                    anyhow::bail!("VLOOKUP: value at result column is not numeric");
                }
            }
        }

        anyhow::bail!("VLOOKUP: no match found for '{}'", lookup_value)
    }

    pub(crate) fn evaluate_sumif(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.len() < 2 || args.len() > 3 {
            anyhow::bail!("SUMIF requires 2-3 arguments: SUMIF(range, criteria, [sum_range])");
        }

        let criteria_range = self.extract_range(&format!("X({})", args[0]))?;
        let criteria = args[1].trim().trim_matches('"').to_string();

        let sum_range = if args.len() == 3 {
            self.extract_range(&format!("X({})", args[2]))?
        } else {
            criteria_range.clone()
        };

        let mut sum = 0.0;

        for row_offset in 0..=(criteria_range.end_row - criteria_range.start_row) {
            let criteria_row = criteria_range.start_row + row_offset;
            let sum_row = sum_range.start_row + row_offset;

            for col_offset in 0..=(criteria_range.end_col - criteria_range.start_col) {
                let criteria_col = criteria_range.start_col + col_offset;
                let sum_col = sum_range.start_col + col_offset;

                if let Some(cell_text) =
                    self.get_cell_text_by_index(criteria_row, criteria_col, data)
                {
                    let matches = self.matches_criteria(&cell_text, &criteria);

                    if matches
                        && let Some(value) = self.get_cell_value_by_index(sum_row, sum_col, data)
                    {
                        sum += value;
                    }
                }
            }
        }

        Ok(sum)
    }

    pub(crate) fn evaluate_countif(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.len() != 2 {
            anyhow::bail!("COUNTIF requires 2 arguments: COUNTIF(range, criteria)");
        }

        let range = self.extract_range(&format!("X({})", args[0]))?;
        let criteria = args[1].trim().trim_matches('"').to_string();

        let mut count = 0;

        for row in range.start_row..=range.end_row {
            for col in range.start_col..=range.end_col {
                if let Some(cell_text) = self.get_cell_text_by_index(row, col, data)
                    && self.matches_criteria(&cell_text, &criteria)
                {
                    count += 1;
                }
            }
        }

        Ok(count as f64)
    }

    /// INDEX(range, row_num, [col_num]) - Returns value at position in range
    /// row_num and col_num are 1-based. If col_num omitted, uses column 1.
    pub(crate) fn evaluate_index(
        &self,
        formula: &str,
        data: &[Vec<String>],
    ) -> Result<super::types::FormulaResult> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.len() < 2 || args.len() > 3 {
            anyhow::bail!("INDEX requires 2-3 arguments: INDEX(range, row_num, [col_num])");
        }

        let range = self.extract_range(&format!("X({})", args[0]))?;
        let row_num = self.evaluate_formula(&args[1], data)? as usize;
        if row_num < 1 {
            anyhow::bail!("INDEX row_num must be >= 1");
        }

        let col_num = if args.len() == 3 {
            self.evaluate_formula(&args[2], data)? as usize
        } else {
            1
        };
        if col_num < 1 {
            anyhow::bail!("INDEX col_num must be >= 1");
        }

        let row = range.start_row + (row_num - 1) as u32;
        let col = range.start_col + (col_num - 1) as u16;

        if let Some(text) = self.get_cell_text_by_index(row, col, data) {
            if let Ok(n) = text.parse::<f64>() {
                return Ok(super::types::FormulaResult::Number(n));
            }
            return Ok(super::types::FormulaResult::Text(text));
        }

        anyhow::bail!(
            "INDEX: cell at row {}, col {} is out of range",
            row_num,
            col_num
        )
    }

    /// MATCH(lookup_value, lookup_array, [match_type]) - Returns 1-based position
    /// match_type: 0 = exact, 1 = less than or equal (ascending), -1 = greater than or equal (descending)
    pub(crate) fn evaluate_match(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.len() < 2 || args.len() > 3 {
            anyhow::bail!(
                "MATCH requires 2-3 arguments: MATCH(lookup_value, lookup_array, [match_type])"
            );
        }

        let lookup_value = if let Ok(num) = self.evaluate_formula(&args[0], data) {
            num.to_string()
        } else {
            args[0].trim().trim_matches('"').to_string()
        };

        let range = self.extract_range(&format!("X({})", args[1]))?;
        let match_type = if args.len() == 3 {
            self.evaluate_formula(&args[2], data)? as i32
        } else {
            1
        };

        let num_rows = (range.end_row - range.start_row + 1) as usize;
        let num_cols = (range.end_col - range.start_col + 1) as usize;

        if num_rows == 1 {
            let mut last_match: Option<f64> = None;
            for col_offset in 0..num_cols {
                let col = range.start_col + col_offset as u16;
                if let Some(cell_text) = self.get_cell_text_by_index(range.start_row, col, data) {
                    let pos = (col_offset + 1) as f64;
                    if self.match_compare(&cell_text, &lookup_value, match_type) {
                        last_match = Some(pos);
                        if match_type == 0 {
                            return Ok(pos);
                        }
                        if match_type == -1 {
                            return Ok(pos);
                        }
                    }
                }
            }
            if let Some(pos) = last_match {
                return Ok(pos);
            }
        } else if num_cols == 1 {
            let mut last_match: Option<f64> = None;
            for row_offset in 0..num_rows {
                let row = range.start_row + row_offset as u32;
                if let Some(cell_text) = self.get_cell_text_by_index(row, range.start_col, data) {
                    let pos = (row_offset + 1) as f64;
                    if self.match_compare(&cell_text, &lookup_value, match_type) {
                        last_match = Some(pos);
                        if match_type == 0 {
                            return Ok(pos);
                        }
                        if match_type == -1 {
                            return Ok(pos);
                        }
                    }
                }
            }
            if let Some(pos) = last_match {
                return Ok(pos);
            }
        } else {
            anyhow::bail!("MATCH lookup_array must be a single row or single column");
        }

        anyhow::bail!("MATCH: no match found for '{}'", lookup_value)
    }

    fn match_compare(&self, cell_text: &str, lookup_value: &str, match_type: i32) -> bool {
        match match_type {
            0 => {
                if let (Ok(cell_num), Ok(lookup_num)) =
                    (cell_text.parse::<f64>(), lookup_value.parse::<f64>())
                {
                    (cell_num - lookup_num).abs() < f64::EPSILON
                } else {
                    cell_text.to_uppercase() == lookup_value.to_uppercase()
                }
            }
            1 => {
                if let (Ok(cell_num), Ok(lookup_num)) =
                    (cell_text.parse::<f64>(), lookup_value.parse::<f64>())
                {
                    cell_num <= lookup_num
                } else {
                    cell_text.to_uppercase() <= lookup_value.to_uppercase()
                }
            }
            -1 => {
                if let (Ok(cell_num), Ok(lookup_num)) =
                    (cell_text.parse::<f64>(), lookup_value.parse::<f64>())
                {
                    cell_num >= lookup_num
                } else {
                    cell_text.to_uppercase() >= lookup_value.to_uppercase()
                }
            }
            _ => false,
        }
    }

    pub(crate) fn matches_criteria(&self, value: &str, criteria: &str) -> bool {
        let criteria = criteria.trim();

        if let Some(rest) = criteria.strip_prefix(">=") {
            if let (Ok(v), Ok(c)) = (value.parse::<f64>(), rest.trim().parse::<f64>()) {
                return v >= c;
            }
        } else if let Some(rest) = criteria.strip_prefix("<=") {
            if let (Ok(v), Ok(c)) = (value.parse::<f64>(), rest.trim().parse::<f64>()) {
                return v <= c;
            }
        } else if let Some(rest) = criteria.strip_prefix("<>") {
            return value != rest.trim();
        } else if let Some(rest) = criteria.strip_prefix("!=") {
            return value != rest.trim();
        } else if let Some(rest) = criteria.strip_prefix('>') {
            if let (Ok(v), Ok(c)) = (value.parse::<f64>(), rest.trim().parse::<f64>()) {
                return v > c;
            }
        } else if let Some(rest) = criteria.strip_prefix('<') {
            if let (Ok(v), Ok(c)) = (value.parse::<f64>(), rest.trim().parse::<f64>()) {
                return v < c;
            }
        } else if let Some(rest) = criteria.strip_prefix('=') {
            return value == rest.trim();
        }

        // Exact match
        value.to_uppercase() == criteria.to_uppercase()
    }

    pub(crate) fn evaluate_arithmetic(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let mut error = None;
        let expr = cell_reference_regex().replace_all(formula, |caps: &regex::Captures| match self
            .get_cell_value(&caps[0], data)
        {
            Ok(value) => value.to_string(),
            Err(err) => {
                if error.is_none() {
                    error = Some(err);
                }
                String::new()
            }
        });

        if let Some(error) = error {
            return Err(error);
        }

        self.evaluate_simple_arithmetic(&expr)
    }

    fn evaluate_simple_arithmetic(&self, expr: &str) -> Result<f64> {
        let expr = expr.replace(" ", "");

        if let Ok(num) = expr.parse::<f64>() {
            return Ok(num);
        }

        // Handle + and - (left to right, lowest precedence)
        let mut depth = 0;
        for (i, c) in expr.chars().rev().enumerate() {
            let pos = expr.len() - 1 - i;
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                '+' if depth == 0 && pos > 0 => {
                    let left = self.evaluate_simple_arithmetic(&expr[..pos])?;
                    let right = self.evaluate_simple_arithmetic(&expr[pos + 1..])?;
                    return Ok(left + right);
                }
                '-' if depth == 0 && pos > 0 => {
                    let left = self.evaluate_simple_arithmetic(&expr[..pos])?;
                    let right = self.evaluate_simple_arithmetic(&expr[pos + 1..])?;
                    return Ok(left - right);
                }
                _ => {}
            }
        }

        // Handle * and /
        depth = 0;
        for (i, c) in expr.chars().rev().enumerate() {
            let pos = expr.len() - 1 - i;
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                '*' if depth == 0 => {
                    let left = self.evaluate_simple_arithmetic(&expr[..pos])?;
                    let right = self.evaluate_simple_arithmetic(&expr[pos + 1..])?;
                    return Ok(left * right);
                }
                '/' if depth == 0 => {
                    let left = self.evaluate_simple_arithmetic(&expr[..pos])?;
                    let right = self.evaluate_simple_arithmetic(&expr[pos + 1..])?;
                    if right == 0.0 {
                        anyhow::bail!("Division by zero");
                    }
                    return Ok(left / right);
                }
                _ => {}
            }
        }

        // Handle parentheses
        if expr.starts_with('(') && expr.ends_with(')') {
            return self.evaluate_simple_arithmetic(&expr[1..expr.len() - 1]);
        }

        anyhow::bail!("Cannot evaluate expression: {}", expr)
    }

    /// COUNTA(range) - Count non-empty cells (any content counts, unlike COUNT
    /// which only counts numeric values)
    pub(crate) fn evaluate_counta(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let range = self.extract_range(formula)?;
        let mut count = 0;

        for row in range.start_row..=range.end_row {
            for col in range.start_col..=range.end_col {
                if let Some(cell_text) = self.get_cell_text_by_index(row, col, data)
                    && !cell_text.trim().is_empty()
                {
                    count += 1;
                }
            }
        }

        Ok(count as f64)
    }

    /// AVERAGEIF(range, criteria, [average_range]) - Average of cells matching
    /// criteria. Returns 0.0 when nothing matches (house convention, matches
    /// `evaluate_average`'s empty-range behavior).
    pub(crate) fn evaluate_averageif(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.len() < 2 || args.len() > 3 {
            anyhow::bail!(
                "AVERAGEIF requires 2-3 arguments: AVERAGEIF(range, criteria, [average_range])"
            );
        }

        let criteria_range = self.extract_range(&format!("X({})", args[0]))?;
        let criteria = args[1].trim().trim_matches('"').to_string();

        let average_range = if args.len() == 3 {
            self.extract_range(&format!("X({})", args[2]))?
        } else {
            criteria_range.clone()
        };

        let mut sum = 0.0;
        let mut count = 0;

        for row_offset in 0..=(criteria_range.end_row - criteria_range.start_row) {
            let criteria_row = criteria_range.start_row + row_offset;
            let avg_row = average_range.start_row + row_offset;

            for col_offset in 0..=(criteria_range.end_col - criteria_range.start_col) {
                let criteria_col = criteria_range.start_col + col_offset;
                let avg_col = average_range.start_col + col_offset;

                if let Some(cell_text) =
                    self.get_cell_text_by_index(criteria_row, criteria_col, data)
                    && self.matches_criteria(&cell_text, &criteria)
                    && let Some(value) = self.get_cell_value_by_index(avg_row, avg_col, data)
                {
                    sum += value;
                    count += 1;
                }
            }
        }

        if count == 0 {
            Ok(0.0)
        } else {
            Ok(sum / count as f64)
        }
    }

    /// MOD(number, divisor) - Remainder with the sign of the divisor
    /// (Excel semantics; Rust's `%` follows the dividend instead).
    pub(crate) fn evaluate_mod(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.len() != 2 {
            anyhow::bail!("MOD requires 2 arguments: MOD(number, divisor)");
        }

        let number = self.evaluate_formula(&args[0], data)?;
        let divisor = self.evaluate_formula(&args[1], data)?;

        if divisor == 0.0 {
            anyhow::bail!("MOD division by zero");
        }

        Ok((number % divisor + divisor) % divisor)
    }

    /// INT(number) - Round down to the nearest integer (toward negative infinity)
    pub(crate) fn evaluate_int(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        let value = self.evaluate_formula(&inner, data)?;
        Ok(value.floor())
    }

    /// POWER(base, exponent)
    pub(crate) fn evaluate_power(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.len() != 2 {
            anyhow::bail!("POWER requires 2 arguments: POWER(base, exponent)");
        }

        let base = self.evaluate_formula(&args[0], data)?;
        let exponent = self.evaluate_formula(&args[1], data)?;
        Ok(base.powf(exponent))
    }

    /// SQRT(number) - Square root; negative input is an error (Excel #NUM!)
    pub(crate) fn evaluate_sqrt(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        let inner = self.extract_function_args(formula)?;
        let value = self.evaluate_formula(&inner, data)?;

        if value < 0.0 {
            anyhow::bail!("SQRT of a negative number");
        }

        Ok(value.sqrt())
    }

    /// ROUNDUP(number, [digits]) - Round away from zero
    pub(crate) fn evaluate_roundup(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        self.round_directional(formula, data, true)
    }

    /// ROUNDDOWN(number, [digits]) - Round toward zero
    pub(crate) fn evaluate_rounddown(&self, formula: &str, data: &[Vec<String>]) -> Result<f64> {
        self.round_directional(formula, data, false)
    }

    fn round_directional(
        &self,
        formula: &str,
        data: &[Vec<String>],
        away_from_zero: bool,
    ) -> Result<f64> {
        let name = if away_from_zero {
            "ROUNDUP"
        } else {
            "ROUNDDOWN"
        };
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.is_empty() || args.len() > 2 {
            anyhow::bail!(
                "{} requires 1-2 arguments: {}(number, [digits])",
                name,
                name
            );
        }

        let value = self.evaluate_formula(&args[0], data)?;
        let digits = if args.len() > 1 {
            self.evaluate_formula(&args[1], data)? as i32
        } else {
            0
        };

        let multiplier = 10f64.powi(digits);
        // Epsilon guards against float representation drift (e.g.
        // 3.14 * 100 = 314.00000000000006) so exact values don't round
        // in the wrong direction.
        const EPS: f64 = 1e-9;
        let rounded = if away_from_zero {
            // Excel ROUNDUP: any nonzero remainder rounds away from zero
            let scaled = value.abs() * multiplier;
            let r = (scaled - EPS).ceil() / multiplier;
            if value.is_sign_negative() { -r } else { r }
        } else {
            // Excel ROUNDDOWN: toward zero
            let scaled = value * multiplier;
            let biased = if value.is_sign_negative() {
                scaled - EPS
            } else {
                scaled + EPS
            };
            biased.trunc() / multiplier
        };
        Ok(rounded)
    }

    /// Try to interpret the aggregate argument as an array (CSE) expression —
    /// ranges and/or scalars combined with arithmetic, e.g. `A1:A3*B1:B3`,
    /// `(A1:A2+B1:B2)*2`, `A1:A3*2`. Returns `None` when the expression has
    /// no top-level operator (caller falls back to the plain range path).
    pub(crate) fn try_array_expression(
        &self,
        expr: &str,
        data: &[Vec<String>],
    ) -> Result<Option<Vec<f64>>> {
        let cleaned = expr
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .trim();
        if !Self::has_top_level_operator(cleaned) {
            return Ok(None);
        }
        Ok(Some(self.array_expr_values(cleaned, 0, data)?))
    }

    fn has_top_level_operator(expr: &str) -> bool {
        let bytes = expr.as_bytes();
        let (mut depth, mut i) = (0i32, 0usize);
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
                _ if depth == 0 => match bytes[i] {
                    b'+' | b'*' | b'/' => return true,
                    b'-' if i > 0 => return true,
                    _ => {}
                },
                _ => {}
            }
            i += 1;
        }
        false
    }

    /// Recursive array-expression evaluator with Excel precedence
    /// (`*`/`/` before `+`/`-`, left-associative).
    fn array_expr_values(
        &self,
        expr: &str,
        depth: usize,
        data: &[Vec<String>],
    ) -> Result<Vec<f64>> {
        if depth > 8 {
            anyhow::bail!("Array expression nesting too deep");
        }

        // Strip parentheses that wrap the entire expression
        let mut e = expr.trim();
        while e.starts_with('(') {
            let Some(close) = Self::matching_paren(e) else {
                anyhow::bail!("Unbalanced parentheses in array expression: {e}");
            };
            if close == e.len() - 1 {
                e = e[1..close].trim();
            } else {
                break;
            }
        }

        if let Some((op, left, right)) = Self::split_top_level(e) {
            let l = self.array_expr_values(&left, depth + 1, data)?;
            let r = self.array_expr_values(&right, depth + 1, data)?;
            return Self::combine_elementwise(op, &l, &r);
        }

        // Leaf: range or scalar
        if e.contains(':') {
            let range = self.extract_range(&format!("X({e})"))?;
            return Ok(self.range_values_positional(&range, data));
        }
        if let Ok(num) = self.evaluate_formula(e, data) {
            return Ok(vec![num]);
        }
        // Non-numeric text evaluates to 0 in array context (Excel-blank-ish)
        Ok(vec![0.0])
    }

    /// Split at the first top-level arithmetic operator, honoring precedence:
    /// lowest-precedence `+`/`-` first (skipping unary signs), then `*`/`/`.
    fn split_top_level(expr: &str) -> Option<(char, String, String)> {
        let bytes = expr.as_bytes();
        let top_level_op = |want_mul: bool| -> Option<usize> {
            let (mut depth, mut i) = (0i32, 0usize);
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
                        let is_pm = matches!(bytes[i], b'+' | b'-');
                        let is_md = matches!(bytes[i], b'*' | b'/');
                        if (want_mul && is_md) || (!want_mul && is_pm && i > 0) {
                            // Skip unary +/- (sign): previous non-space char is an operator or '('
                            let prev = expr[..i].trim_end().chars().last();
                            let unary = matches!(
                                prev,
                                Some('(') | Some('+') | Some('-') | Some('*') | Some('/') | None
                            );
                            if !unary {
                                return Some(i);
                            }
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            None
        };

        let pos = top_level_op(false).or_else(|| top_level_op(true))?;
        let op = expr[pos..].chars().next()?;
        Some((
            op,
            expr[..pos].trim().to_string(),
            expr[pos + op.len_utf8()..].trim().to_string(),
        ))
    }

    fn matching_paren(expr: &str) -> Option<usize> {
        let bytes = expr.as_bytes();
        let mut depth = 0i32;
        for (i, &b) in bytes.iter().enumerate() {
            match b {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Element-wise combine with scalar broadcasting (1-element side applies
    /// to every position). Blanks/text inside ranges count as 0.
    fn combine_elementwise(op: char, l: &[f64], r: &[f64]) -> Result<Vec<f64>> {
        if l.is_empty() || r.is_empty() {
            return Ok(Vec::new());
        }
        let len = if l.len() == r.len() {
            l.len()
        } else if l.len() == 1 {
            r.len()
        } else if r.len() == 1 {
            l.len()
        } else {
            anyhow::bail!("Array shape mismatch: {} vs {} values", l.len(), r.len());
        };
        Ok((0..len)
            .map(|i| {
                let a = if l.len() == 1 { l[0] } else { l[i] };
                let b = if r.len() == 1 { r[0] } else { r[i] };
                match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    '/' => {
                        if b == 0.0 {
                            0.0
                        } else {
                            a / b
                        }
                    }
                    _ => 0.0,
                }
            })
            .collect())
    }

    /// Positional range values: one entry per cell, blanks and non-numeric
    /// text counted as 0.0 (required for element-wise array arithmetic).
    /// Bounded by `MAX_FORMULA_RANGE_CELLS`.
    fn range_values_positional(&self, range: &CellRange, data: &[Vec<String>]) -> Vec<f64> {
        let max_row = (data.len() as u32).saturating_sub(1);
        let end_row = range.end_row.min(max_row);
        let mut values = Vec::new();
        if range.start_row > end_row {
            return values;
        }
        'outer: for row in range.start_row..=end_row {
            let row_data = &data[row as usize];
            for col in range.start_col..=range.end_col {
                if values.len() >= crate::limits::MAX_FORMULA_RANGE_CELLS {
                    break 'outer;
                }
                let v = row_data
                    .get(col as usize)
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                values.push(v);
            }
        }
        values
    }

    /// Resolve a function argument as a logical value: comparison expression,
    /// nested AND/OR/NOT, TRUE/FALSE literal, or nonzero number.
    pub(crate) fn eval_bool_arg(&self, arg: &str, data: &[Vec<String>]) -> Result<bool> {
        let arg = arg.trim();
        if let Ok(b) = self.evaluate_condition(arg, data) {
            return Ok(b);
        }
        match arg.to_uppercase().as_str() {
            "TRUE" => return Ok(true),
            "FALSE" => return Ok(false),
            _ => {}
        }
        if let Ok(num) = self.evaluate_formula(arg, data) {
            return Ok(num != 0.0);
        }
        anyhow::bail!("Cannot interpret '{}' as a logical value", arg)
    }

    /// AND(condition1, [condition2], ...) - TRUE when every argument is TRUE
    pub(crate) fn evaluate_and(&self, formula: &str, data: &[Vec<String>]) -> Result<bool> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.is_empty() {
            anyhow::bail!("AND requires at least 1 argument");
        }

        for arg in &args {
            if !self.eval_bool_arg(arg, data)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// OR(condition1, [condition2], ...) - TRUE when any argument is TRUE
    pub(crate) fn evaluate_or(&self, formula: &str, data: &[Vec<String>]) -> Result<bool> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.is_empty() {
            anyhow::bail!("OR requires at least 1 argument");
        }

        for arg in &args {
            if self.eval_bool_arg(arg, data)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// NOT(condition) - Logical negation
    pub(crate) fn evaluate_not(&self, formula: &str, data: &[Vec<String>]) -> Result<bool> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.len() != 1 {
            anyhow::bail!("NOT requires exactly 1 argument");
        }

        Ok(!self.eval_bool_arg(&args[0], data)?)
    }

    /// IFERROR(value, value_if_error) - Fallback when the first argument
    /// fails to evaluate or yields an Excel error literal (#DIV/0!, #N/A, ...)
    pub(crate) fn evaluate_iferror(
        &self,
        formula: &str,
        data: &[Vec<String>],
    ) -> Result<FormulaResult> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.len() != 2 {
            anyhow::bail!("IFERROR requires 2 arguments: IFERROR(value, value_if_error)");
        }

        match self.evaluate_formula_full(&args[0], data) {
            Ok(FormulaResult::Text(t)) if Self::is_error_literal(&t) => {
                self.evaluate_formula_full(&args[1], data)
            }
            Ok(result) => Ok(result),
            Err(_) => self.evaluate_formula_full(&args[1], data),
        }
    }

    /// Excel error literals that IFERROR catches when stored as cell text
    fn is_error_literal(s: &str) -> bool {
        matches!(
            s.trim(),
            "#N/A" | "#VALUE!" | "#REF!" | "#DIV/0!" | "#NUM!" | "#NAME?" | "#NULL!"
        )
    }

    /// Resolve a function argument as text: quoted literal, cell reference, or
    /// evaluated expression. Shared by the text functions.
    fn resolve_text_arg(&self, arg: &str, data: &[Vec<String>]) -> String {
        let arg = arg.trim();
        if arg.starts_with('"') && arg.ends_with('"') && arg.len() >= 2 {
            return arg[1..arg.len() - 1].to_string();
        }
        if let Ok((row, col)) = self.parse_cell_reference(arg)
            && let Some(text) = self.get_cell_text_by_index(row, col, data)
        {
            return text;
        }
        if let Ok(num) = self.evaluate_formula(arg, data) {
            return num.to_string();
        }
        arg.to_string()
    }

    /// UPPER(text)
    pub(crate) fn evaluate_upper(
        &self,
        formula: &str,
        data: &[Vec<String>],
    ) -> Result<FormulaResult> {
        let inner = self.extract_function_args(formula)?;
        Ok(FormulaResult::Text(
            self.resolve_text_arg(&inner, data).to_uppercase(),
        ))
    }

    /// LOWER(text)
    pub(crate) fn evaluate_lower(
        &self,
        formula: &str,
        data: &[Vec<String>],
    ) -> Result<FormulaResult> {
        let inner = self.extract_function_args(formula)?;
        Ok(FormulaResult::Text(
            self.resolve_text_arg(&inner, data).to_lowercase(),
        ))
    }

    /// TRIM(text) - Strip leading/trailing whitespace and collapse internal
    /// whitespace runs to single spaces (Excel semantics)
    pub(crate) fn evaluate_trim(
        &self,
        formula: &str,
        data: &[Vec<String>],
    ) -> Result<FormulaResult> {
        let inner = self.extract_function_args(formula)?;
        let text = self.resolve_text_arg(&inner, data);
        Ok(FormulaResult::Text(
            text.split_whitespace().collect::<Vec<_>>().join(" "),
        ))
    }

    /// LEFT(text, [num_chars]) - First N characters (default 1)
    pub(crate) fn evaluate_left(
        &self,
        formula: &str,
        data: &[Vec<String>],
    ) -> Result<FormulaResult> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.is_empty() || args.len() > 2 {
            anyhow::bail!("LEFT requires 1-2 arguments: LEFT(text, [num_chars])");
        }

        let text = self.resolve_text_arg(&args[0], data);
        let n = if args.len() > 1 {
            self.evaluate_formula(&args[1], data)?.max(0.0) as usize
        } else {
            1
        };

        Ok(FormulaResult::Text(text.chars().take(n).collect()))
    }

    /// RIGHT(text, [num_chars]) - Last N characters (default 1)
    pub(crate) fn evaluate_right(
        &self,
        formula: &str,
        data: &[Vec<String>],
    ) -> Result<FormulaResult> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.is_empty() || args.len() > 2 {
            anyhow::bail!("RIGHT requires 1-2 arguments: RIGHT(text, [num_chars])");
        }

        let text = self.resolve_text_arg(&args[0], data);
        let n = if args.len() > 1 {
            self.evaluate_formula(&args[1], data)?.max(0.0) as usize
        } else {
            1
        };

        let skip = text.chars().count().saturating_sub(n);
        Ok(FormulaResult::Text(text.chars().skip(skip).collect()))
    }

    /// MID(text, start, num_chars) - 1-based substring
    pub(crate) fn evaluate_mid(
        &self,
        formula: &str,
        data: &[Vec<String>],
    ) -> Result<FormulaResult> {
        let inner = self.extract_function_args(formula)?;
        let args = self.split_args(&inner)?;

        if args.len() != 3 {
            anyhow::bail!("MID requires 3 arguments: MID(text, start, num_chars)");
        }

        let text = self.resolve_text_arg(&args[0], data);
        let start = self.evaluate_formula(&args[1], data)?;
        let n = self.evaluate_formula(&args[2], data)?.max(0.0) as usize;

        if start < 1.0 {
            anyhow::bail!("MID start must be >= 1");
        }

        Ok(FormulaResult::Text(
            text.chars().skip((start - 1.0) as usize).take(n).collect(),
        ))
    }
}
