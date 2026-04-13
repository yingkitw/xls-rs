//! Data transformation operations

use super::core::DataOperations;
use super::types::SortOrder;
use crate::regex_cache::where_clause_regex;
use anyhow::Result;
use rayon::prelude::*;

#[derive(Clone, Copy)]
enum RollingAgg {
    Mean,
    Sum,
}

struct QueryCondition {
    column: usize,
    operator: String,
    value: String,
}

impl DataOperations {
    /// Query with SQL-like WHERE clause
    pub fn query(&self, data: &[Vec<String>], where_clause: &str) -> Result<Vec<Vec<String>>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let header = &data[0];
        let mut result = Vec::with_capacity(data.len());
        result.push(header.clone());

        let conditions = self.parse_where_clause(where_clause, header)?;

        for row in data.iter().skip(1) {
            if self.evaluate_conditions(row, &conditions, header)? {
                result.push(row.clone());
            }
        }

        Ok(result)
    }

    fn parse_where_clause(&self, clause: &str, header: &[String]) -> Result<Vec<QueryCondition>> {
        let mut conditions = Vec::new();
        let re_pattern = where_clause_regex();

        for cap in re_pattern.captures_iter(clause) {
            let col_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let op = cap.get(2).map(|m| m.as_str()).unwrap_or("=");
            let value = cap.get(3).map(|m| m.as_str().trim()).unwrap_or("");

            let col_idx = header
                .iter()
                .position(|h| h == col_name)
                .ok_or_else(|| anyhow::anyhow!("Column '{}' not found", col_name))?;

            conditions.push(QueryCondition {
                column: col_idx,
                operator: op.to_string(),
                value: value.to_string(),
            });
        }

        Ok(conditions)
    }

    fn evaluate_conditions(
        &self,
        row: &[String],
        conditions: &[QueryCondition],
        _header: &[String],
    ) -> Result<bool> {
        for cond in conditions {
            let cell_value = row.get(cond.column).map(|s| s.as_str()).unwrap_or("");
            if !self.evaluate_filter_condition(cell_value, &cond.operator, &cond.value)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Add computed column using formula
    pub fn mutate(
        &self,
        data: &mut Vec<Vec<String>>,
        new_col_name: &str,
        formula: &str,
    ) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        data[0].push(new_col_name.to_string());
        let header = data[0].clone();

        for row_idx in 1..data.len() {
            let value = self.evaluate_row_formula(formula, &data[row_idx], &header)?;
            data[row_idx].push(value);
        }

        Ok(())
    }

    fn evaluate_row_formula(
        &self,
        formula: &str,
        row: &[String],
        header: &[String],
    ) -> Result<String> {
        let mut expr = formula.to_string();

        for (idx, col_name) in header.iter().enumerate() {
            if expr.contains(col_name) {
                let val = row.get(idx).cloned().unwrap_or_default();
                expr = expr.replace(col_name, &val);
            }
        }

        for idx in 0..row.len() {
            let letter = (b'A' + idx as u8) as char;
            let pattern = format!("{}", letter);
            if expr.contains(&pattern) {
                let val = row.get(idx).cloned().unwrap_or_default();
                expr = expr.replace(&pattern, &val);
            }
        }

        if let Ok(result) = self.eval_arithmetic(&expr) {
            return Ok(format!("{:.2}", result));
        }

        Ok(expr)
    }

    pub(crate) fn eval_arithmetic(&self, expr: &str) -> Result<f64> {
        let expr = expr.replace(" ", "");

        if let Ok(n) = expr.parse::<f64>() {
            return Ok(n);
        }

        if let Some(pos) = expr.rfind('*') {
            let left = self.eval_arithmetic(&expr[..pos])?;
            let right = self.eval_arithmetic(&expr[pos + 1..])?;
            return Ok(left * right);
        }
        if let Some(pos) = expr.rfind('/') {
            let left = self.eval_arithmetic(&expr[..pos])?;
            let right = self.eval_arithmetic(&expr[pos + 1..])?;
            if right == 0.0 {
                anyhow::bail!("Division by zero");
            }
            return Ok(left / right);
        }

        let bytes = expr.as_bytes();
        for i in (1..bytes.len()).rev() {
            if bytes[i] == b'+' {
                let left = self.eval_arithmetic(&expr[..i])?;
                let right = self.eval_arithmetic(&expr[i + 1..])?;
                return Ok(left + right);
            }
            if bytes[i] == b'-' {
                let left = self.eval_arithmetic(&expr[..i])?;
                let right = self.eval_arithmetic(&expr[i + 1..])?;
                return Ok(left - right);
            }
        }

        anyhow::bail!("Cannot evaluate: {}", expr)
    }

    /// Cast column to specified type
    pub fn astype(&self, data: &mut Vec<Vec<String>>, column: usize, dtype: &str) -> Result<usize> {
        if data.is_empty() {
            return Ok(0);
        }

        let mut converted = 0;
        for row in data.iter_mut().skip(1) {
            if let Some(cell) = row.get_mut(column) {
                let new_val = match dtype.to_lowercase().as_str() {
                    "int" | "integer" => {
                        if let Ok(f) = cell.parse::<f64>() {
                            converted += 1;
                            (f as i64).to_string()
                        } else {
                            cell.clone()
                        }
                    }
                    "float" | "double" => {
                        if let Ok(f) = cell.parse::<f64>() {
                            converted += 1;
                            format!("{:.2}", f)
                        } else {
                            cell.clone()
                        }
                    }
                    "string" | "str" => {
                        converted += 1;
                        cell.clone()
                    }
                    "bool" | "boolean" => {
                        let lower = cell.to_lowercase();
                        converted += 1;
                        if lower == "true" || lower == "1" || lower == "yes" {
                            "true".to_string()
                        } else if lower == "false" || lower == "0" || lower == "no" {
                            "false".to_string()
                        } else {
                            cell.clone()
                        }
                    }
                    _ => anyhow::bail!("Unknown type: {}. Use: int, float, string, bool", dtype),
                };
                *cell = new_val;
            }
        }

        Ok(converted)
    }

    /// Sort by multiple columns
    pub fn sort_by_columns(
        &self,
        data: &mut Vec<Vec<String>>,
        columns: &[(usize, SortOrder)],
    ) -> Result<()> {
        if data.len() <= 1 || columns.is_empty() {
            return Ok(());
        }

        let header = data.remove(0);

        // Use parallel sort for better performance on large datasets
        data.par_sort_by(|a, b| {
            for (col, order) in columns {
                let val_a = a.get(*col).map(|s| s.as_str()).unwrap_or("");
                let val_b = b.get(*col).map(|s| s.as_str()).unwrap_or("");

                let cmp = match (val_a.parse::<f64>(), val_b.parse::<f64>()) {
                    (Ok(num_a), Ok(num_b)) => num_a
                        .partial_cmp(&num_b)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    _ => val_a.cmp(val_b),
                };

                let cmp = match order {
                    SortOrder::Ascending => cmp,
                    SortOrder::Descending => cmp.reverse(),
                };

                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
            }
            std::cmp::Ordering::Equal
        });

        data.insert(0, header);
        Ok(())
    }

    /// Apply a function to each cell in a column
    pub fn apply_column<F>(&self, data: &mut Vec<Vec<String>>, column: usize, f: F) -> Result<()>
    where
        F: Fn(&str) -> String,
    {
        for row in data.iter_mut().skip(1) {
            if let Some(cell) = row.get_mut(column) {
                *cell = f(cell);
            }
        }
        Ok(())
    }

    /// Clip values to a range
    pub fn clip(
        &self,
        data: &mut Vec<Vec<String>>,
        column: usize,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Result<usize> {
        let mut clipped = 0;

        for row in data.iter_mut().skip(1) {
            if let Some(cell) = row.get_mut(column) {
                if let Ok(val) = cell.parse::<f64>() {
                    let mut new_val = val;
                    if let Some(min_val) = min {
                        if val < min_val {
                            new_val = min_val;
                            clipped += 1;
                        }
                    }
                    if let Some(max_val) = max {
                        if val > max_val {
                            new_val = max_val;
                            clipped += 1;
                        }
                    }
                    if new_val != val {
                        *cell = format!("{:.2}", new_val);
                    }
                }
            }
        }

        Ok(clipped)
    }

    /// Normalize column values (0-1 range)
    pub fn normalize(&self, data: &mut Vec<Vec<String>>, column: usize) -> Result<()> {
        let values: Vec<f64> = data
            .par_iter()
            .skip(1)
            .filter_map(|row| row.get(column))
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();

        if values.is_empty() {
            return Ok(());
        }

        // Use parallel reduce for min/max calculation on large datasets
        let (min_val, max_val) = if values.len() > 1000 {
            let (min, max) = values.par_iter().fold(
                || (f64::INFINITY, f64::NEG_INFINITY),
                |(acc_min, acc_max), &val| (acc_min.min(val), acc_max.max(val)),
            ).reduce(
                || (f64::INFINITY, f64::NEG_INFINITY),
                |(min1, max1), (min2, max2)| (min1.min(min2), max1.max(max2)),
            );
            (min, max)
        } else {
            let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            (min_val, max_val)
        };

        let range = max_val - min_val;

        if range == 0.0 {
            return Ok(());
        }

        for row in data.iter_mut().skip(1) {
            if let Some(cell) = row.get_mut(column) {
                if let Ok(val) = cell.parse::<f64>() {
                    let normalized = (val - min_val) / range;
                    *cell = format!("{:.4}", normalized);
                }
            }
        }

        Ok(())
    }

    /// Standardize numeric column to z-scores using population mean and standard deviation.
    ///
    /// Non-numeric cells are left unchanged. If the column has fewer than two numeric values
    /// or the standard deviation is zero, cells are unchanged.
    pub fn zscore(&self, data: &mut Vec<Vec<String>>, column: usize) -> Result<()> {
        let values: Vec<f64> = data
            .iter()
            .skip(1)
            .filter_map(|row| row.get(column))
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();

        if values.len() < 2 {
            return Ok(());
        }

        let n = values.len() as f64;
        let mean = values.iter().sum::<f64>() / n;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
        let std = variance.sqrt();

        if std < f64::EPSILON {
            return Ok(());
        }

        for row in data.iter_mut().skip(1) {
            if let Some(cell) = row.get_mut(column) {
                if let Ok(val) = cell.parse::<f64>() {
                    let z = (val - mean) / std;
                    *cell = format!("{:.6}", z);
                }
            }
        }

        Ok(())
    }

    /// Append a column with rolling **mean** of `value_col` over the last `window` data rows (inclusive).
    /// Header is row 0; partial windows at the start use available rows (min_periods = 1).
    pub fn rolling_mean_column(
        &self,
        data: &mut Vec<Vec<String>>,
        value_col: usize,
        window: usize,
        new_col_name: &str,
    ) -> Result<()> {
        self.rolling_column(data, value_col, window, new_col_name, RollingAgg::Mean)
    }

    /// Append a column with rolling **sum** of `value_col` over the last `window` data rows (inclusive).
    pub fn rolling_sum_column(
        &self,
        data: &mut Vec<Vec<String>>,
        value_col: usize,
        window: usize,
        new_col_name: &str,
    ) -> Result<()> {
        self.rolling_column(data, value_col, window, new_col_name, RollingAgg::Sum)
    }

    fn rolling_column(
        &self,
        data: &mut Vec<Vec<String>>,
        value_col: usize,
        window: usize,
        new_col_name: &str,
        agg: RollingAgg,
    ) -> Result<()> {
        if window == 0 {
            anyhow::bail!("window must be >= 1");
        }
        if data.is_empty() {
            return Ok(());
        }

        let max_len = data.iter().map(|r| r.len()).max().unwrap_or(0);
        if value_col >= max_len {
            anyhow::bail!("column index {} out of range (max {})", value_col, max_len.saturating_sub(1));
        }

        for row in data.iter_mut() {
            while row.len() < max_len {
                row.push(String::new());
            }
        }

        data[0].push(new_col_name.to_string());

        for i in 1..data.len() {
            let win_start = i.saturating_sub(window - 1).max(1);
            let vals: Vec<f64> = (win_start..=i)
                .filter_map(|r| data[r].get(value_col).and_then(|s| s.parse::<f64>().ok()))
                .collect();

            let cell = if vals.is_empty() {
                String::new()
            } else {
                match agg {
                    RollingAgg::Mean => {
                        let m = vals.iter().sum::<f64>() / vals.len() as f64;
                        format!("{:.6}", m)
                    }
                    RollingAgg::Sum => {
                        let s: f64 = vals.iter().sum();
                        format!("{:.6}", s)
                    }
                }
            };
            data[i].push(cell);
        }

        Ok(())
    }

    /// Parse and reformat date column
    pub fn parse_date(
        &self,
        data: &mut Vec<Vec<String>>,
        column: usize,
        from_format: &str,
        to_format: &str,
    ) -> Result<usize> {
        use chrono::NaiveDate;

        let mut converted = 0;
        for row in data.iter_mut().skip(1) {
            if let Some(cell) = row.get_mut(column) {
                if cell.is_empty() {
                    continue;
                }
                if let Ok(date) = NaiveDate::parse_from_str(cell, from_format) {
                    *cell = date.format(to_format).to_string();
                    converted += 1;
                }
            }
        }

        Ok(converted)
    }

    /// Filter rows by regex pattern
    pub fn regex_filter(
        &self,
        data: &[Vec<String>],
        column: usize,
        pattern: &str,
    ) -> Result<Vec<Vec<String>>> {
        let re = regex::Regex::new(pattern)?;

        let mut result = Vec::with_capacity(data.len());
        result.push(data[0].clone());

        for row in data.iter().skip(1) {
            if let Some(cell) = row.get(column) {
                if re.is_match(cell) {
                    result.push(row.clone());
                }
            }
        }

        Ok(result)
    }

    /// Replace values using regex pattern
    pub fn regex_replace(
        &self,
        data: &mut Vec<Vec<String>>,
        column: usize,
        pattern: &str,
        replacement: &str,
    ) -> Result<usize> {
        let re = regex::Regex::new(pattern)?;

        let mut replaced = 0;
        for row in data.iter_mut().skip(1) {
            if let Some(cell) = row.get_mut(column) {
                let new_val = re.replace_all(cell, replacement).to_string();
                if &new_val != cell {
                    *cell = new_val;
                    replaced += 1;
                }
            }
        }

        Ok(replaced)
    }

    /// Extract date parts (year, month, day, weekday)
    pub fn extract_date_part(
        &self,
        data: &mut Vec<Vec<String>>,
        column: usize,
        part: &str,
        new_col_name: &str,
        date_format: &str,
    ) -> Result<()> {
        use chrono::{Datelike, NaiveDate};

        if data.is_empty() {
            return Ok(());
        }

        data[0].push(new_col_name.to_string());

        for row in data.iter_mut().skip(1) {
            let value = if let Some(cell) = row.get(column) {
                if let Ok(date) = NaiveDate::parse_from_str(cell, date_format) {
                    match part.to_lowercase().as_str() {
                        "year" => date.year().to_string(),
                        "month" => date.month().to_string(),
                        "day" => date.day().to_string(),
                        "weekday" => date.weekday().to_string(),
                        "quarter" => ((date.month() - 1) / 3 + 1).to_string(),
                        "dayofyear" => date.ordinal().to_string(),
                        _ => String::new(),
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            row.push(value);
        }

        Ok(())
    }
}
