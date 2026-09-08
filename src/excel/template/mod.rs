//! Template-based XLSX generation
//!
//! This module provides functionality for reading existing XLSX files as templates,
//! identifying placeholder cells (e.g., `{{placeholder}}`), and filling them with
//! actual data while preserving formatting and structure.

mod template_filler;
mod template_reader;

pub use template_filler::TemplateFiller;
pub use template_reader::{PlaceholderInfo, TemplateData, TemplateReader};
