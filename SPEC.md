# SPEC

## Project

`xls-rs` provides:

- A **CLI** (`xls-rs`) for spreadsheet I/O, conversion, and transformations.
- A **Rust library** (`xls_rs` crate) exposing the same capabilities for programmatic use.

## Primary use cases

- Convert between formats (CSV ↔ Excel ↔ Parquet ↔ Avro).
- Read specific sheets/ranges from Excel files.
- Apply formula evaluation and transformations.
- Batch/process large files (streaming-oriented operations where possible).

## Non-functional requirements

- **Rust edition**: 2024
- **Quality gates**: `cargo build` and `cargo test` must pass.
- **Cross-platform**: should work on macOS/Linux/Windows (where dependencies allow).

