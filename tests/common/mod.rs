//! Shared test utilities

use std::path::Path;

/// Get the path to an example fixture file as a String
pub fn example_path(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
        .to_string_lossy()
        .to_string()
}

/// Ensure example fixture files exist (creates minimal XLSX fixtures if missing)
pub fn ensure_example_fixtures() {
    let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    if !examples_dir.exists() {
        std::fs::create_dir_all(&examples_dir).ok();
    }
}
