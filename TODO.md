# TODO

## North star

- [ ] Maintain **capability parity** across **library** (`xls_rs`), **CLI** (`xls-rs`), and **MCP server** (`XlsRsMcpServer`) so the same operations and formats are available everywhere with consistent semantics, errors, and defaults.

## Parity work (library ↔ CLI ↔ MCP)

- [x] Define a single “capability catalog” (operations + I/O formats + options) and track parity gaps. (`src/capability_catalog.rs`)
- [x] Ensure every CLI command maps 1:1 to a library entry point (no hidden behavior in CLI). — All commands now delegate to `IoCommandHandler`, `TransformCommandHandler`, `PandasCommandHandler`, or `AdvancedCommandHandler`.
- [x] Every MCP tool delegates to `CapabilityRegistry::execute` (same entry points as capabilities; error wrapping in `src/mcp.rs` + `src/mcp_enrichment.rs`).
- [x] Normalize error surface:
  - [x] Stable error codes/messages for CLI + MCP (same root causes, same wording) (`ErrorKind::code`, `mcp_error_data`)
  - [x] Structured MCP error payloads with actionable fields (file, sheet, range, cell, I/O paths)
    - [x] JSON-RPC `error.data` with `kind` / `detail` on tool failures (`src/mcp.rs`)
    - [x] Rich fields: request context (input/output/sheet/range/cell) plus heuristics from error text (`src/mcp_enrichment.rs`)
- [x] Add parity tests that run the same use case through:
  - [x] library API
  - [x] CLI command (smoke) (`tests/test_parity_smoke.rs`)
  - [x] CLI read format + config (`tests/test_cli_read_format.rs`)
  - [x] Capability registry (same code path as MCP tools) (`tests/test_mcp_registry.rs`)
  - [x] compare normalized outputs for deterministic parity (`tests/test_excel_parity.rs` — `test_read_range_normalized_parity_cli_vs_library`)

## XLS/XLSX manipulation (core)

- [~] **Read parity**:
  - [x] Range reads: CLI `read --range` and HTTP `api` read use `CellRange` + `filter_by_range` (same helper as columnar paths)
  - [~] Range reads identical across all backends where semantics differ today
  - [~] Sheet selection behavior consistent (default sheet, missing sheet errors)
    - [x] Excel / ODS: exact sheet name required when specified; missing sheet error lists available names (`ExcelHandler::resolve_sheet_selection`)
- [~] **Write parity**:
  - [x] XLSX writer: formulas/styles/charts/sparklines/condfmt APIs reachable from CLI + MCP (`write_styled`, `add_chart`, `add_sparkline`, `conditional_format` capabilities + MCP tools)
  - [x] Cell typing rules (number/date/string/empty) consistent across writers (`classify_cell` / `add_cell_to_row` used by `XlsxWriter`, `StreamingXlsxWriter`, and `ExcelHandler` write paths)
- [x] **Edit operations** (in-place style transforms):
  - [x] “apply formula” to a range (not just a single cell)
  - [x] “write range” that can expand sheet bounds safely (`ExcelHandler::write_range_expand`)
  - [x] preserve/overwrite behavior explicitly configurable (`WriteMode::Preserve/Overwrite/Expand`; CLI `--mode` on `write-range`)

## Format coverage & fidelity

- [x] Confirm support matrix and document it in README (what’s read-only vs read/write): (`README.md` — “Format support”)
  - [x] `.xlsx`
  - [x] `.xls`
  - [x] `.ods`
  - [x] `.csv`
  - [x] `.parquet`
  - [x] `.avro`
- [~] Ensure round-trip expectations are tested:
  - [x] CSV → XLSX → CSV (`tests/test_converter.rs` — `test_roundtrip_csv_xlsx_csv_data_preserved`)
  - [x] XLSX → Parquet/Avro → CSV (`test_roundtrip_xlsx_parquet_csv_preserves_grid`, `test_roundtrip_xlsx_avro_csv_preserves_grid`)
- [x] Add explicit constraints for unsupported features (merged cells, pivot tables, etc.) and fail with clear errors.
  - [x] Documented high-level limitations in README (“Read limitations”)
  - [x] XLSX: `FeatureDetector::detect_potential_issues` scans the zip (worksheets, charts) and returns structured `UnsupportedFeature` values; use with `validate_for_write` or custom reporting. Optional stricter “fail on read” mode still open.

## CLI UX & reliability

- [x] Add `--config <path>` to override config discovery.
- [x] Add `--quiet` and `--verbose` modes.
- [x] Add guardrails for destructive overwrites (`--overwrite` required).
- [x] Add `xls-rs examples-generate` to generate `examples/` artifacts deterministically.
- [x] Add `--format` defaults that are consistent with config + subcommands (`default_format` in config; `read` / `read-all` omit flag → config → csv).
- [x] Improve output consistency:
  - [x] `read` prints data to stdout; status via `runtime::log` → stderr when not `--quiet`
  - [x] Transform + pandas: “wrote …” / rolling / pivot / join / concat / glob warnings → `runtime::log` (stderr; respects `--quiet`)
  - [x] Inspect-only commands (`value-counts`, `info`, `corr`, …) print to stdout by design

## MCP server (tooling completeness)

- [x] Tool naming: consistent verbs and nouns (read/write/convert/sort/filter/…).
- [~] Add missing tools for advanced operations (validation/profile/chart/encrypt/batch/stream) if not already exposed.
  - [x] `convert_data` MCP tool + `ConvertCapability`
  - [x] `validate_data` MCP tool + `ValidateCapability`
  - [x] `profile_data` MCP tool + `ProfileCapability`
  - [x] `stream_data` MCP tool + `StreamCapability`
  - [~] chart/encrypt/batch still optional
- [~] Ensure MCP tools accept the same option schema as CLI flags (sheet, range, format, etc.).
  - [x] `read_excel` accepts `format` (csv, jsonl, markdown)
- [x] Add an MCP “capabilities” tool that returns the supported operations + formats at runtime.

## Performance & large files

- [x] Streaming mode parity (CLI + library + MCP):
  - [x] chunked reads/writes for big CSV (`CsvStreamingReader` + CLI `stream` command + MCP `stream_data` tool)
  - [x] avoid loading whole datasets when not needed (head/tail/schema/info) (`streaming_ops`)
- [x] Add basic benchmarks for key paths (read XLSX, write XLSX, convert to parquet, range read). — `cargo bench -p xls-rs --bench performance` (`benches/performance.rs`, `criterion` in `Cargo.toml`)

## Safety & correctness

- [~] Keep CSV formula-injection sanitization consistent across all write paths.
  - [x] `Converter`: stdout CSV (`-`) and temp CSV for Excel use `sanitize_csv_row` / `write_records_safe`
  - [x] Audit direct `write_record` paths: `DataWriter` for CSV uses `write_records_safe` / `append_records_safe`; `write_from_csv`, `write_range` flush, `StreamingCsvWriter::write_row`, and formula-evaluator CSV output sanitize (`src/csv_handler.rs`, `src/formula/evaluator.rs`). Low-level `write_records` / `append_records` remain for explicit/test use.
- [x] Path validation rules consistent for CLI commands that write files.
  - [x] `ensure_can_write`: reject empty path and embedded `\0` (besides `-`)
  - [x] Block `..` path components for CLI input and output (`ensure_safe_input`, `ensure_can_write` in `src/cli/runtime.rs`)

## Testing & fixtures

- [x] Consolidate example/fixture generation in one place and make it deterministic (CLI `examples-generate` + test fixtures).
- [x] Add golden-file tests for XLSX writer output structure (beyond current smoke checks). — `tests/test_xlsx_writer_golden.rs`
- [x] Add property-like tests for range parsing and column name resolution.
  - [x] Range parsing + `filter_by_range` (`src/helpers.rs` tests)
  - [x] Column name resolution (`select_columns_by_name` — `tests/test_operations.rs`)

## API server (`--features api`)

- [x] `POST /api/read` supports `range` (A1-style) via shared grid slicing.
- [x] Build with `api` feature (handlers aligned with current library types).

## Google Sheets

- [x] List sheet titles when `google_sheets.api_key` is set (`GoogleSheetsHandler::list_sheet_titles`, CLI `gsheets list`).
- [x] Full read/write/append via access token (`google_sheets.access_token` config + Google Sheets API v4 calls)

## Workflow

- [x] `WorkflowExecutor::execute_config` — MCP / callers avoid temp JSON files.

## Styled Excel export

- [x] CLI `export-styled` presets: `default`, `minimal`, `report`, `executive` / `corporate`.

## Hygiene

- [x] Keep `.gitignore` aligned with generated artifacts (`target/`, `*.tmp.csv`, generated `examples/*.{xlsx,xls,parquet,avro}`).
