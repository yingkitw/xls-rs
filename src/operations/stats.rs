//! Statistical operations

use super::core::DataOperations;
use super::types::AggFunc;
use anyhow::Result;

impl DataOperations {
    /// Describe/summary statistics for all numeric columns
    pub fn describe(&self, data: &[Vec<String>]) -> Result<Vec<Vec<String>>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let header = &data[0];
        let num_cols = header.len();

        let mut columns: Vec<Vec<f64>> = vec![Vec::new(); num_cols];
        for row in data.iter().skip(1) {
            for (idx, val) in row.iter().enumerate() {
                if let Ok(num) = val.parse::<f64>() {
                    columns[idx].push(num);
                }
            }
        }

        let mut result = Vec::new();

        let mut stat_header = vec!["stat".to_string()];
        stat_header.extend(header.iter().cloned());
        result.push(stat_header);

        let stats = ["count", "mean", "std", "min", "25%", "50%", "75%", "max"];
        for stat in stats {
            let mut row = vec![stat.to_string()];
            for col_values in &columns {
                let value = if col_values.is_empty() {
                    "NaN".to_string()
                } else {
                    match stat {
                        "count" => col_values.len().to_string(),
                        "mean" => format!(
                            "{:.2}",
                            col_values.iter().sum::<f64>() / col_values.len() as f64
                        ),
                        "std" => {
                            let mean = col_values.iter().sum::<f64>() / col_values.len() as f64;
                            let variance =
                                col_values.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
                                    / col_values.len() as f64;
                            format!("{:.2}", variance.sqrt())
                        }
                        "min" => format!(
                            "{:.2}",
                            col_values.iter().cloned().fold(f64::INFINITY, f64::min)
                        ),
                        "max" => format!(
                            "{:.2}",
                            col_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                        ),
                        "25%" | "50%" | "75%" => {
                            let mut sorted = col_values.clone();
                            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                            let p = match stat {
                                "25%" => 0.25,
                                "50%" => 0.50,
                                "75%" => 0.75,
                                _ => 0.5,
                            };
                            let idx = ((sorted.len() - 1) as f64 * p) as usize;
                            format!("{:.2}", sorted[idx])
                        }
                        _ => "".to_string(),
                    }
                };
                row.push(value);
            }
            result.push(row);
        }

        Ok(result)
    }

    /// Count unique values in a column
    pub fn value_counts(&self, data: &[Vec<String>], column: usize) -> Vec<Vec<String>> {
        use std::collections::HashMap;

        let mut counts: HashMap<String, usize> = HashMap::new();
        for row in data.iter().skip(1) {
            if let Some(val) = row.get(column) {
                *counts.entry(val.clone()).or_insert(0) += 1;
            }
        }

        let mut result: Vec<(String, usize)> = counts.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1));

        let mut output = vec![vec!["value".to_string(), "count".to_string()]];
        for (val, count) in result {
            output.push(vec![val, count.to_string()]);
        }
        output
    }

    /// Pivot table
    pub fn pivot(
        &self,
        data: &[Vec<String>],
        index_col: usize,
        columns_col: usize,
        values_col: usize,
        agg: AggFunc,
    ) -> Result<Vec<Vec<String>>> {
        use std::collections::{BTreeSet, HashMap};

        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut col_values: BTreeSet<String> = BTreeSet::new();
        let mut index_values: BTreeSet<String> = BTreeSet::new();
        let mut groups: HashMap<(String, String), Vec<f64>> = HashMap::new();

        for row in data.iter().skip(1) {
            let idx = row.get(index_col).cloned().unwrap_or_default();
            let col = row.get(columns_col).cloned().unwrap_or_default();
            let val = row
                .get(values_col)
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0);

            index_values.insert(idx.clone());
            col_values.insert(col.clone());
            groups.entry((idx, col)).or_default().push(val);
        }

        let col_values: Vec<String> = col_values.into_iter().collect();
        let index_values: Vec<String> = index_values.into_iter().collect();

        let mut result = Vec::new();

        let index_name = data[0]
            .get(index_col)
            .cloned()
            .unwrap_or_else(|| "index".to_string());
        let mut header = vec![index_name];
        header.extend(col_values.iter().cloned());
        result.push(header);

        for idx in &index_values {
            let mut row = vec![idx.clone()];
            for col in &col_values {
                let values = groups.get(&(idx.clone(), col.clone()));
                let agg_val = match values {
                    Some(vals) => agg.apply(vals),
                    None => 0.0,
                };
                row.push(format!("{:.2}", agg_val));
            }
            result.push(row);
        }

        Ok(result)
    }

    /// Frequency crosstab: counts of `(row_col, col_col)` pairs (two categorical columns).
    ///
    /// First row is the header: row dimension name, then distinct values from `col_col`.
    /// First column lists distinct values from `row_col`; cell `(r, c)` is the count.
    pub fn crosstab(
        &self,
        data: &[Vec<String>],
        row_col: usize,
        col_col: usize,
    ) -> Result<Vec<Vec<String>>> {
        use std::collections::{BTreeSet, HashMap};

        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut row_vals: BTreeSet<String> = BTreeSet::new();
        let mut col_vals: BTreeSet<String> = BTreeSet::new();
        let mut counts: HashMap<(String, String), usize> = HashMap::new();

        for row in data.iter().skip(1) {
            let r = row.get(row_col).cloned().unwrap_or_default();
            let c = row.get(col_col).cloned().unwrap_or_default();
            row_vals.insert(r.clone());
            col_vals.insert(c.clone());
            *counts.entry((r, c)).or_insert(0) += 1;
        }

        let row_vals: Vec<String> = row_vals.into_iter().collect();
        let col_vals: Vec<String> = col_vals.into_iter().collect();

        let row_name = data[0]
            .get(row_col)
            .cloned()
            .unwrap_or_else(|| "row".to_string());

        let mut header = vec![row_name];
        header.extend(col_vals.iter().cloned());

        let mut result = vec![header];

        for rv in &row_vals {
            let mut out_row = vec![rv.clone()];
            for cv in &col_vals {
                let n = counts
                    .get(&(rv.clone(), cv.clone()))
                    .copied()
                    .unwrap_or(0);
                out_row.push(n.to_string());
            }
            result.push(out_row);
        }

        Ok(result)
    }

    /// Correlation matrix
    pub fn correlation(&self, data: &[Vec<String>], columns: &[usize]) -> Result<Vec<Vec<String>>> {
        if data.is_empty() || columns.is_empty() {
            return Ok(Vec::new());
        }

        let header = &data[0];

        let mut col_data: Vec<Vec<f64>> = vec![Vec::new(); columns.len()];
        for row in data.iter().skip(1) {
            for (i, &col_idx) in columns.iter().enumerate() {
                if let Some(val) = row.get(col_idx).and_then(|v| v.parse::<f64>().ok()) {
                    col_data[i].push(val);
                }
            }
        }

        let mut result = Vec::new();

        let mut corr_header = vec!["".to_string()];
        for &col_idx in columns {
            corr_header.push(
                header
                    .get(col_idx)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{}", col_idx)),
            );
        }
        result.push(corr_header);

        for (i, &col_i) in columns.iter().enumerate() {
            let col_name = header
                .get(col_i)
                .cloned()
                .unwrap_or_else(|| format!("col_{}", col_i));
            let mut row = vec![col_name];

            for (j, _) in columns.iter().enumerate() {
                let corr = self.pearson_correlation(&col_data[i], &col_data[j]);
                row.push(format!("{:.4}", corr));
            }
            result.push(row);
        }

        Ok(result)
    }

    pub(crate) fn pearson_correlation(&self, x: &[f64], y: &[f64]) -> f64 {
        let n = x.len().min(y.len());
        if n == 0 {
            return 0.0;
        }

        let mean_x = x.iter().take(n).sum::<f64>() / n as f64;
        let mean_y = y.iter().take(n).sum::<f64>() / n as f64;

        let mut cov = 0.0;
        let mut var_x = 0.0;
        let mut var_y = 0.0;

        for i in 0..n {
            let dx = x[i] - mean_x;
            let dy = y[i] - mean_y;
            cov += dx * dy;
            var_x += dx * dx;
            var_y += dy * dy;
        }

        if var_x == 0.0 || var_y == 0.0 {
            return 0.0;
        }

        cov / (var_x.sqrt() * var_y.sqrt())
    }

    /// Infer column types
    pub fn dtypes(&self, data: &[Vec<String>]) -> Vec<Vec<String>> {
        if data.is_empty() {
            return Vec::new();
        }

        let header = &data[0];
        let mut result = vec![vec![
            "column".to_string(),
            "dtype".to_string(),
            "non_null".to_string(),
        ]];

        for (col_idx, col_name) in header.iter().enumerate() {
            let mut int_count = 0;
            let mut float_count = 0;
            let mut bool_count = 0;
            let mut non_null = 0;
            let total = data.len() - 1;

            for row in data.iter().skip(1) {
                if let Some(val) = row.get(col_idx) {
                    if val.is_empty() {
                        continue;
                    }
                    non_null += 1;

                    if val.parse::<i64>().is_ok() {
                        int_count += 1;
                    } else if val.parse::<f64>().is_ok() {
                        float_count += 1;
                    } else if val.eq_ignore_ascii_case("true") || val.eq_ignore_ascii_case("false")
                    {
                        bool_count += 1;
                    }
                }
            }

            let dtype = if non_null == 0 {
                "empty"
            } else if int_count == non_null {
                "int"
            } else if int_count + float_count == non_null {
                "float"
            } else if bool_count == non_null {
                "bool"
            } else {
                "string"
            };

            result.push(vec![
                col_name.clone(),
                dtype.to_string(),
                format!("{}/{}", non_null, total),
            ]);
        }

        result
    }

    /// Get unique values in a column
    pub fn unique(&self, data: &[Vec<String>], column: usize) -> Vec<Vec<String>> {
        use std::collections::HashSet;

        let mut seen: HashSet<String> = HashSet::new();
        let mut result = vec![vec!["value".to_string()]];

        for row in data.iter().skip(1) {
            if let Some(val) = row.get(column) {
                if seen.insert(val.clone()) {
                    result.push(vec![val.clone()]);
                }
            }
        }

        result
    }

    /// Count unique values in a column
    pub fn nunique(&self, data: &[Vec<String>], column: usize) -> usize {
        use std::collections::HashSet;

        let unique: HashSet<&String> = data
            .iter()
            .skip(1)
            .filter_map(|row| row.get(column))
            .collect();

        unique.len()
    }

    /// Get info about the dataset
    pub fn info(&self, data: &[Vec<String>]) -> Vec<Vec<String>> {
        if data.is_empty() {
            return Vec::new();
        }

        let header = &data[0];
        let num_rows = data.len() - 1;
        let num_cols = header.len();

        let mut result = vec![
            vec!["metric".to_string(), "value".to_string()],
            vec!["rows".to_string(), num_rows.to_string()],
            vec!["columns".to_string(), num_cols.to_string()],
        ];

        let total_chars: usize = data
            .iter()
            .flat_map(|row| row.iter())
            .map(|s| s.len())
            .sum();
        result.push(vec!["memory_bytes".to_string(), total_chars.to_string()]);

        for (idx, col_name) in header.iter().enumerate() {
            let non_null: usize = data
                .iter()
                .skip(1)
                .filter(|row| row.get(idx).map(|s| !s.is_empty()).unwrap_or(false))
                .count();
            let null_count = num_rows - non_null;
            let unique_count = self.nunique(data, idx);

            result.push(vec![
                format!("col_{}", col_name),
                format!(
                    "non_null={}, null={}, unique={}",
                    non_null, null_count, unique_count
                ),
            ]);
        }

        result
    }
}
