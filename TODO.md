# TODO

**Version**: 0.1.17 | **Last updated**: 2026-09-08 | **License**: Apache-2.0

## Table of Contents
- [Vision](#vision)
- [Completed work](#completed-work)
- [Maintenance backlog](#maintenance-backlog)
- [Roadmap — medium-term (v0.3–0.4)](#roadmap--medium-term-v0304)
- [Roadmap — long-term (v1.0)](#roadmap--long-term-v10)
- [Brainstorming](#brainstorming)

## Vision

xls-rs is **the pure-Rust XLSX toolkit**. Read, write, and manipulate Excel XLSX files with charts, styles, conditional formatting, and formula evaluation — from the shell or from Rust. No Microsoft Excel, Python, or JVM required.

**Design principles:**

- **Pure Rust, no external runtime** — No Excel, Python, JVM, or LibreOffice dependency.
- **Honest scope** — Practical formula subset, not a full Excel calc engine. Eager operations, not a lazy query engine. Spreadsheet-first, not a pandas replacement.
- **Production safety** — Overwrite guards, path validation, memory caps.
- **Surgical changes** — Touch only what needs changing. Match existing patterns. No speculative abstractions.

## Completed work

All items below are done and tested.

### Core format support
- Native XLSX read + write with charts, sparklines, conditional formatting, structured tables, merged cells, hyperlinks, comments, data validation, print setup, freeze panes, auto-filter, row/column grouping
- Streaming XLSX reader (`xlsx_streaming_reader.rs`) and streaming writer (`xlsx_writer/streaming.rs`)
- XLSX style reading (`xlsx_style_reader.rs`) with round-trip write→read→verify
- Template-based generation with `{{placeholder}}` cells
- LaTeX, Markdown, JSON, JSONL, HTML, and CSV presentation output formats for read/inspection commands
- **Image embedding** (v0.1.17): `insert_image` writer API with PNG/JPEG/GIF/BMP format auto-detection, EMU dimension support, unified drawing XML for charts+images, reader exposure of `xl/media/` with cell anchors
- **Rich text writing** (v0.1.17): Multi-run formatted strings (`CellData::RichText`) with per-run bold/italic/underline/font-size/font-name/color, whitespace preservation, reader concatenation
- **Worksheet & workbook protection** (v0.1.17): `SheetProtection` (per-sheet flags + Excel 16-bit password hash) and `WorkbookProtection` (lock structure/windows), read + write with round-trip
- **Document properties** (v0.1.17): `docProps/core.xml` read + write (title, creator, subject, description, keywords, category, contentStatus, created, modified, lastModifiedBy)

### Data operations
- Inspection: head, tail, sample, describe, info, dtypes, value-counts, unique
- Transformations: sort, filter, replace, dedupe, transpose, select, rename, drop, mutate, astype, clip, normalize, zscore, fillna, dropna
- Reshaping: groupby, join, concat, pivot, pivot-longer, pivot-wider, melt, rolling, crosstab
- Statistics: Pearson/Spearman/Kendall tau-b correlation, percentiles, skewness, kurtosis, simple linear regression
- Text: regex filter/replace, histograms, date parsing, diffs, string distances (Levenshtein, Jaro, Jaro-Winkler, Hamming)

### CLI
- ~50 subcommands (I/O, transforms, analytics, advanced)
- Global flags: `--config`, `--quiet`, `--verbose`, `--overwrite`
- Config discovery: `.xls-rs.toml` → `~/.xls-rs.toml` → XDG config dir
- `examples-generate` for deterministic fixtures

### Performance & safety
- Buffered I/O, streaming reader/writer for large files
- Memory caps for malicious files (dense grids, ZIP entry sizes, join/melt output, formula depth/range)
- XLSX write optimizations: zero-copy XML escaping, reusable buffers, single-sort describe
- CSV formula-injection sanitization on presentation output paths
- Path traversal prevention (`..` and embedded nulls blocked)

### Testing
- 18 test files covering all major features
- 468 tests total (184 unit + 284 integration/doc), clippy clean
- Round-trip tests: XLSX write→read→verify (values, formulas, styles, tables, images, rich text, protection, document properties)
- Golden-file tests for XLSX writer structure (`test_xlsx_writer_golden.rs`)
- Streaming vs full reader parity tests (`test_xlsx_streaming.rs`)
- CLI smoke tests against the real binary (`test_parity_smoke.rs`)

### Maintainability
- XLSX writer decomposed into focused submodules (`xml_gen`, `style_registry`, `chart_xml`, `cond_fmt_xml`, `sparkline_xml`, `image_xml`, `streaming`)
- Deduplicated outline lookup and writer initialization
- `.gitignore` aligned with generated artifacts

### Removed in 0.1.16 refocus
CSV/Parquet/Avro/ODS/XLS(BIFF) format support, Google Sheets integration, MCP server + capability catalog, plugin/workflow/geospatial/timeseries/anomaly modules, CSV index, piping mode, file watch, shell completions, and password-decryption reader APIs were removed to refocus the project as a pure XLSX toolkit. Earlier near-term deliverables in those areas (MCP transport, CSV index, piping ergonomics, anomaly-detect/resample CLI) are retired with them.

## Maintenance backlog

All items below were completed on 2026-09-08:

- [x] **`xlsx_crypto.rs` deleted**: was not declared in `excel/mod.rs` (never compiled). Decision: remove — recoverable from git history if the encryption story is revisited.
- [x] **Stale fixtures deleted**: `examples/*.avro`, `examples/*.parquet`, `test_rt_xlsx_avro_11.xlsx` removed.
- [x] **Stale names fixed**: `tests/test_xlsx_snapshots.rs` (was `test_xls_snapshots`), `examples/write_xlsx.rs`, `examples/write_rich_xlsx.rs`.
- [x] **Stale limit constants removed**: `MAX_ODS_CELL_REPEAT`, `MAX_ODS_ROW_REPEAT`, `MAX_CFB_SECTOR_HOPS` (all unused).
- [x] **`examples-generate` fixed**: was calling `Converter::convert` with CSV input (fails — Converter is XLSX-only) and generating a Parquet artifact. Now uses `ExcelHandler::write_from_csv` and generates both `sales.xlsx` and `employees.xlsx` (fresh-clone safe). Verified end-to-end in an empty directory.
- [x] **Corrupt fixture regenerated**: committed `examples/employees.xlsx` had been written by pre-refocus code with values truncated at spaces ("Alice Johnson" → "Alice") and a dropped column. Regenerated; `test_read_excel_employees_example` now asserts exact multi-word values and column counts; added `test_excel_write_from_csv_preserves_spaces_and_columns` regression.
- [x] **`common::format` tightened**: `from_extension`/`is_supported` now reflect XLSX-only scope (previously advertised csv/ods/parquet/avro/json).

## Roadmap — medium-term (v0.3–0.4)

Focus: production infrastructure and depth within the XLSX-only scope.

- [x] **Wire or delete `xlsx_crypto.rs`**: Done 2026-09-08 — deleted (was never compiled; recoverable from git history).
- [x] **Formula engine expansion — function coverage**: Done 2026-09-08. Added 14 functions: `COUNTA`, `AVERAGEIF`, `MOD` (divisor-sign semantics), `INT`, `POWER`, `SQRT`, `ROUNDUP`/`ROUNDDOWN` (true away-from-zero/toward-zero with float-epsilon guards), `UPPER`, `LOWER`, `TRIM` (whitespace-run collapse), `LEFT`, `RIGHT`, `MID`. Text functions route through `evaluate_formula_full`; numeric through `evaluate_formula_inner`. 10 tests in `tests/test_formula.rs`. Array formulas and named ranges remain open.
- [x] **Reader bug — formula cells stealing next value**: Done 2026-09-08. Cells written as `<f>` without cached `<v>` (what `xls-rs formula` produces) caused `find_open_tag("v")` to leak past `</c>` and consume the next cell's value. Value searches in both `xlsx_reader.rs` and `xlsx_streaming_reader.rs` are now bounded to the cell body (`find_open_tag_within`). Regression test: `test_formula_cell_without_cached_value_does_not_leak_next_value`.

## Maintenance backlog (new findings)

- [x] **Test artifact leak into repo root**: Done 2026-09-08. The three `unique_path`-based test files (`test_excel.rs`, `test_advanced_excel.rs`, `test_template.rs`) wrote XLSX artifacts into the CWD with manual `fs::remove_file` cleanup — a panicked test leaked `test_beyond_grid_out_1.xlsx` into the working tree. Migrated all three to per-test `tempfile::tempdir()` (auto-cleanup on drop, even on panic); dropped the global `AtomicUsize` counter (tempdir isolation makes it unnecessary). 417 tests green, clippy clean. See [[TempDir Convention for Test Artifacts]] in MEMORY.md.

- [x] **`formula --cell` beyond-grid no-op**: Done 2026-09-08. `apply_to_excel` now extends the grid to the target cell (new rows/columns padded with empties; the writer emits no XML for empty cells). Test: `test_apply_formula_beyond_existing_grid`.
- [x] **CRITICAL reader bug — every other shared string dropped**: Done 2026-09-08. `read_shared_strings` (full reader) jumped into the next `<si>` while skipping the current one, silently losing every second string on any file using a shared-strings table — i.e. **any real Excel file**. Undetected by tests because our writer emits `inlineStr` (no shared-strings table), so all self-produced fixtures sidestepped the path. Fixed both readers; si-local `<t>` search + rich-text runs now concatenated fully (previously truncated to the first run) with verbatim whitespace via `read_text_verbatim_until_close`. Committed fixtures: `tests/fixtures/shared_strings_basic.xlsx`, `shared_strings_richtext.xlsx`; tests `test_read_shared_strings_*`, `test_streaming_parity_shared_strings_fixtures`.
- [x] **Streaming rich-text runs**: Done 2026-09-08 — same bounded-search treatment; parity test covers both fixtures.
- [x] **Parallel Excel reading**: Done 2026-09-08. `XlsxReader::from_archive` buffers sheet XML sequentially (ZIP entry reads need `&mut`), then parses sheets in parallel via rayon. Guarded by `PARALLEL_SHEET_PARSE_MIN_SHEETS` (2) and `PARALLEL_SHEET_PARSE_MAX_BYTES` (128 MiB) in `limits.rs`; falls back to sequential below thresholds. Indexed `collect` preserves sheet order. Measured ~2.7× on an 8-sheet/1.92M-cell workbook (1418ms → 512ms). Test: `test_parallel_read_multi_sheet_order_and_content` (tests/test_excel.rs).
- [x] **Formula engine expansion — named ranges**: Done 2026-09-08. `XlsxReader` parses `<definedName>` entries (`defined_names()` accessor); `FormulaEvaluator::with_defined_names(HashMap)` enables `SUM(MyData)` and named single cells (`Threshold`) in formulas, including sheet-qualified/absolute refs (`T!$A$1:$A$3`). Fixed a pre-existing gap: top-level comparisons with function-led left sides (`SUM(MyData)>15`) now evaluate. Fixture: `tests/fixtures/named_ranges.xlsx`.
- [x] **Formula logic functions (AND/OR/NOT/IFERROR)**: Done 2026-09-08. Added `FormulaResult::Bool` (renders `TRUE`/`FALSE`; coerces 1/0); AND/OR/NOT compose inside `IF` conditions (condition evaluator checks them before operator scanning) and work standalone; `IFERROR` catches evaluation errors and Excel error-literal cell text. 6 tests in `tests/test_formula.rs`.
- [x] **Cached formula-value coverage**: Done 2026-09-08. Fixture `tests/fixtures/formula_cached.xlsx` covers numeric cached (`<f>..</f><v>42</v>`), string-result (`t="str"`), and uncached formula cells — real-Excel shapes the writer never produces.
- [x] **Formula engine expansion — array formulas**: Done 2026-09-08 (complete).
  - *Write/read slice*: `RowData::add_array_formula(formula, reference)` emits `<f t="array" ref="B2:B4">…</f>`; cached array results read in both readers; fixture `tests/fixtures/array_formula.xlsx`.
  - *Evaluation*: element-wise array arithmetic in aggregates — `SUM(A1:A3*B1:B3)`, `SUM((A1:A2+B1:B2)*2)`, scalar broadcasting (`A1:A3*2`), blanks/text as 0, shape-mismatch errors, precedence-aware recursive parser (`array_expr_values` in `functions.rs`). Wired into SUM/AVERAGE/COUNT/MIN/MAX with fallback to the plain range path. 7 tests in `tests/test_formula.rs`.

## Declined

- **Docker image** — declined by maintainer (2026-09-08).
- **GitHub Action / CI** — declined: no GitHub Actions CI per maintainer preference.
- **Incremental Excel append** — declined 2026-09-08: appending new ZIP entries while leaving stale worksheet/workbook parts duplicates names, and OOXML readers resolve duplicate entry names inconsistently (Excel may reject). Safe append requires rewriting the archive, which `xls-rs formula`/`write-range` already do. Data fidelity outweighs the size win.

## Roadmap — long-term (v1.0)

Focus: reach production maturity for the XLSX toolkit.

- [ ] **WebAssembly target**: Compile to WASM for browser-based spreadsheet processing.
- [ ] **REPL / interactive mode**: Drop into an interactive shell for chained operations and intermediate inspection.
- [ ] **TUI data explorer**: Terminal UI with arrow-key navigation, sorting, filtering.
- [ ] **Slicers / timelines**: Excel UI slicers connected to tables.
- [ ] **API stability audit**: Review all public types and APIs for 1.0 readiness. Lock down breaking changes.

## Brainstorming

Features under consideration (not committed, no timeline):

**Scope expansion** (would end the strict XLSX-only scope — treat as a product decision, not a given):
- **JSON / JSONLines first-class read/write**: Native read with nested flattening/unflattening.
- **SQLite read/write**: Read from `.db`/`.sqlite` and write back.
- **Apache Arrow IPC (Feather)**: `.arrow`/`.ipc` format for zero-copy Arrow ecosystem interop.
- **ODS write** (and read): Native OpenDocument spreadsheet support.
- **Database connectors**: PostgreSQL, MySQL direct read/write.

**Within scope**:
- [x] **Images (insert + read)** — *competitive intel 2026-09-08, highest-value gap.* Done in v0.1.17: `XlsxWriter::insert_image` + `XlsxReader::images()` with PNG/JPEG/GIF/BMP auto-detection, unified drawing XML, round-trip verified.
- [x] **Rich text writing** — Done in v0.1.17: `CellData::RichText(Vec<RichTextRun>)` with per-run formatting, whitespace preservation, reader concatenation.
- [x] **Worksheet & workbook protection** — Done in v0.1.17: `SheetProtection` + `WorkbookProtection` with password hashing, read + write.
- [x] **Workbook / document properties** — Done in v0.1.17: `DocumentProperties` → `docProps/core.xml` read + write.
- **Native Excel pivot tables** — excelize `AddPivotTable`; openpyxl read/write `pivotTable`. xls-rs only has in-memory pandas-style `pivot` (a transform, not an XLSX object). Native pivot tables are complex but high-value for the "author XLSX" use case.
- **Chart richness** — 3D/stacked chart subtypes, trendlines, data tables under charts, error bars (rust_xlsxwriter cookbook covers these). Extends the existing chart module.
- **SQL query engine**: Embed DuckDB via FFI or a lightweight SQL parser for `SELECT ... FROM sheet WHERE ...`.
- **Lazy evaluation**: Polars-style lazy operations with predicate pushdown.
- **PDF export**: Convert tabular data to PDF tables.
- **YAML / TOML read**: Read as tabular with dot-path flattening.
- **Delta Lake / Iceberg**: Read modern table formats.
- **Fuzzy join**: Approximate string matching joins (string_distance.rs already provides the primitives).
- **SIMD-accelerated numeric ops**: SIMD for parse + aggregate.
- **Pivot charts**: Charts bound to pivot tables.
- **Dynamic shell completion**: Complete column names, sheet names from actual files.
