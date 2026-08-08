# ARCHITECTURE

**Version**: 0.1.11 | **Last updated**: 2026-08-08 | **License**: Apache-2.0

## Table of Contents
- [High level](#high-level)
- [Key modules](#key-modules)
- [Data flow](#data-flow)
- [Testing layout](#testing-layout)

## High level

This repository builds:

- **CLI binary**: `xls-rs` (`src/main.rs`, clap definitions in `src/cli/`)
- **Library crate**: `xls_rs` (`src/lib.rs`)

The CLI delegates command execution to domain handlers under `src/cli/commands/` and uses the library modules for actual implementations. MCP tools use the same library entry points through `CapabilityRegistry::execute`.

## Key modules

### I/O Layer

- `src/csv_handler.rs`: CSV read/write with formula-injection sanitization (`sanitize_csv_row` / `write_records_safe`).
- `src/excel/`: Excel read (native readers for `.xlsx`, `.xls`, and `.ods`) + write (`XlsxWriter`, `StreamingXlsxWriter`, `XlsWriter`). Includes `WriteMode` (Expand/Preserve/Overwrite). Writer supports charts, sparklines, conditional formatting, merged cells, hyperlinks, comments, data validation, print setup, row/column grouping (outline), freeze panes, and auto-filter. XLSX writer is modular: `xml_gen.rs` (worksheet/styles XML), `style_registry.rs` (cell style registry), `cond_fmt_xml.rs` (conditional formatting XML), `sparkline_xml.rs` (sparkline XML), `chart_xml.rs` (chart XML), `types.rs` (data types), `streaming.rs` (streaming writer). **XLS (BIFF8) write path is implemented from scratch in `src/excel/xls_writer/`** — see below.
- `src/columnar/`: Parquet (`arrow` / `parquet`) and Avro (`apache-avro`) handlers.
- `src/google_sheets.rs`: Google Sheets API v4 client for read/write/append/list; uses `ureq` for HTTP.
- `src/converter.rs`: `Converter` — format-agnostic entry point that routes to the correct handler by extension.
- `src/handler_registry.rs`: Maps file extensions to `DataReader` / `DataWriter` implementations.

### Operations Layer

- `src/operations/`: pandas-style operations — sort, filter, join, concat, groupby, pivot, melt, rolling, crosstab, transpose, select, dedupe, sample, clip, normalize, zscore, fillna, dropna, rename, drop, mutate, astype, unique, value-counts, corr (Pearson & Spearman), describe (with percentiles, skewness, kurtosis), simple linear regression (`regress`), head, tail, info, dtypes.
- `src/formula/`: Excel formula parsing and evaluation (`FormulaEvaluator`).
- `src/validation.rs`: Data validation rules engine (`DataValidator`).
- `src/profiling.rs` / `src/profiling_handler.rs`: Column profiles and data-quality reports.
- `src/anomaly.rs`: Statistical outlier detection — Z-score, Modified Z-score (MAD-based), IQR, and percentile methods.
- `src/quality.rs`: Quality issue reporting.
- `src/text_analysis.rs` / `src/text_analysis_handler.rs`: Keyword, language, and sentiment analysis.
- `src/timeseries.rs`: Temporal resampling, rolling aggregates, trend detection.
- `src/geospatial.rs`: Coordinate parsing and distance/bearing calculations.

### Capabilities Layer

- `src/capabilities/`: Individual capability implementations (`SortCapability`, `ReadExcelCapability`, `ApplyFormulaCapability`, `ConvertCapability`, `FilterCapability`, etc.).
- `src/capability_catalog.rs`: Static catalog of operations + formats; used for parity tracking between library / CLI / MCP.
- `src/capabilities/registry.rs`: `CapabilityRegistry` — runtime registry that MCP tools and CLI handlers call into.

### Streaming Layer

- `src/streaming.rs`: Core streaming traits (`StreamingDataReader`, `StreamingDataWriter`) + `CsvStreamingReader` for chunked CSV I/O.
- `src/streaming_ops.rs`: Schema inference (`infer_schema`), `head`, `tail`, `get_info` without loading entire datasets.

### Server Layer

- `src/mcp.rs`: `XlsRsMcpServer` — MCP tool definitions and routing to `CapabilityRegistry`.
- `src/mcp_enrichment.rs`: Builds structured `error.data` with request context and stable error codes.
- `src/api.rs`: Optional HTTP API server (`--features api`) with `POST /api/read`.

### Support Layer

- `src/error.rs` / `src/error_traits.rs`: `XlsRsError`, `ErrorKind`, stable error codes, and trait-based error categorization.
- `src/config.rs`: TOML config discovery and typed `Config` struct (includes `google_sheets.access_token`, `default_format`, etc.).
- `src/common/`: Shared utilities — format detection, validation helpers, string utilities, collection helpers.
- `src/helpers.rs`: Grid slicing (`filter_by_range`), cell-reference parsing, safe numeric parsing.
- `src/limits.rs`: Hard caps for dense-grid materialization, ODS repeats, ZIP/CSV slurps, join/melt output, formula depth/range size, string-distance length, and profiler sampling — mitigates spreadsheet zip/memory bombs.
- `src/types.rs`: Core types (`CellValue`, `DataSet`, `DataRow`, `DataType`).
- `src/traits.rs`: Shared traits (`DataReader`, `DataWriter`, `FileHandler`, `DataOperator`, etc.).
- `src/lineage.rs`: Transformation lineage tracking.
- `src/encryption.rs`: File-level encryption/decryption.
- `src/workflow.rs`: `WorkflowExecutor` for config-driven multi-step pipelines.
- `src/plugins.rs`: Plugin registry for user-defined functions.

### XLS (legacy BIFF8) writer — from scratch

`src/excel/xls_writer/` implements the legacy `.xls` format using only `std`:

- `cfb.rs` — OLE2 / Compound File Binary writer (v3, 512-byte sectors, mini-stream for streams < 4096 bytes, balanced directory tree, FAT / mini-FAT / DIFAT chains).
- `biff.rs` — BIFF8 record encoder (BOF, CodePage, Window1, Font, XF, DateMode, BoundSheet, UseSelfs, Country, SST, Window2, BOF sheet, Dimensions, Window2, Row, cells, EOF).
- `ptg.rs` — Basic Excel formula encoder (cell references, ranges, integer / float / boolean literals, arithmetic, comparisons, ~25 common functions).
- `mod.rs` — `XlsWriter`, `XlsRowData`, `XlsSheetData` (mirrors the `XlsxWriter` API).

The output is valid OLE2 / CFB + BIFF8. Round-tripped through our native `XlsReader` in the integration tests under `tests/test_xls_writer.rs`.

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
                                native readers / csv / parquet / avro / ureq
                                            ↓ (XLS write only)
                                XlsWriter (src/excel/xls_writer/) — std only
```

## Testing layout

- `tests/`: integration tests (34 test files, 676 tests covering all major features)
- `tests/common/mod.rs`: shared paths + example fixture creation for tests
- `benches/performance.rs`: Criterion benchmarks for read/write/convert hot paths

