//! xls-rs - A library for reading, writing, and converting spreadsheet files
//!
//! Supports CSV, Excel (xlsx/xls), ODS, Parquet, and Avro formats with formula evaluation.

#![allow(dead_code)] // Library exports many public APIs not used internally

pub mod anomaly;
pub mod api;
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
pub mod google_sheets;
pub mod handler_registry;
pub mod helpers;
pub mod lineage;
pub mod mcp;
#[cfg(test)]
pub mod mocks;
pub mod operations;
pub mod plugins;
pub mod profiling;
pub mod profiling_handler;
pub mod quality;
pub mod regex_cache;
pub mod streaming;
pub mod streaming_ops;
pub mod string_utils;
pub mod text_analysis;
pub mod text_analysis_handler;
pub mod timeseries;
pub mod traits;
pub mod types;
pub mod validation;
pub mod workflow;
pub mod capability_catalog;

pub use anomaly::{Anomaly, AnomalyDetector, AnomalyMethod, AnomalyResult};
pub use api::{ApiConfig, ApiRequest, ApiResponse, ApiServer};
pub use columnar::{AvroHandler, ParquetHandler};
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
    add_cell_to_row, add_cells_to_row, classify_cell, CellData, CellStyle, ChartConfig,
    ConditionalFormat, ConditionalRule, DataChartType, ExcelHandler, FeatureDetector,
    FeatureSeverity, RowData, Sparkline, SparklineGroup, SparklineType, StreamingXlsxWriter,
    UnsupportedFeature, WriteMode, WriteOptions, XlsxWriter,
};
pub use format_detector::DefaultFormatDetector;
pub use formula::{FormulaEvaluator, FormulaResult};
pub use geospatial::{Coordinate, GeospatialCalculator};
pub use google_sheets::GoogleSheetsHandler;
pub use handler_registry::HandlerRegistry;
pub use helpers::{
    default_column_names, filter_by_range, matches_extension, max_column_count,
    parse_safe_f64, parse_safe_i64, parse_safe_usize,
    with_cell_context, with_file_context, with_full_context,
    validate_row_index, validate_column_index,
};
pub use lineage::{LineageNode, LineageTracker};
pub use mcp::XlsRsMcpServer;
pub use operations::{
    AggFunc, DataOperations, JoinType, NoProgress, ProgressCallback, SortOrder, StderrProgress,
};
pub use plugins::{FunctionMetadata, PluginFunction, PluginMetadata, PluginRegistry};
pub use profiling::{ColumnProfile, DataProfile, DataProfiler};
pub use quality::{IssueSeverity, QualityIssue, QualityReport, QualityReportGenerator};
pub use streaming::{
    DataChunk, StreamingChannel, StreamingDataReader, StreamingDataWriter, StreamingProcessor,
};
pub use streaming_ops::{get_info, head, infer_schema, tail, ColumnType, Schema};
pub use string_utils::{
    join_cell_reference, join_with_capacity, string_with_capacity, StringBuilder,
    estimate_csv_row_capacity, estimate_json_array_capacity,
};
pub use text_analysis::{KeywordResult, LanguageResult, SentimentResult, TextAnalyzer, TextStats};
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
