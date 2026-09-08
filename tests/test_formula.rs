//! Formula evaluation tests

use xls_rs::FormulaEvaluator;

fn parse_csv_data(content: &str) -> Vec<Vec<String>> {
    content
        .lines()
        .map(|line| line.split(',').map(|s| s.to_string()).collect())
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
    let result = evaluator
        .evaluate_formula_full("SUM(A1:A4)", &data)
        .unwrap();
    assert!(result.to_string().contains("100"));
}

#[test]
fn test_formula_sum_row() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("1,2,3,4,5\n");
    let result = evaluator
        .evaluate_formula_full("SUM(A1:E1)", &data)
        .unwrap();
    assert!(result.to_string().contains("15"));
}

#[test]
fn test_formula_average_decimal() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("1\n2\n3\n");
    let result = evaluator
        .evaluate_formula_full("AVERAGE(A1:A3)", &data)
        .unwrap();
    assert!(result.to_string().contains("2"));
}

#[test]
fn test_formula_min_mixed() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("5,3\n8,1\n2,9\n");
    let result = evaluator
        .evaluate_formula_full("MIN(A1:B3)", &data)
        .unwrap();
    assert!(result.to_string().contains("1"));
}

#[test]
fn test_formula_max_mixed() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("5,3\n8,1\n2,9\n");
    let result = evaluator
        .evaluate_formula_full("MAX(A1:B3)", &data)
        .unwrap();
    assert!(result.to_string().contains("9"));
}

// ============ Conditional Function Tests ============

#[test]
fn test_formula_if_equal() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("5,5\n");
    let result = evaluator
        .evaluate_formula_full("IF(A1=B1, 1, 0)", &data)
        .unwrap();
    assert!(result.to_string().contains("1"));
}

#[test]
fn test_formula_if_less_than() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("3,5\n");
    let result = evaluator
        .evaluate_formula_full("IF(A1<B1, 100, 0)", &data)
        .unwrap();
    assert!(result.to_string().contains("100"));
}

#[test]
fn test_formula_sumif_greater() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("5\n15\n25\n35\n");
    let result = evaluator
        .evaluate_formula_full("SUMIF(A1:A4, \">10\")", &data)
        .unwrap();
    assert!(result.to_string().contains("75"));
}

#[test]
fn test_formula_countif_equal() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("A\nB\nA\nC\nA\n");
    let result = evaluator
        .evaluate_formula_full("COUNTIF(A1:A5, \"A\")", &data)
        .unwrap();
    assert!(result.to_string().contains("3"));
}

// ============ String Function Tests ============

#[test]
fn test_formula_concat_strings() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("Hello,World\n");
    let result = evaluator
        .evaluate_formula_full("CONCAT(A1, \" \", B1)", &data)
        .unwrap();
    assert!(result.to_string().contains("Hello World"));
}

#[test]
fn test_formula_concat_numbers() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("123,456\n");
    let result = evaluator
        .evaluate_formula_full("CONCAT(A1, B1)", &data)
        .unwrap();
    assert!(result.to_string().contains("123456"));
}

// ============ Math Function Tests ============

#[test]
fn test_formula_round_up() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("3.567\n");
    let result = evaluator
        .evaluate_formula_full("ROUND(A1, 1)", &data)
        .unwrap();
    assert!(result.to_string().contains("3.6"));
}

#[test]
fn test_formula_round_down() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("3.123\n");
    let result = evaluator
        .evaluate_formula_full("ROUND(A1, 1)", &data)
        .unwrap();
    assert!(result.to_string().contains("3.1"));
}

// ============ VLOOKUP Tests ============

#[test]
fn test_formula_vlookup_first_row() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("1,Alice,90\n2,Bob,85\n3,Carol,95\n");
    let result = evaluator
        .evaluate_formula_full("VLOOKUP(1, A1:C3, 3)", &data)
        .unwrap();
    assert!(result.to_string().contains("90"));
}

#[test]
fn test_formula_vlookup_last_column() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("1,Alice,90\n2,Bob,85\n3,Carol,95\n");
    let result = evaluator
        .evaluate_formula_full("VLOOKUP(2, A1:C3, 3)", &data)
        .unwrap();
    assert!(result.to_string().contains("85"));
}

// ============ Complex Formula Tests ============

#[test]
fn test_formula_nested_arithmetic() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("10,5,2\n");
    let result = evaluator
        .evaluate_formula_full("(A1+B1)*C1", &data)
        .unwrap();
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
    let result = evaluator
        .evaluate_formula_full("INDEX(A1:C3, 2, 3)", &data)
        .unwrap();
    assert!(result.to_string().contains("60"));
}

#[test]
fn test_formula_index_row_only() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("a\nb\nc\nd");
    let result = evaluator
        .evaluate_formula_full("INDEX(A1:A4, 3)", &data)
        .unwrap();
    assert!(result.to_string().contains("c"));
}

// ============ MATCH Tests ============

#[test]
fn test_formula_match_exact() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("apple,banana,cherry,date");
    let result = evaluator
        .evaluate_formula_full("MATCH(\"banana\", A1:D1, 0)", &data)
        .unwrap();
    assert!(result.to_string().contains("2"));
}

#[test]
fn test_formula_match_numeric() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("10\n20\n30\n40\n");
    let result = evaluator
        .evaluate_formula_full("MATCH(30, A1:A4, 0)", &data)
        .unwrap();
    assert!(result.to_string().contains("3"));
}

// ============ Extended Function Tests (COUNTA, AVERAGEIF, MOD, INT, POWER, SQRT, ROUNDUP/ROUNDDOWN, text fns) ============

#[test]
fn test_formula_counta_counts_non_empty_including_text() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("10,apple\n,3\ntext,\n");
    // COUNT counts only numerics (10, 3) = 2; COUNTA counts all non-empty = 4
    let count = evaluator
        .evaluate_formula_full("COUNTA(A1:B3)", &data)
        .unwrap();
    assert_eq!(count.to_string(), "4");
    let count = evaluator
        .evaluate_formula_full("COUNT(A1:B3)", &data)
        .unwrap();
    assert_eq!(count.to_string(), "2");
}

#[test]
fn test_formula_averageif() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("10,1\n20,2\n30,1\n");
    let result = evaluator
        .evaluate_formula_full("AVERAGEIF(A1:A3,\">15\")", &data)
        .unwrap();
    assert_eq!(result.to_string(), "25");
}

#[test]
fn test_formula_averageif_with_average_range() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("x,10\ny,20\nx,30\n");
    let result = evaluator
        .evaluate_formula_full("AVERAGEIF(A1:A3,\"x\",B1:B3)", &data)
        .unwrap();
    assert_eq!(result.to_string(), "20");
}

#[test]
fn test_formula_averageif_no_match_returns_zero() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("10,20\n");
    let result = evaluator
        .evaluate_formula_full("AVERAGEIF(A1:A1,\">999\")", &data)
        .unwrap();
    assert_eq!(result.to_string(), "0");
}

#[test]
fn test_formula_mod_excel_sign_semantics() {
    let evaluator = FormulaEvaluator::new();
    // Excel MOD: result takes the sign of the divisor
    let r = evaluator
        .evaluate_formula_full("MOD(3,2)", &data_empty())
        .unwrap();
    assert_eq!(r.to_string(), "1");
    let r = evaluator
        .evaluate_formula_full("MOD(-3,2)", &data_empty())
        .unwrap();
    assert_eq!(r.to_string(), "1");
    let r = evaluator
        .evaluate_formula_full("MOD(3,-2)", &data_empty())
        .unwrap();
    assert_eq!(r.to_string(), "-1");
    assert!(
        evaluator
            .evaluate_formula_full("MOD(1,0)", &data_empty())
            .is_err()
    );
}

#[test]
fn test_formula_int_floors_toward_negative_infinity() {
    let evaluator = FormulaEvaluator::new();
    assert_eq!(
        evaluator
            .evaluate_formula_full("INT(8.9)", &data_empty())
            .unwrap()
            .to_string(),
        "8"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("INT(-8.1)", &data_empty())
            .unwrap()
            .to_string(),
        "-9"
    );
}

#[test]
fn test_formula_power_and_sqrt() {
    let evaluator = FormulaEvaluator::new();
    assert_eq!(
        evaluator
            .evaluate_formula_full("POWER(2,10)", &data_empty())
            .unwrap()
            .to_string(),
        "1024"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("SQRT(81)", &data_empty())
            .unwrap()
            .to_string(),
        "9"
    );
    assert!(
        evaluator
            .evaluate_formula_full("SQRT(-1)", &data_empty())
            .is_err()
    );
}

#[test]
fn test_formula_roundup_rounddown() {
    let evaluator = FormulaEvaluator::new();
    // ROUNDUP: away from zero; ROUNDDOWN: toward zero
    assert_eq!(
        evaluator
            .evaluate_formula_full("ROUNDUP(3.141,2)", &data_empty())
            .unwrap()
            .to_string(),
        "3.15"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("ROUNDDOWN(3.141,2)", &data_empty())
            .unwrap()
            .to_string(),
        "3.14"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("ROUNDUP(-3.2,0)", &data_empty())
            .unwrap()
            .to_string(),
        "-4"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("ROUNDDOWN(-3.8,0)", &data_empty())
            .unwrap()
            .to_string(),
        "-3"
    );
}

#[test]
fn test_formula_upper_lower_trim() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("hello world\n");
    assert_eq!(
        evaluator
            .evaluate_formula_full("UPPER(A1)", &data)
            .unwrap()
            .to_string(),
        "HELLO WORLD"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("LOWER(\"MiXeD\")", &data)
            .unwrap()
            .to_string(),
        "mixed"
    );
    // TRIM collapses internal whitespace runs (Excel semantics)
    assert_eq!(
        evaluator
            .evaluate_formula_full("TRIM(\"  too   many spaces  \")", &data)
            .unwrap()
            .to_string(),
        "too many spaces"
    );
}

#[test]
fn test_formula_left_right_mid() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("spreadsheet\n");
    // Default count is 1
    assert_eq!(
        evaluator
            .evaluate_formula_full("LEFT(A1)", &data)
            .unwrap()
            .to_string(),
        "s"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("LEFT(A1,6)", &data)
            .unwrap()
            .to_string(),
        "spread"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("RIGHT(A1,5)", &data)
            .unwrap()
            .to_string(),
        "sheet"
    );
    // MID is 1-based
    assert_eq!(
        evaluator
            .evaluate_formula_full("MID(A1,7,5)", &data)
            .unwrap()
            .to_string(),
        "sheet"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("MID(A1,1,3)", &data)
            .unwrap()
            .to_string(),
        "spr"
    );
    assert!(
        evaluator
            .evaluate_formula_full("MID(A1,0,3)", &data)
            .is_err()
    );
}

fn data_empty() -> Vec<Vec<String>> {
    vec![vec![String::new()]]
}

// ============ Logic Function Tests (AND, OR, NOT, IFERROR) ============

#[test]
fn test_formula_and_or_not_standalone() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("5,10\n");
    assert_eq!(
        evaluator
            .evaluate_formula_full("AND(A1>1,B1>1)", &data)
            .unwrap()
            .to_string(),
        "TRUE"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("AND(A1>1,B1>100)", &data)
            .unwrap()
            .to_string(),
        "FALSE"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("OR(A1>100,B1>1)", &data)
            .unwrap()
            .to_string(),
        "TRUE"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("OR(A1>100,B1>100)", &data)
            .unwrap()
            .to_string(),
        "FALSE"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("NOT(A1>1)", &data)
            .unwrap()
            .to_string(),
        "FALSE"
    );
    // TRUE/FALSE literals and numbers as arguments
    assert_eq!(
        evaluator
            .evaluate_formula_full("AND(TRUE,1)", &data)
            .unwrap()
            .to_string(),
        "TRUE"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("AND(TRUE,0)", &data)
            .unwrap()
            .to_string(),
        "FALSE"
    );
}

#[test]
fn test_formula_logic_inside_if() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("5,10\n");
    // The whole point of AND/OR: composition inside IF conditions
    let r = evaluator
        .evaluate_formula_full("IF(AND(A1>1,B1>1),\"both\",\"not both\")", &data)
        .unwrap();
    assert_eq!(r.to_string(), "both");
    let r = evaluator
        .evaluate_formula_full("IF(OR(A1>100,B1>100),\"big\",\"small\")", &data)
        .unwrap();
    assert_eq!(r.to_string(), "small");
    let r = evaluator
        .evaluate_formula_full("IF(NOT(A1>100),\"not big\",\"big\")", &data)
        .unwrap();
    assert_eq!(r.to_string(), "not big");
}

#[test]
fn test_formula_iferror_catches_evaluation_errors() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("4\n");
    // SQRT(-1) errors; IFERROR falls back
    let r = evaluator
        .evaluate_formula_full("IFERROR(SQRT(-1),99)", &data)
        .unwrap();
    assert_eq!(r.to_string(), "99");
    // Non-error passes through untouched
    let r = evaluator
        .evaluate_formula_full("IFERROR(SQRT(A1),0)", &data)
        .unwrap();
    assert_eq!(r.to_string(), "2");
}

#[test]
fn test_formula_iferror_catches_error_literals() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("#DIV/0!\n");
    // Cell containing an Excel error literal counts as an error
    let r = evaluator
        .evaluate_formula_full("IFERROR(A1,-1)", &data)
        .unwrap();
    assert_eq!(r.to_string(), "-1");
}

// ============ Defined Name (Named Range) Tests ============

#[test]
fn test_formula_named_range_sum_and_scalar() {
    let path = format!(
        "{}/tests/fixtures/named_ranges.xlsx",
        env!("CARGO_MANIFEST_DIR")
    );
    let wb = xls_rs::excel::xlsx_reader::XlsxReader::from_path(&path).unwrap();
    let names: std::collections::HashMap<String, String> =
        wb.defined_names().iter().cloned().collect();
    let evaluator = FormulaEvaluator::new().with_defined_names(names);
    let data = wb.get_sheet_by_name("T").unwrap().to_string_vec().to_vec();

    // MyData = T!$A$1:$A$3 → 10 + 20 + 30
    assert_eq!(
        evaluator
            .evaluate_formula_full("SUM(MyData)", &data)
            .unwrap()
            .to_string(),
        "60"
    );
    // Top-level comparison with a named single cell on the right
    // (numeric coercion of booleans: TRUE renders as 1)
    assert_eq!(
        evaluator
            .evaluate_formula_full("SUM(MyData)>Threshold", &data)
            .unwrap()
            .to_string(),
        "1"
    );
    // Composition inside IF
    let r = evaluator
        .evaluate_formula_full("IF(SUM(MyData)>Threshold,\"over\",\"under\")", &data)
        .unwrap();
    assert_eq!(r.to_string(), "over");
}

#[test]
fn test_formula_defined_names_accessor() {
    let path = format!(
        "{}/tests/fixtures/named_ranges.xlsx",
        env!("CARGO_MANIFEST_DIR")
    );
    let wb = xls_rs::excel::xlsx_reader::XlsxReader::from_path(&path).unwrap();
    let names = wb.defined_names();
    assert!(names.contains(&("MyData".to_string(), "T!$A$1:$A$3".to_string())));
    assert!(names.contains(&("Threshold".to_string(), "T!$B$1".to_string())));
}

// ============ Array (CSE) Arithmetic Tests ============

#[test]
fn test_formula_array_sum_products() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("1,10\n2,20\n3,30\n");
    // Classic CSE: element-wise product then aggregate
    assert_eq!(
        evaluator
            .evaluate_formula_full("SUM(A1:A3*B1:B3)", &data)
            .unwrap()
            .to_string(),
        "140"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("SUM(A1:A3+B1:B3)", &data)
            .unwrap()
            .to_string(),
        "66"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("SUM(B1:B3/A1:A3)", &data)
            .unwrap()
            .to_string(),
        "30"
    );
}

#[test]
fn test_formula_array_scalar_broadcast() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("1\n2\n3\n");
    assert_eq!(
        evaluator
            .evaluate_formula_full("SUM(A1:A3*2)", &data)
            .unwrap()
            .to_string(),
        "12"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("AVERAGE(A1:A3*2)", &data)
            .unwrap()
            .to_string(),
        "4"
    );
    // COUNT of an array = number of elements (all are numeric)
    assert_eq!(
        evaluator
            .evaluate_formula_full("COUNT(A1:A3*B1:B3)", &data)
            .unwrap()
            .to_string(),
        "3"
    );
}

#[test]
fn test_formula_array_nested_parens_and_precedence() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("1,10\n2,20\n");
    // (1+10)*2 + (2+20)*2 = 66
    assert_eq!(
        evaluator
            .evaluate_formula_full("SUM((A1:A2+B1:B2)*2)", &data)
            .unwrap()
            .to_string(),
        "66"
    );
    // Precedence: A1:A2*2+B1:B2 = (2+10) + (4+20) = 36
    assert_eq!(
        evaluator
            .evaluate_formula_full("SUM(A1:A2*2+B1:B2)", &data)
            .unwrap()
            .to_string(),
        "36"
    );
}

#[test]
fn test_formula_array_min_max_with_negatives() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("1,10\n2,20\n3,30\n");
    assert_eq!(
        evaluator
            .evaluate_formula_full("MAX(A1:A3-B1:B3)", &data)
            .unwrap()
            .to_string(),
        "-9"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("MIN(A1:A3-B1:B3)", &data)
            .unwrap()
            .to_string(),
        "-27"
    );
}

#[test]
fn test_formula_array_shape_mismatch_errors() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("1,10\n2,20\n3,\n");
    // 2-value range vs 3-value range without broadcasting
    let r = evaluator.evaluate_formula_full("SUM(A1:A2+B1:B3)", &data);
    assert!(r.is_err(), "shape mismatch must error");
}

#[test]
fn test_formula_array_blanks_count_as_zero() {
    let evaluator = FormulaEvaluator::new();
    // A2 blank → 0 in element-wise arithmetic
    let data = parse_csv_data("1,x\n,x\n");
    assert_eq!(
        evaluator
            .evaluate_formula_full("SUM(A1:A2*2)", &data)
            .unwrap()
            .to_string(),
        "2"
    );
}

#[test]
fn test_formula_plain_ranges_still_work() {
    let evaluator = FormulaEvaluator::new();
    let data = parse_csv_data("1,10\n2,20\n3,30\n");
    // No operator → plain range path must be untouched
    assert_eq!(
        evaluator
            .evaluate_formula_full("SUM(A1:A3)", &data)
            .unwrap()
            .to_string(),
        "6"
    );
    assert_eq!(
        evaluator
            .evaluate_formula_full("MAX(B1:B3)", &data)
            .unwrap()
            .to_string(),
        "30"
    );
}
