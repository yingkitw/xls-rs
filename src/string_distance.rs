//! String distance metrics for fuzzy matching
//!
//! Provides Levenshtein, Jaro-Winkler, and Hamming distance/similarity functions.

/// Levenshtein edit distance between two strings.
///
/// Counts the minimum number of single-character insertions, deletions,
/// and substitutions required to transform `a` into `b`.
pub fn levenshtein(a: &str, b: &str) -> usize {
    // Cap length to bound O(|a|×|b|) DP memory on pathological inputs.
    let a: Vec<char> = a.chars().take(crate::limits::MAX_STRING_DISTANCE_CHARS).collect();
    let b: Vec<char> = b.chars().take(crate::limits::MAX_STRING_DISTANCE_CHARS).collect();
    let (m, n) = (a.len(), b.len());

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr: Vec<usize> = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Jaro similarity score between two strings (0.0 = no similarity, 1.0 = identical).
pub fn jaro(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().take(crate::limits::MAX_STRING_DISTANCE_CHARS).collect();
    let b: Vec<char> = b.chars().take(crate::limits::MAX_STRING_DISTANCE_CHARS).collect();
    let (m, n) = (a.len(), b.len());

    if m == 0 && n == 0 {
        return 1.0;
    }
    if m == 0 || n == 0 {
        return 0.0;
    }

    let match_distance = (m.max(n) / 2).saturating_sub(1);
    let mut a_matches = vec![false; m];
    let mut b_matches = vec![false; n];
    let mut matches = 0usize;

    for i in 0..m {
        let start = i.saturating_sub(match_distance);
        let end = (i + match_distance + 1).min(n);
        for j in start..end {
            if b_matches[j] {
                continue;
            }
            if a[i] != b[j] {
                continue;
            }
            a_matches[i] = true;
            b_matches[j] = true;
            matches += 1;
            break;
        }
    }

    if matches == 0 {
        return 0.0;
    }

    let mut transpositions = 0usize;
    let mut k = 0usize;
    for i in 0..m {
        if !a_matches[i] {
            continue;
        }
        while !b_matches[k] {
            k += 1;
        }
        if a[i] != b[k] {
            transpositions += 1;
        }
        k += 1;
    }
    transpositions /= 2;

    let mf = matches as f64;
    (mf / m as f64 + mf / n as f64 + (mf - transpositions as f64) / mf) / 3.0
}

/// Jaro-Winkler similarity score (boosts strings that share a common prefix).
///
/// `p` is the prefix scaling factor (typically 0.1), `max_prefix` is the maximum
/// common prefix length to consider (typically 4).
pub fn jaro_winkler(a: &str, b: &str) -> f64 {
    let jaro_sim = jaro(a, b);
    let prefix_len = a
        .chars()
        .zip(b.chars())
        .take(4)
        .take_while(|(x, y)| x == y)
        .count();
    jaro_sim + 0.1 * prefix_len as f64 * (1.0 - jaro_sim)
}

/// Hamming distance between two strings of equal length.
///
/// Returns `None` if the strings have different lengths.
pub fn hamming(a: &str, b: &str) -> Option<usize> {
    let a: Vec<char> = a.chars().take(crate::limits::MAX_STRING_DISTANCE_CHARS).collect();
    let b: Vec<char> = b.chars().take(crate::limits::MAX_STRING_DISTANCE_CHARS).collect();
    if a.len() != b.len() {
        return None;
    }
    Some(a.iter().zip(b.iter()).filter(|(x, y)| x != y).count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("flaw", "lawn"), 2);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", ""), 3);
    }

    #[test]
    fn test_jaro() {
        assert!((jaro("", "") - 1.0).abs() < 1e-9);
        assert!((jaro("abc", "abc") - 1.0).abs() < 1e-9);
        assert!((jaro("MARTHA", "MARHTA") - 0.9444).abs() < 0.001);
        assert!((jaro("DIXON", "DICKSONX") - 0.7666).abs() < 0.001);
    }

    #[test]
    fn test_jaro_winkler() {
        assert!((jaro_winkler("", "") - 1.0).abs() < 1e-9);
        assert!((jaro_winkler("abc", "abc") - 1.0).abs() < 1e-9);
        assert!((jaro_winkler("MARTHA", "MARHTA") - 0.9611).abs() < 0.001);
    }

    #[test]
    fn test_hamming() {
        assert_eq!(hamming("", ""), Some(0));
        assert_eq!(hamming("abc", "abc"), Some(0));
        assert_eq!(hamming("toned", "roses"), Some(3));
        assert_eq!(hamming("abc", "abcd"), None);
    }
}
