//! String utilities for performance optimization
//!
//! This module provides helper functions and traits for efficient string operations
//! including capacity pre-allocation and joining operations.

/// Pre-allocate a String with estimated capacity
///
/// This helper avoids reallocations by pre-allocating based on estimated size.
/// Use this when building strings incrementally.
///
/// # Arguments
/// * `estimated_size` - Estimated number of characters needed
///
/// # Returns
/// A String with pre-allocated capacity
#[inline]
pub fn string_with_capacity(estimated_size: usize) -> String {
    String::with_capacity(estimated_size)
}

/// Join strings with a separator, pre-allocating based on estimated total size
///
/// More efficient than standard join when you know the approximate size upfront.
///
/// # Arguments
/// * `parts` - Slice of string references to join
/// * `separator` - Separator string
/// * `estimated_part_size` - Average estimated size of each part
///
/// # Returns
/// A new String with all parts joined by separator
pub fn join_with_capacity(
    parts: &[&str],
    separator: &str,
    estimated_part_size: usize,
) -> String {
    let total_capacity = parts.len() * estimated_part_size + (parts.len() * separator.len());
    let mut result = String::with_capacity(total_capacity);

    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            result.push_str(separator);
        }
        result.push_str(part);
    }

    result
}

/// Estimate CSV row string length based on number of columns
///
/// # Arguments
/// * `num_cols` - Number of columns in the row
///
/// # Returns
/// Estimated string capacity for a CSV row
#[inline]
pub const fn estimate_csv_row_capacity(num_cols: usize) -> usize {
    // Assume average cell length of 10 chars + comma separator
    num_cols * 11
}

/// Estimate JSON array string length
///
/// # Arguments
/// * `num_rows` - Number of rows
/// * `num_cols` - Number of columns per row
/// * `avg_cell_size` - Average size of cell values
///
/// # Returns
/// Estimated string capacity for JSON array
#[inline]
pub const fn estimate_json_array_capacity(
    num_rows: usize,
    num_cols: usize,
    avg_cell_size: usize,
) -> usize {
    // Each cell: "value", (avg_cell_size + 3 quotes)
    // Row: [cell,cell,] (3 extra chars)
    // Array: [rows...] (2 brackets)
    let cell_capacity = (avg_cell_size + 3) * num_cols;
    let row_capacity = cell_capacity + 3;
    row_capacity * num_rows + 2
}

/// Extension trait for efficient string building
pub trait StringBuilder {
    /// Build a String from an iterator with pre-allocated capacity
    fn from_iter_with_capacity<I>(iter: I, estimated_capacity: usize) -> Self
    where
        I: IntoIterator<Item = String>;
}

impl StringBuilder for String {
    fn from_iter_with_capacity<I>(iter: I, estimated_capacity: usize) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut result = String::with_capacity(estimated_capacity);
        for s in iter {
            result.push_str(&s);
        }
        result
    }
}

/// Join cell references efficiently (e.g., ["A", "1"] -> "A1")
#[inline]
pub fn join_cell_reference(col: &str, row: usize) -> String {
    let mut result = String::with_capacity(col.len() + 4);
    result.push_str(col);
    result.push_str(&row.to_string());
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_with_capacity() {
        let parts = vec!["hello", "world", "test"];
        let result = join_with_capacity(&parts, ", ", 5);
        assert_eq!(result, "hello, world, test");
    }

    #[test]
    fn test_join_cell_reference() {
        assert_eq!(join_cell_reference("A", 1), "A1");
        assert_eq!(join_cell_reference("AB", 123), "AB123");
    }

    #[test]
    fn test_estimates() {
        assert_eq!(estimate_csv_row_capacity(5), 55);
        assert_eq!(estimate_json_array_capacity(10, 3, 10), (10 * 3 * 13) + 32);
    }

    #[test]
    fn test_string_builder() {
        let parts = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = String::from_iter_with_capacity(parts.into_iter(), 10);
        assert_eq!(result, "abc");
    }
}
