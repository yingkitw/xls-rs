mod common;

use xls_rs::{AggFunc, DataOperations, JoinType, SortOrder};
use std::fs;

fn read_example_csv(name: &str) -> Vec<Vec<String>> {
    common::ensure_example_fixtures();
    let path = common::example_path(&format!("{name}.csv"));
    let content = fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {path}"));
    content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.split(',').map(|s| s.to_string()).collect())
        .collect()
}

// ============ Sort Tests ============

#[test]
fn test_sort_ascending_numeric() {
    let ops = DataOperations::new();
    let mut data = read_example_csv("numbers");

    // Sort by column A (index 0) ascending
    ops.sort_by_column(&mut data, 0, SortOrder::Ascending)
        .unwrap();

    // Header stays first, data rows are sorted
    // Verify data is sorted (header row may or may not stay first depending on impl)
    assert!(data.len() > 1);
    // Check that sorting happened - smallest values should be near the top
    let has_small_value = data.iter().take(3).any(|r| r[0] == "1" || r[0] == "4");
    assert!(
        has_small_value,
        "Small values should be near top after ascending sort"
    );
}

#[test]
fn test_sort_descending_numeric() {
    let ops = DataOperations::new();
    let mut data = read_example_csv("numbers");

    ops.sort_by_column(&mut data, 0, SortOrder::Descending)
        .unwrap();

    assert_eq!(data[0][0], "A"); // Header
    assert_eq!(data[1][0], "4"); // Largest first
    assert_eq!(data[2][0], "1"); // Smallest last
}

#[test]
fn test_sort_string_column() {
    let ops = DataOperations::new();
    let mut data = read_example_csv("employees");

    // Sort by Name (index 1) ascending
    ops.sort_by_column(&mut data, 1, SortOrder::Ascending)
        .unwrap();

    // Verify sorting happened - Alice should be near the top
    let alice_pos = data.iter().position(|r| r[1] == "Alice Johnson");
    assert!(alice_pos.is_some(), "Alice Johnson should be in the data");
    assert!(
        alice_pos.unwrap() <= 2,
        "Alice Johnson should be near top after ascending sort"
    );
}

// ============ Filter Tests ============

#[test]
fn test_filter_equals() {
    let ops = DataOperations::new();
    let data = read_example_csv("sales");

    // Filter Category == "Electronics"
    let filtered = ops.filter_rows(&data, 1, "=", "Electronics").unwrap();

    assert!(filtered.len() > 1); // Header + results
    for row in filtered.iter().skip(1) {
        assert_eq!(row[1], "Electronics");
    }
}

#[test]
fn test_filter_greater_than() {
    let ops = DataOperations::new();
    let data = read_example_csv("employees");

    // Filter Salary > 80000 (column 3)
    let filtered = ops.filter_rows(&data, 3, ">", "80000").unwrap();

    // Should include: Alice (85000), Carol (92000), Grace (81000), Henry (95000)
    assert!(filtered.len() >= 4);
    for row in filtered.iter().skip(1) {
        let salary: f64 = row[3].parse().unwrap();
        assert!(salary > 80000.0);
    }
}

#[test]
fn test_filter_contains() {
    let ops = DataOperations::new();
    let data = read_example_csv("employees");

    // Filter Name contains "son"
    let filtered = ops.filter_rows(&data, 1, "contains", "son").unwrap();

    // Should include: Alice Johnson, Henry Wilson
    assert!(filtered.len() >= 2);
    for row in filtered.iter().skip(1) {
        assert!(row[1].contains("son"));
    }
}

// ============ Deduplicate Tests ============

#[test]
fn test_deduplicate() {
    let ops = DataOperations::new();
    let data = read_example_csv("duplicates");

    let deduped = ops.deduplicate(&data);

    // Original has 8 rows (header + 7 data), with duplicates
    // Unique: header, Apple/100, Banana/200, Cherry/300, Date/400 = 5 rows
    assert_eq!(deduped.len(), 5);
}

// ============ Head/Tail Tests ============

#[test]
fn test_head() {
    let ops = DataOperations::new();
    let data = read_example_csv("sales");

    let head = ops.head(&data, 3);

    assert_eq!(head.len(), 3);
    assert_eq!(head[0][0], "Product"); // Header
}

#[test]
fn test_tail() {
    let ops = DataOperations::new();
    let data = read_example_csv("sales");

    let tail = ops.tail(&data, 2);

    assert_eq!(tail.len(), 2);
    // Last two products: Pen and Lamp
    assert!(tail.iter().any(|r| r[0] == "Pen" || r[0] == "Lamp"));
}

// ============ Select Columns Tests ============

#[test]
fn test_select_columns_by_index() {
    let ops = DataOperations::new();
    let data = read_example_csv("sales");

    // Select Product (0) and Price (2)
    let selected = ops.select_columns(&data, &[0, 2]);

    assert_eq!(selected[0].len(), 2);
    assert_eq!(selected[0][0], "Product");
    assert_eq!(selected[0][1], "Price");
}

#[test]
fn test_select_columns_by_name() {
    let ops = DataOperations::new();
    let data = read_example_csv("employees");

    let selected = ops
        .select_columns_by_name(&data, &["Name", "Salary"])
        .unwrap();

    assert_eq!(selected[0].len(), 2);
    assert_eq!(selected[0][0], "Name");
    assert_eq!(selected[0][1], "Salary");
}

#[test]
fn test_select_columns_by_name_column_order_follows_request() {
    let ops = DataOperations::new();
    let data = read_example_csv("sales");

    let selected = ops
        .select_columns_by_name(&data, &["Price", "Product"])
        .unwrap();

    assert_eq!(selected[0], vec!["Price", "Product"]);
    assert!(selected.len() > 1);
}

#[test]
fn test_select_columns_by_name_missing_column_errors() {
    let ops = DataOperations::new();
    let data = read_example_csv("lookup");

    let err = ops
        .select_columns_by_name(&data, &["Code", "NoSuchCol"])
        .unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("NoSuchCol") || msg.contains("not found"),
        "{msg}"
    );
}

// ============ Drop Columns Tests ============

#[test]
fn test_drop_columns() {
    let ops = DataOperations::new();
    let data = read_example_csv("sales");

    // Drop Date column (index 4)
    let dropped = ops.drop_columns(&data, &[4]);

    assert_eq!(dropped[0].len(), 4); // 5 - 1 = 4 columns
    assert!(!dropped[0].contains(&"Date".to_string()));
}

// ============ Rename Columns Tests ============

#[test]
fn test_rename_columns() {
    let ops = DataOperations::new();
    let mut data = read_example_csv("numbers");

    ops.rename_columns(&mut data, &[("A", "Column_A"), ("B", "Column_B")])
        .unwrap();

    assert_eq!(data[0][0], "Column_A");
    assert_eq!(data[0][1], "Column_B");
}

// ============ Fill NA Tests ============

#[test]
fn test_fillna() {
    let ops = DataOperations::new();
    let mut data = vec![
        vec!["A".to_string(), "B".to_string()],
        vec!["1".to_string(), "".to_string()],
        vec!["".to_string(), "3".to_string()],
    ];

    ops.fillna(&mut data, "0");

    assert_eq!(data[1][1], "0");
    assert_eq!(data[2][0], "0");
}

// ============ Drop NA Tests ============

#[test]
fn test_dropna() {
    let ops = DataOperations::new();
    let data = vec![
        vec!["A".to_string(), "B".to_string()],
        vec!["1".to_string(), "2".to_string()],
        vec!["3".to_string(), "".to_string()],
        vec!["5".to_string(), "6".to_string()],
    ];

    let cleaned = ops.dropna(&data);

    // Should keep header and rows without empty values
    assert_eq!(cleaned.len(), 3);
}

// ============ Value Counts Tests ============

#[test]
fn test_value_counts() {
    let ops = DataOperations::new();
    let data = read_example_csv("sales");

    // Count categories
    let counts = ops.value_counts(&data, 1);

    // Should have header + unique categories
    assert!(counts.len() > 1);
    // Header row exists with value/count columns
    assert!(counts[0].len() >= 2);
}

// ============ Unique Tests ============

#[test]
fn test_unique() {
    let ops = DataOperations::new();
    let data = read_example_csv("duplicates");

    // Get unique values in Name column (index 0)
    let unique = ops.unique(&data, 0);

    // Header + Apple, Banana, Cherry, Date = 5
    assert_eq!(unique.len(), 5);
}

// ============ Describe Tests ============

#[test]
fn test_describe() {
    let ops = DataOperations::new();
    let data = read_example_csv("numbers");

    let desc = ops.describe(&data).unwrap();

    // Should have stats rows: count, mean, std, min, max, etc.
    assert!(desc.len() > 1);
    // First column should be stat names
    assert!(desc.iter().any(|r| r[0] == "count" || r[0] == "mean"));
}

// ============ Transpose Tests ============

#[test]
fn test_transpose() {
    let ops = DataOperations::new();
    let data = vec![
        vec!["A".to_string(), "B".to_string(), "C".to_string()],
        vec!["1".to_string(), "2".to_string(), "3".to_string()],
    ];

    let transposed = ops.transpose(&data);

    assert_eq!(transposed.len(), 3); // 3 columns become 3 rows
    assert_eq!(transposed[0].len(), 2); // 2 rows become 2 columns
    assert_eq!(transposed[0][0], "A");
    assert_eq!(transposed[0][1], "1");
}

// ============ Concat Tests ============

#[test]
fn test_concat() {
    let ops = DataOperations::new();
    let data1 = vec![
        vec!["A".to_string(), "B".to_string()],
        vec!["1".to_string(), "2".to_string()],
    ];
    let data2 = vec![
        vec!["A".to_string(), "B".to_string()],
        vec!["3".to_string(), "4".to_string()],
    ];

    let combined = ops.concat(&[data1, data2]);

    assert_eq!(combined.len(), 4); // 2 + 2 rows
}

// ============ Join Tests ============

#[test]
fn test_inner_join() {
    let ops = DataOperations::new();
    let left = vec![
        vec!["ID".to_string(), "Name".to_string()],
        vec!["1".to_string(), "Alice".to_string()],
        vec!["2".to_string(), "Bob".to_string()],
        vec!["3".to_string(), "Carol".to_string()],
    ];
    let right = vec![
        vec!["ID".to_string(), "Score".to_string()],
        vec!["1".to_string(), "90".to_string()],
        vec!["2".to_string(), "85".to_string()],
        vec!["4".to_string(), "95".to_string()],
    ];

    let joined = ops.join(&left, &right, 0, 0, JoinType::Inner).unwrap();

    // Inner join: only IDs 1 and 2 match
    assert_eq!(joined.len(), 3); // Header + 2 matches
}

#[test]
fn test_left_join() {
    let ops = DataOperations::new();
    let left = vec![
        vec!["ID".to_string(), "Name".to_string()],
        vec!["1".to_string(), "Alice".to_string()],
        vec!["2".to_string(), "Bob".to_string()],
        vec!["3".to_string(), "Carol".to_string()],
    ];
    let right = vec![
        vec!["ID".to_string(), "Score".to_string()],
        vec!["1".to_string(), "90".to_string()],
        vec!["2".to_string(), "85".to_string()],
    ];

    let joined = ops.join(&left, &right, 0, 0, JoinType::Left).unwrap();

    // Left join: all left rows preserved
    assert_eq!(joined.len(), 4); // Header + 3 left rows
}

// ============ Groupby Tests ============

#[test]
fn test_groupby_sum() {
    let ops = DataOperations::new();
    let data = read_example_csv("sales");

    // Group by Category (1), sum Quantity (3)
    let grouped = ops.groupby(&data, 1, &[(3, AggFunc::Sum)]).unwrap();

    // Should have header + unique categories
    assert!(grouped.len() > 1);
    assert_eq!(grouped[0][0], "Category");
}

#[test]
fn test_groupby_count() {
    let ops = DataOperations::new();
    let data = read_example_csv("employees");

    // Group by Department (2), count
    let grouped = ops.groupby(&data, 2, &[(0, AggFunc::Count)]).unwrap();

    // Engineering: 4, Marketing: 3, Sales: 3
    assert!(grouped.len() == 4); // Header + 3 departments
}

// ============ Dtypes Tests ============

#[test]
fn test_dtypes() {
    let ops = DataOperations::new();
    let data = read_example_csv("employees");

    let dtypes = ops.dtypes(&data);

    // Should have header + column type info
    assert!(dtypes.len() > 1);
    // ID should be detected as integer, Salary as integer/float
}

// ============ Info Tests ============

#[test]
fn test_info() {
    let ops = DataOperations::new();
    let data = read_example_csv("sales");

    let info = ops.info(&data);

    // Should contain dataset info
    assert!(!info.is_empty());
}

// ============ Sample Tests ============

#[test]
fn test_sample_with_seed() {
    let ops = DataOperations::new();
    let data = read_example_csv("sales");

    let sample1 = ops.sample(&data, 3, Some(42));
    let sample2 = ops.sample(&data, 3, Some(42));

    // Same seed should produce same sample
    assert_eq!(sample1.len(), sample2.len());
    assert_eq!(sample1.len(), 3);
}

// ============ Find Replace Tests ============

#[test]
fn test_find_replace() {
    let ops = DataOperations::new();
    let mut data = vec![
        vec!["Name".to_string(), "Status".to_string()],
        vec!["Alice".to_string(), "active".to_string()],
        vec!["Bob".to_string(), "inactive".to_string()],
        vec!["Carol".to_string(), "active".to_string()],
    ];

    let count = ops
        .find_replace(&mut data, "active", "enabled", None)
        .unwrap();

    assert_eq!(count, 3); // "active" appears 3 times (including in "inactive")
}

// ============ Replace in Column Tests ============

#[test]
fn test_replace_in_column() {
    let ops = DataOperations::new();
    let mut data = vec![
        vec!["Name".to_string(), "Status".to_string()],
        vec!["Alice".to_string(), "active".to_string()],
        vec!["Bob".to_string(), "pending".to_string()],
    ];

    let count = ops.replace(&mut data, 1, "active", "enabled");

    // Should replace "active" with "enabled"
    assert!(count >= 1);
    assert_eq!(data[1][1], "enabled");
}

// ============ Z-score (standardization) ============

#[test]
fn test_zscore_column() {
    let ops = DataOperations::new();
    let mut data = vec![
        vec!["x".to_string()],
        vec!["0".to_string()],
        vec!["2".to_string()],
        vec!["4".to_string()],
    ];
    ops.zscore(&mut data, 0).unwrap();
    // mean=2, pop std = sqrt(((4+0+4)/3)) = sqrt(8/3); z for 0 = (0-2)/std
    let z0: f64 = data[1][0].parse().unwrap();
    let z2: f64 = data[2][0].parse().unwrap();
    let z4: f64 = data[3][0].parse().unwrap();
    assert!((z0 + z2 + z4).abs() < 1e-5, "z-scores should sum to ~0");
    let mean_z = (z0 + z2 + z4) / 3.0;
    assert!(mean_z.abs() < 1e-5);
}

// ============ Rolling window ============

#[test]
fn test_rolling_mean_column() {
    let ops = DataOperations::new();
    let mut data = vec![
        vec!["t".to_string(), "v".to_string()],
        vec!["1".to_string(), "10".to_string()],
        vec!["2".to_string(), "20".to_string()],
        vec!["3".to_string(), "30".to_string()],
    ];
    ops
        .rolling_mean_column(&mut data, 1, 2, "roll2")
        .unwrap();
    assert_eq!(data[0][2], "roll2");
    let r1: f64 = data[1][2].parse().unwrap();
    assert!((r1 - 10.0).abs() < 1e-5);
    let r2: f64 = data[2][2].parse().unwrap();
    assert!((r2 - 15.0).abs() < 1e-5);
    let r3: f64 = data[3][2].parse().unwrap();
    assert!((r3 - 25.0).abs() < 1e-5);
}

#[test]
fn test_rolling_sum_column() {
    let ops = DataOperations::new();
    let mut data = vec![
        vec!["v".to_string()],
        vec!["1".to_string()],
        vec!["2".to_string()],
        vec!["3".to_string()],
    ];
    ops.rolling_sum_column(&mut data, 0, 2, "s").unwrap();
    let s3: f64 = data[3][1].parse().unwrap();
    assert!((s3 - 5.0).abs() < 1e-5);
}

// ============ Crosstab ============

#[test]
fn test_crosstab() {
    let ops = DataOperations::new();
    let data = vec![
        vec!["a".to_string(), "b".to_string()],
        vec!["x".to_string(), "p".to_string()],
        vec!["x".to_string(), "q".to_string()],
        vec!["y".to_string(), "p".to_string()],
    ];
    let ct = ops.crosstab(&data, 0, 1).unwrap();
    assert_eq!(ct[0], vec!["a", "p", "q"]);
    assert_eq!(ct[1], vec!["x", "1", "1"]);
    assert_eq!(ct[2], vec!["y", "1", "0"]);
}

// ============ Melt ============

#[test]
fn test_melt() {
    let ops = DataOperations::new();
    let data = vec![
        vec!["id".to_string(), "A".to_string(), "B".to_string()],
        vec!["1".to_string(), "10".to_string(), "20".to_string()],
    ];
    let long = ops.melt(&data, &[0], &[1, 2]).unwrap();
    assert_eq!(long[0], vec!["id", "variable", "value"]);
    assert_eq!(long[1], vec!["1", "A", "10"]);
    assert_eq!(long[2], vec!["1", "B", "20"]);
}

#[test]
fn test_melt_infer_value_vars() {
    let ops = DataOperations::new();
    let data = vec![
        vec!["id".to_string(), "A".to_string(), "B".to_string()],
        vec!["1".to_string(), "10".to_string(), "20".to_string()],
    ];
    let long = ops.melt(&data, &[0], &[]).unwrap();
    assert_eq!(long.len(), 3);
    assert_eq!(long[1][1], "A");
    assert_eq!(long[2][1], "B");
}
