# xls-rs

**Version**: 0.1.11 | **Last updated**: 2026-08-08

<!-- SEO/GEO: Rust spreadsheet tool, XLSX writer, Excel CLI, CSV converter, MCP server -->

**xls-rs is a Rust spreadsheet CLI, library, and Model Context Protocol (MCP) server for reading, writing, converting, and analyzing Excel (XLSX/XLS), CSV, ODS, Parquet, and Avro files.** It combines pandas-style data operations, Excel formula evaluation, native XLSX authoring, streaming CSV processing, and data-quality tools — all without requiring Microsoft Excel, Python, or a JVM.

[![Crates.io](https://img.shields.io/crates/v/xls-rs.svg)](https://crates.io/crates/xls-rs)
[![Documentation](https://docs.rs/xls-rs/badge.svg)](https://docs.rs/xls-rs)
[![License](https://img.shields.io/crates/l/xls-rs.svg)](#license)
[![Rust 1.70+](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

```bash
cargo install xls-rs
xls-rs --help
```

## What is xls-rs?

xls-rs is a command-line tool and embeddable Rust crate for spreadsheet and tabular-data workflows. Use it to convert CSV to XLSX, XLSX to Parquet, Excel to CSV, or Avro to CSV; inspect and transform data from the terminal; generate styled Excel workbooks with charts and conditional formatting; or expose selected spreadsheet tools to AI agents through MCP.

| Surface | Name | Best for |
|---|---|---|
| CLI | `xls-rs` | Shell scripts, CI/CD, ETL, and interactive analysis |
| Rust library | `xls_rs` | Rust services and custom data pipelines |
| MCP server type | `XlsRsMcpServer` | AI agents and spreadsheet automation |
| Optional HTTP API | `ApiServer` with the `api` feature | Embedding spreadsheet endpoints in a Rust application |

## Why use xls-rs?

- **One Rust toolkit for common spreadsheet formats:** CSV, XLSX, XLS, ODS, Parquet, Avro, and Google Sheets.
- **Pandas-style operations from the shell:** sort, filter, join, groupby, pivot, melt, describe, correlation, regression, sampling, missing-value handling, and more.
- **Native XLSX generation:** formulas, styles, charts, sparklines, conditional formatting, merged cells, hyperlinks, comments, validation, print setup, freeze panes, and auto-filter.
- **Native XLS (BIFF8) generation:** legacy `.xls` format written from scratch using only `std` — no external dependencies.
- **Production-oriented CSV safety:** formula-injection sanitization on write paths and explicit overwrite guards.
- **Large-file support:** buffered and chunked CSV readers, writers, and streaming commands.
- **AI integration:** 18 MCP tools, including capability discovery, conversion, filtering, validation, profiling, and Excel authoring.
- **No Microsoft Excel dependency:** reads and writes use native Rust format handlers. No Office, Python, or JVM required.

### Who is it for?

- **Data engineers** converting spreadsheet and columnar formats in ETL pipelines.
- **Rust developers** who need an Excel and CSV library with a CLI surface.
- **Analysts** looking for pandas-style spreadsheet operations without Python.
- **AI-agent developers** building an MCP spreadsheet server or automation workflow.
- **DevOps teams** needing a lightweight, dependency-free spreadsheet CLI for CI/CD pipelines.

## Format support

| Format | Read | Native write | Notes |
|---|---:|---:|---|
| CSV (`.csv`) | Yes | Yes | Formula-injection sanitization on safe write paths |
| Excel (`.xlsx`) | Yes | Yes | Full native writer and advanced Excel features |
| Legacy Excel (`.xls`) | Yes | Yes | BIFF8 / OLE2 writer implemented from scratch with only `std`; supports multiple sheets, strings, numbers, booleans, basic formulas, and column widths |
| OpenDocument (`.ods`) | Yes | No | Convert ODS input to a supported writable format |
| Parquet (`.parquet`) | Yes | Yes | Apache Arrow-based columnar data |
| Avro (`.avro`) | Yes | Yes | Schema generated from tabular headers |
| Google Sheets | Yes | Yes | Access token required for read/write/append; API key supports sheet listing |

JSON, JSONL, Markdown, and HTML are available as presentation output formats for read and inspection commands. They are not first-class storage handlers.

For operation-level parity across the library, CLI, and MCP surfaces, see [`TODO.md`](TODO.md) and [`src/capability_catalog.rs`](src/capability_catalog.rs).

### XLS writer (BIFF8 / OLE2) — implemented from scratch

The legacy `.xls` format (Microsoft Compound File Binary container + BIFF8 records) is implemented in pure Rust using only the standard library. No `zip`, no external file-format crates, and no platform-specific code are used in the write path.

```rust
use xls_rs::XlsWriter;

fn main() -> anyhow::Result<()> {
    let mut w = XlsWriter::new();
    w.add_sheet("People")?;
    let mut header = XlsRowData::new();
    header.add_string("Name");
    header.add_string("Age");
    w.add_row(header);
    let mut row = XlsRowData::new();
    row.add_string("Alice");
    row.add_number(30.0);
    row.add_bool(true);
    w.add_row(row);
    w.save("people.xls")?;
    Ok(())
}
```

Currently supported: multiple sheets (31-char names, validated), strings (ASCII + UTF-16, including astral codepoints), numbers (`f64`), booleans, basic formulas (cell references, ranges, `+ - * / ^`, comparisons, function calls for SUM/AVERAGE/MIN/MAX/COUNT/IF/ABS/ROUND/IFERROR/VLOOKUP/etc.), column widths, and auto-fit. The output is verified to round-trip through our native `XlsReader` and any standard BIFF8 reader.

## Installation

### Install the CLI from crates.io

```bash
cargo install xls-rs
```

### Build from source

```bash
git clone https://github.com/yingkitw/xls-rs.git
cd xls-rs
cargo build --release
./target/release/xls-rs --help
```

The default build enables file watching and shell completions. To compile every optional surface:

```bash
cargo build --release --all-features
```

## Quick start

### Read Excel or CSV without Microsoft Excel

```bash
xls-rs read --input examples/sales.csv
xls-rs read --input report.xlsx --sheet Sheet1 --range A1:C20 --format markdown
```

`--format` accepts `csv`, `json`, `jsonl`, `markdown`, or `html`.

### Convert CSV to XLSX

```bash
xls-rs convert --input sales.csv --output sales.xlsx
```

### Convert XLSX to Parquet or CSV

```bash
xls-rs convert --input report.xlsx --output report.parquet
xls-rs convert --input report.xlsx --output report.csv --sheet Sheet1
```

### Analyze tabular data

```bash
xls-rs describe --input sales.csv --format markdown
xls-rs corr --input sales.csv --columns Price,Quantity --method spearman
xls-rs regress --input sales.csv --x-column Price --y-column Quantity
xls-rs filter --input sales.csv --output premium.csv --where-clause "Price > 100"
```

### Use xls-rs as a Rust spreadsheet library

```rust
use xls_rs::DataOperations;

fn main() -> anyhow::Result<()> {
    let converter = Converter::new();
    let data = converter
        .read_any_data("sales.csv", None)
        .expect("failed to read data");

    let summary = DataOperations::new()
        .describe(&data)
        .expect("failed to describe data");

    println!("{summary:#?}");
    Ok(())
}
```

Convert between supported formats with the library API:

```rust
use xls_rs::Converter;

fn main() -> anyhow::Result<()> {
    Converter::new()
        .convert("sales.csv", "sales.xlsx", None)
        .expect("conversion failed");
    Ok(())
}
```

## Capabilities

| Capability | Library | CLI | MCP |
|---|---:|---:|---:|
| Read and convert tabular files | Yes | Yes | Selected tools |
| Sort and filter | Yes | Yes | Yes |
| Join, groupby, pivot, melt, and rolling operations | Yes | Yes | Workflow only |
| Descriptive statistics, correlation, and regression | Yes | Yes | No |
| XLSX styles, charts, sparklines, and conditional formatting | Yes | Yes | Yes |
| Validation and data-quality profiling | Yes | Yes | Yes |
| Chunked CSV streaming | Yes | Yes | Yes |
| Anomaly, time-series, geospatial, lineage, and text analysis | Yes | No | No |
| Google Sheets read/write/append | Yes | Yes (generic I/O commands) | No |

### Pandas-style operations

The CLI and `DataOperations` API include:

- Inspection: `head`, `tail`, `sample`, `describe`, `info`, `dtypes`, `value-counts`, `unique`.
- Transformations: `sort`, `filter`, `replace`, `dedupe`, `transpose`, `select`, `rename`, `drop`, `mutate`, `astype`, `clip`, `normalize`, `zscore`, `fillna`, `dropna`.
- Reshaping and combining: `groupby`, `join`, `concat`, `pivot`, `melt`, `rolling`, `crosstab`.
- Statistics: Pearson, Spearman, and Kendall tau-b correlation; percentiles; skewness; kurtosis; and simple linear regression.
- Text and utility operations: regex filtering/replacement, histograms, date parsing, diffs, and Levenshtein, Jaro, Jaro-Winkler, and Hamming distances.

### Advanced XLSX authoring

The native XLSX writer supports:

- Cell formulas and typed values.
- Styles and styled export presets: `default`, `minimal`, `report`, and `executive`.
- Column, bar, line, area, pie, doughnut, and scatter charts.
- Sparklines and conditional formatting.
- Row and column grouping, merged cells, hyperlinks, comments, and data validation.
- Freeze panes, auto-filter, print areas, margins, orientation, scale, and fit-to-page settings.

### Comparison with alternatives

| Feature | xls-rs | Calamine | openpyxl | xsv | Polars | xlsxwriter |
|---|---|---:|---:|---:|---:|---:|
| Language | Rust | Rust | Python | Rust | Rust | Python |
| XLSX read | Yes | Yes | Yes | No | No | No |
| XLSX write | Yes | No | Yes | No | No | Yes |
| XLS (BIFF8) write | Yes | No | No | No | No | No |
| CSV read/write | Yes | No | No | Yes | Yes | No |
| Parquet/Avro | Yes | No | No | No | Yes | No |
| Charts/styles | Yes | No | Yes | No | No | Yes |
| Pandas-style ops | Yes | No | No | Limited | Yes | No |
| CLI | Yes | No | No | Yes | Yes | No |
| MCP server | Yes | No | No | No | No | No |
| Excel dependency | None | None | None | None | None | None |

### Common use cases

- **CSV to Excel conversion:** `xls-rs convert --input data.csv --output report.xlsx`
- **Excel to Parquet for analytics:** `xls-rs convert --input report.xlsx --output data.parquet`
- **Generate styled XLSX with charts:** Use `XlsxWriter` in Rust or `xls-rs export-styled` CLI
- **Filter and sort spreadsheet data:** `xls-rs filter --input sales.csv --output filtered.csv --where-clause "Revenue > 1000"`
- **Read Excel ranges:** `xls-rs read --input report.xlsx --sheet Sheet1 --range A1:D20 --format json`
- **AI agent spreadsheet automation:** Embed `XlsRsMcpServer` in a Tokio/RMCP host for LLM-driven Excel workflows
- **ETL pipeline spreadsheet processing:** Use the library API in Rust services for format conversion and data transformation

## CLI reference

### Global flags

- `--config <path>`: use a specific configuration file.
- `--quiet`: suppress non-data logs and progress output.
- `--verbose`: print additional diagnostics.
- `--overwrite`: allow destructive output replacement; place it before the subcommand.

### Command groups

- **I/O:** `read`, `write`, `convert`, `sheets`, `read-all`, `write-range`, `append`.
- **Transforms:** `sort`, `filter`, `replace`, `dedupe`, `transpose`, `select`, `mutate`, `rename`, `drop`, `fillna`, `dropna`, `astype`, `unique`, `clip`, `normalize`, `zscore`.
- **Analytics:** `head`, `tail`, `sample`, `describe`, `value-counts`, `corr`, `regress`, `info`, `dtypes`, `groupby`, `join`, `concat`, `pivot`, `rolling`, `crosstab`, `melt`, `query`, `parse-date`, `regex-filter`, `regex-replace`, `diff`, `histogram`, `str-distance`.
- **Advanced:** `formula`, `apply-formula-range`, `chart`, `add-chart`, `add-sparkline`, `conditional-format`, `export-styled`, `validate`, `profile`, `schema`, `to-sql`, `encrypt`, `decrypt`, `batch`, `plugin`, `stream`.
- **Project and integration:** `examples-generate`, `config-init`, `completions`, `watch`, `gsheets-list`, `gsheets-auth`, `gsheets-set-default`, `serve`.

`query` currently provides WHERE-style filtering rather than a general SQL engine. Run `xls-rs <command> --help` for command-specific arguments.

### Write-range modes

- `--mode expand` (default): write from the target cell and expand sheet bounds.
- `--mode preserve`: patch an existing workbook while keeping cells outside the range.
- `--mode overwrite`: replace the target range area.

## MCP spreadsheet server

`XlsRsMcpServer` exposes 18 Model Context Protocol tools. Each tool delegates to the shared capability registry for consistent request handling and errors.

Available tools:

- `capabilities`, `read_excel`, `read_all_sheets`, and `list_sheets`.
- `convert_data`, `sort_data`, `filter_data`, and `execute_workflow`.
- `write_styled`, `add_chart`, `add_sparkline`, and `conditional_format`.
- `apply_formula`, `validate_data`, `profile_data`, and `stream_data`.
- `encrypt_file` and `batch_process`.

The `xls-rs serve` command is currently an informational placeholder; it does not start an MCP transport. Embed the server type in a Tokio/RMCP host and choose the transport required by your agent runtime:

```rust
use xls_rs::XlsRsMcpServer;

fn main() {
    let _server = XlsRsMcpServer::new();
}
```

CLI and MCP operations share stable error codes such as `column_not_found`, `invalid_cell_ref`, and `unsupported_format`.

## Configuration

Run `xls-rs config-init` to generate a configuration file. The CLI checks the first existing path:

1. `.xls-rs.toml` in the project directory.
2. `~/.xls-rs.toml`.
3. The platform configuration directory, such as `~/.config/xls-rs/config.toml` on Linux.

Google Sheets read/write/append requires `google_sheets.access_token`. Sheet-title listing can use `google_sheets.api_key`.

## Performance and large files

xls-rs includes buffered CSV I/O, chunked streaming, a bounded-memory `tail`, cached cell-reference matching, and reduced-allocation XLSX generation. Criterion benchmarks cover XLSX read/write, range reads, CSV-to-Parquet conversion, streaming tail, and formula-derived columns.

```bash
cargo bench --bench performance
```

For large CSV files, use `xls-rs stream` or the `CsvStreamingReader` API. XLSX reads currently materialize worksheets through our native reader.

## Current limitations

- **MCP hosting:** `xls-rs serve` does not yet launch a transport; embed `XlsRsMcpServer` in an async host.
- **Legacy XLS writes:** native `.xls` (BIFF8) output is implemented from scratch using only `std`. `.ods` output is not implemented; writer routing may accept `.ods` but emits XLSX content, so write `.xlsx` instead.
- **Encryption:** `EncryptionAlgorithm::Aes256` currently delegates to the XOR test implementation. Do not use the encryption API for production security.
- **Excel fidelity:** grid reads do not execute VBA macros or expand pivot tables. Merged ranges usually expose only the top-left value.
- **Sheet enumeration:** `sheets` and `read-all` currently use the XLSX-specific reader path; XLS sheet enumeration uses the native BIFF8 reader.
- **Streaming:** CSV supports chunked processing; XLSX does not yet provide a true row-by-row SAX reader.
- **Formula coverage:** the built-in evaluator supports a practical subset of Excel formulas, not the complete Excel calculation engine.
- **Surface parity:** advanced library analytics are not all exposed through CLI and MCP.

## FAQ

### Can xls-rs convert CSV to Excel XLSX?

Yes. Run `xls-rs convert --input data.csv --output data.xlsx` or call `Converter::convert` from Rust. The XLSX writer supports styles, charts, formulas, freeze panes, auto-filter, and more.

### Can xls-rs read XLSX files without Microsoft Excel?

Yes. xls-rs reads Excel files natively in Rust and does not require Office, LibreOffice, or a JVM. It parses the OOXML (ZIP + XML) format directly.

### Can xls-rs write XLS (legacy Excel) files?

Yes. xls-rs includes a from-scratch BIFF8/OLE2 writer implemented in pure Rust using only the standard library. It supports multiple sheets, strings (UTF-16), numbers, booleans, formulas, and column widths.

### Is xls-rs an alternative to openpyxl, Calamine, xsv, or Polars?

It overlaps with each tool but has a different scope. Compared with Calamine or other read-only libraries, xls-rs adds native XLSX writing, conversion, analytics, CLI, and MCP surfaces. Compared with openpyxl, it is Rust-native and supports Parquet and Avro, but has less template and macro fidelity. Compared with Polars or xsv, it focuses more on spreadsheet formats and Excel authoring than on a full lazy query engine. See the [comparison table](#comparison-with-alternatives) above.

### Does xls-rs support pandas-style DataFrame operations?

It supports many familiar tabular operations, including groupby, join, pivot, melt, describe, correlation, sampling, and missing-value handling. It is not a drop-in pandas DataFrame implementation and does not yet include lazy query planning.

### Can xls-rs create Excel charts and conditional formatting?

Yes. The XLSX writer supports bar, column, line, area, pie, doughnut, and scatter charts, plus color scales, data bars, icon sets, and formula-based conditional formatting. Sparklines (line, column, win/loss) are also supported.

### Is xls-rs suitable for ETL pipelines?

Yes. The CLI is designed for shell scripts and CI/CD with `--overwrite` guards, `--quiet` mode, and stdin/stdout support. The library API works well in Rust services for format conversion and data transformation.

### Is the MCP server ready to use from the CLI?

The MCP tool implementation is available through `XlsRsMcpServer`, but the `serve` subcommand does not yet host a transport. Embed the type in an RMCP/Tokio application.

### Is the encryption feature secure?

No. The current `Aes256` enum path uses the XOR test implementation and must not be used to protect sensitive data.

### What is the minimum supported Rust version?

The current package metadata requires Rust 1.88 or newer. Edition 2024.

## Development

See [`AGENTS.md`](AGENTS.md) for the development workflow and [`MEMORY.md`](MEMORY.md) for patterns and conventions.

```bash
cargo test                  # all unit tests + integration tests
cargo test --examples       # examples compile and run
cargo clippy --all-targets --all-features                # lint pass (warnings acceptable but noted)
cargo bench --bench performance  # performance benchmarks
```

### Contributing

We welcome contributions! Please see [`AGENTS.md`](AGENTS.md) for development guidelines and [`MEMORY.md`](MEMORY.md) for proven patterns. All changes should include tests and follow the established conventions for code style, error handling, and documentation.

Additional project documentation:

- [Architecture](ARCHITECTURE.md)
- [Technical specification](SPEC.md)
- [Roadmap and capability parity](TODO.md)
- [Development patterns and memory](MEMORY.md)
- [Agent development workflow](AGENTS.md)

## License

Licensed under the Apache License 2.0. The SPDX license declaration is defined in [`Cargo.toml`](Cargo.toml).

## Links

- [Source repository](https://github.com/yingkitw/xls-rs)
- [Crate on crates.io](https://crates.io/crates/xls-rs)
- [API documentation on docs.rs](https://docs.rs/xls-rs)
- [Issue tracker](https://github.com/yingkitw/xls-rs/issues)

---

<details>
<summary>Keywords</summary>

Rust spreadsheet library, Rust XLSX writer, Rust Excel reader, CSV converter, XLSX to Parquet, CSV to Excel, Rust Excel CLI, MCP spreadsheet server, BIFF8 writer, XLS writer Rust, pandas-style operations Rust, Excel without Microsoft Office, Rust data analysis CLI, spreadsheet ETL Rust, conditional formatting Rust, Excel charts Rust, sparklines Rust, Avro converter, ODS reader Rust, Google Sheets API Rust, Rust tabular data toolkit, openpyxl alternative Rust, Calamine alternative, xsv alternative, Polars spreadsheet, xlsxwriter Rust equivalent.

</details>
