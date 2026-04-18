//! Excel file handling module

mod cell_typer;
pub mod chart;
mod feature_detector;
mod reader;
pub mod types;
mod writer;
pub mod xlsx_writer;

#[allow(unused_imports)]
pub use cell_typer::{add_cell_to_row, add_cells_to_row, classify_cell};
#[allow(unused_imports)]
pub use chart::{ChartConfig, DataChartType};
pub use feature_detector::{FeatureDetector, FeatureSeverity, UnsupportedFeature};
pub use reader::ExcelHandler;
pub use writer::WriteMode;
#[allow(unused_imports)]
pub use types::{CellStyle, WriteOptions};
pub use xlsx_writer::{
    CellData, ConditionalFormat, ConditionalRule, RowData, Sparkline, SparklineGroup,
    SparklineType, XlsxWriter,
    streaming::StreamingXlsxWriter,
};
