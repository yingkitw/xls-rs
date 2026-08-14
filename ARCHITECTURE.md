# ARCHITECTURE

**Version**: 0.1.16 | **Last updated**: 2026-08-15 | **License**: Apache-2.0

## Table of Contents
- [High level](#high-level)
- [Design decisions](#design-decisions)
- [Key modules](#key-modules)
- [Data flow](#data-flow)
- [Testing layout](#testing-layout)

## High level

xls-rs is **the pure-Rust XLSX toolkit**. It provides two surfaces from one codebase:

- **CLI binary**: `xls-rs` (`src/main.rs`, clap definitions in `src/cli/`)
- **Library crate**: `xls_rs` (`src/lib.rs`)

The CLI delegates command execution to domain handlers under `src/cli/commands/` and uses library modules for the underlying spreadsheet and data operations. Both surfaces share identical behavior, error codes, and defaults.

## Design decisions

- **Native XLSX Reader & Writer in pure Rust**: Full OOXML (ZIP + XML) reader and writer without heavy runtime dependencies.
- **Rich XLSX Authoring**: Styles, charts, conditional formatting, sparklines, structured tables, merged cells, hyperlinks, comments, print setup, freeze panes, auto-filter.
- **Eager operations**: All data operations are eager (no lazy query planning). This keeps the codebase simple, lean, and predictable.
- **Memory safety caps**: `src/limits.rs` enforces hard caps on cell counts, range dimensions, formula depth, and string distance lengths to prevent resource exhaustion attacks.
- **Modular XML Generation**: `src/excel/xlsx_writer/` splits XML generation into dedicated submodules (`xml_gen.rs`, `style_registry.rs`, `chart_xml.rs`, `cond_fmt_xml.rs`, `sparkline_xml.rs`, `streaming.rs`).

## Key modules

### Excel Layer (`src/excel/`)

- `src/excel/xlsx_reader.rs`: `NativeXlsxReader` — reads sheets, cells, dimensions, shared strings, and tables.
- `src/excel/xlsx_streaming_reader.rs`: `XlsxStreamingReader` — streaming row-by-row XML parser for large files.
- `src/excel/xlsx_writer/`: Modular writer generating valid OOXML spreadsheets with styles, charts, sparklines, tables, conditional formats, and streaming support.
- `src/excel/xlsx_style_reader.rs`: Parses and inspects cell styles and number formats.
- `src/excel/reader.rs` / `src/excel/writer.rs`: `ExcelHandler` high-level entry points.
- `src/excel/cell_typer.rs`: Fast cell type classification.
- `src/excel/chart.rs`: Chart definitions and configuration.
- `src/excel/template/`: Template processing with `{{placeholder}}` token replacement.

### Operations Layer (`src/operations/`)

- `src/operations/core.rs`: Filtering, column selection, sorting, renaming, type casting, deduplication.
- `src/operations/pandas.rs`: GroupBy, joins (inner/left), pivot (wider/longer), melt, crosstab, transpose.
- `src/operations/stats.rs`: Descriptive statistics (mean, median, variance, std dev, min, max, percentiles, skewness, kurtosis), correlation (Pearson, Spearman, Kendall Tau), simple linear regression, z-score.
- `src/operations/transform.rs`: Fillna, dropna, string operations, numeric operations, sampling.

### Formula & Profiling Layer

- `src/formula/`: Parser (`parser.rs`), evaluator (`evaluator.rs`), and spreadsheet functions (`functions.rs`).
- `src/profiling/`: Column profiling, statistical summaries, data quality scores.
- `src/quality.rs`: Data quality issue checks and accuracy scoring.
- `src/validation.rs`: Data validation rule definitions and schema evaluation.

### Support & CLI Layer

- `src/cli/`: Clap CLI parser, runtime execution context, output formatting (table, CSV, JSON, Markdown, HTML, LaTeX), and command handlers (`src/cli/commands/`).
- `src/error.rs` / `src/error_traits.rs`: Typed `XlsError` / `ErrorKind` hierarchy with stable error codes.
- `src/config.rs`: TOML config loader (`.xls-rs.toml`).
- `src/limits.rs`: Safety limits and memory caps.
- `src/types.rs`: Core types (`Cell`, `Row`, `Sheet`, `Workbook`, `CellValue`).
- `src/traits.rs`: Core traits (`DataReader`, `DataWriter`, `DataOperator`, `CellRangeProvider`).

## Data flow

```
CLI command (xls-rs <subcommand>)
     │
     ▼
src/cli/handler.rs (CommandHandler)
     │
     ▼
src/cli/commands/ (io, pandas, transform, advanced)
     │
     ▼
xls-rs core library (ExcelHandler, DataOperations, FormulaEvaluator)
     │
     ▼
Native XLSX Reader / Writer / Streaming Engine
```

## Testing layout

- `tests/`: Integration tests covering Excel reading/writing, formula parsing/evaluation, parity, streaming, styles, tables, templates, and operations.
- `tests/common/mod.rs`: Shared test fixture helpers.
- `src/`: Unit tests inline in respective modules.


