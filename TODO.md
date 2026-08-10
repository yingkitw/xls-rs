# TODO

**Version**: 0.1.14 | **Last updated**: 2026-08-10 | **License**: Apache-2.0

## Table of Contents
- [Vision](#vision)
- [Completed work](#completed-work)
- [Roadmap — near-term (v0.2)](#roadmap--near-term-v02)
- [Roadmap — medium-term (v0.3–0.4)](#roadmap--medium-term-v03v04)
- [Roadmap — long-term (v1.0)](#roadmap--long-term-v10)
- [Brainstorming](#brainstorming)

## Vision

xls-rs is **the pure-Rust spreadsheet toolkit**. Three pillars:

1. **Format mastery** — Best-in-class native read/write for XLSX, XLS, CSV. Read ODS. Read/write Parquet, Avro. Google Sheets via API.
2. **Format conversion** — The simplest way to bridge spreadsheet and columnar formats from the shell or Rust.
3. **Three surfaces** — CLI, library, and MCP server from one codebase with consistent semantics.

**Design principles:**

- **Pure Rust, no external runtime** — No Excel, Python, JVM, or LibreOffice dependency.
- **Honest scope** — Practical formula subset, not a full Excel calc engine. Eager operations, not a lazy query engine. Spreadsheet-first, not a pandas replacement.
- **Production safety** — Formula-injection sanitization, overwrite guards, path validation, memory caps.
- **Surgical changes** — Touch only what needs changing. Match existing patterns. No speculative abstractions.

## Completed work

All items below are done and tested. See [`MEMORY.md`](MEMORY.md) for patterns and conventions.

### Core format support
- Native XLS (BIFF8) read + write from scratch in pure `std` — `src/excel/xls_writer/`, `src/excel/xls_reader/`
- Native XLSX read + write with charts, sparklines, conditional formatting, structured tables, merged cells, hyperlinks, comments, data validation, print setup, freeze panes, auto-filter, row/column grouping
- CSV read/write with formula-injection sanitization
- ODS read (no write — convert to XLSX instead)
- Parquet read/write (Apache Arrow)
- Avro read/write
- Google Sheets read/write/append/list via API v4
- Password-protected XLSX decryption (AES-256-CBC, MS-OFFCRYPTO Agile Encryption) via `password` feature
- XLSX style reading (`xlsx_style_reader.rs`) with round-trip write→read→verify
- `.xlsm` macro-enabled read/write (VBA project preservation)
- Template-based generation with `{{placeholder}}` cells
- LaTeX and HTML table export

### Data operations
- Inspection: head, tail, sample, describe, info, dtypes, value-counts, unique
- Transformations: sort, filter, replace, dedupe, transpose, select, rename, drop, mutate, astype, clip, normalize, zscore, fillna, dropna
- Reshaping: groupby, join, concat, pivot, pivot-longer, pivot-wider, melt, rolling, crosstab
- Statistics: Pearson/Spearman/Kendall tau-b correlation, percentiles, skewness, kurtosis, simple linear regression
- Text: regex filter/replace, histograms, date parsing, diffs, string distances (Levenshtein, Jaro, Jaro-Winkler, Hamming)
- Anomaly detection: Z-score, Modified Z-score, IQR, percentile methods
- Sampling: stratified, systematic

### Three-surface parity
- Capability catalog (`src/capability_catalog.rs`) tracks operations across library/CLI/MCP
- All CLI commands delegate to library entry points (no hidden behavior)
- All MCP tools delegate to `CapabilityRegistry::execute`
- Stable error codes with structured MCP error payloads
- Parity tests: library API, CLI smoke, capability registry, normalized output comparison

### CLI
- 50+ subcommands (I/O, transforms, analytics, advanced)
- Global flags: `--config`, `--quiet`, `--verbose`, `--overwrite`
- Config discovery: `.xls-rs.toml` → `~/.xls-rs.toml` → XDG config dir
- `examples-generate` for deterministic fixtures
- Shell completions generation
- File watch mode

### MCP server
- 18 tools: capabilities, read_excel, read_all_sheets, list_sheets, convert_data, sort_data, filter_data, execute_workflow, write_styled, add_chart, add_sparkline, conditional_format, apply_formula, validate_data, profile_data, stream_data, encrypt_file, batch_process
- Structured error responses with request context

### Performance & safety
- Buffered CSV I/O, chunked streaming, bounded-memory tail
- Memory caps for malicious files (dense grids, ODS repeats, ZIP/CSV sizes, join/melt output, formula depth)
- XLSX write optimizations: zero-copy XML escaping, reusable buffers, single-sort describe
- Criterion benchmarks for read/write/convert hot paths
- CSV formula-injection sanitization on all write paths
- Path traversal prevention (`..` and embedded nulls blocked)

### Testing
- 34 test files, 676+ tests covering all major features
- Round-trip tests: CSV↔XLSX, XLSX→Parquet/Avro→CSV, XLS write→read
- Golden-file tests for XLSX writer structure
- Property-like tests for range parsing and column resolution
- Feature-flag-gated tests for Parquet, Avro, Google Sheets, password

### Maintainability
- XLSX writer decomposed into focused helpers (worksheet sections, styles)
- Deduplicated outline lookup and writer initialization
- `.gitignore` aligned with generated artifacts

## Roadmap — near-term (v0.2)

Focus: close the most impactful gaps for existing users.

- [x] **MCP transport hosting**: `xls-rs serve` now launches a stdio transport via tokio + rmcp. Works out of the box with any MCP-compatible client. Test in `tests/test_mcp_serve.rs`.
- [x] **True streaming XLSX read**: Row-by-row parsing via `XlsxStreamingReader` with buffered ZIP entry reader. Yields `Vec<XlsxCellValue>` per row without full materialization. Test in `tests/test_xlsx_streaming.rs`.
- [x] **CSV index**: Row-offset index (`CsvIndex`) with `.idx` sidecar persistence. `tail` uses O(1) seek; CLI `csv-index` subcommand for build/query/info. Tests in `tests/test_csv_index.rs`.
- [x] **Piping ergonomics**: `--quiet` suppresses text labels and switches labeled commands (value-counts, unique, dtypes, corr) to CSV output. `print_csv` uses proper csv::Writer with quoting. 11 tests in `tests/test_piping.rs` covering stdin→stdout, chained pipes, values with commas, and 3-stage sort→filter→head.
- [~] **Pre-built binaries**: Skipped — no GitHub Actions CI per user preference.
- [x] **Expose advanced analytics through CLI**: `anomaly-detect` subcommand (zscore, modified-zscore, iqr, percentile) and `resample` subcommand (hourly/daily/weekly/monthly/quarterly/yearly with sum/mean/median/min/max/first/last/count). Both support stdin/stdout piping. Tests in `tests/test_cli_analytics.rs`.

## Roadmap — medium-term (v0.3–0.4)

Focus: deepen format coverage and distribution.

- [ ] **JSON / JSONLines first-class read/write**: Native read with nested flattening/unflattening.
- [ ] **SQLite read/write**: Read from `.db`/`.sqlite` and write back.
- [ ] **Apache Arrow IPC (Feather)**: `.arrow`/`.ipc` format for zero-copy Arrow ecosystem interop.
- [ ] **Memory-mapped CSV reads**: `memmap2` for zero-copy access to large CSVs.
- [ ] **Parallel Excel reading**: Multi-threaded sheet parsing (one thread per worksheet).
- [ ] **Incremental Excel append**: Append rows to existing `.xlsx` without rewriting the entire ZIP.
- [ ] **Docker image**: Official `xls-rs` Docker image for CI pipelines.
- [ ] **GitHub Action**: `uses: yingkitw/xls-rs-action@v1` for workflows.
- [ ] **XLSX password encryption**: Write password-protected `.xlsx` (currently only decryption is supported).
- [ ] **ODS write**: Native OpenDocument spreadsheet output.

## Roadmap — long-term (v1.0)

Focus: reach production maturity for the three pillars.

- [ ] **WebAssembly target**: Compile to WASM for browser-based spreadsheet processing.
- [ ] **REPL / interactive mode**: Drop into an interactive shell for chained operations and intermediate inspection.
- [ ] **TUI data explorer**: Terminal UI with arrow-key navigation, sorting, filtering.
- [ ] **Slicers / timelines**: Excel UI slicers connected to tables.
- [ ] **Formula engine expansion**: Broader function coverage, array formulas, named ranges.
- [ ] **API stability audit**: Review all public types and APIs for 1.0 readiness. Lock down breaking changes.

## Brainstorming

Features under consideration (not committed, no timeline):

- **SQL query engine**: Embed DuckDB via FFI or a lightweight SQL parser for `SELECT ... FROM file WHERE ...`.
- **Lazy evaluation**: Polars-style lazy DataFrames with predicate pushdown and projection pushdown.
- **Database connectors**: PostgreSQL, MySQL direct read/write.
- **PDF export**: Convert tabular data to PDF tables.
- **YAML / TOML read**: Read as tabular with dot-path flattening.
- **Delta Lake / Iceberg**: Read modern table formats.
- **Fuzzy join**: Approximate string matching joins.
- **Isolation Forest**: Simple anomaly detection beyond statistical methods.
- **SIMD-accelerated numeric ops**: Arrow compute kernels or SIMD for parse + aggregate.
- **Pivot charts**: Charts bound to pivot tables.
- **Dynamic shell completion**: Complete column names, sheet names from actual files.
