# xls-rs

`xls-rs` is a Rust CLI for reading, writing, converting, and transforming spreadsheet-like files.

- **CLI binary**: `xls-rs`
- **Rust library crate**: `xls_rs` (this is the crate name you `use` in Rust code)

Supported formats include CSV, Excel (`.xlsx`, `.xls`), ODS, Parquet, and Avro, with formula evaluation and a growing set of pandas-style operations.

## What's New

- **Extended Statistics**: `describe` now reports 10/25/50/75/90/95/99 percentiles, skewness, and kurtosis (NumPy-compatible interpolation). `corr` supports `--method spearman` for rank correlation and `--method kendall` for Kendall tau-b. `regress` computes simple linear regression (slope, intercept, r²). `str-distance` computes Levenshtein, Jaro-Winkler, and Hamming distance between strings. `sample` supports `--method stratified` and `--method systematic`.
- **Excel Grouping (Outline)**: Collapsible row/column groups via `XlsxWriter::add_row_group()` / `add_col_group()` with configurable outline level and collapse state.
- **HTML Output**: `read` and inspect commands support `--format html` for HTML table output.
- **Modified Z-score Outlier Detection**: `AnomalyMethod::ModifiedZScore` uses MAD (Median Absolute Deviation) for robust outlier detection.
- **MCP Server**: `XlsRsMcpServer` exposes all capabilities to AI agents and automation tools via the Model Context Protocol.
- **Styled Excel Export**: Presets (`default`, `minimal`, `report`, `executive`) for professional formatting with charts, conditional formatting, and sparklines.
- **Workflow Engine**: Config-driven batch operations via `WorkflowExecutor`.
- **Streaming Mode**: Memory-efficient chunked CSV processing (`CsvStreamingReader` + CLI `stream` command).
- **Time Series & Geospatial**: Temporal analysis and location-based data.

## Why xls-rs?

Unlike single-purpose libraries, xls-rs provides a **unified surface** across library, CLI, and MCP server — the same operations work identically everywhere.

**Key differentiators:**

- **Production Safety**: CSV formula-injection sanitization on all write paths; overwrite guards (`--overwrite` required); stable error codes across CLI and MCP
- **Advanced Excel Features**: Charts, conditional formatting, sparklines, grouping (outline), merged cells, hyperlinks, comments, data validation, print setup, freeze panes, auto-filter — not just raw cell values
- **Data Quality Built-in**: Validation rules, profiling, anomaly detection, and data lineage tracking
- **Pandas-Style Ops**: `head`, `tail`, `describe` (with percentiles/skewness/kurtosis), `sort`, `filter`, `dedupe`, `transpose`, `select`, `join`, `concat`, `groupby`, `pivot`, `melt`, `rolling`, `crosstab`, `sample`, `clip`, `normalize`, `zscore`, `fillna`, `dropna`, `rename`, `drop`, `mutate`, `astype`, `unique`, `value-counts`, `corr` (Pearson, Spearman & Kendall tau-b), `regress`, `str-distance` (Levenshtein, Jaro-Winkler, Hamming)
- **Formula Evaluation**: Built-in evaluator for Excel formulas (not just reading stored values)
- **Encryption**: File-level encryption/decryption support for sensitive data
- **Parquet & Avro**: Native columnar format support with schema inference from headers
- **Google Sheets**: Full read/write/append via Google Sheets API v4 when `google_sheets.access_token` is configured; list sheets with `google_sheets.api_key`
- **Text Analysis**: Keyword extraction, language detection, sentiment analysis
- **Anomaly Detection**: Statistical outlier detection on numeric columns
- **Data Lineage**: Track transformations through multi-step pipelines
- **Regex Operations**: Filter and replace by regex pattern
- **SQL Generation**: `to-sql` command generates `INSERT` statements from tabular data
- **Shell Completions**: `completions --shell <shell>` generates tab-completion scripts
- **File Watch**: `watch` feature re-runs a command when input files change

## Format support (high level)

| Format | Read (library / CLI) | Write (library / CLI) | Notes |
|--------|----------------------|-------------------------|--------|
| `.csv` | Yes | Yes | Formula-injection sanitization on writes |
| `.xlsx` | Yes | Yes | Charts, conditional formatting, sparklines, etc. via library APIs |
| `.xls` | Yes | Yes | Legacy Excel; same pipeline as xlsx in many paths |
| `.ods` | Yes | Via conversion paths | OpenDocument spreadsheet |
| `.parquet` | Yes | Yes | Columnar; schema from headers when present |
| `.avro` | Yes | Yes | Columnar; field names from headers when present |
| Google Sheets (`gsheet://`, URL, ID) | Yes (access token) | Yes (access token) | `list` with `api_key`; read/write/append with `access_token` |

For the latest parity detail across library, CLI, and MCP, see `TODO.md` and `src/capability_catalog.rs`.

### Read limitations (grid extraction)

Grid reads return tabular data similar to CSV: they do **not** execute VBA macros or expand pivot tables. **Merged cells** usually appear as a value on the top-left cell only; other cells in the merge range may be empty. For full-fidelity layout and features, open the file in the authoring application.

## Install / build

```bash
cargo build
```

## Run

```bash
cargo run -- --help
```

Example:

```bash
cargo run -- read --input examples/sales.csv
```

### Global flags

- `--config <path>`: use a specific config file (overrides discovery)
- `--quiet`: suppress non-data output (logs/progress)
- `--verbose`: print additional debug logs
- `--overwrite`: allow overwriting output files

### `read` output format

- With `-f` / `--format`: use that output (`csv`, `json`, `jsonl`, `markdown`).
- Without `--format`: uses `default_format` from the resolved config file if set; otherwise `csv`.

### `write-range` modes

- `--mode expand` (default): writes data starting at the given cell, expanding sheet bounds if needed.
- `--mode preserve`: patches an existing Excel file, keeping cells outside the target range intact.
- `--mode overwrite`: replaces the target range area directly.

### CLI command overview

**I/O**: `read`, `write`, `convert`, `sheets`, `read-all`, `write-range`, `append`

**Transforms**: `sort`, `filter`, `replace`, `dedupe`, `transpose`, `select`, `mutate`, `rename`, `drop`, `fillna`, `dropna`, `astype`, `unique`, `clip`, `normalize`, `zscore`

**Analytics**: `head`, `tail`, `sample`, `describe`, `value-counts`, `corr`, `regress`, `info`, `dtypes`, `groupby`, `join`, `concat`, `pivot`, `rolling`, `crosstab`, `melt`, `query`, `parse-date`, `regex-filter`, `regex-replace`, `diff`, `histogram`

**Advanced**: `formula`, `apply-formula-range`, `chart`, `add-chart`, `add-sparkline`, `conditional-format`, `export-styled`, `validate`, `profile`, `schema`, `to-sql`, `encrypt`, `decrypt`, `batch`, `plugin`, `stream`, `examples-generate`, `config-init`, `completions`, `gsheets-list`, `gsheets-auth`, `gsheets-set-default`

**Server**: `serve` (starts MCP server), `watch` (file watcher; requires `watch` feature)

Run `cargo run -- --help` or `xls-rs --help` for the full list.

### Generate examples

```bash
cargo run -- examples-generate
```

This creates deterministic files under `./examples/` (CSV fixtures plus derived artifacts like `sales.xlsx` and `sales.parquet`).

## Configuration

The CLI loads config from the first existing path:

- `.xls-rs.toml` (project directory)
- `~/.xls-rs.toml`
- `$XDG_CONFIG_HOME/xls-rs/config.toml`

## MCP server

`XlsRsMcpServer` exposes tools for programmatic automation via the Model Context Protocol. It delegates to the same `CapabilityRegistry` used by the CLI, so behavior is identical across surfaces. It also includes a `capabilities` tool that returns supported operations and formats at runtime.

### Error parity

CLI and MCP share the same error taxonomy. `ErrorKind` provides stable string codes (e.g., `column_not_found`, `invalid_cell_ref`, `unsupported_format`) that appear in MCP `error.data.code` for programmatic handling.

## Test

```bash
cargo test
```

### Benchmarks

```bash
cargo bench -p xls-rs --bench performance
```

