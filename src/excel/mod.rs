//! Excel XLSX file handling module

mod cell_typer;
pub mod chart;
pub mod feature_detector;
pub mod reader;
pub mod types;
mod writer;
pub mod xlsx_writer;
pub mod xlsx_reader;
pub mod xlsx_streaming_reader;
pub mod xlsx_style_reader;
pub mod template;

// Public API exports - unused internally but part of library interface
#[allow(unused_imports)]
pub use cell_typer::{add_cell_to_row, add_cells_to_row, classify_cell};
pub use chart::{ChartConfig, DataChartType};
pub use feature_detector::{FeatureDetector, FeatureSeverity, UnsupportedFeature};
pub use reader::ExcelHandler;
pub use writer::WriteMode;
#[allow(unused_imports)]
pub use types::{CellStyle, WriteOptions};
pub use xlsx_writer::{
    CellComment, CellData, ColGroup, ConditionalFormat, ConditionalRule, DataValidation,
    Hyperlink, MergeCell, Operator, PageMargins, PageOrientation, PrintSetup, RowData, RowGroup,
    Sparkline, SparklineGroup, SparklineType, Table, TableStyleInfo, ValidationType, XlsxCellStyle, XlsxWriter,
    streaming::StreamingXlsxWriter,
    style_registry::{SharedStrings, StyleRegistry},
};
pub use template::{PlaceholderInfo, TemplateData, TemplateFiller, TemplateReader};
pub use xlsx_reader::{XlsxCellValue, XlsxReader, XlsxSheetData, XlsxTableInfo};
pub use xlsx_streaming_reader::{RowIterator as XlsxRowIterator, XlsxStreamingReader};
pub use xlsx_style_reader::XlsxStyleTable;
