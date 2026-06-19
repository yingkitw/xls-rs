# ARCHITECTURE

## High level

This repository builds:

- **CLI binary**: `xls-rs` (`src/main.rs`, clap definitions in `src/cli/`)
- **Library crate**: `xls_rs` (`src/lib.rs`)

The CLI delegates command execution to domain handlers under `src/cli/commands/` and uses the library modules for actual implementations. MCP tools use the same library entry points through `CapabilityRegistry::execute`.

## Key modules

### I/O layer

- `src/csv_handler.rs`: CSV read/write with formula-injection sanitization (`sanitize_csv_row` / `write_records_safe`).
- `src/excel/`: Excel read (`calamine` based) + write (`XlsxWriter`, `StreamingXlsxWriter`). Includes `WriteMode` (Expand/Preserve/Overwrite).
- `src/columnar/`: Parquet (`arrow` / `parquet`) and Avro (`apache-avro`) handlers.
- `src/google_sheets.rs`: Google Sheets API v4 client for read/write/append/list; uses `ureq` for HTTP.
- `src/converter.rs`: `Converter` — format-agnostic entry point that routes to the correct handler by extension.
- `src/handler_registry.rs`: Maps file extensions to `DataReader` / `DataWriter` implementations.

### Operations layer

- `src/operations/`: pandas-style operations — sort, filter, join, concat, groupby, pivot, melt, rolling, crosstab, transpose, select, dedupe, sample, clip, normalize, zscore, fillna, dropna, rename, drop, mutate, astype, unique, value-counts, corr, describe, head, tail, info, dtypes.
- `src/formula/`: Excel formula parsing and evaluation (`FormulaEvaluator`).
- `src/validation.rs`: Data validation rules engine (`DataValidator`).
- `src/profiling.rs` / `src/profiling_handler.rs`: Column profiles and data-quality reports.
- `src/anomaly.rs`: Statistical outlier detection.
- `src/quality.rs`: Quality issue reporting.
- `src/text_analysis.rs` / `src/text_analysis_handler.rs`: Keyword, language, and sentiment analysis.
- `src/timeseries.rs`: Temporal resampling, rolling aggregates, trend detection.
- `src/geospatial.rs`: Coordinate parsing and distance/bearing calculations.

### Capabilities layer

- `src/capabilities/`: Individual capability implementations (`SortCapability`, `ReadExcelCapability`, `ApplyFormulaCapability`, `ConvertCapability`, `FilterCapability`, etc.).
- `src/capability_catalog.rs`: Static catalog of operations + formats; used for parity tracking between library / CLI / MCP.
- `src/capabilities/registry.rs`: `CapabilityRegistry` — runtime registry that MCP tools and CLI handlers call into.

### Streaming layer

- `src/streaming.rs`: Core streaming traits (`StreamingDataReader`, `StreamingDataWriter`) + `CsvStreamingReader` for chunked CSV I/O.
- `src/streaming_ops.rs`: Schema inference (`infer_schema`), `head`, `tail`, `get_info` without loading entire datasets.

### Server layer

- `src/mcp.rs`: `XlsRsMcpServer` — MCP tool definitions and routing to `CapabilityRegistry`.
- `src/mcp_enrichment.rs`: Builds structured `error.data` with request context and stable error codes.
- `src/api.rs`: Optional HTTP API server (`--features api`) with `POST /api/read`.

### Support layer

- `src/error.rs` / `src/error_traits.rs`: `XlsRsError`, `ErrorKind`, stable error codes, and trait-based error categorization.
- `src/config.rs`: TOML config discovery and typed `Config` struct (includes `google_sheets.access_token`, `default_format`, etc.).
- `src/common/`: Shared utilities — format detection, validation helpers, string utilities, collection helpers.
- `src/helpers.rs`: Grid slicing (`filter_by_range`), cell-reference parsing, safe numeric parsing.
- `src/types.rs`: Core types (`CellValue`, `DataSet`, `DataRow`, `DataType`).
- `src/traits.rs`: Shared traits (`DataReader`, `DataWriter`, `FileHandler`, `DataOperator`, etc.).
- `src/lineage.rs`: Transformation lineage tracking.
- `src/encryption.rs`: File-level encryption/decryption.
- `src/workflow.rs`: `WorkflowExecutor` for config-driven multi-step pipelines.
- `src/plugins.rs`: Plugin registry for user-defined functions.

## Data flow

```
CLI command ──→ DefaultCommandHandler ──→ CapabilityRegistry::execute
                                            │
MCP tool    ──→ XlsRsMcpServer ────────────┤
                                            │
Library API ──→ direct call ────────────────┘
                                            ↓
                                SortCapability / ReadExcelCapability / etc.
                                            ↓
                                ExcelHandler / Converter / DataOperations
                                            ↓
                                calamine / csv / parquet / avro / ureq
```

## Testing layout

- `tests/`: integration tests (28 test files covering all major features)
- `tests/common/mod.rs`: shared paths + example fixture creation for tests
- `benches/performance.rs`: Criterion benchmarks for read/write/convert hot paths

