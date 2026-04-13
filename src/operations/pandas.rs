//! Pandas-inspired data operations

use super::core::DataOperations;
use super::types::{AggFunc, JoinType};
use anyhow::Result;

impl DataOperations {
    /// Select specific columns by index
    pub fn select_columns(&self, data: &[Vec<String>], columns: &[usize]) -> Vec<Vec<String>> {
        data.iter()
            .map(|row| {
                columns
                    .iter()
                    .map(|&idx| row.get(idx).cloned().unwrap_or_default())
                    .collect()
            })
            .collect()
    }

    /// Select columns by name (first row is header)
    pub fn select_columns_by_name(
        &self,
        data: &[Vec<String>],
        names: &[&str],
    ) -> Result<Vec<Vec<String>>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let header = &data[0];
        let indices: Vec<usize> = names
            .iter()
            .map(|name| {
                header
                    .iter()
                    .position(|h| h == *name)
                    .ok_or_else(|| anyhow::anyhow!("Column '{}' not found", name))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(self.select_columns(data, &indices))
    }

    /// Get first n rows (head)
    pub fn head(&self, data: &[Vec<String>], n: usize) -> Vec<Vec<String>> {
        data.iter().take(n).cloned().collect()
    }

    /// Get last n rows (tail)
    pub fn tail(&self, data: &[Vec<String>], n: usize) -> Vec<Vec<String>> {
        let len = data.len();
        if n >= len {
            data.to_vec()
        } else {
            data[len - n..].to_vec()
        }
    }

    /// Sample random rows
    pub fn sample(&self, data: &[Vec<String>], n: usize, seed: Option<u64>) -> Vec<Vec<String>> {
        use std::collections::HashSet;

        if n >= data.len() {
            return data.to_vec();
        }

        let mut rng_state = seed.unwrap_or(42);
        let mut next_rand = || {
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
            rng_state
        };

        let mut indices = HashSet::new();
        while indices.len() < n {
            let idx = (next_rand() as usize) % data.len();
            indices.insert(idx);
        }

        let mut result: Vec<Vec<String>> = indices.iter().map(|&idx| data[idx].clone()).collect();
        result.sort_by_key(|_| next_rand());
        result
    }

    /// Drop columns by index
    pub fn drop_columns(&self, data: &[Vec<String>], columns: &[usize]) -> Vec<Vec<String>> {
        let drop_set: std::collections::HashSet<usize> = columns.iter().copied().collect();
        data.iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .filter(|(idx, _)| !drop_set.contains(idx))
                    .map(|(_, val)| val.clone())
                    .collect()
            })
            .collect()
    }

    /// Rename columns (first row is header)
    pub fn rename_columns(
        &self,
        data: &mut Vec<Vec<String>>,
        renames: &[(&str, &str)],
    ) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        let header = &mut data[0];
        for (old_name, new_name) in renames {
            if let Some(pos) = header.iter().position(|h| h == *old_name) {
                header[pos] = new_name.to_string();
            }
        }
        Ok(())
    }

    /// Fill missing/empty values
    pub fn fillna(&self, data: &mut Vec<Vec<String>>, value: &str) {
        for row in data.iter_mut() {
            for cell in row.iter_mut() {
                if cell.is_empty() {
                    *cell = value.to_string();
                }
            }
        }
    }

    /// Drop rows with any empty values
    pub fn dropna(&self, data: &[Vec<String>]) -> Vec<Vec<String>> {
        data.iter()
            .filter(|row| !row.iter().any(|cell| cell.is_empty()))
            .cloned()
            .collect()
    }

    /// Concatenate multiple datasets vertically
    pub fn concat(&self, datasets: &[Vec<Vec<String>>]) -> Vec<Vec<String>> {
        let mut result = Vec::new();
        for dataset in datasets {
            result.extend(dataset.iter().cloned());
        }
        result
    }

    /// Join two datasets on a column
    pub fn join(
        &self,
        left: &[Vec<String>],
        right: &[Vec<String>],
        left_col: usize,
        right_col: usize,
        how: JoinType,
    ) -> Result<Vec<Vec<String>>> {
        use std::collections::HashMap;

        if left.is_empty() || right.is_empty() {
            return Ok(Vec::new());
        }

        let mut right_index: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, row) in right.iter().enumerate() {
            if let Some(key) = row.get(right_col) {
                right_index.entry(key.clone()).or_default().push(idx);
            }
        }

        let right_width = right.iter().map(|r| r.len()).max().unwrap_or(0);
        let empty_right: Vec<String> = vec![String::new(); right_width];

        let mut result = Vec::new();
        let mut matched_right: std::collections::HashSet<usize> = std::collections::HashSet::new();

        for left_row in left {
            let key = left_row.get(left_col).cloned().unwrap_or_default();

            if let Some(right_indices) = right_index.get(&key) {
                for &right_idx in right_indices {
                    matched_right.insert(right_idx);
                    let mut new_row = left_row.clone();
                    for (idx, val) in right[right_idx].iter().enumerate() {
                        if idx != right_col {
                            new_row.push(val.clone());
                        }
                    }
                    result.push(new_row);
                }
            } else if matches!(how, JoinType::Left | JoinType::Outer) {
                let mut new_row = left_row.clone();
                for (idx, val) in empty_right.iter().enumerate() {
                    if idx != right_col {
                        new_row.push(val.clone());
                    }
                }
                result.push(new_row);
            }
        }

        if matches!(how, JoinType::Right | JoinType::Outer) {
            let left_width = left.iter().map(|r| r.len()).max().unwrap_or(0);
            let empty_left: Vec<String> = vec![String::new(); left_width];

            for (idx, right_row) in right.iter().enumerate() {
                if !matched_right.contains(&idx) {
                    let mut new_row = empty_left.clone();
                    if let Some(key) = right_row.get(right_col) {
                        if left_col < new_row.len() {
                            new_row[left_col] = key.clone();
                        }
                    }
                    for (i, val) in right_row.iter().enumerate() {
                        if i != right_col {
                            new_row.push(val.clone());
                        }
                    }
                    result.push(new_row);
                }
            }
        }

        Ok(result)
    }

    /// Group by column with aggregations
    pub fn groupby(
        &self,
        data: &[Vec<String>],
        group_col: usize,
        aggregations: &[(usize, AggFunc)],
    ) -> Result<Vec<Vec<String>>> {
        use std::collections::HashMap;

        if data.is_empty() {
            return Ok(Vec::new());
        }

        let header = &data[0];
        let mut groups: HashMap<String, Vec<Vec<f64>>> = HashMap::new();

        for row in data.iter().skip(1) {
            let key = row.get(group_col).cloned().unwrap_or_default();
            let entry = groups
                .entry(key)
                .or_insert_with(|| vec![Vec::new(); aggregations.len()]);

            for (i, (col, _)) in aggregations.iter().enumerate() {
                if let Some(val) = row.get(*col).and_then(|v| v.parse::<f64>().ok()) {
                    entry[i].push(val);
                }
            }
        }

        let mut result = Vec::new();

        // Header
        let mut result_header = vec![
            header
                .get(group_col)
                .cloned()
                .unwrap_or_else(|| "group".to_string()),
        ];
        for (col, agg) in aggregations {
            let col_name = header
                .get(*col)
                .cloned()
                .unwrap_or_else(|| format!("col_{}", col));
            result_header.push(format!("{}_{}", agg.name(), col_name));
        }
        result.push(result_header);

        // Data
        let mut keys: Vec<_> = groups.keys().cloned().collect();
        keys.sort();

        for key in keys {
            let values = &groups[&key];
            let mut row = vec![key];
            for (i, (_, agg)) in aggregations.iter().enumerate() {
                let agg_val = agg.apply(&values[i]);
                row.push(format!("{:.2}", agg_val));
            }
            result.push(row);
        }

        Ok(result)
    }

    /// Unpivot wide → long: repeat `id_vars` for each `value_vars` column; add `variable` and `value`.
    ///
    /// If `value_vars` is empty, uses every column index not listed in `id_vars`.
    pub fn melt(
        &self,
        data: &[Vec<String>],
        id_vars: &[usize],
        value_vars: &[usize],
    ) -> Result<Vec<Vec<String>>> {
        use std::collections::HashSet;

        if data.is_empty() {
            return Ok(Vec::new());
        }

        let header = &data[0];
        let max_len = data.iter().map(|r| r.len()).max().unwrap_or(0);

        for &i in id_vars {
            if i >= max_len {
                anyhow::bail!("id column index {} out of range", i);
            }
        }

        let id_set: HashSet<usize> = id_vars.iter().copied().collect();
        let value_indices: Vec<usize> = if value_vars.is_empty() {
            (0..header.len()).filter(|i| !id_set.contains(i)).collect()
        } else {
            for &i in value_vars {
                if i >= max_len {
                    anyhow::bail!("value column index {} out of range", i);
                }
                if id_set.contains(&i) {
                    anyhow::bail!("value column {} cannot also be an id column", i);
                }
            }
            value_vars.to_vec()
        };

        if value_indices.is_empty() {
            anyhow::bail!("melt: no value columns (add id_vars or pass value_vars)");
        }

        let mut out_header: Vec<String> = id_vars
            .iter()
            .map(|&i| {
                header
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{}", i))
            })
            .collect();
        out_header.push("variable".to_string());
        out_header.push("value".to_string());

        let mut result = vec![out_header];

        for row in data.iter().skip(1) {
            for &v in &value_indices {
                let mut new_row: Vec<String> = id_vars
                    .iter()
                    .map(|&i| row.get(i).cloned().unwrap_or_default())
                    .collect();
                let var_name = header
                    .get(v)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{}", v));
                let val = row.get(v).cloned().unwrap_or_default();
                new_row.push(var_name);
                new_row.push(val);
                result.push(new_row);
            }
        }

        Ok(result)
    }
}
