//! Capabilities module for xls-rs
//!
//! Provides a unified way to register and execute capabilities,
//! including CLI commands and MCP tools.

pub mod batch;
pub mod convert;
pub mod core;
pub mod encrypt;
pub mod excel_read;
pub mod excel_write;
pub mod filter;
pub mod formula;
pub mod profile;
pub mod registry;
pub mod sort;
pub mod stream;
pub mod validate;
pub mod workflow;

pub use batch::BatchCapability;
pub use convert::ConvertCapability;
pub use core::{Capability, CapabilityMetadata};
pub use encrypt::EncryptCapability;
pub use excel_read::{ListSheetsCapability, ReadAllSheetsCapability, ReadExcelCapability};
pub use excel_write::{
    AddChartCapability, AddSparklineCapability, ConditionalFormatCapability, WriteStyledCapability,
};
pub use filter::FilterCapability;
pub use formula::ApplyFormulaCapability;
pub use profile::ProfileCapability;
pub use registry::CapabilityRegistry;
pub use sort::SortCapability;
pub use stream::StreamCapability;
pub use validate::ValidateCapability;
pub use workflow::WorkflowCapability;
