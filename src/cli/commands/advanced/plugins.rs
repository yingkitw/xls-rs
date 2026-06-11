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
/// Processes a large CSV file in chunks to reduce memory usage.
pub fn handle_stream(input: String, output: String, chunk_size: usize) -> Result<()> {
    use xls_rs::streaming::{CsvStreamingReader, StreamingDataReader};
    use csv::WriterBuilder;

    if !input.ends_with(".csv") {
        println!("Streaming is optimized for CSV. Falling back to normal processing...");
        let converter = Converter::new();
        let data = converter.read_any_data(&input, None)?;
        converter.write_any_data(&output, &data, None)?;
        println!("Processed {} rows; wrote {}", data.len(), output);
        return Ok(());
    }

    let mut reader = CsvStreamingReader::new(&input)?;
    let mut writer = WriterBuilder::new()
        .from_path(&output)
        .map_err(|e| anyhow::anyhow!("Failed to create output CSV: {}", e))?;

    let chunk_size = if chunk_size == 0 { 1000 } else { chunk_size };
    let mut total_rows = 0usize;
    let mut total_chunks = 0usize;

    while reader.has_more() {
        if let Some(chunk) = reader.read_chunk(chunk_size)? {
            for row in &chunk.data {
                writer.write_record(row)?;
            }
            total_rows += chunk.data.len();
            total_chunks += 1;
        }
    }

    writer.flush()?;
    println!("Streamed {} chunks ({} rows); wrote {}", total_chunks, total_rows, output);

    Ok(())
}
