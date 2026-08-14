# SPEC

**Version**: 0.1.16 | **Last updated**: 2026-08-15 | **License**: Apache-2.0

## Project

`xls-rs` is **the pure-Rust XLSX toolkit**. Read, write, and manipulate Excel XLSX files with charts, styles, conditional formatting, and formula evaluation — from the shell or from Rust. No Microsoft Excel, Python, or JVM required.

## Design principles

- **Pure Rust, no external runtime** — No Excel, Python, JVM, or LibreOffice dependency.
- **Honest scope** — Practical formula subset, not a full Excel calc engine. Eager operations, not a lazy query engine. Spreadsheet-first, not a pandas replacement.
- **Production safety** — Overwrite guards, path validation, memory caps.
- **Surgical changes** — Touch only what needs changing. Match existing patterns. No speculative abstractions.

## Primary use cases

- Read specific sheets/ranges from XLSX files.
- Apply formula evaluation and transformations.
- Author styled XLSX with charts, sparklines, conditional formatting, structured tables.
- In-place Excel edits: apply formulas to ranges, write ranges with expand/preserve/overwrite semantics.
- Export tabular data to presentations (JSON, JSONL, Markdown, HTML, LaTeX).

## Capability surfaces

### Library (`xls_rs`)

- Core types: `ExcelHandler`, `Converter`, `NativeXlsxReader`, `NativeXlsxWriter`.
- Operations: `DataOperations` (sort, filter, join, concat, groupby, pivot, describe with percentiles/skewness/kurtosis, correlation (Pearson/Spearman), simple linear regression, etc.), `DataValidator`, `DataProfiler`.
- Excel-specific: `NativeXlsxWriter`, `StreamingXlsxWriter`, `WriteMode`, charts, sparklines, conditional formatting, merged cells, hyperlinks, comments, data validation, print setup, row/column grouping, freeze panes, auto-filter. XLSX writer XML generation is modular (`xml_gen.rs` with focused helpers for worksheet sections and styles, `style_registry.rs`, `cond_fmt_xml.rs`, `sparkline_xml.rs`, `chart_xml.rs`).
- Formula: `FormulaEvaluator` for in-memory evaluation of Excel expressions.

### CLI (`xls-rs`)

- Comprehensive subcommands covering I/O, transforms, analytics, and styling features.
- Global flags: `--config`, `--quiet`, `--verbose`, `--overwrite`.
- Config file resolution: `.xls-rs.toml` → `~/.xls-rs.toml` → `$XDG_CONFIG_HOME/xls-rs/config.toml`.
- Output guards: `--overwrite` required for existing files; path validation blocks `..` and embedded nulls.

## Error normalization

- Every `ErrorKind` variant has a stable string `code()` for programmatic matching (e.g., `column_not_found`, `invalid_cell_ref`, `unsupported_format`).
- CLI prints human-friendly messages.

## Scope boundaries

xls-rs is a practical toolkit, not a complete Excel engine or analytics platform:

- **Formula evaluation**: practical subset (arithmetic, comparisons, common spreadsheet functions). Not a full Excel calculation engine.
- **No lazy evaluation**: operations are eager. No query planning or predicate pushdown.
- **XLSX streaming**: `XlsxStreamingReader` and `StreamingXlsxWriter` provide row-by-row parsing and writing. Full-materialization `NativeXlsxReader` available for random access.

## Non-functional requirements

- **Rust edition**: 2024
- **Minimum supported Rust version**: 1.85+ (edition 2024)
- **Quality gates**: `cargo build` and `cargo test` must pass cleanly.
- **Cross-platform**: macOS, Linux, Windows.
- **Safety**: Overwrite guards; path traversal prevention; memory caps for malicious files.


