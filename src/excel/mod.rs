//! Excel XLSX file handling module

mod cell_typer;
pub mod chart;
pub mod feature_detector;
pub mod reader;
pub mod template;
pub mod types;
mod writer;
pub mod xlsx_reader;
pub mod xlsx_streaming_reader;
pub mod xlsx_style_reader;
pub mod xlsx_writer;

// Public API exports - unused internally but part of library interface
#[allow(unused_imports)]
pub use cell_typer::{add_cell_to_row, add_cells_to_row, classify_cell};
pub use chart::{ChartConfig, DataChartType};
pub use feature_detector::{FeatureDetector, FeatureSeverity, UnsupportedFeature};
pub use reader::ExcelHandler;
pub use template::{PlaceholderInfo, TemplateData, TemplateFiller, TemplateReader};
#[allow(unused_imports)]
pub use types::{CellStyle, WriteOptions};
pub use writer::WriteMode;
pub use xlsx_reader::{XlsxCellValue, XlsxImage, XlsxReader, XlsxSheetData, XlsxTableInfo};
pub use xlsx_streaming_reader::{RowIterator as XlsxRowIterator, XlsxStreamingReader};
pub use xlsx_style_reader::XlsxStyleTable;
pub use xlsx_writer::{
    CellComment, CellData, ColGroup, ConditionalFormat, ConditionalRule, DataValidation,
    DocumentProperties, Hyperlink, Image, ImageFormat, MergeCell, Operator, PageMargins,
    PageOrientation, PrintSetup, RichTextRun, RichTextRunStyle, RowData, RowGroup, SheetProtection,
    Sparkline, SparklineGroup, SparklineType, Table, TableStyleInfo, ValidationType,
    WorkbookProtection, XlsxCellStyle, XlsxWriter,
    streaming::StreamingXlsxWriter,
    style_registry::{SharedStrings, StyleRegistry},
};
