//! Diff operation for comparing two datasets
//!
//! Compares left and right datasets, reporting added, removed, and changed rows.

use anyhow::Result;
use std::collections::{HashMap, HashSet};

/// Result of diffing two datasets
#[derive(Debug, Default)]
pub struct DiffResult {
    /// Rows only in left (removed)
    pub removed: Vec<Vec<String>>,
    /// Rows only in right (added)
    pub added: Vec<Vec<String>>,
    /// Rows that exist in both but have different values (changed)
    pub changed: Vec<ChangedRow>,
}

/// A row that changed between left and right
#[derive(Debug)]
pub struct ChangedRow {
    /// The key that matched
    pub key: String,
    /// Row from left
    pub left: Vec<String>,
    /// Row from right
    pub right: Vec<String>,
}

/// Compare two datasets
///
/// If key_col is Some, rows are matched by that column's value.
/// Otherwise, rows are matched by position (index).
pub fn diff(
    left: &[Vec<String>],
    right: &[Vec<String>],
    key_col: Option<usize>,
) -> Result<DiffResult> {
    let mut result = DiffResult::default();

    if left.is_empty() && right.is_empty() {
        return Ok(result);
    }

    let (_left_header, left_rows) = if left.is_empty() {
        (Vec::new(), &[] as &[Vec<String>])
    } else {
        (left[0].clone(), &left[1..])
    };

    let (_right_header, right_rows) = if right.is_empty() {
        (Vec::new(), &[] as &[Vec<String>])
    } else {
        (right[0].clone(), &right[1..])
    };

    match key_col {
        Some(col) => diff_by_key(left_rows, right_rows, col, &mut result),
        None => diff_by_index(left_rows, right_rows, &mut result),
    }

    Ok(result)
}

fn row_key(row: &[String], col: usize) -> String {
    row.get(col).map(|s| s.as_str()).unwrap_or("").to_string()
}

fn rows_equal(a: &[String], b: &[String]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(x, y)| x == y)
}

fn diff_by_key(
    left: &[Vec<String>],
    right: &[Vec<String>],
    key_col: usize,
    result: &mut DiffResult,
) {
    let left_map: HashMap<String, Vec<String>> = left
        .iter()
        .map(|r| (row_key(r, key_col), r.clone()))
        .collect();

    let right_map: HashMap<String, Vec<String>> = right
        .iter()
        .map(|r| (row_key(r, key_col), r.clone()))
        .collect();

    let left_keys: HashSet<&String> = left_map.keys().collect();
    let right_keys: HashSet<&String> = right_map.keys().collect();

    for key in left_keys.difference(&right_keys) {
        if let Some(row) = left_map.get(*key) {
            result.removed.push(row.clone());
        }
    }

    for key in right_keys.difference(&left_keys) {
        if let Some(row) = right_map.get(*key) {
            result.added.push(row.clone());
        }
    }

    for key in left_keys.intersection(&right_keys) {
        let left_row = left_map.get(*key).unwrap();
        let right_row = right_map.get(*key).unwrap();
        if !rows_equal(left_row, right_row) {
            result.changed.push(ChangedRow {
                key: (*key).clone(),
                left: left_row.clone(),
                right: right_row.clone(),
            });
        }
    }
}

fn diff_by_index(left: &[Vec<String>], right: &[Vec<String>], result: &mut DiffResult) {
    let left_set: HashSet<Vec<String>> = left.iter().cloned().collect();
    let right_set: HashSet<Vec<String>> = right.iter().cloned().collect();

    for row in left_set.difference(&right_set) {
        result.removed.push(row.clone());
    }

    for row in right_set.difference(&left_set) {
        result.added.push(row.clone());
    }

    // Rows in both sets are identical - no change to report for set-based diff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_empty() {
        let result = diff(&[], &[], None).unwrap();
        assert!(result.removed.is_empty());
        assert!(result.added.is_empty());
        assert!(result.changed.is_empty());
    }

    #[test]
    fn test_diff_by_index_added_removed() {
        // Include header row - diff compares data rows only
        let left = vec![
            vec!["id".into(), "val".into()],
            vec!["a".into(), "1".into()],
            vec!["b".into(), "2".into()],
        ];
        let right = vec![
            vec!["id".into(), "val".into()],
            vec!["b".into(), "2".into()],
            vec!["c".into(), "3".into()],
        ];
        let result = diff(&left, &right, None).unwrap();
        assert_eq!(result.removed.len(), 1);
        assert_eq!(result.added.len(), 1);
        assert_eq!(result.removed[0][0], "a");
        assert_eq!(result.added[0][0], "c");
    }

    #[test]
    fn test_diff_by_key_changed() {
        let left = vec![
            vec!["id".into(), "name".into()],
            vec!["1".into(), "Alice".into()],
            vec!["2".into(), "Bob".into()],
        ];
        let right = vec![
            vec!["id".into(), "name".into()],
            vec!["1".into(), "Alice".into()],
            vec!["2".into(), "Robert".into()],
            vec!["3".into(), "Carol".into()],
        ];
        let result = diff(&left, &right, Some(0)).unwrap();
        assert_eq!(result.removed.len(), 0);
        assert_eq!(result.added.len(), 1);
        assert_eq!(result.added[0][1], "Carol");
        assert_eq!(result.changed.len(), 1);
        assert_eq!(result.changed[0].key, "2");
        assert_eq!(result.changed[0].left[1], "Bob");
        assert_eq!(result.changed[0].right[1], "Robert");
    }
}
