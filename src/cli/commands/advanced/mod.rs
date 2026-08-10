//! Advanced command handlers module

pub mod chart;
pub mod profile;
pub mod schema;
pub mod to_sql;
pub mod utils;
pub mod validation;
pub mod examples;

// Re-export all handlers for convenience
pub use chart::handle_chart;
pub use profile::handle_profile;
pub use schema::handle_schema;
pub use to_sql::handle_to_sql;
pub use utils::{
    handle_add_chart, handle_add_sparkline, handle_apply_formula_range,
    handle_conditional_format, handle_config_init, handle_export_styled,
};
pub use validation::handle_validate;
pub use examples::handle_examples_generate;
