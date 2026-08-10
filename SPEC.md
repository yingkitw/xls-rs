# SPEC

**Version**: 0.1.14 | **Last updated**: 2026-08-10 | **License**: Apache-2.0

## Project

`xls-rs` is **the pure-Rust spreadsheet toolkit**. Three pillars:

1. **Format mastery** — Native read/write for XLSX, XLS (BIFF8 from scratch), CSV, Parquet, Avro. Read ODS. Google Sheets via API v4.
2. **Format conversion** — Bridge between spreadsheet and columnar formats.
3. **Three surfaces** — CLI, library, and MCP server from one codebase.

All three surfaces delegate to the same operations registered in `CapabilityRegistry`, ensuring identical behavior, errors, and defaults everywhere.

## Design principles

- **Pure Rust, no external runtime** — No Excel, Python, JVM, or LibreOffice dependency.
- **Honest scope** — Practical formula subset, not a full Excel calc engine. Eager operations, not a lazy query engine. Spreadsheet-first, not a pandas replacement.
- **Production safety** — Formula-injection sanitization, overwrite guards, path validation, memory caps.
- **Surgical changes** — Touch only what needs changing. Match existing patterns. No speculative abstractions.

## Primary use cases

- Convert between formats (CSV ↔ XLSX ↔ Parquet ↔ Avro).
- Read specific sheets/ranges from Excel files.
- Apply formula evaluation and transformations.
- Author styled XLSX with charts, conditional formatting, structured tables.
- In-place Excel edits: apply formulas to ranges, write ranges with expand/preserve/overwrite semantics.
- Batch/process large CSV files (streaming).
- Read/write Google Sheets via API v4.
- Expose spreadsheet capabilities to AI agents via MCP.

## Capability surfaces

### Library (`xls_rs`)

- Core types: `ExcelHandler`, `Converter`, `CsvHandler`, `ParquetHandler`, `AvroHandler`, `GoogleSheetsHandler`.
- Operations: `DataOperations` (sort, filter, join, concat, groupby, pivot, describe with percentiles/skewness/kurtosis, correlation (Pearson/Spearman), simple linear regression, etc.), `DataValidator`, `DataProfiler`, `AnomalyDetector` (Z-score, Modified Z-score, IQR, percentile), `TextAnalyzer`.
- Excel-specific: `XlsxWriter`, `StreamingXlsxWriter`, `XlsWriter` (BIFF8 / OLE2 from scratch using only `std`), `WriteMode`, charts, sparklines, conditional formatting, merged cells, hyperlinks, comments, data validation, print setup, row/column grouping, freeze panes, auto-filter. XLSX writer XML generation is modular (`xml_gen.rs` with focused helpers for worksheet sections and styles, `style_registry.rs`, `cond_fmt_xml.rs`, `sparkline_xml.rs`, `chart_xml.rs`).
- Streaming: `CsvStreamingReader`, `StreamingProcessor`.
- Formula: `FormulaEvaluator` for in-memory evaluation of Excel expressions.

### CLI (`xls-rs`)

- 50+ subcommands covering I/O, transforms, analytics, and advanced features.
- Global flags: `--config`, `--quiet`, `--verbose`, `--overwrite`.
- Config file resolution: `.xls-rs.toml` → `~/.xls-rs.toml` → `$XDG_CONFIG_HOME/xls-rs/config.toml`.
- Output guards: `--overwrite` required for existing files; path validation blocks `..` and embedded nulls.

### MCP (`XlsRsMcpServer`)

- Each tool delegates to `CapabilityRegistry::execute` with the same name used by the CLI capability catalog.
- `read_excel` supports `format` option (`csv`, `jsonl`, `markdown`, `json`, `html`, `latex`).
- `capabilities` tool returns the runtime catalog of operations and formats.
- Error responses include structured `error.data` with stable `code`, `file`, `sheet`, `range`, and `cell` fields.

## Error normalization

- Every `ErrorKind` variant has a stable string `code()` for programmatic matching (e.g., `column_not_found`, `invalid_cell_ref`, `unsupported_format`). See [`MEMORY.md`](MEMORY.md#security-considerations) for security-related error handling.
- CLI prints human-friendly messages; MCP embeds the same code in JSON-RPC `error.data`.
- `mcp_error_data` enriches errors with request context (input/output paths, sheet, range, cell) and heuristics parsed from the error text.

## Scope boundaries

xls-rs is a practical toolkit, not a complete Excel engine or analytics platform:

- **Formula evaluation**: practical subset (arithmetic, comparisons, ~25 common functions). Not a full Excel calculation engine.
- **No lazy evaluation**: operations are eager. No query planning or predicate pushdown.
- **XLSX streaming**: CSV supports chunked processing; XLSX reads materialize the whole sheet.
- **MCP hosting**: `xls-rs serve` does not yet launch a transport. Embed `XlsRsMcpServer` in an async host.
- **ODS write**: not implemented. Convert to XLSX instead.
- **Surface parity**: advanced library analytics (anomaly, time-series, geospatial, text analysis) are not all exposed through CLI or MCP.

## Non-functional requirements

- **Rust edition**: 2024
- **Minimum supported Rust version**: 1.85+ (edition 2024)
- **Quality gates**: `cargo build` and `cargo test` must pass. See [`MEMORY.md`](MEMORY.md#testing-patterns) for testing conventions.
- **Cross-platform**: macOS, Linux, Windows (where dependencies allow).
- **Safety**: CSV formula-injection sanitization on all write paths; overwrite guards; path traversal prevention; memory caps for malicious files. See [`MEMORY.md`](MEMORY.md#security-considerations) for security patterns.

