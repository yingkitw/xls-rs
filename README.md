# xls-rs

`xls-rs` is a Rust CLI for reading, writing, converting, and transforming spreadsheet-like files.

- **CLI binary**: `xls-rs`
- **Rust library crate**: `xls_rs` (this is the crate name you `use` in Rust code)

Supported formats include CSV, Excel (`.xlsx`, `.xls`), ODS, Parquet, and Avro, with formula evaluation and a growing set of pandas-style operations.

## What's New

- **MCP Server**: New `XlsRsMcpServer` exposes all capabilities to AI agents and automation tools via the Model Context Protocol
- **Styled Excel Export**: New presets (`default`, `minimal`, `report`, `executive`) for professional spreadsheet formatting with charts, conditional formatting, and sparklines
- **Workflow Engine**: Config-driven batch operations via `WorkflowExecutor` — no temp JSON files needed
- **Streaming Mode**: Memory-efficient processing for large CSV and Excel files (chunked reads/writes)
- **Time Series & Geospatial**: Built-in support for temporal analysis and location-based data

## Why xls-rs?

Unlike single-purpose libraries, xls-rs provides a **unified surface** across library, CLI, and MCP server — the same operations work identically everywhere.

**Key differentiators:**

- **Production Safety**: CSV formula-injection sanitization on all write paths; overwrite guards (`--overwrite` required)
- **Advanced Excel Features**: Charts, conditional formatting, sparklines, and styling — not just raw cell values
- **Data Quality Built-in**: Validation rules, profiling, anomaly detection, and data lineage tracking
- **Pandas-Style Ops**: `head`, `tail`, `describe`, `sort`, `filter`, `dedupe`, `transpose`, `select`, `join`, `concat`
- **Formula Evaluation**: Built-in evaluator for Excel formulas (not just reading stored values)
- **Encryption**: File-level encryption support for sensitive data
- **Parquet & Avro**: Native columnar format support with schema inference from headers
- **Google Sheets**: List and prepare for OAuth integration (API key support today)

## Format support (high level)

| Format | Read (library / CLI) | Write (library / CLI) | Notes |
|--------|----------------------|-------------------------|--------|
| `.csv` | Yes | Yes | Formula-injection sanitization on writes |
| `.xlsx` | Yes | Yes | Charts, conditional formatting, sparklines, etc. via library APIs |
| `.xls` | Yes | Yes | Legacy Excel; same pipeline as xlsx in many paths |
| `.ods` | Yes | Via conversion paths | OpenDocument spreadsheet |
| `.parquet` | Yes | Yes | Columnar; schema from headers when present |
| `.avro` | Yes | Yes | Columnar; field names from headers when present |
| Google Sheets (`gsheet://`, URL, ID) | Stub / API-key metadata | Stub | `list` with `google_sheets.api_key`; full read/write needs future OAuth / service account |

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

`XlsRsMcpServer` exposes tools for programmatic automation. It also includes a `capabilities` tool that returns supported operations and formats.

## Test

```bash
cargo test
```

