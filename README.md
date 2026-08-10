# xls-rs

**Version**: 0.1.15 | **Last updated**: 2026-08-10

**The pure-Rust XLSX toolkit.** Read, write, and manipulate Excel XLSX files with charts, styles, conditional formatting, and formula evaluation — from the shell or from Rust. No Microsoft Excel, Python, or JVM required.

[![Crates.io](https://img.shields.io/crates/v/xls-rs.svg)](https://crates.io/crates/xls-rs)
[![Documentation](https://docs.rs/xls-rs/badge.svg)](https://docs.rs/xls-rs)
[![License](https://img.shields.io/crates/l/xls-rs.svg)](#license)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

```bash
cargo install xls-rs
xls-rs --help
```

## What is xls-rs?

xls-rs is a pure-Rust XLSX toolkit with two surfaces built from one codebase:

| Surface | Name | Best for |
|---|---|---|
| CLI | `xls-rs` | Shell scripts, CI/CD, spreadsheet inspection |
| Rust library | `xls_rs` | Rust services and custom pipelines |

**Core capabilities:**

1. **XLSX read and write** — Native OOXML (ZIP + XML) reader and writer. No external format crates.
2. **Rich Excel authoring** — Formulas, styles, charts, sparklines, conditional formatting, structured tables, merged cells, hyperlinks, comments, data validation, print setup, freeze panes, auto-filter, row/column grouping.
3. **Practical data operations** — Sort, filter, join, groupby, pivot, describe, correlate, and profile tabular data.

## Why use xls-rs?

- **Pure Rust XLSX reader and writer** — No dependencies on Excel, Python, or JVM.
- **Rich XLSX authoring** — Formulas, styles, charts, sparklines, conditional formatting, structured tables, merged cells, hyperlinks, comments, data validation, print setup, freeze panes, auto-filter, row/column grouping.
- **Production safety** — Overwrite guards, path traversal prevention, memory caps for malicious files.
- **CLI + library** — Same codebase, consistent behavior.

### Who is it for?

- **Rust developers** who need an Excel library with a CLI surface.
- **Analysts** who need quick spreadsheet inspection and transformation from the shell.
- **DevOps teams** needing a dependency-free spreadsheet CLI for CI/CD.

## Format support

| Format | Read | Write | Notes |
|---|---:|---:|---|
| Excel (`.xlsx`) | Yes | Yes | Full native reader and writer with advanced features |

JSON, JSONL, Markdown, HTML, and LaTeX are available as presentation output formats for read and inspection commands.

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

## Quick start

### Read Excel without Microsoft Excel

```bash
xls-rs read --input report.xlsx --sheet Sheet1 --range A1:C20 --format markdown
```

`--format` accepts `csv`, `json`, `jsonl`, `markdown`, `html`, or `latex`.

### Analyze tabular data

```bash
xls-rs describe --input sales.xlsx --format markdown
xls-rs corr --input sales.xlsx --columns Price,Quantity --method spearman
xls-rs filter --input sales.xlsx --output filtered.xlsx --where-clause "Price > 100"
```

### Use xls-rs as a Rust spreadsheet library

```rust
use xls_rs::{Converter, DataOperations};

fn main() -> anyhow::Result<()> {
    let converter = Converter::new();
    let data = converter
        .read_any_data("sales.xlsx", None)
        .expect("failed to read data");

    let summary = DataOperations::new()
        .describe(&data)
        .expect("failed to describe data");

    println!("{summary:#?}");
    Ok(())
}
```

## Capabilities

| Capability | Library | CLI |
|---|---:|---:|
| Read and write XLSX | Yes | Yes |
| Sort and filter | Yes | Yes |
| Join, groupby, pivot, melt, rolling | Yes | Yes |
| Statistics, correlation, regression | Yes | Yes |
| XLSX styles, charts, sparklines, cond. formatting | Yes | Yes |
| Validation and data-quality profiling | Yes | Yes |
| Formula evaluation | Yes | Yes |

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

| | xls-rs | Calamine | openpyxl | xlsxwriter |
|---|---|---:|---:|---:|
| Language | Rust | Rust | Python | Python |
| XLSX read | Yes | Yes | Yes | No |
| XLSX write | Yes | No | Yes | Yes |
| Charts/styles | Yes | No | Yes | Yes |
| Data operations | Yes | No | No | No |
| CLI | Yes | No | No | No |
| External deps | None | None | None | None |

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
- **Advanced:** `formula`, `apply-formula-range`, `chart`, `add-chart`, `add-sparkline`, `conditional-format`, `export-styled`, `validate`, `profile`, `schema`, `to-sql`.
- **Project:** `examples-generate`, `config-init`.

### Write-range modes

- `--mode expand` (default): write from the target cell and expand sheet bounds.
- `--mode preserve`: patch an existing workbook while keeping cells outside the range.
- `--mode overwrite`: replace the target range area.

## Configuration

Run `xls-rs config-init` to generate a configuration file. The CLI checks the first existing path:

1. `.xls-rs.toml` in the project directory.
2. `~/.xls-rs.toml`.
3. The platform configuration directory, such as `~/.config/xls-rs/config.toml` on Linux.

## Scope and limitations

xls-rs is a practical toolkit, not a complete Excel engine. Key boundaries:

- **Formula evaluation:** practical subset (arithmetic, comparisons, ~25 common functions). Not a full Excel calculation engine.
- **XLSX only:** no CSV, XLS, ODS, Parquet, Avro, or Google Sheets support.
- **Excel fidelity:** no VBA macro execution, no pivot table expansion, merged ranges expose only top-left value.
- **No lazy evaluation:** operations are eager. No query planning or predicate pushdown.

## FAQ

### Can it read XLSX without Microsoft Excel?

Yes. xls-rs parses OOXML (ZIP + XML) directly in Rust. No Office, LibreOffice, or JVM required.

### Is it an alternative to openpyxl or Calamine?

It overlaps with each but has a different scope. xls-rs differentiates by combining format read/write, data operations, and CLI in one pure-Rust crate.

### Does it support pandas-style operations?

Many familiar operations are available (groupby, join, pivot, melt, describe, correlation, sampling). It is not a drop-in pandas replacement — no lazy evaluation, no query planning.

### Minimum Rust version?

Edition 2024 requires Rust 1.85+.

## Development

See [`AGENTS.md`](AGENTS.md) for the development workflow and [`MEMORY.md`](MEMORY.md) for patterns and conventions.

```bash
cargo test                  # all unit tests + integration tests
cargo test --examples       # examples compile and run
cargo clippy --all-targets  # lint pass (warnings acceptable but noted)
```

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
