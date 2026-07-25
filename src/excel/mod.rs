//! Excel file handling module

mod cell_typer;
pub mod chart;
pub mod feature_detector;
mod reader;
pub mod types;
mod writer;
pub mod xls_writer;
pub mod xls_reader;
pub mod xlsx_writer;
pub mod xlsx_reader;
pub mod ods_reader;
pub mod template;

#[allow(unused_imports)]
pub use cell_typer::{add_cell_to_row, add_cells_to_row, classify_cell};
#[allow(unused_imports)]
pub use chart::{ChartConfig, DataChartType};
pub use feature_detector::{FeatureDetector, FeatureSeverity, UnsupportedFeature};
pub use reader::ExcelHandler;
pub use writer::WriteMode;
#[allow(unused_imports)]
pub use types::{CellStyle, WriteOptions};
pub use xls_writer::{RowData as XlsRowData, SheetData as XlsSheetData, XlsWriter};
pub use xlsx_writer::{
    CellComment, CellData, ColGroup, ConditionalFormat, ConditionalRule, DataValidation,
    Hyperlink, MergeCell, PageMargins, PageOrientation, PrintSetup, RowData, RowGroup, Sparkline,
    SparklineGroup, SparklineType, ValidationType, XlsxWriter,
    streaming::StreamingXlsxWriter,
};
pub use template::{PlaceholderInfo, TemplateData, TemplateFiller, TemplateReader};
