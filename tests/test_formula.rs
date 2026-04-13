//! Additional formula tests

use xls_rs::FormulaEvaluator;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_path(prefix: &str) -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("test_{prefix}_{id}.csv")
}

// ============ Arithmetic Tests ============

#[test]
fn test_formula_addition() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("add_in");
    let output = unique_path("add_out");

    fs::write(&input, "10,20\n30,40\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "A1+B1", "C1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("30")); // 10 + 20 = 30

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_subtraction() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("sub_in");
    let output = unique_path("sub_out");

    fs::write(&input, "50,20\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "A1-B1", "C1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("30")); // 50 - 20 = 30

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_multiplication() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("mul_in");
    let output = unique_path("mul_out");

    fs::write(&input, "6,7\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "A1*B1", "C1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("42")); // 6 * 7 = 42

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_division() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("div_in");
    let output = unique_path("div_out");

    fs::write(&input, "100,4\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "A1/B1", "C1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("25")); // 100 / 4 = 25

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

// ============ Aggregate Function Tests ============

#[test]
fn test_formula_sum_column() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("sum_col_in");
    let output = unique_path("sum_col_out");

    fs::write(&input, "10\n20\n30\n40\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "SUM(A1:A4)", "B1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("100")); // 10+20+30+40 = 100

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_sum_row() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("sum_row_in");
    let output = unique_path("sum_row_out");

    fs::write(&input, "1,2,3,4,5\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "SUM(A1:E1)", "F1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("15")); // 1+2+3+4+5 = 15

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_average_decimal() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("avg_dec_in");
    let output = unique_path("avg_dec_out");

    fs::write(&input, "1\n2\n3\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "AVERAGE(A1:A3)", "B1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("2")); // (1+2+3)/3 = 2

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_min_mixed() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("min_mix_in");
    let output = unique_path("min_mix_out");

    fs::write(&input, "5,3\n8,1\n2,9\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "MIN(A1:B3)", "C1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("1")); // min of all = 1

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_max_mixed() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("max_mix_in");
    let output = unique_path("max_mix_out");

    fs::write(&input, "5,3\n8,1\n2,9\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "MAX(A1:B3)", "C1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("9")); // max of all = 9

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

// ============ Conditional Function Tests ============

#[test]
fn test_formula_if_equal() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("if_eq_in");
    let output = unique_path("if_eq_out");

    fs::write(&input, "5,5\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "IF(A1=B1, 1, 0)", "C1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("1")); // 5 = 5, so true

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_if_less_than() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("if_lt_in");
    let output = unique_path("if_lt_out");

    fs::write(&input, "3,5\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "IF(A1<B1, 100, 0)", "C1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("100")); // 3 < 5, so true

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_sumif_greater() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("sumif_gt_in");
    let output = unique_path("sumif_gt_out");

    // Values: 5, 15, 25, 35
    // Sum where > 10: 15 + 25 + 35 = 75
    fs::write(&input, "5\n15\n25\n35\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "SUMIF(A1:A4, \">10\")", "B1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("75"));

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_countif_equal() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("countif_eq_in");
    let output = unique_path("countif_eq_out");

    fs::write(&input, "A\nB\nA\nC\nA\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "COUNTIF(A1:A5, \"A\")", "B1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("3")); // 3 A's

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

// ============ String Function Tests ============

#[test]
fn test_formula_concat_strings() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("concat_str_in");
    let output = unique_path("concat_str_out");

    fs::write(&input, "Hello,World\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "CONCAT(A1, \" \", B1)", "C1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("Hello World"));

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_concat_numbers() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("concat_num_in");
    let output = unique_path("concat_num_out");

    fs::write(&input, "123,456\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "CONCAT(A1, B1)", "C1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("123456"));

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

// ============ Math Function Tests ============

#[test]
fn test_formula_round_up() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("round_up_in");
    let output = unique_path("round_up_out");

    fs::write(&input, "3.567\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "ROUND(A1, 1)", "B1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("3.6"));

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_round_down() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("round_down_in");
    let output = unique_path("round_down_out");

    fs::write(&input, "3.123\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "ROUND(A1, 1)", "B1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("3.1"));

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

// ============ VLOOKUP Tests ============

#[test]
fn test_formula_vlookup_first_row() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("vlookup_first_in");
    let output = unique_path("vlookup_first_out");

    // ID, Name, Score
    fs::write(&input, "1,Alice,90\n2,Bob,85\n3,Carol,95\n").unwrap();

    // Look up ID=1 and return Score (third column)
    evaluator
        .apply_to_csv(&input, &output, "VLOOKUP(1, A1:C3, 3)", "D1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("90"));

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_vlookup_last_column() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("vlookup_last_in");
    let output = unique_path("vlookup_last_out");

    fs::write(&input, "1,Alice,90\n2,Bob,85\n3,Carol,95\n").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "VLOOKUP(2, A1:C3, 3)", "D1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("85"));

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

// ============ Complex Formula Tests ============

#[test]
fn test_formula_nested_arithmetic() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("nested_in");
    let output = unique_path("nested_out");

    fs::write(&input, "10,5,2\n").unwrap();

    // (10 + 5) * 2 = 30
    evaluator
        .apply_to_csv(&input, &output, "(A1+B1)*C1", "D1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("30"));

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_with_constants() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("const_in");
    let output = unique_path("const_out");

    fs::write(&input, "10\n").unwrap();

    // 10 * 2 + 5 = 25
    evaluator
        .apply_to_csv(&input, &output, "A1*2+5", "B1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("25"));

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

// ============ INDEX Tests ============

#[test]
fn test_formula_index_numeric() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("index_in");
    let output = unique_path("index_out");

    // CSV: row0=10,20,30 row1=40,50,60 row2=70,80,90
    // INDEX(A1:C3, 2, 3) = 2nd row, 3rd col = 60
    fs::write(&input, "10,20,30\n40,50,60\n70,80,90").unwrap();

    evaluator
        .apply_to_csv(&input, &output, "INDEX(A1:C3, 2, 3)", "D1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("60"), "Expected 60 in output: {}", content);

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_index_row_only() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("index_in");
    let output = unique_path("index_out");

    fs::write(&input, "a\nb\nc\nd").unwrap();

    // INDEX(A1:A4, 3) = c
    evaluator
        .apply_to_csv(&input, &output, "INDEX(A1:A4, 3)", "B1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("c"));

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

// ============ MATCH Tests ============

#[test]
fn test_formula_match_exact() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("match_in");
    let output = unique_path("match_out");

    fs::write(&input, "apple,banana,cherry,date").unwrap();

    // MATCH("banana", A1:D1, 0) = 2
    evaluator
        .apply_to_csv(&input, &output, "MATCH(\"banana\", A1:D1, 0)", "E1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("2"));

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}

#[test]
fn test_formula_match_numeric() {
    let evaluator = FormulaEvaluator::new();
    let input = unique_path("match_in");
    let output = unique_path("match_out");

    fs::write(&input, "10\n20\n30\n40\n").unwrap();

    // MATCH(30, A1:A4, 0) = 3
    evaluator
        .apply_to_csv(&input, &output, "MATCH(30, A1:A4, 0)", "B1")
        .unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("3"));

    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();
}
