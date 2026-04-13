//! CLI command handler modules
//!
//! Organized by functionality:
//! - `io`: File I/O operations (read, write, convert)
//! - `transform`: Data transformations (sort, filter, replace, etc.)
//! - `pandas`: Pandas-style operations (head, tail, join, groupby, etc.)
//! - `advanced`: Advanced features (validate, chart, batch, etc.)

pub mod advanced;
pub mod advanced_handler;
pub mod io;
pub mod pandas;
pub mod transform;

pub use advanced_handler::AdvancedCommandHandler;

use anyhow::Result;

/// Command handler trait
///
/// All command handlers must implement this trait.
pub trait CommandHandler {
    /// Handle a command and return the result
    fn handle(&self, command: super::Commands) -> Result<()>;
}
