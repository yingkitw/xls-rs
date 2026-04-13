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

## Configuration

The CLI loads config from the first existing path:

- `.xls-rs.toml` (project directory)
- `~/.xls-rs.toml`
- `$XDG_CONFIG_HOME/xls-rs/config.toml`

## Test

```bash
cargo test
```

