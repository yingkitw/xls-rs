# xls-rs

`xls-rs` is a Rust CLI for reading, writing, converting, and transforming spreadsheet-like files.

- **CLI binary**: `xls-rs`
- **Rust library crate**: `xls_rs` (this is the crate name you `use` in Rust code)

Supported formats include CSV, Excel (`.xlsx`, `.xls`), ODS, Parquet, and Avro, with formula evaluation and a growing set of pandas-style operations.

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

