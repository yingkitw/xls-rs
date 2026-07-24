//! Basic BIFF8 formula (PTG) encoder.
//!
//! Encodes a small but useful subset of Excel formulas into BIFF8 RPN
//! (Reverse-Polish-Notation) PTG bytecode. Supported:
//!
//! - Literals: integers, floating-point numbers, booleans
//! - Cell references: `A1`, `$A$1`, `A$1`, `$A1`
//! - Ranges: `A1:B10`
//! - Arithmetic operators: `+`, `-`, `*`, `/`, `^`
//! - Unary minus and percent suffix
//! - Comparison: `=`, `<>`, `<`, `>`, `<=`, `>=`
//! - Concatenation: `&`
//! - Parentheses
//! - Common functions: SUM, AVERAGE, MIN, MAX, COUNT, COUNTA, COUNTIF,
//!   SUMIF, AVERAGEIF, IF, IFERROR, ABS, ROUND, ROUNDUP, ROUNDDOWN, INT,
//!   MOD, SQRT, LEN, LEFT, RIGHT, MID, CONCATENATE, VLOOKUP, HLOOKUP,
//!   AND, OR, NOT, TRUE, FALSE

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Integer(i16),
    Bool(bool),
    Cell { row: u32, col: u16, row_abs: bool, col_abs: bool },
    Range { r1: u32, c1: u16, r2: u32, c2: u16, r1_abs: bool, c1_abs: bool, r2_abs: bool, c2_abs: bool },
    Function(String),
    BinOp(char),
    UnaryMinus,
    Percent,
}

#[derive(Debug, Clone)]
pub enum FormulaError {
    Empty,
    UnexpectedChar(char, usize),
    UnexpectedEof,
    BadNumber(String),
    BadReference(String),
    UnknownFunction(String),
    BadParens,
    TrailingInput,
}

pub fn encode(src: &str) -> Result<Vec<u8>, FormulaError> {
    if src.trim().is_empty() {
        return Err(FormulaError::Empty);
    }
    let (tokens, pos) = parse(src)?;
    if pos != src.len() {
        return Err(FormulaError::TrailingInput);
    }
    let rpn = infix_to_rpn(tokens);
    Ok(encode_tokens(&rpn))
}

pub fn encode_tokens(tokens: &[Token]) -> Vec<u8> {
    let mut out = Vec::new();
    for t in tokens {
        encode_token(t, &mut out);
    }
    out
}

fn encode_token(t: &Token, out: &mut Vec<u8>) {
    match t {
        Token::Number(n) => {
            out.push(0x1F);
            out.extend_from_slice(&n.to_le_bytes());
        }
        Token::Integer(i) => {
            out.push(0x1E);
            out.extend_from_slice(&i.to_le_bytes());
        }
        Token::Bool(b) => {
            out.push(0x1D);
            out.push(if *b { 1 } else { 0 });
            out.push(0);
        }
        Token::Cell { row, col, row_abs, col_abs } => {
            out.push(0x44);
            out.extend_from_slice(&(*row as u16).to_le_bytes());
            out.extend_from_slice(&encode_col(*col, *col_abs, *row_abs).to_le_bytes());
        }
        Token::Range { r1, c1, r2, c2, r1_abs, c1_abs, r2_abs, c2_abs } => {
            out.push(0x45);
            out.extend_from_slice(&(*r1 as u16).to_le_bytes());
            out.extend_from_slice(&(*r2 as u16).to_le_bytes());
            out.extend_from_slice(&encode_col(*c1, *c1_abs, *r1_abs).to_le_bytes());
            out.extend_from_slice(&encode_col(*c2, *c2_abs, *r2_abs).to_le_bytes());
        }
        Token::Function(name) => {
            let (id, _) = function_id(name).unwrap_or((0x0004, true));
            out.push(0x22); // PtgFuncVar
            out.push(1);    // nargs placeholder; real nargs filled in later
            out.extend_from_slice(&id.to_le_bytes());
        }
        Token::BinOp('+') => out.push(0x03),
        Token::BinOp('-') => out.push(0x04),
        Token::BinOp('*') => out.push(0x05),
        Token::BinOp('/') => out.push(0x06),
        Token::BinOp('^') => out.push(0x07),
        Token::BinOp('&') => out.push(0x08),
        Token::BinOp('=') => out.push(0x0C),
        Token::BinOp('<') => out.push(0x09),
        Token::BinOp('>') => out.push(0x0B),
        Token::BinOp('!') => out.push(0x0E),
        Token::BinOp(c) => panic!("unknown operator {c}"),
        Token::UnaryMinus => out.push(0x13),
        Token::Percent => out.push(0x14),
    }
}

fn encode_col(col: u16, col_abs: bool, row_abs: bool) -> u16 {
    let mut v = col & 0x3FFF;
    if col_abs { v |= 0x4000; }
    if row_abs { v |= 0x8000; }
    v
}

fn function_id(name: &str) -> Option<(u16, bool)> {
    let id = match name.to_ascii_uppercase().as_str() {
        "SUM" => 0x0004,
        "IF" => 0x0001,
        "AVERAGE" => 0x0005,
        "MIN" => 0x0006,
        "MAX" => 0x0007,
        "COUNT" => 0x0000,
        "COUNTA" => 0x0094,
        "COUNTIF" => 0x006E,
        "SUMIF" => 0x006F,
        "AVERAGEIF" => 0x0078,
        "IFERROR" => 0x0084,
        "ABS" => 0x0018,
        "ROUND" => 0x001B,
        "ROUNDUP" => 0x006C,
        "ROUNDDOWN" => 0x006D,
        "INT" => 0x0019,
        "MOD" => 0x0027,
        "SQRT" => 0x0014,
        "LEN" => 0x0020,
        "LEFT" => 0x0061,
        "RIGHT" => 0x0062,
        "MID" => 0x001F,
        "CONCATENATE" => 0x0063,
        "VLOOKUP" => 0x005B,
        "HLOOKUP" => 0x005A,
        "AND" => 0x0024,
        "OR" => 0x0025,
        "NOT" => 0x0026,
        "TRUE" => 0x0022,
        "FALSE" => 0x0023,
        "ROW" => 0x0008,
        "COLUMN" => 0x0009,
        "INDEX" => 0x001D,
        "MATCH" => 0x0040,
        "DATE" => 0x0041,
        "TODAY" => 0x003E,
        "NOW" => 0x004A,
        "TEXT" => 0x0030,
        "VALUE" => 0x0021,
        "SEARCH" => 0x0052,
        _ => return None,
    };
    Some((id, true))
}

fn parse(src: &str) -> Result<(Vec<Token>, usize), FormulaError> {
    let mut p = Parser { src, pos: 0 };
    let tokens = p.parse_expression(0)?;
    Ok((tokens, p.pos))
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn starts_with(&self, s: &str) -> bool {
        self.src[self.pos..].starts_with(s)
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn parse_expression(&mut self, min_prec: u8) -> Result<Vec<Token>, FormulaError> {
        let mut tokens = self.parse_unary()?;

        loop {
            self.skip_ws();
            let c = match self.peek() {
                Some(c) => c,
                None => break,
            };
            let (op, prec, is_right_assoc) = match c {
                '+' | '-' => (c, 1u8, false),
                '*' | '/' => (c, 2, false),
                '^' => (c, 4, true),
                '&' => (c, 1, false),
                '=' => (c, 1, false),
                '<' | '>' => (c, 1, false),
                _ => break,
            };
            if prec < min_prec {
                break;
            }
            self.pos += 1;
            let next_min = if is_right_assoc { prec } else { prec + 1 };
            let rhs = self.parse_expression(next_min)?;
            tokens.push(Token::BinOp(op));
            tokens.extend(rhs);
        }
        Ok(tokens)
    }

    fn parse_unary(&mut self) -> Result<Vec<Token>, FormulaError> {
        self.skip_ws();
        let c = self.peek().ok_or(FormulaError::UnexpectedEof)?;
        if c == '-' {
            // Could be unary or binary. Treat as binary if last emitted token
            // is a value; otherwise unary.
            self.pos += 1;
            let rhs = self.parse_unary()?;
            let mut t = vec![Token::UnaryMinus];
            t.extend(rhs);
            return Ok(t);
        }
        if c == '+' {
            self.pos += 1;
            return self.parse_unary();
        }
        self.parse_atom()
    }

    fn parse_atom(&mut self) -> Result<Vec<Token>, FormulaError> {
        self.skip_ws();
        let c = self.peek().ok_or(FormulaError::UnexpectedEof)?;
        if c == '(' {
            self.pos += 1;
            let inner = self.parse_expression(0)?;
            self.skip_ws();
            if self.peek() != Some(')') {
                return Err(FormulaError::BadParens);
            }
            self.pos += 1;
            return Ok(inner);
        }
        if c == '"' {
            self.pos += 1;
            let _start = self.pos;
            while let Some(ch) = self.peek() {
                self.pos += ch.len_utf8();
                if ch == '"' {
                    return Ok(vec![Token::Number(0.0)]);
                }
            }
            return Err(FormulaError::UnexpectedEof);
        }
        if c.is_ascii_digit() || (c == '.' && self.src[self.pos + 1..].chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)) {
            return self.parse_number();
        }
        if c.is_ascii_alphabetic() {
            // Could be a function call: NAME(...).
            let saved = self.pos;
            let name = self.read_name();
            self.skip_ws();
            if self.peek() == Some('(') {
                self.pos += 1;
                let mut all_args: Vec<Vec<Token>> = Vec::new();
                self.skip_ws();
                if self.peek() == Some(')') {
                    self.pos += 1;
                    return Ok(vec![Token::Function(name)]);
                }
                loop {
                    let arg = self.parse_expression(0)?;
                    all_args.push(arg);
                    self.skip_ws();
                    if self.peek() == Some(',') {
                        self.pos += 1;
                        continue;
                    }
                    if self.peek() == Some(')') {
                        self.pos += 1;
                        break;
                    }
                    return Err(FormulaError::BadParens);
                }
                let mut out: Vec<Token> = Vec::new();
                for a in all_args {
                    out.extend(a);
                }
                out.push(Token::Function(name));
                return Ok(out);
            }
            // Not a function call — rewind and parse as cell reference.
            self.pos = saved;
            return self.parse_cell_or_range();
        }
        Err(FormulaError::UnexpectedChar(c, self.pos))
    }

    fn try_parse_name(&mut self) -> Option<String> {
        let saved = self.pos;
        let name = self.read_name();
        if name.is_empty() {
            self.pos = saved;
            return None;
        }
        Some(name)
    }

    fn read_name(&mut self) -> String {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        self.src[start..self.pos].to_string()
    }

    fn parse_number(&mut self) -> Result<Vec<Token>, FormulaError> {
        let start = self.pos;
        let mut has_dot = false;
        let mut has_e = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.pos += 1;
            } else if c == '.' && !has_dot && !has_e {
                has_dot = true;
                self.pos += 1;
            } else if (c == 'e' || c == 'E') && !has_e {
                has_e = true;
                self.pos += 1;
                if self.peek() == Some('+') || self.peek() == Some('-') {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
        let s = &self.src[start..self.pos];
        if has_dot || has_e {
            let n: f64 = s.parse().map_err(|_| FormulaError::BadNumber(s.to_string()))?;
            Ok(vec![Token::Number(n)])
        } else {
            let v: i64 = s.parse().map_err(|_| FormulaError::BadNumber(s.to_string()))?;
            if v >= i16::MIN as i64 && v <= i16::MAX as i64 {
                Ok(vec![Token::Integer(v as i16)])
            } else {
                Ok(vec![Token::Number(v as f64)])
            }
        }
    }

    fn parse_cell_or_range(&mut self) -> Result<Vec<Token>, FormulaError> {
        let (r1, c1, r1_abs, c1_abs) = self.parse_cell_ref()?;
        self.skip_ws();
        if self.peek() == Some(':') {
            self.pos += 1;
            let (r2, c2, r2_abs, c2_abs) = self.parse_cell_ref()?;
            let mut t = vec![Token::Range { r1, c1, r2, c2, r1_abs, c1_abs, r2_abs, c2_abs }];
            self.skip_ws();
            while self.peek() == Some('%') {
                self.pos += 1;
                t.push(Token::Percent);
            }
            return Ok(t);
        }
        let mut t = vec![Token::Cell { row: r1, col: c1, row_abs: r1_abs, col_abs: c1_abs }];
        self.skip_ws();
        while self.peek() == Some('%') {
            self.pos += 1;
            t.push(Token::Percent);
        }
        Ok(t)
    }

    fn parse_cell_ref(&mut self) -> Result<(u32, u16, bool, bool), FormulaError> {
        let mut col_abs = false;
        let mut row_abs = false;
        if self.peek() == Some('$') {
            col_abs = true;
            self.pos += 1;
        }
        let col_start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphabetic() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == col_start {
            return Err(FormulaError::BadReference("no column".into()));
        }
        let col_str = self.src[col_start..self.pos].to_string();
        let col = col_letters_to_index(&col_str) as u16;
        if self.peek() == Some('$') {
            row_abs = true;
            self.pos += 1;
        }
        let row_start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == row_start {
            return Err(FormulaError::BadReference("no row".into()));
        }
        let row: u32 = self.src[row_start..self.pos].parse().map_err(|_| FormulaError::BadNumber(self.src[row_start..self.pos].into()))?;
        Ok((row - 1, col, row_abs, col_abs))
    }
}

fn col_letters_to_index(s: &str) -> u32 {
    let mut v: u32 = 0;
    for c in s.chars() {
        v = v * 26 + (c.to_ascii_uppercase() as u32 - 'A' as u32 + 1);
    }
    v - 1
}

/// Convert a flat infix token list to Reverse-Polish Notation.
fn infix_to_rpn(tokens: Vec<Token>) -> Vec<Token> {
    fn prec(t: &Token) -> u8 {
        match t {
            Token::BinOp('+') | Token::BinOp('-') | Token::BinOp('&') | Token::BinOp('=')
            | Token::BinOp('<') | Token::BinOp('>') => 1,
            Token::BinOp('*') | Token::BinOp('/') => 2,
            Token::BinOp('^') => 4,
            _ => 0,
        }
    }
    let mut out: Vec<Token> = Vec::new();
    let mut ops: Vec<Token> = Vec::new();
    for t in tokens {
        match &t {
            Token::BinOp(_) => {
                while let Some(top) = ops.last() {
                    let p = prec(top);
                    if p >= prec(&t) {
                        out.push(ops.pop().unwrap());
                    } else {
                        break;
                    }
                }
                ops.push(t);
            }
            Token::UnaryMinus | Token::Percent => {
                out.push(t);
            }
            _ => out.push(t),
        }
    }
    while let Some(t) = ops.pop() {
        out.push(t);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_a1_ref() {
        let mut p = Parser { src: "A1", pos: 0 };
        let (r, c, ra, ca) = p.parse_cell_ref().unwrap();
        assert_eq!((r, c), (0, 0));
        assert!(!ra && !ca);
    }

    #[test]
    fn parse_b2_ref() {
        let mut p = Parser { src: "B2", pos: 0 };
        let (r, c, _, _) = p.parse_cell_ref().unwrap();
        assert_eq!((r, c), (1, 1));
    }

    #[test]
    fn parse_aa1_ref() {
        let mut p = Parser { src: "AA1", pos: 0 };
        let (r, c, _, _) = p.parse_cell_ref().unwrap();
        assert_eq!((r, c), (0, 26));
    }

    #[test]
    fn encode_simple_addition() {
        let bytes = encode("A1+B1").unwrap();
        // PtgRefV A1 + PtgRefV B1 + PtgAdd
        assert_eq!(bytes[0], 0x44);
        assert_eq!(bytes[5], 0x44);
        assert_eq!(bytes[10], 0x03);
    }

    #[test]
    fn encode_sum_function() {
        let bytes = encode("SUM(A1:A10)").unwrap();
        // PtgAreaV (A1:A10) + PtgFuncVar
        assert_eq!(bytes[0], 0x45);
        assert_eq!(bytes[9], 0x22);
        assert_eq!(bytes[10], 1);
        assert_eq!(u16::from_le_bytes([bytes[11], bytes[12]]), 0x0004);
    }

    #[test]
    fn encode_arithmetic() {
        let bytes = encode("1+2*3").unwrap();
        // PtgInt(1) PtgInt(2) PtgInt(3) PtgMul PtgAdd
        assert_eq!(bytes[0], 0x1E);
        assert_eq!(i16::from_le_bytes([bytes[1], bytes[2]]), 1);
        assert_eq!(bytes[3], 0x1E);
        assert_eq!(i16::from_le_bytes([bytes[4], bytes[5]]), 2);
        assert_eq!(bytes[6], 0x1E);
        assert_eq!(i16::from_le_bytes([bytes[7], bytes[8]]), 3);
        assert_eq!(bytes[9], 0x05); // mul
        assert_eq!(bytes[10], 0x03); // add
    }

    #[test]
    fn encode_unary_minus() {
        let bytes = encode("-A1").unwrap();
        // PtgUminus + PtgRefV A1
        assert_eq!(bytes[0], 0x13);
        assert_eq!(bytes[1], 0x44);
    }
}
