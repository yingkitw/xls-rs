# TODO

## North star

- [ ] Maintain **capability parity** across **library** (`xls_rs`), **CLI** (`xls-rs`), and **MCP server** (`XlsRsMcpServer`) so the same operations and formats are available everywhere with consistent semantics, errors, and defaults.

## Parity work (library ↔ CLI ↔ MCP)

- [x] Define a single “capability catalog” (operations + I/O formats + options) and track parity gaps. (`src/capability_catalog.rs`)
- [ ] Ensure every CLI command maps 1:1 to a library entry point (no hidden behavior in CLI).
- [ ] Ensure every MCP tool maps 1:1 to a library entry point (no bespoke MCP-only logic).
- [ ] Normalize error surface:
  - [ ] Stable error codes/messages for CLI + MCP (same root causes, same wording)
  - [~] Structured MCP error payloads with actionable fields (file, sheet, range, row/col)
    - [x] JSON-RPC `error.data` with `{ "kind": "xls_rs_error", "detail": "..." }` on tool failures (`src/mcp.rs`)
    - [ ] Rich fields (file, sheet, range) parsed from errors
- [~] Add parity tests that run the same use case through:
  - [x] library API
  - [x] CLI command (smoke) (`tests/test_parity_smoke.rs`)
  - [x] CLI read format + config (`tests/test_cli_read_format.rs`)
  - [x] Capability registry (same code path as MCP tools) (`tests/test_mcp_registry.rs`)
  - [ ] compare normalized outputs for deterministic parity

## XLS/XLSX manipulation (core)

- [~] **Read parity**:
  - [x] Range reads: CLI `read --range` and HTTP `api` read use `CellRange` + `filter_by_range` (same helper as columnar paths)
  - [ ] Range reads identical across all backends where semantics differ today
  - [~] Sheet selection behavior consistent (default sheet, missing sheet errors)
    - [x] Excel / ODS: exact sheet name required when specified; missing sheet error lists available names (`ExcelHandler::resolve_sheet_selection`)
- [ ] **Write parity**:
  - [ ] XLSX writer: ensure formulas/styles/charts/sparklines/condfmt APIs are reachable from CLI + MCP
  - [ ] Cell typing rules (number/date/string/empty) consistent across writers
- [ ] **Edit operations** (in-place style transforms):
  - [ ] “apply formula” to a range (not just a single cell)
  - [ ] “write range” that can expand sheet bounds safely
  - [ ] preserve/overwrite behavior explicitly configurable

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
- [~] Add explicit constraints for unsupported features (merged cells, pivot tables, etc.) and fail with clear errors.
  - [x] Documented high-level limitations in README (“Read limitations”)
  - [ ] Structured errors when a feature is detected (not just documentation)

## CLI UX & reliability

- [x] Add `--config <path>` to override config discovery.
- [x] Add `--quiet` and `--verbose` modes.
- [x] Add guardrails for destructive overwrites (`--overwrite` required).
- [x] Add `xls-rs examples-generate` to generate `examples/` artifacts deterministically.
- [x] Add `--format` defaults that are consistent with config + subcommands (`default_format` in config; `read` / `read-all` omit flag → config → csv).
- [~] Improve output consistency:
  - [x] `read` prints data to stdout; status via `runtime::log` → stderr when not `--quiet`
  - [x] Transform + pandas: “wrote …” / rolling / pivot / join / concat / glob warnings → `runtime::log` (stderr; respects `--quiet`)
  - [ ] Inspect-only commands (`value-counts`, `info`, `corr`, …) still print to stdout by design

## MCP server (tooling completeness)

- [ ] Tool naming: consistent verbs and nouns (read/write/convert/sort/filter/…).
- [~] Add missing tools for advanced operations (validation/profile/chart/encrypt/batch/stream) if not already exposed.
  - [x] `convert_data` MCP tool + `ConvertCapability` (registry parity test in `tests/test_mcp_registry.rs`)
- [ ] Ensure MCP tools accept the same option schema as CLI flags (sheet, range, format, etc.).
- [x] Add an MCP “capabilities” tool that returns the supported operations + formats at runtime.

## Performance & large files

- [ ] Streaming mode parity (CLI + library + MCP):
  - [ ] chunked reads/writes for big CSV and big XLSX where feasible
  - [ ] avoid loading whole datasets when not needed (head/tail/schema/info)
- [ ] Add basic benchmarks for key paths (read XLSX, write XLSX, convert to parquet).

## Safety & correctness

- [~] Keep CSV formula-injection sanitization consistent across all write paths.
  - [x] `Converter`: stdout CSV (`-`) and temp CSV for Excel use `sanitize_csv_row` / `write_records_safe`
  - [x] Audit direct `write_record` paths: `DataWriter` for CSV uses `write_records_safe` / `append_records_safe`; `write_from_csv`, `write_range` flush, `StreamingCsvWriter::write_row`, and formula-evaluator CSV output sanitize (`src/csv_handler.rs`, `src/formula/evaluator.rs`). Low-level `write_records` / `append_records` remain for explicit/test use.
- [~] Path validation rules consistent for CLI commands that write files.
  - [x] `ensure_can_write`: reject empty path and embedded `\0` (besides `-`)
  - [ ] Optional: canonicalize / block `..` if desired for untrusted inputs

## Testing & fixtures

- [x] Consolidate example/fixture generation in one place and make it deterministic (CLI `examples-generate` + test fixtures).
- [ ] Add golden-file tests for XLSX writer output structure (beyond current smoke checks).
- [x] Add property-like tests for range parsing and column name resolution.
  - [x] Range parsing + `filter_by_range` (`src/helpers.rs` tests)
  - [x] Column name resolution (`select_columns_by_name` — `tests/test_operations.rs`)

## API server (`--features api`)

- [x] `POST /api/read` supports `range` (A1-style) via shared grid slicing.
- [x] Build with `api` feature (handlers aligned with current library types).

## Google Sheets

- [x] List sheet titles when `google_sheets.api_key` is set (`GoogleSheetsHandler::list_sheet_titles`, CLI `gsheets list`).
- [ ] Full read/write/append via OAuth2 or service account.

## Workflow

- [x] `WorkflowExecutor::execute_config` — MCP / callers avoid temp JSON files.

## Styled Excel export

- [x] CLI `export-styled` presets: `default`, `minimal`, `report`, `executive` / `corporate`.

## Hygiene

- [x] Keep `.gitignore` aligned with generated artifacts (`target/`, `*.tmp.csv`, generated `examples/*.{xlsx,xls,parquet,avro}`).
