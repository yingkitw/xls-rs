# ARCHITECTURE

## High level

This repository builds:

- **CLI binary**: `xls-rs` (`src/main.rs`, clap definitions in `src/cli/`)
- **Library crate**: `xls_rs` (`src/lib.rs`)

The CLI delegates command execution to domain handlers under `src/cli/commands/` and uses the library modules for actual implementations.

## Key modules

- `src/cli/`: clap definitions + command handlers
- `src/excel/`: Excel read/write logic (including XLSX writer, `WriteMode`)
- `src/columnar/`: Parquet/Avro handlers
- `src/operations/`: pandas-style operations (sort/filter/groupby/etc.)
- `src/formula/`: formula parsing/evaluation
- `src/config.rs`: config discovery + default config template
- `src/capabilities/`: capability registry + individual capabilities (`SortCapability`, `ReadExcelCapability`, `ApplyFormulaCapability`, etc.)
- `src/capability_catalog.rs`: single source of truth for operations, formats, and parity tracking
- `src/streaming.rs`: chunked data structures and `CsvStreamingReader`
- `src/google_sheets.rs`: Google Sheets API v4 client (read/write/append/list)
- `src/mcp.rs`: `XlsRsMcpServer` — MCP tool router delegating to `CapabilityRegistry`
- `src/mcp_enrichment.rs`: structured `error.data` with file/sheet/range/cell context
- `src/error.rs`: `XlsRsError` / `ErrorKind` with stable error codes for CLI/MCP parity

## Testing layout

- `tests/`: integration tests
- `tests/common/mod.rs`: shared paths + example fixture creation for tests

