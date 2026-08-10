# xls-rs

**Version**: 0.1.14 | **Last updated**: 2026-08-10

<!-- SEO/GEO: Rust spreadsheet toolkit, XLSX writer, Excel CLI, CSV converter, MCP server -->

**The pure-Rust spreadsheet toolkit.** Read, write, and convert XLSX, XLS, CSV, ODS, Parquet, and Avro — from the shell, from Rust, or from AI agents. No Microsoft Excel, Python, or JVM required.

[![Crates.io](https://img.shields.io/crates/v/xls-rs.svg)](https://crates.io/crates/xls-rs)
[![Documentation](https://docs.rs/xls-rs/badge.svg)](https://docs.rs/xls-rs)
[![License](https://img.shields.io/crates/l/xls-rs.svg)](#license)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

```bash
cargo install xls-rs
xls-rs --help
```

## What is xls-rs?

xls-rs is a spreadsheet toolkit with three surfaces built from one Rust codebase:

| Surface | Name | Best for |
|---|---|---|
| CLI | `xls-rs` | Shell scripts, CI/CD, format conversion |
| Rust library | `xls_rs` | Rust services and custom pipelines |
| MCP server | `XlsRsMcpServer` | AI agents and spreadsheet automation |

**Three pillars:**

1. **Format mastery** — Native read and write for XLSX (OOXML), XLS (BIFF8 from scratch in pure `std`), CSV, Parquet, and Avro. Read-only for ODS. Google Sheets via API v4.
2. **Format conversion** — Bridge between spreadsheet and columnar formats: CSV ↔ XLSX ↔ Parquet ↔ Avro. One command, one API call.
3. **Practical data operations** — Sort, filter, join, groupby, pivot, describe, correlate, and profile tabular data. Enough for shell-based analysis without leaving the terminal.

**What it is not:** xls-rs is not a pandas replacement (no lazy evaluation), not a SQL engine (WHERE-style filtering only), and not a full Excel calculation engine (practical formula subset). It is a fast, dependency-free spreadsheet toolkit that excels at format conversion, Excel authoring, and lightweight data inspection.

## Why use xls-rs?

- **Only Rust crate that writes both XLSX and XLS** — XLS (BIFF8/OLE2) is implemented from scratch in pure `std`, no external format crates.
- **Three surfaces, one codebase** — CLI, Rust library, and MCP server all delegate to the same capability registry. Consistent behavior, errors, and defaults everywhere.
- **Format bridge** — Convert CSV ↔ XLSX ↔ Parquet ↔ Avro in one command. Read ODS. Read/write Google Sheets via API v4.
- **Rich XLSX authoring** — Formulas, styles, charts, sparklines, conditional formatting, structured tables, merged cells, hyperlinks, comments, data validation, print setup, freeze panes, auto-filter, row/column grouping.
- **Password-protected XLSX decryption** — AES-256-CBC (MS-OFFCRYPTO Agile Encryption) via the `password` feature.
- **Production safety** — CSV formula-injection sanitization on all write paths, overwrite guards, path traversal prevention, memory caps for malicious files.
- **No dependencies on Excel, Python, or JVM** — Pure Rust format handlers. Works in CI/CD, containers, and serverless.

### Who is it for?

- **Data engineers** converting between spreadsheet and columnar formats in ETL pipelines.
- **Rust developers** who need an Excel/CSV library with a CLI surface.
- **Analysts** who need quick spreadsheet inspection and transformation from the shell.
- **AI-agent developers** building MCP spreadsheet automation workflows.
- **DevOps teams** needing a dependency-free spreadsheet CLI for CI/CD.

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

JSON, JSONL, Markdown, HTML, and LaTeX are available as presentation output formats for read and inspection commands. They are not first-class storage handlers.

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

The default build enables file watching, shell completions, MCP, Parquet, Avro, and Google Sheets support. To compile every optional surface:

```bash
cargo build --release --all-features
```

Available feature flags:

| Feature | Description |
|---|---|
| `watch` | File watch mode (default) |
| `completions` | Shell completions generation (default) |
| `mcp` | MCP server type for AI agents (default) |
| `parquet` | Parquet read/write via Apache Arrow (default) |
| `avro` | Avro read/write (default) |
| `gsheets` | Google Sheets API integration (default) |
| `password` | Password-protected XLSX decryption (AES-256-CBC) |

## Quick start

### Read Excel or CSV without Microsoft Excel

```bash
xls-rs read --input examples/sales.csv
xls-rs read --input report.xlsx --sheet Sheet1 --range A1:C20 --format markdown
```

`--format` accepts `csv`, `json`, `jsonl`, `markdown`, `html`, or `latex`.

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
| Read and convert tabular files | Yes | Yes | Yes |
| Sort and filter | Yes | Yes | Yes |
| Join, groupby, pivot, melt, rolling | Yes | Yes | Workflow only |
| Statistics, correlation, regression | Yes | Yes | No |
| XLSX styles, charts, sparklines, cond. formatting | Yes | Yes | Yes |
| Validation and data-quality profiling | Yes | Yes | Yes |
| Chunked CSV streaming | Yes | Yes | Yes |
| Anomaly, time-series, geospatial, text analysis | Yes | No | No |
| Google Sheets read/write/append | Yes | Yes | No |

### Data operations

The CLI and `DataOperations` API provide practical tabular operations:

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
- Structured tables (Excel Table objects) with auto-expanding ranges, banded rows, and table styles.
- Row and column grouping, merged cells, hyperlinks, comments, and data validation.
- Freeze panes, auto-filter, print areas, margins, orientation, scale, and fit-to-page settings.

### How it compares

| | xls-rs | Calamine | openpyxl | xlsxwriter | xsv | Polars |
|---|---|---:|---:|---:|---:|---:|
| Language | Rust | Rust | Python | Python | Rust | Rust |
| XLSX read | Yes | Yes | Yes | No | No | No |
| XLSX write | Yes | No | Yes | Yes | No | No |
| XLS (BIFF8) write | Yes | No | No | No | No | No |
| CSV read/write | Yes | No | No | No | Yes | Yes |
| Parquet/Avro | Yes | No | No | No | No | Yes |
| Charts/styles | Yes | No | Yes | Yes | No | No |
| Data operations | Yes | No | No | No | Limited | Yes |
| CLI | Yes | No | No | No | Yes | Yes |
| MCP server | Yes | No | No | No | No | No |
| External deps | None | None | None | None | None | None |

**Positioning:** xls-rs occupies the intersection of spreadsheet format libraries (Calamine, openpyxl, xlsxwriter) and data tools (xsv, Polars). It is not as deep as any single tool in that tool's specialty — Calamine reads more Excel edge cases, Polars has a full lazy query engine, openpyxl has richer template support. xls-rs differentiates by combining format read/write, conversion, CLI, and MCP in one pure-Rust crate with no external runtime dependencies.

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
- **Analytics:** `head`, `tail`, `sample`, `describe`, `value-counts`, `corr`, `regress`, `info`, `dtypes`, `groupby`, `join`, `concat`, `pivot`, `pivot-longer`, `pivot-wider`, `rolling`, `crosstab`, `melt`, `query`, `parse-date`, `regex-filter`, `regex-replace`, `diff`, `histogram`, `str-distance`.
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

## Scope and limitations

xls-rs is a practical toolkit, not a complete Excel engine. Key boundaries:

- **Formula evaluation:** practical subset (arithmetic, comparisons, ~25 common functions). Not a full Excel calculation engine.
- **XLSX streaming:** CSV supports chunked processing; XLSX reads materialize the whole sheet (no SAX-style row reader yet).
- **MCP hosting:** `xls-rs serve` does not yet launch a transport. Embed `XlsRsMcpServer` in an async host.
- **ODS write:** not implemented. Writer routing may accept `.ods` but emits XLSX content — write `.xlsx` instead.
- **Password-protected XLSX:** decryption (reading) supported via `password` feature. Encryption (writing) not yet supported.
- **Excel fidelity:** no VBA macro execution, no pivot table expansion, merged ranges expose only top-left value.
- **Surface parity:** advanced library analytics (anomaly, time-series, geospatial, text analysis) are not exposed through CLI or MCP.
- **No lazy evaluation:** operations are eager. No query planning or predicate pushdown.

## FAQ

### Can xls-rs convert CSV to Excel?

Yes. `xls-rs convert --input data.csv --output data.xlsx` or `Converter::convert` from Rust.

### Can it read XLSX without Microsoft Excel?

Yes. xls-rs parses OOXML (ZIP + XML) directly in Rust. No Office, LibreOffice, or JVM required.

### Can it write legacy `.xls` files?

Yes. The BIFF8/OLE2 writer is implemented from scratch in pure `std`. Supports multiple sheets, strings (UTF-16), numbers, booleans, formulas, and column widths.

### Is it an alternative to openpyxl, Calamine, xsv, or Polars?

It overlaps with each but has a different scope. See the [comparison table](#how-it-compares) above. xls-rs differentiates by combining format read/write, conversion, CLI, and MCP in one pure-Rust crate.

### Does it support pandas-style operations?

Many familiar operations are available (groupby, join, pivot, melt, describe, correlation, sampling). It is not a drop-in pandas replacement — no lazy evaluation, no query planning.

### Is the MCP server ready?

The tool implementation is available via `XlsRsMcpServer`. The `serve` subcommand does not yet host a transport — embed the type in an RMCP/Tokio application.

### Minimum Rust version?

Edition 2024 requires Rust 1.85+.

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

Rust spreadsheet toolkit, Rust XLSX writer, Rust Excel reader, CSV converter, XLSX to Parquet, CSV to Excel, Rust Excel CLI, MCP spreadsheet server, BIFF8 writer, XLS writer Rust, Excel without Microsoft Office, spreadsheet ETL Rust, conditional formatting Rust, Excel charts Rust, sparklines Rust, Avro converter, ODS reader Rust, Google Sheets API Rust, Rust tabular data toolkit, openpyxl alternative Rust, Calamine alternative, xsv alternative, xlsxwriter Rust equivalent, LaTeX table export Rust, password-protected XLSX Rust, pure Rust spreadsheet, format conversion Rust.

</details>
