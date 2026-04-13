//! Histogram operation for numeric column distribution
//!
//! Produces ASCII histogram for quick terminal-based EDA.

use anyhow::Result;

/// Compute histogram bins for numeric values
///
/// Returns (bins, labels) where bins[i] is count for range [labels[i], labels[i+1])
pub fn histogram(
    data: &[Vec<String>],
    col_idx: usize,
    num_bins: usize,
) -> Result<Vec<(f64, f64, usize)>> {
    if data.len() < 2 {
        return Ok(Vec::new());
    }

    let values: Vec<f64> = data[1..]
        .iter()
        .filter_map(|row| row.get(col_idx).and_then(|s| s.parse::<f64>().ok()))
        .collect();

    if values.is_empty() {
        return Ok(Vec::new());
    }

    let min_val = values
        .iter()
        .cloned()
        .fold(f64::INFINITY, |a, b| a.min(b));
    let max_val = values
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, |a, b| a.max(b));

    if min_val >= max_val {
        return Ok(vec![(min_val, max_val, values.len())]);
    }

    let bin_width = (max_val - min_val) / num_bins as f64;
    let mut bins = vec![0usize; num_bins];

    for v in values {
        let bin_idx = if v >= max_val {
            num_bins - 1
        } else {
            ((v - min_val) / bin_width).floor() as usize
        };
        bins[bin_idx.min(num_bins - 1)] += 1;
    }

    let result: Vec<(f64, f64, usize)> = bins
        .into_iter()
        .enumerate()
        .map(|(i, count)| {
            let lo = min_val + i as f64 * bin_width;
            let hi = min_val + (i + 1) as f64 * bin_width;
            (lo, hi, count)
        })
        .collect();

    Ok(result)
}

/// Render ASCII histogram to string
pub fn render_histogram(
    bins: &[(f64, f64, usize)],
    width: usize,
    show_labels: bool,
) -> String {
    if bins.is_empty() {
        return "No numeric data".to_string();
    }

    let max_count = bins.iter().map(|(_, _, c)| *c).max().unwrap_or(1);
    let mut lines = Vec::new();

    for (lo, hi, count) in bins {
        let bar_len = if max_count > 0 {
            (count * width / max_count).max(0)
        } else {
            0
        };
        let bar = "█".repeat(bar_len);
        let label = if show_labels {
            format!(" [{:.2}-{:.2}]", lo, hi)
        } else {
            String::new()
        };
        lines.push(format!("{:>6} |{}{}", count, bar, label));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram_basic() {
        let data = vec![
            vec!["x".into()],
            vec!["1".into()],
            vec!["2".into()],
            vec!["3".into()],
            vec!["4".into()],
            vec!["5".into()],
        ];
        let bins = histogram(&data, 0, 5).unwrap();
        assert_eq!(bins.len(), 5);
        assert!(bins.iter().map(|(_, _, c)| c).sum::<usize>() == 5);
    }

    #[test]
    fn test_histogram_empty() {
        let data = vec![vec!["x".into()]];
        let bins = histogram(&data, 0, 5).unwrap();
        assert!(bins.is_empty());
    }
}
