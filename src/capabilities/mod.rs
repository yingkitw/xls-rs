//! Capabilities module for xls-rs
//! 
//! Provides a unified way to register and execute capabilities,
//! including CLI commands and MCP tools.

pub mod core;
pub mod registry;
pub mod sort;
pub mod filter;
pub mod workflow;

pub use core::{Capability, CapabilityMetadata};
pub use registry::CapabilityRegistry;
pub use sort::SortCapability;
pub use filter::FilterCapability;
pub use workflow::WorkflowCapability;
