# SPEC

## Project

`xls-rs` provides three capability surfaces built from the same core:

- **Library** (`xls_rs` crate): Rust APIs for programmatic use.
- **CLI** (`xls-rs` binary): Human-friendly command-line interface.
- **MCP Server** (`XlsRsMcpServer`): Tool-based automation via the Model Context Protocol.

All three surfaces delegate to the same underlying operations registered in `CapabilityRegistry`, ensuring identical behavior, errors, and defaults everywhere.

## Primary use cases

- Convert between formats (CSV ↔ Excel ↔ Parquet ↔ Avro).
- Read specific sheets/ranges from Excel files.
- Apply formula evaluation and transformations.
- Batch/process large files (streaming-oriented operations where possible).
- In-place Excel edits: apply formulas to ranges, write ranges with expand/preserve/overwrite semantics.
- Read/write Google Sheets via API v4 when authenticated.
- Expose the same capabilities to AI agents via the MCP server (`XlsRsMcpServer`).

## Capability surfaces

### Library (`xls_rs`)

- Core types: `ExcelHandler`, `Converter`, `CsvHandler`, `ParquetHandler`, `AvroHandler`, `GoogleSheetsHandler`.
- Operations: `DataOperations` (sort, filter, join, concat, groupby, pivot, describe with percentiles/skewness/kurtosis, correlation (Pearson/Spearman), simple linear regression, etc.), `DataValidator`, `DataProfiler`, `AnomalyDetector` (Z-score, Modified Z-score, IQR, percentile), `TextAnalyzer`.
- Excel-specific: `XlsxWriter`, `StreamingXlsxWriter`, `XlsWriter` (BIFF8 / OLE2 from scratch using only `std`), `WriteMode`, charts, sparklines, conditional formatting, merged cells, hyperlinks, comments, data validation, print setup, row/column grouping, freeze panes, auto-filter.
- Streaming: `CsvStreamingReader`, `StreamingProcessor`.
- Formula: `FormulaEvaluator` for in-memory evaluation of Excel expressions.

### CLI (`xls-rs`)

- 50+ subcommands covering I/O, transforms, analytics, and advanced features.
- Global flags: `--config`, `--quiet`, `--verbose`, `--overwrite`.
- Config file resolution: `.xls-rs.toml` → `~/.xls-rs.toml` → `$XDG_CONFIG_HOME/xls-rs/config.toml`.
- Output guards: `--overwrite` required for existing files; path validation blocks `..` and embedded nulls.

### MCP (`XlsRsMcpServer`)

- Each tool delegates to `CapabilityRegistry::execute` with the same name used by the CLI capability catalog.
- `read_excel` supports `format` option (`csv`, `jsonl`, `markdown`, `json`, `html`).
- `capabilities` tool returns the runtime catalog of operations and formats.
- Error responses include structured `error.data` with stable `code`, `file`, `sheet`, `range`, and `cell` fields.

## Error normalization

- Every `ErrorKind` variant has a stable string `code()` for programmatic matching (e.g., `column_not_found`, `invalid_cell_ref`, `unsupported_format`).
- CLI prints human-friendly messages; MCP embeds the same code in JSON-RPC `error.data`.
- `mcp_error_data` enriches errors with request context (input/output paths, sheet, range, cell) and heuristics parsed from the error text.

## Non-functional requirements

- **Rust edition**: 2024
- **Quality gates**: `cargo build` and `cargo test` must pass.
- **Cross-platform**: should work on macOS/Linux/Windows (where dependencies allow).
- **Safety**: CSV formula-injection sanitization on all write paths; overwrite guards; path traversal prevention.

