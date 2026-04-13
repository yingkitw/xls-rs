# ARCHITECTURE

## High level

This repository builds:

- **CLI binary**: `xls-rs` (`src/main.rs`, clap definitions in `src/cli/`)
- **Library crate**: `xls_rs` (`src/lib.rs`)

The CLI delegates command execution to domain handlers under `src/cli/commands/` and uses the library modules for actual implementations.

## Key modules

- `src/cli/`: clap definitions + command handlers
- `src/excel/`: Excel read/write logic (including XLSX writer)
- `src/columnar/`: Parquet/Avro handlers
- `src/operations/`: pandas-style operations (sort/filter/groupby/etc.)
- `src/formula/`: formula parsing/evaluation
- `src/config.rs`: config discovery + default config template

## Testing layout

- `tests/`: integration tests
- `tests/common/mod.rs`: shared paths + example fixture creation for tests

