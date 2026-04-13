//! Cached regex patterns for performance
//!
//! This module pre-compiles and caches frequently used regex patterns
//! to avoid repeated compilation overhead.

use std::sync::OnceLock;
use regex::Regex;

/// Cell reference regex (A1, B2, AA10, etc.)
pub fn cell_reference_regex() -> &'static Regex {
    static CELL_REF: OnceLock<Regex> = OnceLock::new();
    CELL_REF.get_or_init(|| {
        Regex::new(r#"([A-Za-z]+)(\d+)"#).expect("Invalid cell reference regex")
    })
}

/// WHERE clause parsing regex
pub fn where_clause_regex() -> &'static Regex {
    static WHERE_CLAUSE: OnceLock<Regex> = OnceLock::new();
    WHERE_CLAUSE.get_or_init(|| {
        Regex::new(
            r#"(\w+)\s*(>=|<=|!=|<>|=|>|<|contains|starts_with|ends_with)\s*['"]?([^'"]+)['"]?"#
        ).expect("Invalid where clause regex")
    })
}

/// Email validation regex
pub fn email_regex() -> &'static Regex {
    static EMAIL: OnceLock<Regex> = OnceLock::new();
    EMAIL.get_or_init(|| {
        Regex::new(
            r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
        ).expect("Invalid email regex")
    })
}

/// URL validation regex
pub fn url_regex() -> &'static Regex {
    static URL: OnceLock<Regex> = OnceLock::new();
    URL.get_or_init(|| {
        Regex::new(
            r"^https?://[^\s/$.?#].[^\s]*$"
        ).expect("Invalid URL regex")
    })
}

/// Numeric regex (matches integers and floats)
pub fn numeric_regex() -> &'static Regex {
    static NUMERIC: OnceLock<Regex> = OnceLock::new();
    NUMERIC.get_or_init(|| {
        Regex::new(r"^-?\d+\.?\d*$").expect("Invalid numeric regex")
    })
}

/// Date format detection regex (YYYY-MM-DD, DD/MM/YYYY, etc.)
pub fn date_regex() -> &'static Regex {
    static DATE: OnceLock<Regex> = OnceLock::new();
    DATE.get_or_init(|| {
        Regex::new(
            r"^\d{4}-\d{2}-\d{2}$|^\d{2}/\d{2}/\d{4}$|^\d{2}-\d{2}-\d{4}$"
        ).expect("Invalid date regex")
    })
}

/// Phone number regex (flexible international format)
pub fn phone_regex() -> &'static Regex {
    static PHONE: OnceLock<Regex> = OnceLock::new();
    PHONE.get_or_init(|| {
        Regex::new(
            r"^[\d\s\-\+\(\)]{7,20}$"
        ).expect("Invalid phone regex")
    })
}

/// UUID regex
pub fn uuid_regex() -> &'static Regex {
    static UUID: OnceLock<Regex> = OnceLock::new();
    UUID.get_or_init(|| {
        Regex::new(
            r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"
        ).expect("Invalid UUID regex")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_reference_regex() {
        let re = cell_reference_regex();
        assert!(re.is_match("A1"));
        assert!(re.is_match("B2"));
        assert!(re.is_match("AA10"));
        assert!(re.is_match("Z100"));

        let caps = re.captures("A1").unwrap();
        assert_eq!(&caps[1], "A");
        assert_eq!(&caps[2], "1");

        let caps = re.captures("AB12").unwrap();
        assert_eq!(&caps[1], "AB");
        assert_eq!(&caps[2], "12");
    }

    #[test]
    fn test_where_clause_regex() {
        let re = where_clause_regex();
        assert!(re.is_match("age >= 18"));
        assert!(re.is_match("name = 'John'"));
        assert!(re.is_match("status != \"active\""));

        let caps = re.captures("age >= 18").unwrap();
        assert_eq!(&caps[1], "age");
        assert_eq!(&caps[2], ">=");
        assert_eq!(&caps[3], "18");
    }

    #[test]
    fn test_email_regex() {
        let re = email_regex();
        assert!(re.is_match("test@example.com"));
        assert!(re.is_match("user.name+tag@domain.co.uk"));
        assert!(!re.is_match("invalid.email"));
        assert!(!re.is_match("@example.com"));
    }

    #[test]
    fn test_url_regex() {
        let re = url_regex();
        assert!(re.is_match("https://example.com"));
        assert!(re.is_match("http://test.org/path"));
        assert!(!re.is_match("not a url"));
        assert!(!re.is_match("ftp://example.com"));
    }

    #[test]
    fn test_numeric_regex() {
        let re = numeric_regex();
        assert!(re.is_match("123"));
        assert!(re.is_match("-456"));
        assert!(re.is_match("123.45"));
        assert!(re.is_match("-789.01"));
        assert!(!re.is_match("12.34.56"));
        assert!(!re.is_match("abc"));
    }

    #[test]
    fn test_date_regex() {
        let re = date_regex();
        assert!(re.is_match("2023-12-25"));
        assert!(re.is_match("25/12/2023"));
        assert!(re.is_match("25-12-2023"));
        assert!(!re.is_match("2023/12/25"));
        // Note: The regex is permissive and matches MM-DD-YYYY format
        // This is acceptable for general date detection
        assert!(re.is_match("12-25-2023"));
    }

    #[test]
    fn test_uuid_regex() {
        let re = uuid_regex();
        assert!(re.is_match("550e8400-e29b-41d4-a716-446655440000"));
        assert!(re.is_match("6ba7b810-9dad-11d1-80b4-00c04fd430c8"));
        assert!(!re.is_match("not-a-uuid"));
        assert!(!re.is_match("550e8400-e29b-41d4-a716"));
    }
}
