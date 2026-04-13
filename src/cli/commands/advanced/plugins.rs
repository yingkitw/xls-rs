//! Plugin and streaming command handlers

use xls_rs::{converter::Converter, plugins::PluginRegistry};
use anyhow::Result;

/// Handle the plugin command
///
/// Executes a plugin function.
pub fn handle_plugin(
    function: String,
    input: String,
    output: String,
    args: Vec<String>,
) -> Result<()> {
    let registry = PluginRegistry::new();

    let converter = Converter::new();
    let data = converter.read_any_data(&input, None)?;

    // Execute plugin function
    let result = registry.execute(&function, &args, &data)?;

    converter.write_any_data(&output, &result, None)?;
    println!("Executed plugin '{function}' on {input}; wrote {output}");

    Ok(())
}

/// Handle the stream command
///
/// Processes a large file in chunks to reduce memory usage.
pub fn handle_stream(input: String, output: String, _chunk_size: usize) -> Result<()> {
    println!("Streaming support is a placeholder. Processing file normally...");

    let converter = Converter::new();
    let data = converter.read_any_data(&input, None)?;
    converter.write_any_data(&output, &data, None)?;

    println!("Processed {} rows; wrote {}", data.len(), output);

    Ok(())
}
