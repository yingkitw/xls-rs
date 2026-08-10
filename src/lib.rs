//! xls-rs - A pure-Rust XLSX toolkit
//!
//! Read, write, and manipulate Excel XLSX files with charts, styles,
//! conditional formatting, and formula evaluation.

#![allow(dead_code)]

pub mod common;
pub mod config;
pub mod converter;
pub mod error;
pub mod error_traits;
pub mod excel;
pub mod format_detector;
pub mod formula;
pub mod handler_registry;
pub mod helpers;
pub mod limits;
pub mod operations;
pub mod profiling;
pub mod quality;
pub mod regex_cache;
pub mod string_distance;
pub mod string_utils;
pub mod traits;
pub mod types;
pub mod validation;

pub use config::Config;
pub use converter::Converter;
pub use error::{ErrorContext, ErrorKind, ResultExt, XlsRsError, XlsRsResult};
pub use error_traits::{
    ErrorCategory, ErrorCategoryType, ErrorContextProvider, ErrorSeverity, RecoverableError,
    ToTraitBasedError, TraitBasedError, UserFriendlyError,
};
pub use excel::{
    add_cell_to_row, add_cells_to_row, classify_cell, CellComment, CellData, CellStyle,
    ChartConfig, ColGroup, ConditionalFormat, ConditionalRule, DataChartType, DataValidation,
    ExcelHandler, FeatureDetector, FeatureSeverity, Hyperlink, MergeCell, Operator, PageMargins,
    PageOrientation, PlaceholderInfo, PrintSetup, RowData, RowGroup, SharedStrings, Sparkline,
    SparklineGroup, SparklineType, StreamingXlsxWriter, StyleRegistry, Table, TableStyleInfo,
    TemplateData, TemplateFiller, TemplateReader, UnsupportedFeature, ValidationType, WriteMode,
    WriteOptions, XlsxCellStyle, XlsxStyleTable, XlsxWriter,
    XlsxStreamingReader, XlsxRowIterator,
};
pub use format_detector::DefaultFormatDetector;
pub use formula::{FormulaEvaluator, FormulaResult};
pub use handler_registry::HandlerRegistry;
pub use helpers::{
    default_column_names, filter_by_range, matches_extension, max_column_count,
    parse_safe_f64, parse_safe_i64, parse_safe_usize,
    with_cell_context, with_file_context, with_full_context,
    validate_row_index, validate_column_index,
};
pub use operations::{
    AggFunc, DataOperations, JoinType, NoProgress, ProgressCallback, SortOrder, StderrProgress,
};
pub use profiling::{ColumnProfile, DataProfile, DataProfiler};
pub use quality::{IssueSeverity, QualityIssue, QualityReport, QualityReportGenerator};
pub use string_distance::{hamming, jaro, jaro_winkler, levenshtein};
pub use string_utils::{
    join_cell_reference, join_with_capacity, string_with_capacity, StringBuilder,
};
pub use traits::{
    CellRangeProvider, DataOperator, DataReader, DataWriteOptions, DataWriter, FileHandler,
    FilterCondition, FilterOperator, FormatDetector, SchemaProvider, SortOperator,
    TransformOperation, TransformOperator,
};
pub use types::{CellValue, DataSet, DataType, DataRow};
pub use validation::{DataValidator, ValidationConfig, ValidationResult, ValidationRule};
