//! xls-rs - A library for reading, writing, and converting spreadsheet files
//!
//! Supports CSV, Excel (xlsx/xls), ODS, Parquet, and Avro formats with formula evaluation.

// Library crate: public items are the API surface, not "dead code" even if
// not called internally. Suppressing here is standard for library crates.
#![allow(dead_code)]

pub mod anomaly;
pub mod capabilities;
pub mod columnar;
pub mod common;
pub mod config;
pub mod converter;
pub mod csv_handler;
pub mod encryption;
pub mod error;
pub mod error_traits;
pub mod excel;
pub mod format_detector;
pub mod formula;
pub mod geospatial;
#[cfg(feature = "gsheets")]
pub mod google_sheets;
pub mod handler_registry;
pub mod helpers;
pub mod limits;
#[cfg(feature = "mcp")]
mod mcp_enrichment;
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod operations;
pub mod plugins;
pub mod profiling;
pub mod quality;
pub mod regex_cache;
pub mod streaming;
pub mod streaming_ops;
pub mod string_distance;
pub mod string_utils;
pub mod timeseries;
pub mod traits;
pub mod types;
pub mod validation;
pub mod workflow;
pub mod capability_catalog;

pub use anomaly::{Anomaly, AnomalyDetector, AnomalyMethod, AnomalyResult};
#[cfg(feature = "avro")]
pub use columnar::AvroHandler;
#[cfg(feature = "parquet")]
pub use columnar::ParquetHandler;
pub use config::Config;
pub use converter::Converter;
pub use csv_handler::{
    CellRange, CellRangeHelper, CsvHandler, StreamingCsvReader, StreamingCsvWriter,
    sanitize_csv_value, sanitize_csv_row,
};
pub use encryption::{DataEncryptor, EncryptionAlgorithm};
pub use error::{ErrorContext, ErrorKind, ResultExt, XlsRsError, XlsRsResult};
pub use error_traits::{
    ErrorCategory, ErrorCategoryType, ErrorContextProvider, ErrorSeverity, RecoverableError,
    ToTraitBasedError, TraitBasedError, UserFriendlyError,
};
pub use excel::{
    add_cell_to_row, add_cells_to_row, classify_cell, CellComment, CellData, CellStyle,
    ChartConfig, ColGroup, ConditionalFormat, ConditionalRule, DataChartType, DataValidation,
    ExcelHandler, FeatureDetector, FeatureSeverity, Hyperlink, MergeCell, PageMargins,
    PageOrientation, PlaceholderInfo, PrintSetup, RowData, RowGroup, Sparkline, SparklineGroup,
    SparklineType, StreamingXlsxWriter, TemplateData, TemplateFiller, TemplateReader,
    UnsupportedFeature, ValidationType, WriteMode, WriteOptions, XlsRowData, XlsSheetData, XlsWriter,
    XlsxWriter,
};
pub use format_detector::DefaultFormatDetector;
pub use formula::{FormulaEvaluator, FormulaResult};
pub use geospatial::{Coordinate, GeospatialCalculator};
#[cfg(feature = "gsheets")]
pub use google_sheets::GoogleSheetsHandler;
pub use handler_registry::HandlerRegistry;
pub use helpers::{
    default_column_names, filter_by_range, matches_extension, max_column_count,
    parse_safe_f64, parse_safe_i64, parse_safe_usize,
    with_cell_context, with_file_context, with_full_context,
    validate_row_index, validate_column_index,
};
#[cfg(feature = "mcp")]
pub use mcp::XlsRsMcpServer;
pub use operations::{
    AggFunc, DataOperations, JoinType, NoProgress, ProgressCallback, SortOrder, StderrProgress,
};
pub use plugins::{FunctionMetadata, PluginFunction, PluginMetadata, PluginRegistry};
pub use profiling::{ColumnProfile, DataProfile, DataProfiler};
pub use quality::{IssueSeverity, QualityIssue, QualityReport, QualityReportGenerator};
pub use streaming::{
    CsvStreamingReader, DataChunk, StreamingDataReader, StreamingDataWriter,
    StreamingProcessor,
};
pub use streaming_ops::{get_info, head, infer_schema, tail, ColumnType, Schema};
pub use string_distance::{hamming, jaro, jaro_winkler, levenshtein};
pub use string_utils::{
    join_cell_reference, join_with_capacity, string_with_capacity, StringBuilder,
    estimate_csv_row_capacity, estimate_json_array_capacity,
};
pub use timeseries::{
    ResampleInterval, RollingWindow, TimeSeriesAgg, TimeSeriesPoint, TimeSeriesProcessor,
    TrendDirection,
};
pub use traits::{
    CellRangeProvider, DataOperator, DataReader, DataWriteOptions, DataWriter, FileHandler,
    FilterCondition, FilterOperator, FormatDetector, SchemaProvider, SortOperator, StreamingReader,
    StreamingWriter, TransformOperation, TransformOperator,
};
pub use types::{CellValue, DataSet, DataType, DataRow};
pub use validation::{DataValidator, ValidationConfig, ValidationResult, ValidationRule};
pub use workflow::{WorkflowConfig, WorkflowExecutor, WorkflowStep};
