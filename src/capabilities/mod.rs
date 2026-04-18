//! Capabilities module for xls-rs
//!
//! Provides a unified way to register and execute capabilities,
//! including CLI commands and MCP tools.

pub mod convert;
pub mod core;
pub mod excel_read;
pub mod excel_write;
pub mod filter;
pub mod formula;
pub mod registry;
pub mod sort;
pub mod workflow;

pub use convert::ConvertCapability;
pub use core::{Capability, CapabilityMetadata};
pub use excel_read::{ListSheetsCapability, ReadAllSheetsCapability, ReadExcelCapability};
pub use excel_write::{
    AddChartCapability, AddSparklineCapability, ConditionalFormatCapability, WriteStyledCapability,
};
pub use filter::FilterCapability;
pub use formula::ApplyFormulaCapability;
pub use registry::CapabilityRegistry;
pub use sort::SortCapability;
pub use workflow::WorkflowCapability;
