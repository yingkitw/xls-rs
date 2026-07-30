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

- [x] **Native XLS (BIFF8) write from scratch** — implemented in `src/excel/xls_writer/` using only `std` (no `zip`, no external dependencies for the write path). Produces valid OLE2 / CFB containers with BIFF8 records. Supports multiple sheets, strings (ASCII + UTF-16, including astral codepoints), numbers, booleans, basic formulas (refs, ranges, arithmetic, comparisons, ~25 common functions), column widths, and auto-fit. Round-trips through native `XlsReader`. Wired into `Converter::convert` for `*.xls` outputs and `ExcelHandler::write_xls` for the library API. CLI: `xls-rs convert --input foo.csv --output foo.xls`. Example: `examples/write_xls.rs`. 32 tests pass (20 unit + 12 integration).
- [x] **Native XLS (BIFF8) read from scratch** — implemented in `src/excel/xls_reader/` using only `std`. Parses CFB/BIFF8 format and returns structured cell data with full API compatibility. Supports strings, numbers, booleans, formulas, empty cells, and RK compressed numbers. Integrated into `ExcelHandler` for automatic `.xls` file handling. Verified end-to-end with native writer (write → read → convert). 11 unit tests pass.
  - [x] Range reads: CLI `read --range` and HTTP `api` read use `CellRange` + `filter_by_range` (same helper as columnar paths)
  - [x] Range reads identical across all backends where semantics differ today (`read_sheet_data` returns `Vec<Vec<String>>` directly for XLSX/XLS/ODS without CSV serialization round-trip; `read_range` also returns structured data)
  - [x] Sheet selection behavior consistent (default sheet, missing sheet errors)
    - [x] Excel / ODS: exact sheet name required when specified; missing sheet error lists available names (`ExcelHandler::resolve_sheet_selection`)
    - [x] CSV / Parquet / Avro: no sheet concept; `sheet` parameter gracefully ignored
- [x] **Write parity**:
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
- [x] Ensure round-trip expectations are tested:
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
- [x] Add missing tools for advanced operations (validation/profile/chart/encrypt/batch/stream).
  - [x] `convert_data` MCP tool + `ConvertCapability`
  - [x] `validate_data` MCP tool + `ValidateCapability`
  - [x] `profile_data` MCP tool + `ProfileCapability`
  - [x] `stream_data` MCP tool + `StreamCapability`
  - [x] `encrypt_file` MCP tool + `EncryptCapability`
  - [x] `batch_process` MCP tool + `BatchCapability`
  - [x] `add_chart` MCP tool + `AddChartCapability` (already existed)
- [x] Ensure MCP tools accept the same option schema as CLI flags (sheet, range, format, etc.).
  - [x] `read_excel` accepts `format` (csv, jsonl, markdown) via `format_read_result`
  - [x] `convert_data` accepts `sheet` (same as CLI `--sheet`)
  - [x] Core I/O options (input, output, sheet, range) present on all relevant MCP tools
- [x] Add an MCP “capabilities” tool that returns the supported operations + formats at runtime.

## Performance & large files

- [x] Streaming mode parity (CLI + library + MCP):
  - [x] chunked reads/writes for big CSV (`CsvStreamingReader` + CLI `stream` command + MCP `stream_data` tool)
  - [x] avoid loading whole datasets when not needed (head/tail/schema/info) (`streaming_ops`)
- [x] Add basic benchmarks for key paths (read XLSX, write XLSX, convert to parquet, range read). — `cargo bench -p xls-rs --bench performance` (`benches/performance.rs`, `criterion` in `Cargo.toml`)

## Safety & correctness

- [x] Keep CSV formula-injection sanitization consistent across all write paths.
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

## Competitive capabilities (gaps vs. popular tools)

### Excel fidelity (openpyxl / excelize / SheetJS gaps)

- [x] **Template-based generation**: Read an existing `.xlsx` as a template, fill data into named ranges / placeholder cells, write back. Implemented in `src/excel/template/` with `TemplateReader` (detects `{{placeholder}}` cells via native XLSX reader) and `TemplateFiller` (replaces placeholders and writes via `XlsxWriter`). API: `TemplateFiller::fill_from_file(template, output, &values, sheet)`. 7 unit tests pass.
- [ ] **Read existing styles / images / charts**: Currently we write styles/charts but cannot read them back from existing files. Needed for template workflows and round-trip fidelity.
- [ ] **`.xlsm` (macro-enabled) read/write**: Preserve VBA macros on copy/edit. openpyxl supports this in "keep_vba" mode; SheetJS preserves `vbaProject.bin`.
- [ ] **Password-protected Excel**: Support reading `.xlsx` encrypted with a password (msoffcrypto-style). Excelize and openpyxl both support this.
- [ ] **Excel structured tables**: Read/write `Table` objects (auto-expanding ranges with headers, total rows, banded rows). openpyxl has full `Table` support.
- [x] **Freeze panes**: Set freeze rows/columns on write (`freeze_header` in `WriteOptions`, generates `<pane ySplit="1" state="frozen"/>`).
- [x] **Auto-filter**: Write `autoFilter` range so Excel shows dropdown arrows (`auto_filter` in `WriteOptions`).
- [x] **Row/column grouping (outline)**: Write `outlineLevel` on `<row>` and `<col>` elements for collapsible sections. API: `XlsxWriter::add_row_group(start_row, end_row, level, collapsed)` / `add_col_group(...)`. openpyxl / xlsxwriter support.
- [x] **Hyperlinks**: Write clickable URLs in cells via `<hyperlinks>` + worksheet rels. API: `XlsxWriter::add_hyperlink(cell_ref, url, tooltip)`.
- [x] **Cell comments**: Write comments via `xl/commentsN.xml` + worksheet rels. No VML drawing indicator yet (comments visible in review pane). API: `XlsxWriter::add_comment(cell_ref, text, author)`.
- [x] **Data validation / dropdown lists**: Write `dataValidation` rules (list, whole, decimal, date, textLength, custom). API: `XlsxWriter::add_data_validation(DataValidation { range, validation_type, ... })`.
- [x] **Print setup**: Page margins (customizable), orientation (portrait/landscape), paper size, scale, fit-to-width/height, print area. API: `XlsxWriter::set_print_setup(PrintSetup { ... })`.
- [x] **Merged cells**: Write merged cell ranges via `<mergeCells>`. API: `XlsxWriter::add_merge_cell(start_row, start_col, end_row, end_col)`.
- [ ] **Slicers / timelines**: Write Excel UI slicers connected to tables / pivot tables. Excelize advanced feature.
- [ ] **Pivot charts**: Charts bound to pivot tables (distinct from regular charts). openpyxl.

### Performance & scale (xsv / polars / duckdb gaps)

**Memory safety mitigations (2026-07)**:
- [x] **Dense-grid OOM guards**: XLSX/XLS readers clamp densified sheet dimensions (`src/limits.rs` `MAX_DENSE_CELLS`) so a far-corner cell ref cannot allocate a full Excel matrix.
- [x] **ODS repeat caps**: `number-columns-repeated` / `number-rows-repeated` capped against sheet bounds.
- [x] **CFB hang/OOM**: single owned buffer (no sector double-copy) + visited-set / hop limits on FAT chains.
- [x] **ZIP/CSV size guards**: reject oversized zip entries and full-slurp CSV reads above 512 MiB.
- [x] **Join/melt output caps**, formula depth + range cell budget, string-distance length caps, profiler default sampling + frequency-key cap.

**Recently optimized**:
- [x] **`escape_xml` rewrite**: Eliminated per-character `Vec` allocations (from `flat_map` + `collect` to pre-allocated `String` with capacity). Reduces allocations by ~O(n) per string cell in XLSX write.
- [x] **`escape_xml_into` zero-copy fast path**: Added buffer-direct escaping with early-exit for strings with no special chars; called from the XLSX cell loop, eliminating the intermediate `String` allocation per cell entirely.
- [x] **XLSX cell-writing loop**: Replaced per-cell `format!` + `col_num_to_letter` (2 `String` allocations/cell) with reusable `col_num_to_letter_into` buffer + `write!` into the output XML. ~O(cells) fewer allocations.
- [x] **`add_cell_to_row` double-allocation fix**: Inlined cell classification so string cells allocate once instead of twice (was: `classify_cell` → `to_string` → `add_string` → `to_string`).
- [x] **`describe()` sort-once**: Replaced 7 sorts per column (one per percentile) with a single sort via `ColumnStats` struct. Sort complexity reduced from O(7n log n) to O(n log n) per column.
- [x] **`sort_unstable` profiling**: Switched `calculate_numeric_stats` and `calculate_length_stats` from `sort_by` to `sort_unstable_by` for faster constant factor.
- [x] **Streaming `tail()` ring buffer**: Replaced front-removal from `Vec` with `VecDeque`, reducing last-N row retention from O(rows × N) shifting to O(rows).
- [x] **Formula `AddColumn` two-phase evaluation**: Removed a deep clone of the full dataset by evaluating new values before mutating rows, reducing temporary memory from O(rows × columns) cells to O(rows).
- [x] **Cached arithmetic cell-reference parsing**: Reused the shared cell-reference regex and substituted references in one pass instead of compiling and repeatedly scanning per formula evaluation.

Still open:
- [ ] **CSV index (xsv-style)**: Build a lightweight index (row offsets per block) so `head`/`tail`/random access on huge CSVs is O(1) instead of O(n). xsv does this via `xsv index`.
- [ ] **True streaming XLSX read**: Row-by-row SAX-style parsing without loading entire workbook into memory. Current implementation materializes the whole sheet. `xlsx2csv` streams via `quick-xml`.
- [ ] **Lazy / query-plan evaluation**: Polars-style lazy DataFrames — build an execution graph, optimize (predicate pushdown, projection pushdown), then execute. Would dramatically speed up chained operations.
- [ ] **SIMD-accelerated numeric ops**: Use `arrow` compute kernels or `simd-json`-style SIMD for parse + aggregate on numeric columns. Polars/duckdb leverage this.
- [ ] **Memory-mapped CSV reads**: `memmap2` for zero-copy access to large CSVs on disk. xsv / polars use this.
- [ ] **Parallel Excel reading**: Multi-threaded sheet parsing (one thread per worksheet or chunk). Polars parallelizes Parquet/CSV reads; Excel is harder but feasible per-sheet.
- [ ] **Incremental Excel append**: Append rows to an existing `.xlsx` without rewriting the entire ZIP archive. Currently we rewrite the whole file. openpyxl supports true append for some operations.

### SQL & query engine (q / duckdb / csvkit gaps)

- [ ] **DuckDB-style SQL on files**: Embed a lightweight SQL engine (or call DuckDB via FFI) to run `SELECT * FROM 'sales.csv' WHERE amount > 100`. `q` tool and `duckdb` CLI are the gold standard here.
- [ ] **Window functions**: `ROW_NUMBER()`, `RANK()`, `LEAD()`, `LAG()`, `NTILE()` over ordered partitions. Standard SQL / polars / pandas feature.
- [ ] **CTEs and subqueries**: `WITH` clauses in the existing `query` command. Currently basic WHERE only.
- [ ] **SQL JOIN across files**: `JOIN` two CSV/Excel files by a key column in SQL. csvsql / duckdb do this.
- [ ] **Fuzzy join / record linkage**: Approximate string matching joins (e.g., join on "Company Name" with typos). Python `fuzzywuzzy` / `recordlinkage` libraries; no good Rust equivalent yet.

### Format coverage (polars / miller / csvkit gaps)

- [ ] **JSON / JSONLines first-class**: Native read/write with nested flattening/unflattening. `xsv` has `json` subcommand; `mlr` (miller) is excellent here. Currently only partial JSON support.
- [ ] **YAML read/write**: Read YAML as tabular (with dot-path flattening) and write YAML. `dasel` / `yq` territory.
- [ ] **TOML read/write**: Read TOML as tabular (flatten sections). `dasel` territory.
- [ ] **Apache Arrow IPC (Feather)**: `.arrow` / `.ipc` format. Polars / pandas native. Zero-copy interop with Arrow ecosystem.
- [ ] **Apache ORC**: Hadoop/ORC format. Spark / polars support.
- [ ] **Delta Lake**: Read/write Delta tables (Delta-rs crate). Polars / duckdb support.
- [ ] **Iceberg**: Read Apache Iceberg tables. DuckDB / Spark support.
- [ ] **SQLite read/write**: Read from `.db` / `.sqlite` and write back. `csvsql` / `sqlite-utils` territory.
- [ ] **PostgreSQL / MySQL connectors**: Direct database read/write. `csvsql` / `pandas.read_sql` territory.
- [ ] **PDF export**: Convert tabular data to PDF tables. `libreoffice --headless --convert-to pdf` is the workaround; native Rust would be powerful.
- [x] **HTML table export**: Basic HTML table output via `--format html` on `read`, `head`, `tail`, `describe`, and other inspect commands. Rich CSS styling still open.
- [ ] **LaTeX table export**: Academic paper tables. pandas `to_latex`.

### Interactive & UX (visidata / Tad / xsv gaps)

- [ ] **REPL / interactive mode**: Drop into an interactive shell (`xls-rs repl`) where you can chain operations and inspect intermediate results. Like `visidata` or a lightweight `ipython` for spreadsheets.
- [ ] **TUI data explorer**: Terminal UI with arrow-key navigation, sorting, filtering, column selection. `visidata` / `tad` are the benchmarks.
- [ ] **Shell completion for dynamic values**: Complete column names, sheet names, file paths from actual files. Currently only static completions.
- [ ] **Piping ergonomics**: Better stdin/stdout support so `cat data.csv | xls-rs sort --column 2 | xls-rs head -n 5` works seamlessly. Currently many commands require `--input` / `--output`.

### Advanced analytics (pandas / polars / R gaps)

- [x] **Percentile / quantile**: `describe` now includes 10th, 25th, 50th, 75th, 90th, 95th, 99th percentiles using linear interpolation (NumPy-compatible). API: `DataOperations::describe(data)`.
- [x] **Skewness & kurtosis**: Added to `describe` output. Uses population moment definitions (excess kurtosis = Fisher-1). API: `DataOperations::describe(data)`.
- [x] **Correlation methods**: Spearman rank correlation added. CLI: `--method spearman` on `corr` command. API: `DataOperations::spearman_correlation(data, columns)`.
- [x] **Kendall tau correlation**: Kendall tau-b rank correlation with tie handling. CLI: `--method kendall` on `corr` command. API: `DataOperations::kendall_tau_correlation(data, columns)`.
- [x] **Regression (simple linear)**: `slope`, `intercept`, `r_squared` for two numeric columns. CLI: `regress --x-column X --y-column Y`. API: `DataOperations::simple_linear_regression(data, x_col, y_col)`.
- [x] **String distance metrics**: Levenshtein, Jaro, Jaro-Winkler, and Hamming distance for fuzzy matching. CLI: `str-distance --a <s1> --b <s2> --method <levenshtein|jaro|jaro-winkler|hamming>`. API: `xls_rs::string_distance::{levenshtein, jaro, jaro_winkler, hamming}`.
- [x] **Z-score outlier detection**: Population Z-score via `AnomalyMethod::ZScore { threshold }`. Uses mean and std dev.
- [x] **Modified Z-score outlier detection**: Robust outlier detection via `AnomalyMethod::ModifiedZScore { threshold }`. Uses median and MAD (Median Absolute Deviation).
- [ ] **Isolation Forest**: Even a simple version. sklearn territory.
- [x] **Sampling methods**: Stratified sampling (proportional allocation per stratum) and systematic sampling (every k-th row). CLI: `sample --method stratified --stratum-column <col>` and `sample --method systematic`. API: `DataOperations::stratified_sample(data, n, col, seed)`, `DataOperations::systematic_sample(data, n, seed)`.
- [x] **Reshape (wider / longer)**: `pivot_wider` and `pivot_longer` as first-class tidyverse-style operations. CLI: `pivot-longer --cols Q1,Q2 --names-to quarter --values-to sales` and `pivot-wider --names-from quarter --values-from sales`. API: `DataOperations::pivot_longer(data, cols, names_to, values_to)`, `DataOperations::pivot_wider(data, names_from, values_from, id_cols)`. 9 tests pass.

### Distribution & deployment

- [ ] **WebAssembly build target**: Compile to WASM for browser-based Excel processing (like SheetJS but in Rust). `wasm-bindgen` target.
- [ ] **Pre-built binaries via GitHub Actions**: Release builds for macOS (universal), Linux (x86_64, aarch64), Windows. Homebrew formula.
- [ ] **Docker image**: Official `xls-rs` Docker image for CI pipelines.
- [ ] **GitHub Action**: `uses: yingkitw/xls-rs-action@v1` for workflows.

## Hygiene

- [x] Keep `.gitignore` aligned with generated artifacts (`target/`, `*.tmp.csv`, generated `examples/*.{xlsx,xls,parquet,avro}`).
