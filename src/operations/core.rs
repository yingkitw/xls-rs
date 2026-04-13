//! Core data operations struct and basic methods

use super::types::SortOrder;
use crate::traits::{
    DataOperator, FilterCondition, FilterOperator, SortOperator, TransformOperation,
    TransformOperator,
};
use anyhow::Result;
use rayon::prelude::*;

/// Data operations for spreadsheet manipulation
pub struct DataOperations;

impl DataOperations {
    pub fn new() -> Self {
        Self
    }
}

// Trait implementations for better SOC
impl SortOperator for DataOperations {
    fn sort(&self, data: &mut Vec<Vec<String>>, column: usize, ascending: bool) -> Result<()> {
        let order = if ascending {
            SortOrder::Ascending
        } else {
            SortOrder::Descending
        };
        self.sort_by_column(data, column, order)
    }
}

impl DataOperations {
    /// Sort rows by a specific column (public for backward compatibility)
    pub fn sort_by_column(
        &self,
        data: &mut Vec<Vec<String>>,
        column: usize,
        order: SortOrder,
    ) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        let max_cols = data.iter().map(|r| r.len()).max().unwrap_or(0);
        if column >= max_cols {
            anyhow::bail!(
                "Column index {} out of range (max: {})",
                column,
                max_cols - 1
            );
        }

        // Use parallel sort for better performance on large datasets
        data.par_sort_by(|a, b| {
            let val_a = a.get(column).map(|s| s.as_str()).unwrap_or("");
            let val_b = b.get(column).map(|s| s.as_str()).unwrap_or("");

            let cmp = match (val_a.parse::<f64>(), val_b.parse::<f64>()) {
                (Ok(num_a), Ok(num_b)) => num_a
                    .partial_cmp(&num_b)
                    .unwrap_or(std::cmp::Ordering::Equal),
                _ => val_a.cmp(val_b),
            };

            match order {
                SortOrder::Ascending => cmp,
                SortOrder::Descending => cmp.reverse(),
            }
        });

        Ok(())
    }
}

impl FilterOperator for DataOperations {
    fn filter(
        &self,
        data: &[Vec<String>],
        column: usize,
        condition: FilterCondition,
    ) -> Result<Vec<Vec<String>>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let max_cols = data.iter().map(|r| r.len()).max().unwrap_or(0);
        if column >= max_cols {
            anyhow::bail!(
                "Column index {} out of range (max: {})",
                column,
                max_cols.saturating_sub(1)
            );
        }

        // Pre-filter indices in parallel to avoid cloning until final collection
        let indices: Vec<usize> = data
            .par_iter()
            .enumerate()
            .filter(|(_idx, row)| {
                let cell_value = row.get(column).map(|s| s.as_str()).unwrap_or("");
                self.evaluate_condition(cell_value, &condition)
                    .unwrap_or(false)
            })
            .map(|(idx, _)| idx)
            .collect();

        // Collect only filtered rows to minimize allocations
        let filtered: Vec<Vec<String>> = indices.into_iter().map(|idx| data[idx].clone()).collect();

        Ok(filtered)
    }
}

impl DataOperations {
    /// Filter rows by a condition on a column (legacy method for compatibility)
    pub fn filter_rows(
        &self,
        data: &[Vec<String>],
        column: usize,
        operator: &str,
        value: &str,
    ) -> Result<Vec<Vec<String>>> {
        let condition = self.parse_filter_condition(operator, value)?;
        <Self as FilterOperator>::filter(self, data, column, condition)
    }

    fn parse_filter_condition(&self, operator: &str, value: &str) -> Result<FilterCondition> {
        Ok(match operator {
            "=" | "==" => FilterCondition::Equals(value.to_string()),
            "!=" | "<>" => FilterCondition::NotEquals(value.to_string()),
            ">" => FilterCondition::GreaterThan(value.to_string()),
            ">=" => FilterCondition::GreaterThanOrEqual(value.to_string()),
            "<" => FilterCondition::LessThan(value.to_string()),
            "<=" => FilterCondition::LessThanOrEqual(value.to_string()),
            "contains" => FilterCondition::Contains(value.to_string()),
            "starts_with" => FilterCondition::StartsWith(value.to_string()),
            "ends_with" => FilterCondition::EndsWith(value.to_string()),
            _ => anyhow::bail!("Unknown operator: {}", operator),
        })
    }

    fn evaluate_condition(&self, cell_value: &str, condition: &FilterCondition) -> Result<bool> {
        Ok(match condition {
            FilterCondition::Equals(v) => cell_value == v,
            FilterCondition::NotEquals(v) => cell_value != v,
            FilterCondition::GreaterThan(v) => {
                match (cell_value.parse::<f64>(), v.parse::<f64>()) {
                    (Ok(a), Ok(b)) => a > b,
                    _ => cell_value > v.as_str(),
                }
            }
            FilterCondition::GreaterThanOrEqual(v) => {
                match (cell_value.parse::<f64>(), v.parse::<f64>()) {
                    (Ok(a), Ok(b)) => a >= b,
                    _ => cell_value >= v.as_str(),
                }
            }
            FilterCondition::LessThan(v) => match (cell_value.parse::<f64>(), v.parse::<f64>()) {
                (Ok(a), Ok(b)) => a < b,
                _ => cell_value < v.as_str(),
            },
            FilterCondition::LessThanOrEqual(v) => {
                match (cell_value.parse::<f64>(), v.parse::<f64>()) {
                    (Ok(a), Ok(b)) => a <= b,
                    _ => cell_value <= v.as_str(),
                }
            }
            FilterCondition::Contains(v) => cell_value.contains(v),
            FilterCondition::StartsWith(v) => cell_value.starts_with(v),
            FilterCondition::EndsWith(v) => cell_value.ends_with(v),
            FilterCondition::Regex(pattern) => {
                use regex::Regex;
                let re = Regex::new(pattern)?;
                re.is_match(cell_value)
            }
        })
    }

    /// Evaluate a filter condition (legacy method for compatibility)
    pub fn evaluate_filter_condition(
        &self,
        cell_value: &str,
        operator: &str,
        value: &str,
    ) -> Result<bool> {
        let condition = self.parse_filter_condition(operator, value)?;
        self.evaluate_condition(cell_value, &condition)
    }

    /// Replace values in a column
    pub fn replace(
        &self,
        data: &mut Vec<Vec<String>>,
        column: usize,
        find: &str,
        replace_with: &str,
    ) -> usize {
        let mut count = 0;
        for row in data.iter_mut() {
            if let Some(cell) = row.get_mut(column) {
                if cell.contains(find) {
                    *cell = cell.replace(find, replace_with);
                    count += 1;
                }
            }
        }
        count
    }

    /// Find and replace across all columns
    pub fn find_replace(
        &self,
        data: &mut Vec<Vec<String>>,
        find: &str,
        replace_with: &str,
        _column: Option<usize>,
    ) -> Result<usize> {
        let mut count = 0;
        for row in data.iter_mut() {
            for cell in row.iter_mut() {
                if cell.contains(find) {
                    *cell = cell.replace(find, replace_with);
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    /// Remove duplicate rows (returns new vec)
    pub fn deduplicate(&self, data: &[Vec<String>]) -> Vec<Vec<String>> {
        use std::collections::HashSet;
        let mut seen: HashSet<Vec<String>> = HashSet::with_capacity(data.len());
        data.iter()
            .filter(|row| seen.insert((*row).clone()))
            .cloned()
            .collect()
    }

    /// Remove duplicate rows in place
    pub fn deduplicate_mut(&self, data: &mut Vec<Vec<String>>) -> usize {
        use std::collections::HashSet;
        let original_len = data.len();
        let mut seen: HashSet<Vec<String>> = HashSet::with_capacity(data.len());
        data.retain(|row| seen.insert(row.clone()));
        original_len - data.len()
    }

    /// Transpose data (rows to columns)
    pub fn transpose(&self, data: &[Vec<String>]) -> Vec<Vec<String>> {
        if data.is_empty() {
            return Vec::new();
        }

        let max_cols = data.iter().map(|r| r.len()).max().unwrap_or(0);
        let mut result: Vec<Vec<String>> = Vec::with_capacity(max_cols);

        for col_idx in 0..max_cols {
            let mut new_row = Vec::with_capacity(data.len());
            for row in data.iter() {
                if col_idx < row.len() {
                    new_row.push(row[col_idx].clone());
                } else {
                    new_row.push(String::new());
                }
            }
            result.push(new_row);
        }

        result
    }

    /// Format data as markdown table
    pub fn to_markdown(&self, data: &[Vec<String>]) -> String {
        if data.is_empty() {
            return String::new();
        }

        let mut output = String::with_capacity(data.len() * data[0].len() * 20);

        // Header row
        if let Some(header) = data.first() {
            output.push_str("| ");
            output.push_str(&header.join(" | "));
            output.push_str(" |\n");

            // Separator
            output.push_str("| ");
            let sep: String = header.iter().map(|_| "---").collect::<Vec<_>>().join(" | ");
            output.push_str(&sep);
            output.push_str(" |\n");
        }

        // Data rows
        for row in data.iter().skip(1) {
            output.push_str("| ");
            output.push_str(&row.join(" | "));
            output.push_str(" |\n");
        }

        output
    }

    /// Insert a row at a specific index
    pub fn insert_row(&self, data: &mut Vec<Vec<String>>, index: usize, row: Vec<String>) {
        if index <= data.len() {
            data.insert(index, row);
        }
    }

    /// Delete a row at a specific index
    pub fn delete_row(&self, data: &mut Vec<Vec<String>>, index: usize) -> Option<Vec<String>> {
        if index < data.len() {
            Some(data.remove(index))
        } else {
            None
        }
    }

    /// Insert a column at a specific index
    pub fn insert_column(&self, data: &mut Vec<Vec<String>>, index: usize, values: Vec<String>) {
        for (row_idx, row) in data.iter_mut().enumerate() {
            let value = values.get(row_idx).cloned().unwrap_or_default();
            if index <= row.len() {
                row.insert(index, value);
            } else {
                row.push(value);
            }
        }
    }

    /// Delete a column at a specific index
    pub fn delete_column(&self, data: &mut Vec<Vec<String>>, index: usize) {
        for row in data.iter_mut() {
            if index < row.len() {
                row.remove(index);
            }
        }
    }
}

impl TransformOperator for DataOperations {
    fn transform(&self, data: &mut Vec<Vec<String>>, operation: TransformOperation) -> Result<()> {
        match operation {
            TransformOperation::RenameColumn { from, to } => {
                if let Some(row) = data.first_mut() {
                    if from < row.len() {
                        row[from] = to;
                    }
                }
            }
            TransformOperation::DropColumn(col_idx) => {
                for row in data.iter_mut() {
                    if col_idx < row.len() {
                        row.remove(col_idx);
                    }
                }
            }
            TransformOperation::AddColumn { name, formula } => {
                if let Some(formula_str) = formula {
                    // Use formula evaluator to compute the column value
                    use crate::formula::FormulaEvaluator;
                    let evaluator = FormulaEvaluator::new();

                    // Check if formula contains cell references that might be row-relative
                    // Cell references like A1, B2, C10, etc. suggest per-row evaluation
                    let has_cell_refs = formula_str.chars().any(|c: char| c.is_ascii_uppercase())
                        && formula_str.contains(|c: char| c.is_ascii_digit());

                    if has_cell_refs {
                        // Per-row formula evaluation: evaluate formula for each row
                        // with row-specific cell references (A1 for row 0, A2 for row 1, etc.)
                        // Clone data first to avoid borrow issues
                        let data_clone = data.clone();
                        for (row_idx, row) in data.iter_mut().enumerate() {
                            // Replace row number in cell references with current row index
                            // e.g., A1 -> A{row_idx+1}, B2 -> B{row_idx+1}
                            let row_formula = adjust_cell_references_for_row(&formula_str, row_idx);

                            match evaluator.evaluate_formula_full(&row_formula, &data_clone) {
                                Ok(result) => {
                                    let value = match result {
                                        crate::formula::FormulaResult::Number(n) => n.to_string(),
                                        crate::formula::FormulaResult::Text(s) => s,
                                    };
                                    row.push(value);
                                }
                                Err(_) => {
                                    row.push(format!("#ERROR: {}", name));
                                }
                            }
                        }
                    } else {
                        // Aggregate formula: evaluate once for all rows (SUM, AVERAGE, etc.)
                        match evaluator.evaluate_formula_full(&formula_str, data) {
                            Ok(result) => {
                                let value = match result {
                                    crate::formula::FormulaResult::Number(n) => n.to_string(),
                                    crate::formula::FormulaResult::Text(s) => s,
                                };
                                for row in data.iter_mut() {
                                    row.push(value.clone());
                                }
                            }
                            Err(_) => {
                                for row in data.iter_mut() {
                                    row.push(format!("#ERROR: {}", name));
                                }
                            }
                        }
                    }
                } else {
                    // No formula, just add the column name
                    for row in data.iter_mut() {
                        row.push(name.clone());
                    }
                }
            }
            TransformOperation::FillNa { column, value } => {
                if value.is_empty() {
                    return Ok(());
                }
                for row in data.iter_mut() {
                    if column < row.len() && row[column].is_empty() {
                        row[column] = value.clone();
                    }
                }
            }
        }
        Ok(())
    }
}

impl DataOperator for DataOperations {}

/// Adjust cell references in a formula to be row-specific
///
/// This function takes a formula like "A1*2" and adjusts it to use the
/// current row index, e.g., for row 0 it becomes "A1*2", for row 1 it becomes "A2*2", etc.
///
/// # Arguments
/// * `formula` - The formula string potentially containing cell references
/// * `row_idx` - The zero-based row index
///
/// # Returns
/// A formula string with adjusted cell references
fn adjust_cell_references_for_row(formula: &str, row_idx: usize) -> String {
    use crate::regex_cache::cell_reference_regex;

    let row_num = row_idx + 1;
    let re = cell_reference_regex();

    let result = re.replace_all(formula, |caps: &regex::Captures| {
        let column = &caps[1]; // e.g., "A", "B", "AA"
        let _old_row = &caps[2]; // e.g., "1", "2", "100"
        format!("{}{}", column, row_num)
    });

    result.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adjust_cell_references_for_row() {
        // Single column references
        assert_eq!(adjust_cell_references_for_row("A1", 0), "A1");
        assert_eq!(adjust_cell_references_for_row("A1", 1), "A2");
        assert_eq!(adjust_cell_references_for_row("A1", 9), "A10");

        // Multiple column references
        assert_eq!(adjust_cell_references_for_row("A1+B2", 0), "A1+B1");
        assert_eq!(adjust_cell_references_for_row("A1+B2", 1), "A2+B2");
        assert_eq!(adjust_cell_references_for_row("A1+B2", 2), "A3+B3");

        // Complex formulas
        assert_eq!(
            adjust_cell_references_for_row("SUM(A1:B10)", 0),
            "SUM(A1:B1)"
        );
        assert_eq!(adjust_cell_references_for_row("A1*2+B1", 5), "A6*2+B6");

        // Double-letter columns
        assert_eq!(adjust_cell_references_for_row("AA1", 2), "AA3");
        assert_eq!(adjust_cell_references_for_row("AB12+CD3", 1), "AB2+CD2");

        // Mixed case
        assert_eq!(adjust_cell_references_for_row("a1+b2", 0), "a1+b1");
    }
}
