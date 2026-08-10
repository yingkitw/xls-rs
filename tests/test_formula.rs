//! Formula evaluation tests

use xls_rs::FormulaEvaluator;

fn parse_csv_data(content: &str) -> Vec<Vec<String>> {
    content
        .lines()
        .map(|line| {
            line.split(',')
                .map(|s| s.to_string())
                .collect()
        })
        .collect()
}

// ============ Arithmetic Tests ============

#[test]
fn test_formula_addition() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("10,20\n30,40\n");
    let result = evaluator.evaluate_formula_full("A1+B1", &data).unwrap();
    assert!(result.to_string().contains("30"));
}

#[test]
fn test_formula_overlapping_cell_references() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("2\n0\n0\n0\n0\n0\n0\n0\n0\n3\n");
    let result = evaluator.evaluate_formula_full("A1+A10", &data).unwrap();
    assert!(result.to_string().contains("5"));
}

#[test]
fn test_formula_subtraction() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("50,20\n");
    let result = evaluator.evaluate_formula_full("A1-B1", &data).unwrap();
    assert!(result.to_string().contains("30"));
}

#[test]
fn test_formula_multiplication() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("6,7\n");
    let result = evaluator.evaluate_formula_full("A1*B1", &data).unwrap();
    assert!(result.to_string().contains("42"));
}

#[test]
fn test_formula_division() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("100,4\n");
    let result = evaluator.evaluate_formula_full("A1/B1", &data).unwrap();
    assert!(result.to_string().contains("25"));
}

// ============ Aggregate Function Tests ============

#[test]
fn test_formula_sum_column() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("10\n20\n30\n40\n");
    let result = evaluator.evaluate_formula_full("SUM(A1:A4)", &data).unwrap();
    assert!(result.to_string().contains("100"));
}

#[test]
fn test_formula_sum_row() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("1,2,3,4,5\n");
    let result = evaluator.evaluate_formula_full("SUM(A1:E1)", &data).unwrap();
    assert!(result.to_string().contains("15"));
}

#[test]
fn test_formula_average_decimal() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("1\n2\n3\n");
    let result = evaluator.evaluate_formula_full("AVERAGE(A1:A3)", &data).unwrap();
    assert!(result.to_string().contains("2"));
}

#[test]
fn test_formula_min_mixed() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("5,3\n8,1\n2,9\n");
    let result = evaluator.evaluate_formula_full("MIN(A1:B3)", &data).unwrap();
    assert!(result.to_string().contains("1"));
}

#[test]
fn test_formula_max_mixed() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("5,3\n8,1\n2,9\n");
    let result = evaluator.evaluate_formula_full("MAX(A1:B3)", &data).unwrap();
    assert!(result.to_string().contains("9"));
}

// ============ Conditional Function Tests ============

#[test]
fn test_formula_if_equal() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("5,5\n");
    let result = evaluator.evaluate_formula_full("IF(A1=B1, 1, 0)", &data).unwrap();
    assert!(result.to_string().contains("1"));
}

#[test]
fn test_formula_if_less_than() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("3,5\n");
    let result = evaluator.evaluate_formula_full("IF(A1<B1, 100, 0)", &data).unwrap();
    assert!(result.to_string().contains("100"));
}

#[test]
fn test_formula_sumif_greater() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("5\n15\n25\n35\n");
    let result = evaluator.evaluate_formula_full("SUMIF(A1:A4, \">10\")", &data).unwrap();
    assert!(result.to_string().contains("75"));
}

#[test]
fn test_formula_countif_equal() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("A\nB\nA\nC\nA\n");
    let result = evaluator.evaluate_formula_full("COUNTIF(A1:A5, \"A\")", &data).unwrap();
    assert!(result.to_string().contains("3"));
}

// ============ String Function Tests ============

#[test]
fn test_formula_concat_strings() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("Hello,World\n");
    let result = evaluator.evaluate_formula_full("CONCAT(A1, \" \", B1)", &data).unwrap();
    assert!(result.to_string().contains("Hello World"));
}

#[test]
fn test_formula_concat_numbers() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("123,456\n");
    let result = evaluator.evaluate_formula_full("CONCAT(A1, B1)", &data).unwrap();
    assert!(result.to_string().contains("123456"));
}

// ============ Math Function Tests ============

#[test]
fn test_formula_round_up() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("3.567\n");
    let result = evaluator.evaluate_formula_full("ROUND(A1, 1)", &data).unwrap();
    assert!(result.to_string().contains("3.6"));
}

#[test]
fn test_formula_round_down() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("3.123\n");
    let result = evaluator.evaluate_formula_full("ROUND(A1, 1)", &data).unwrap();
    assert!(result.to_string().contains("3.1"));
}

// ============ VLOOKUP Tests ============

#[test]
fn test_formula_vlookup_first_row() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("1,Alice,90\n2,Bob,85\n3,Carol,95\n");
    let result = evaluator.evaluate_formula_full("VLOOKUP(1, A1:C3, 3)", &data).unwrap();
    assert!(result.to_string().contains("90"));
}

#[test]
fn test_formula_vlookup_last_column() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("1,Alice,90\n2,Bob,85\n3,Carol,95\n");
    let result = evaluator.evaluate_formula_full("VLOOKUP(2, A1:C3, 3)", &data).unwrap();
    assert!(result.to_string().contains("85"));
}

// ============ Complex Formula Tests ============

#[test]
fn test_formula_nested_arithmetic() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("10,5,2\n");
    let result = evaluator.evaluate_formula_full("(A1+B1)*C1", &data).unwrap();
    assert!(result.to_string().contains("30"));
}

#[test]
fn test_formula_with_constants() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("10\n");
    let result = evaluator.evaluate_formula_full("A1*2+5", &data).unwrap();
    assert!(result.to_string().contains("25"));
}

// ============ INDEX Tests ============

#[test]
fn test_formula_index_numeric() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("10,20,30\n40,50,60\n70,80,90");
    let result = evaluator.evaluate_formula_full("INDEX(A1:C3, 2, 3)", &data).unwrap();
    assert!(result.to_string().contains("60"));
}

#[test]
fn test_formula_index_row_only() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("a\nb\nc\nd");
    let result = evaluator.evaluate_formula_full("INDEX(A1:A4, 3)", &data).unwrap();
    assert!(result.to_string().contains("c"));
}

// ============ MATCH Tests ============

#[test]
fn test_formula_match_exact() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("apple,banana,cherry,date");
    let result = evaluator.evaluate_formula_full("MATCH(\"banana\", A1:D1, 0)", &data).unwrap();
    assert!(result.to_string().contains("2"));
}

#[test]
fn test_formula_match_numeric() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("10\n20\n30\n40\n");
    let result = evaluator.evaluate_formula_full("MATCH(30, A1:A4, 0)", &data).unwrap();
    assert!(result.to_string().contains("3"));
}
