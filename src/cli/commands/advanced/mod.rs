//! Advanced command handlers module
//!
//! Implements advanced features like validation, charting, encryption, batch processing, etc.

pub mod batch;
pub mod chart;
pub mod encryption;
pub mod plugins;
pub mod profile;
pub mod schema;
pub mod to_sql;
pub mod utils;
pub mod validation;
pub mod watch;
pub mod examples;

// Re-export all handlers for convenience
pub use batch::handle_batch;
pub use chart::handle_chart;
pub use encryption::{handle_decrypt, handle_encrypt};
pub use plugins::{handle_plugin, handle_stream};
pub use profile::handle_profile;
pub use schema::handle_schema;
pub use to_sql::handle_to_sql;
pub use utils::{handle_completions, handle_config_init, handle_export_styled};
pub use validation::handle_validate;
pub use watch::handle_watch;
pub use examples::handle_examples_generate;
