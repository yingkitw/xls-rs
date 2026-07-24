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

    /// Stratified sample: sample `n` rows proportionally from each stratum
    /// defined by the value in `stratum_col`. Preserves header row.
    pub fn stratified_sample(
        &self,
        data: &[Vec<String>],
        n: usize,
        stratum_col: usize,
        seed: Option<u64>,
    ) -> Result<Vec<Vec<String>>> {
        if data.len() <= 1 {
            return Ok(data.to_vec());
        }

        let header = &data[0];
        let data_rows = &data[1..];

        if n >= data_rows.len() {
            return Ok(data.to_vec());
        }

        use std::collections::HashMap;
        let mut strata: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, row) in data_rows.iter().enumerate() {
            let key = row.get(stratum_col).cloned().unwrap_or_default();
            strata.entry(key).or_default().push(i);
        }

        let total = data_rows.len();
        let mut rng_state = seed.unwrap_or(42);
        let mut next_rand = || {
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
            rng_state
        };

        let mut sampled_indices: Vec<usize> = Vec::with_capacity(n);

        // Allocate proportionally, then distribute remainder
        let stratum_keys: Vec<String> = strata.keys().cloned().collect();
        let mut allocations: Vec<(String, usize)> = Vec::with_capacity(stratum_keys.len());
        let mut allocated = 0usize;
        for key in &stratum_keys {
            let group_size = strata[key].len();
            let proportion = (n * group_size) / total;
            allocations.push((key.clone(), proportion));
            allocated += proportion;
        }
        // Distribute remaining slots to largest strata
        let mut remainder = n - allocated;
        let mut by_size: Vec<usize> = (0..stratum_keys.len()).collect();
        by_size.sort_by(|&a, &b| strata[&stratum_keys[b]].len().cmp(&strata[&stratum_keys[a]].len()));
        for &i in &by_size {
            if remainder == 0 {
                break;
            }
            let available = strata[&stratum_keys[i]].len() - allocations[i].1;
            let extra = remainder.min(available);
            allocations[i].1 += extra;
            remainder -= extra;
        }

        for (key, count) in &allocations {
            let members = &strata[key];
            if members.is_empty() || *count == 0 {
                continue;
            }
            let mut picked: std::collections::HashSet<usize> = std::collections::HashSet::new();
            while picked.len() < *count && picked.len() < members.len() {
                let idx = (next_rand() as usize) % members.len();
                picked.insert(members[idx]);
            }
            sampled_indices.extend(picked);
        }

        sampled_indices.sort();
        let mut result = vec![header.clone()];
        for idx in sampled_indices {
            result.push(data_rows[idx].clone());
        }
        Ok(result)
    }

    /// Systematic sample: pick every k-th row starting at a random offset.
    /// `k` is computed as `total_rows / n`. Preserves header row.
    pub fn systematic_sample(
        &self,
        data: &[Vec<String>],
        n: usize,
        seed: Option<u64>,
    ) -> Vec<Vec<String>> {
        if data.len() <= 1 {
            return data.to_vec();
        }

        let header = &data[0];
        let data_rows = &data[1..];
        let total = data_rows.len();

        if n >= total {
            return data.to_vec();
        }

        let k = total / n;
        if k == 0 {
            return data.to_vec();
        }

        let mut rng_state = seed.unwrap_or(42);
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let start = (rng_state as usize) % k;

        let mut result = vec![header.clone()];
        let mut i = start;
        while i < total && result.len() <= n {
            result.push(data_rows[i].clone());
            i += k;
        }
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

    /// Pivot longer (tidyr-style): reshape wide data to long form.
    ///
    /// `cols` specifies which column indices to pivot into key-value pairs.
    /// All other columns are kept as id columns. `names_to` and `values_to`
    /// set the output column names for the key and value respectively.
    pub fn pivot_longer(
        &self,
        data: &[Vec<String>],
        cols: &[usize],
        names_to: &str,
        values_to: &str,
    ) -> Result<Vec<Vec<String>>> {
        use std::collections::HashSet;

        if data.is_empty() {
            return Ok(Vec::new());
        }

        let header = &data[0];
        let max_len = data.iter().map(|r| r.len()).max().unwrap_or(0);

        let pivot_set: HashSet<usize> = cols.iter().copied().collect();
        for &i in cols {
            if i >= max_len {
                anyhow::bail!("column index {} out of range", i);
            }
        }

        let id_cols: Vec<usize> = (0..header.len()).filter(|i| !pivot_set.contains(i)).collect();

        let mut out_header: Vec<String> = id_cols
            .iter()
            .map(|&i| {
                header
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{}", i))
            })
            .collect();
        out_header.push(names_to.to_string());
        out_header.push(values_to.to_string());

        let mut result = vec![out_header];

        for row in data.iter().skip(1) {
            for &v in cols {
                let mut new_row: Vec<String> = id_cols
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

    /// Pivot wider (tidyr-style): reshape long data to wide form.
    ///
    /// `names_from` is the column index whose values become new column names.
    /// `values_from` is the column index whose values fill the new columns.
    /// `id_cols` lists the column indices that identify each output row;
    /// if empty, all columns except `names_from` and `values_from` are used.
    pub fn pivot_wider(
        &self,
        data: &[Vec<String>],
        names_from: usize,
        values_from: usize,
        id_cols: &[usize],
    ) -> Result<Vec<Vec<String>>> {
        use std::collections::{BTreeSet, HashMap, HashSet};

        if data.is_empty() {
            return Ok(Vec::new());
        }

        let header = &data[0];
        let max_len = data.iter().map(|r| r.len()).max().unwrap_or(0);

        if names_from >= max_len {
            anyhow::bail!("names_from column index {} out of range", names_from);
        }
        if values_from >= max_len {
            anyhow::bail!("values_from column index {} out of range", values_from);
        }

        let excluded: HashSet<usize> = [names_from, values_from].into_iter().collect();
        let id_indices: Vec<usize> = if id_cols.is_empty() {
            (0..header.len()).filter(|i| !excluded.contains(i)).collect()
        } else {
            for &i in id_cols {
                if i >= max_len {
                    anyhow::bail!("id column index {} out of range", i);
                }
            }
            id_cols.to_vec()
        };

        let mut new_col_names: BTreeSet<String> = BTreeSet::new();
        let mut id_values: BTreeSet<Vec<String>> = BTreeSet::new();
        let mut cell_map: HashMap<(Vec<String>, String), String> = HashMap::new();

        for row in data.iter().skip(1) {
            let id_vals: Vec<String> = id_indices
                .iter()
                .map(|&i| row.get(i).cloned().unwrap_or_default())
                .collect();
            let col_name = row.get(names_from).cloned().unwrap_or_default();
            let val = row.get(values_from).cloned().unwrap_or_default();

            new_col_names.insert(col_name.clone());
            id_values.insert(id_vals.clone());
            cell_map.insert((id_vals, col_name), val);
        }

        let mut out_header: Vec<String> = id_indices
            .iter()
            .map(|&i| {
                header
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{}", i))
            })
            .collect();
        out_header.extend(new_col_names.iter().cloned());

        let mut result = vec![out_header];

        for id_vals in &id_values {
            let mut out_row = id_vals.clone();
            for col_name in &new_col_names {
                let val = cell_map
                    .get(&(id_vals.clone(), col_name.clone()))
                    .cloned()
                    .unwrap_or_default();
                out_row.push(val);
            }
            result.push(out_row);
        }

        Ok(result)
    }
}
