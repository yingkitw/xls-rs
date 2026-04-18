//! Tests for time series operations

use chrono::{Datelike, Duration, NaiveDate, Timelike};
use xls_rs::{
    ResampleInterval, RollingWindow, TimeSeriesAgg, TimeSeriesPoint, TimeSeriesProcessor,
    TrendDirection,
};

fn create_test_data() -> Vec<TimeSeriesPoint> {
    (0..30)
        .map(|i| TimeSeriesPoint {
            timestamp: NaiveDate::from_ymd_opt(2023, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                + Duration::days(i),
            value: (i * 10) as f64,
        })
        .collect()
}

fn create_seasonal_data() -> Vec<TimeSeriesPoint> {
    // Create data with clear monthly seasonality
    (0..365)
        .map(|i| {
            let day_of_year = i % 365;
            let seasonal_component = ((day_of_year as f64 / 365.0) * 2.0 * std::f64::consts::PI).sin() * 100.0;
            TimeSeriesPoint {
                timestamp: NaiveDate::from_ymd_opt(2023, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    + Duration::days(i as i64),
                value: 50.0 + seasonal_component + (i as f64 * 0.1), // Trend + seasonality
            }
        })
        .collect()
}

#[test]
fn test_parse_date_standard_format() {
    let processor = TimeSeriesProcessor::new("%Y-%m-%d");

    let date = processor.parse_date("2023-06-15").unwrap();
    assert_eq!(date.year(), 2023);
    assert_eq!(date.month(), 6);
    assert_eq!(date.day(), 15);
}

#[test]
fn test_parse_date_alternative_formats() {
    let processor = TimeSeriesProcessor::new("%Y-%m-%d");

    // European format
    let date = processor.parse_date("15/06/2023").unwrap();
    assert_eq!(date.year(), 2023);
    assert_eq!(date.month(), 6);

    // US format
    let date = processor.parse_date("06/15/2023").unwrap();
    assert_eq!(date.year(), 2023);
    assert_eq!(date.month(), 6);
}

#[test]
fn test_csv_to_timeseries() {
    let processor = TimeSeriesProcessor::new("%Y-%m-%d");

    let csv_data = vec![
        vec!["date".to_string(), "value".to_string()],
        vec!["2023-01-01".to_string(), "100".to_string()],
        vec!["2023-01-02".to_string(), "200".to_string()],
        vec!["2023-01-03".to_string(), "300".to_string()],
    ];

    let ts = processor.csv_to_timeseries(&csv_data, 0, 1).unwrap();

    assert_eq!(ts.len(), 3);
    assert_eq!(ts[0].value, 100.0);
    assert_eq!(ts[1].value, 200.0);
    assert_eq!(ts[2].value, 300.0);
}

#[test]
fn test_csv_to_timeseries_invalid_number() {
    let processor = TimeSeriesProcessor::new("%Y-%m-%d");

    let csv_data = vec![
        vec!["date".to_string(), "value".to_string()],
        vec!["2023-01-01".to_string(), "invalid".to_string()],
    ];

    let result = processor.csv_to_timeseries(&csv_data, 0, 1);
    assert!(result.is_err());
}

#[test]
fn test_resample_daily_to_weekly() {
    let processor = TimeSeriesProcessor::new("%Y-%m-%d");
    let data = create_test_data();

    let resampled = processor
        .resample(&data, &ResampleInterval::Weekly, &TimeSeriesAgg::Mean)
        .unwrap();

    // 30 days should produce about 5 weeks
    assert!(resampled.len() >= 4 && resampled.len() <= 6);
}

#[test]
fn test_resample_daily_to_monthly() {
    let processor = TimeSeriesProcessor::new("%Y-%m-%d");
    let data = create_test_data();

    let resampled = processor
        .resample(&data, &ResampleInterval::Monthly, &TimeSeriesAgg::Sum)
        .unwrap();

    // January 2023 should have one data point
    assert!(!resampled.is_empty());
}

#[test]
fn test_resample_aggregation_functions() {
    let processor = TimeSeriesProcessor::new("%Y-%m-%d");

    // Create data within a single week
    let data: Vec<TimeSeriesPoint> = (0..7)
        .map(|i| TimeSeriesPoint {
            timestamp: NaiveDate::from_ymd_opt(2023, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                + Duration::days(i),
            value: (i + 1) as f64 * 10.0, // 10, 20, 30, ..., 70
        })
        .collect();

    // Test Sum aggregation
    let sum_result = processor
        .resample(&data, &ResampleInterval::Weekly, &TimeSeriesAgg::Sum)
        .unwrap();
    assert_eq!(sum_result[0].value, 280.0); // 10+20+30+40+50+60+70

    // Test Mean aggregation
    let mean_result = processor
        .resample(&data, &ResampleInterval::Weekly, &TimeSeriesAgg::Mean)
        .unwrap();
    assert_eq!(mean_result[0].value, 40.0); // 280/7

    // Test Min aggregation
    let min_result = processor
        .resample(&data, &ResampleInterval::Weekly, &TimeSeriesAgg::Min)
        .unwrap();
    assert_eq!(min_result[0].value, 10.0);

    // Test Max aggregation
    let max_result = processor
        .resample(&data, &ResampleInterval::Weekly, &TimeSeriesAgg::Max)
        .unwrap();
    assert_eq!(max_result[0].value, 70.0);
}

#[test]
fn test_detect_trend_increasing() {
    let processor = TimeSeriesProcessor::new("%Y-%m-%d");

    let data: Vec<TimeSeriesPoint> = (0..20)
        .map(|i| TimeSeriesPoint {
            timestamp: NaiveDate::from_ymd_opt(2023, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                + Duration::days(i),
            value: i as f64 * 10.0,
        })
        .collect();

    let trend = processor.detect_trend(&data);
    assert!(matches!(trend, TrendDirection::Increasing));
}

#[test]
fn test_detect_trend_decreasing() {
    let processor = TimeSeriesProcessor::new("%Y-%m-%d");

    let data: Vec<TimeSeriesPoint> = (0..20)
        .map(|i| TimeSeriesPoint {
            timestamp: NaiveDate::from_ymd_opt(2023, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                + Duration::days(i),
            value: (20 - i) as f64 * 10.0,
        })
        .collect();

    let trend = processor.detect_trend(&data);
    assert!(matches!(trend, TrendDirection::Decreasing));
}

#[test]
fn test_detect_trend_stationary() {
    let processor = TimeSeriesProcessor::new("%Y-%m-%d");

    let data: Vec<TimeSeriesPoint> = (0..20)
        .map(|i| TimeSeriesPoint {
            timestamp: NaiveDate::from_ymd_opt(2023, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                + Duration::days(i),
            value: 100.0 + (i as f64 * 0.0001), // Very slight increase, effectively stationary
        })
        .collect();

    let trend = processor.detect_trend(&data);
    assert!(matches!(trend, TrendDirection::Stationary));
}

#[test]
fn test_calculate_stats() {
    let processor = TimeSeriesProcessor::new("%Y-%m-%d");
    let data = create_test_data();

    let stats = processor.calculate_stats(&data).unwrap();

    assert_eq!(stats.total_points, 30);
    assert!(matches!(stats.trend_direction, TrendDirection::Increasing));
}

#[test]
fn test_seasonality_detection() {
    let processor = TimeSeriesProcessor::new("%Y-%m-%d");
    let seasonal_data = create_seasonal_data();

    let stats = processor.calculate_stats(&seasonal_data).unwrap();

    assert!(stats.seasonality_detected);
}

#[test]
fn test_rolling_mean() {
    let processor = TimeSeriesProcessor::new("%Y-%m-%d");

    let data: Vec<TimeSeriesPoint> = (0..10)
        .map(|i| TimeSeriesPoint {
            timestamp: NaiveDate::from_ymd_opt(2023, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                + Duration::days(i),
            value: (i + 1) as f64 * 10.0,
        })
        .collect();

    let window = RollingWindow {
        window_size: Duration::days(3),
        min_periods: 1,
        center: false,
    };

    let rolling = processor.rolling_mean(&data, &window).unwrap();

    assert!(!rolling.is_empty());
    // First value should be just the first point
    assert_eq!(rolling[0].value, 10.0);
}

#[test]
fn test_timeseries_to_csv() {
    let processor = TimeSeriesProcessor::new("%Y-%m-%d");

    let data: Vec<TimeSeriesPoint> = vec![
        TimeSeriesPoint {
            timestamp: NaiveDate::from_ymd_opt(2023, 6, 15)
                .unwrap()
                .and_hms_opt(14, 30, 0)
                .unwrap(),
            value: 123.45,
        },
    ];

    let csv = processor.timeseries_to_csv(&data);

    assert_eq!(csv.len(), 2); // Header + 1 data row
    assert_eq!(csv[0], vec!["timestamp", "value"]);
    assert!(csv[1][0].contains("2023-06-15"));
    assert!(csv[1][1].contains("123.45"));
}

#[test]
fn test_empty_data() {
    let processor = TimeSeriesProcessor::new("%Y-%m-%d");

    let empty: Vec<TimeSeriesPoint> = vec![];

    let resampled = processor
        .resample(&empty, &ResampleInterval::Daily, &TimeSeriesAgg::Mean)
        .unwrap();
    assert!(resampled.is_empty());

    let stats_result = processor.calculate_stats(&empty);
    assert!(stats_result.is_err());

    let trend = processor.detect_trend(&empty);
    assert!(matches!(trend, TrendDirection::Unknown));
}
