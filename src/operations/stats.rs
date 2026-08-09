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

        let data_rows = data.len().saturating_sub(1);
        let mut columns: Vec<Vec<f64>> = (0..num_cols).map(|_| Vec::with_capacity(data_rows)).collect();
        for row in data.iter().skip(1) {
            for (idx, val) in row.iter().enumerate() {
                if let Ok(num) = val.parse::<f64>() {
                    columns[idx].push(num);
                }
            }
        }

        let mut result = Vec::with_capacity(16);

        let mut stat_header = vec!["stat".to_string()];
        stat_header.extend(header.iter().cloned());
        result.push(stat_header);

        // Precompute all stats per-column (sort once, reuse mean/variance)
        let col_stats: Vec<ColumnStats> = columns
            .iter()
            .map(|vals| ColumnStats::compute(vals))
            .collect();

        let stat_names = [
            "count", "mean", "std", "min", "10%", "25%", "50%", "75%", "90%", "95%", "99%",
            "max", "skewness", "kurtosis",
        ];
        for &name in &stat_names {
            let mut row = vec![name.to_string()];
            for cs in &col_stats {
                row.push(cs.format(name));
            }
            result.push(row);
        }

        Ok(result)
    }

    /// Spearman rank correlation matrix
    pub fn spearman_correlation(
        &self,
        data: &[Vec<String>],
        columns: &[usize],
    ) -> Result<Vec<Vec<String>>> {
        if data.is_empty() || columns.is_empty() {
            return Ok(Vec::new());
        }

        let header = &data[0];

        // Extract numeric values per column
        let data_rows = data.len().saturating_sub(1);
        let mut col_data: Vec<Vec<f64>> = (0..columns.len()).map(|_| Vec::with_capacity(data_rows)).collect();
        for row in data.iter().skip(1) {
            for (i, &col_idx) in columns.iter().enumerate() {
                if let Some(val) = row.get(col_idx).and_then(|v| v.parse::<f64>().ok()) {
                    col_data[i].push(val);
                }
            }
        }

        let mut result = Vec::with_capacity(columns.len() + 1);

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
                let corr = spearman_rho(&col_data[i], &col_data[j]);
                row.push(format!("{:.4}", corr));
            }
            result.push(row);
        }

        Ok(result)
    }

    /// Kendall tau-b rank correlation matrix
    pub fn kendall_tau_correlation(
        &self,
        data: &[Vec<String>],
        columns: &[usize],
    ) -> Result<Vec<Vec<String>>> {
        if data.is_empty() || columns.is_empty() {
            return Ok(Vec::new());
        }

        let header = &data[0];

        let data_rows = data.len().saturating_sub(1);
        let mut col_data: Vec<Vec<f64>> = (0..columns.len()).map(|_| Vec::with_capacity(data_rows)).collect();
        for row in data.iter().skip(1) {
            for (i, &col_idx) in columns.iter().enumerate() {
                if let Some(val) = row.get(col_idx).and_then(|v| v.parse::<f64>().ok()) {
                    col_data[i].push(val);
                }
            }
        }

        let mut result = Vec::with_capacity(columns.len() + 1);

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
                let corr = kendall_tau_b(&col_data[i], &col_data[j]);
                row.push(format!("{:.4}", corr));
            }
            result.push(row);
        }

        Ok(result)
    }

    /// Simple linear regression: slope, intercept, and r_squared for two numeric columns
    pub fn simple_linear_regression(
        &self,
        data: &[Vec<String>],
        x_col: usize,
        y_col: usize,
    ) -> Result<Vec<Vec<String>>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut xs = Vec::with_capacity(data.len());
        let mut ys = Vec::with_capacity(data.len());
        for row in data.iter().skip(1) {
            if let (Some(xv), Some(yv)) = (row.get(x_col), row.get(y_col))
                && let (Ok(x), Ok(y)) = (xv.parse::<f64>(), yv.parse::<f64>()) {
                    xs.push(x);
                    ys.push(y);
                }
        }

        if xs.len() < 2 {
            anyhow::bail!("Need at least 2 valid numeric pairs for regression");
        }

        let n = xs.len() as f64;
        let sum_x: f64 = xs.iter().sum();
        let sum_y: f64 = ys.iter().sum();
        let mean_x = sum_x / n;
        let mean_y = sum_y / n;

        let mut ss_xy = 0.0;
        let mut ss_xx = 0.0;
        let mut ss_yy = 0.0;
        for i in 0..xs.len() {
            let dx = xs[i] - mean_x;
            let dy = ys[i] - mean_y;
            ss_xy += dx * dy;
            ss_xx += dx * dx;
            ss_yy += dy * dy;
        }

        if ss_xx == 0.0 {
            anyhow::bail!("X column has zero variance; cannot compute regression");
        }

        let slope = ss_xy / ss_xx;
        let intercept = mean_y - slope * mean_x;

        let r = if ss_yy > 0.0 {
            ss_xy / (ss_xx.sqrt() * ss_yy.sqrt())
        } else {
            0.0
        };
        let r_squared = r * r;

        Ok(vec![
            vec!["stat".to_string(), "value".to_string()],
            vec!["slope".to_string(), format!("{:.6}", slope)],
            vec!["intercept".to_string(), format!("{:.6}", intercept)],
            vec!["r_squared".to_string(), format!("{:.6}", r_squared)],
            vec!["n".to_string(), format!("{}", xs.len())],
        ])
    }

    /// Count unique values in a column
    pub fn value_counts(&self, data: &[Vec<String>], column: usize) -> Vec<Vec<String>> {
        use std::collections::HashMap;

        let mut counts: HashMap<&str, usize> = HashMap::new();
        for row in data.iter().skip(1) {
            if let Some(val) = row.get(column) {
                *counts.entry(val.as_str()).or_insert(0) += 1;
            }
        }

        let mut result: Vec<(&str, usize)> = counts.into_iter().collect();
        result.sort_by_key(|b| std::cmp::Reverse(b.1));

        let mut output = vec![vec!["value".to_string(), "count".to_string()]];
        for (val, count) in result {
            output.push(vec![val.to_string(), count.to_string()]);
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

        let data_rows = data.len().saturating_sub(1);
        let mut col_data: Vec<Vec<f64>> = (0..columns.len()).map(|_| Vec::with_capacity(data_rows)).collect();
        for row in data.iter().skip(1) {
            for (i, &col_idx) in columns.iter().enumerate() {
                if let Some(val) = row.get(col_idx).and_then(|v| v.parse::<f64>().ok()) {
                    col_data[i].push(val);
                }
            }
        }

        let mut result = Vec::with_capacity(columns.len() + 1);

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
            if let Some(val) = row.get(column)
                && seen.insert(val.clone()) {
                    result.push(vec![val.clone()]);
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

/// Spearman rank correlation coefficient
fn spearman_rho(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n == 0 {
        return 0.0;
    }

    let rank_x = rank_values(&x[..n]);
    let rank_y = rank_values(&y[..n]);

    pearson_rho(&rank_x, &rank_y)
}

/// Assign average ranks to values (handles ties)
fn rank_values(values: &[f64]) -> Vec<f64> {
    let mut indexed: Vec<(usize, f64)> = values.iter().cloned().enumerate().collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut ranks = vec![0.0; values.len()];
    let mut i = 0;
    while i < indexed.len() {
        let mut j = i + 1;
        while j < indexed.len() && indexed[j].1 == indexed[i].1 {
            j += 1;
        }
        let avg_rank = (i + 1 + j) as f64 / 2.0; // 1-based rank average
        for k in i..j {
            ranks[indexed[k].0] = avg_rank;
        }
        i = j;
    }
    ranks
}

/// Kendall tau-b rank correlation coefficient (handles ties)
fn kendall_tau_b(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 {
        return 0.0;
    }

    let mut concordant = 0_i64;
    let mut discordant = 0_i64;
    let mut tied_x = 0_i64;
    let mut tied_y = 0_i64;

    for i in 0..n {
        for j in (i + 1)..n {
            let dx = x[i] - x[j];
            let dy = y[i] - y[j];
            if dx == 0.0 && dy == 0.0 {
                // tied on both — neither concordant nor discordant
            } else if dx == 0.0 {
                tied_x += 1;
            } else if dy == 0.0 {
                tied_y += 1;
            } else if (dx > 0.0) == (dy > 0.0) {
                concordant += 1;
            } else {
                discordant += 1;
            }
        }
    }

    let numerator = (concordant - discordant) as f64;
    let n0 = (n as f64) * (n as f64 - 1.0) / 2.0;
    let n1 = tied_x as f64;
    let n2 = tied_y as f64;
    let denom = ((n0 - n1) * (n0 - n2)).sqrt();

    if denom == 0.0 {
        return 0.0;
    }

    numerator / denom
}

/// Pearson correlation on already-numeric slices (no parsing)
fn pearson_rho(x: &[f64], y: &[f64]) -> f64 {
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

/// Precomputed statistics for a single numeric column
struct ColumnStats {
    count: usize,
    mean: f64,
    std_dev: f64,
    min: f64,
    max: f64,
    p10: f64,
    p25: f64,
    p50: f64,
    p75: f64,
    p90: f64,
    p95: f64,
    p99: f64,
    skewness: f64,
    kurtosis: f64,
    empty: bool,
}

impl ColumnStats {
    fn compute(values: &[f64]) -> Self {
        if values.is_empty() {
            return Self {
                count: 0, mean: f64::NAN, std_dev: f64::NAN,
                min: f64::NAN, max: f64::NAN,
                p10: f64::NAN, p25: f64::NAN, p50: f64::NAN,
                p75: f64::NAN, p90: f64::NAN, p95: f64::NAN, p99: f64::NAN,
                skewness: f64::NAN, kurtosis: f64::NAN, empty: true,
            };
        }

        let count = values.len();
        let sum: f64 = values.iter().sum();
        let mean = sum / count as f64;

        let variance = values.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();

        // values is guaranteed non-empty (empty case returns early above)
        let min = *values.iter().min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(&f64::NAN);
        let max = *values.iter().max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(&f64::NAN);

        // Sort once for all percentiles
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p = |p: f64| {
            let n = sorted.len();
            let idx = (n - 1) as f64 * p;
            let lower = idx.floor() as usize;
            let upper = idx.ceil() as usize;
            if lower == upper {
                sorted[lower]
            } else {
                let frac = idx - lower as f64;
                sorted[lower] * (1.0 - frac) + sorted[upper] * frac
            }
        };

        let p10 = p(0.10);
        let p25 = p(0.25);
        let p50 = p(0.50);
        let p75 = p(0.75);
        let p90 = p(0.90);
        let p95 = p(0.95);
        let p99 = p(0.99);

        let skewness = if std_dev > 0.0 {
            let m3 = values.iter().map(|&x| (x - mean).powi(3)).sum::<f64>() / count as f64;
            m3 / std_dev.powi(3)
        } else {
            f64::NAN
        };

        let kurtosis = if variance > 0.0 {
            let m4 = values.iter().map(|&x| (x - mean).powi(4)).sum::<f64>() / count as f64;
            m4 / variance.powi(2) - 3.0
        } else {
            f64::NAN
        };

        Self {
            count, mean, std_dev, min, max,
            p10, p25, p50, p75, p90, p95, p99,
            skewness, kurtosis, empty: false,
        }
    }

    fn format(&self, name: &str) -> String {
        if self.empty {
            return "NaN".to_string();
        }
        match name {
            "count" => self.count.to_string(),
            "mean" => format!("{:.2}", self.mean),
            "std" => format!("{:.2}", self.std_dev),
            "min" => format!("{:.2}", self.min),
            "10%" => format!("{:.2}", self.p10),
            "25%" => format!("{:.2}", self.p25),
            "50%" => format!("{:.2}", self.p50),
            "75%" => format!("{:.2}", self.p75),
            "90%" => format!("{:.2}", self.p90),
            "95%" => format!("{:.2}", self.p95),
            "99%" => format!("{:.2}", self.p99),
            "max" => format!("{:.2}", self.max),
            "skewness" => format!("{:.4}", self.skewness),
            "kurtosis" => format!("{:.4}", self.kurtosis),
            _ => "".to_string(),
        }
    }
}
