//! Excel file handling module

mod chart;
mod reader;
mod types;
mod writer;
pub mod xlsx_writer;

#[allow(unused_imports)]
pub use chart::{ChartConfig, DataChartType};
pub use reader::ExcelHandler;
#[allow(unused_imports)]
pub use types::{CellStyle, WriteOptions};
pub use xlsx_writer::{
    CellData, ConditionalFormat, ConditionalRule, RowData, Sparkline, SparklineGroup,
    SparklineType, XlsxWriter,
    streaming::StreamingXlsxWriter,
};
