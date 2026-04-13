//! Capabilities module for xls-rs
//! 
//! Provides a unified way to register and execute capabilities,
//! including CLI commands and MCP tools.

pub mod convert;
pub mod core;
pub mod filter;
pub mod registry;
pub mod sort;
pub mod workflow;

pub use convert::ConvertCapability;
pub use core::{Capability, CapabilityMetadata};
pub use filter::FilterCapability;
pub use registry::CapabilityRegistry;
pub use sort::SortCapability;
pub use workflow::WorkflowCapability;
