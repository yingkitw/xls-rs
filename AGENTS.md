# Agent Development Loop

**Version**: 0.1.11 | **Last updated**: 2026-08-08 | **License**: Apache-2.0

This document defines the continuous improvement cycle for the **xls-rs** crate — a pure-Rust spreadsheet CLI, library, and MCP server for reading, writing, converting, and analyzing XLSX, XLS, CSV, ODS, Parquet, and Avro files.

## Project Structure

```
.
├── src/
│   ├── lib.rs              # crate root, module declarations, public API re-exports
│   ├── main.rs             # CLI entry point (clap dispatch)
│   ├── types.rs            # core data types: Cell, Row, Sheet, Workbook, CellValue
│   ├── error.rs            # XlsError + Result alias
│   ├── error_traits.rs     # error trait implementations for std/serde
│   ├── common.rs           # shared utilities, format-agnostic helpers
│   ├── config.rs           # TOML config loading, CLI option overrides
│   ├── helpers.rs          # misc helper functions
│   ├── limits.rs           # resource limits and safety bounds
│   ├── traits.rs           # trait definitions (DataHandler, etc.)
│   ├── handler_registry.rs # format handler registry and dispatch
│   ├── format_detector.rs  # file format detection by extension/content
│   ├── validation.rs       # data validation rules, schema checks
│   ├── quality.rs          # data quality scoring and reporting
│   ├── streaming.rs        # streaming reader/writer for large files
│   ├── streaming_ops.rs    # streaming-aware data operations
│   ├── converter.rs        # cross-format conversion logic
│   ├── csv_handler.rs      # CSV reader/writer with type inference
│   ├── google_sheets.rs    # Google Sheets API integration
│   ├── encryption.rs       # file encryption/decryption support
│   ├── geospatial.rs       # geospatial data helpers
│   ├── timeseries.rs       # time-series analysis and resampling
│   ├── string_utils.rs     # string manipulation utilities
│   ├── string_distance.rs  # fuzzy matching and edit distance
│   ├── regex_cache.rs      # compiled regex cache for performance
│   ├── anomaly.rs          # anomaly detection in data
│   ├── plugins.rs          # plugin system
│   ├── workflow.rs         # multi-step workflow orchestration
│   ├── mcp.rs              # MCP server implementation (rmcp)
│   ├── mcp_enrichment.rs   # MCP capability enrichment
│   ├── capability_catalog.rs  # capability catalog for MCP/CLI
│   ├── excel/              # Excel format readers/writers
│   │   ├── mod.rs          # module root
│   │   ├── reader.rs       # generic Excel reader dispatch
│   │   ├── writer.rs       # generic Excel writer dispatch
│   │   ├── xlsx_reader.rs  # XLSX reader (OOXML, zip-based)
│   │   ├── xlsx_writer/    # XLSX writer submodules (styles, charts, etc.)
│   │   ├── xls_reader/     # XLS (BIFF) reader submodules
│   │   ├── xls_writer/     # XLS (BIFF) writer submodules
│   │   ├── ods_reader.rs   # ODS (OpenDocument) reader
│   │   ├── cell_typer.rs   # cell type inference
│   │   ├── chart.rs        # chart definitions
│   │   ├── feature_detector.rs  # Excel feature detection (charts, styles, etc.)
│   │   ├── template/       # template handling
│   │   └── types.rs        # Excel-specific types
│   ├── formula/            # formula engine
│   │   ├── mod.rs          # module root
│   │   ├── parser.rs       # formula parser (A1/R1C1, functions, operators)
│   │   ├── evaluator.rs    # formula evaluator with cell references
│   │   ├── functions.rs    # built-in spreadsheet functions
│   │   └── types.rs        # formula AST types
│   ├── operations/         # pandas-style data operations
│   │   ├── mod.rs          # module root
│   │   ├── core.rs         # core operations (filter, select, rename)
│   │   ├── pandas.rs       # pandas-style API (groupby, merge, pivot)
│   │   ├── stats.rs        # statistical operations (describe, correlation)
│   │   ├── transform.rs    # data transforms (apply, map, fillna)
│   │   ├── diff.rs         # row/column diff
│   │   ├── histogram.rs    # histogram binning
│   │   └── types.rs        # operation types
│   ├── capabilities/       # MCP capability definitions
│   │   ├── mod.rs          # module root, registry
│   │   ├── registry.rs     # capability registry
│   │   ├── core.rs         # core capabilities
│   │   ├── batch.rs        # batch operations
│   │   ├── convert.rs      # conversion capabilities
│   │   ├── encrypt.rs      # encryption capabilities
│   │   ├── excel_read.rs   # Excel read capabilities
│   │   ├── excel_write.rs  # Excel write capabilities
│   │   ├── filter.rs       # filter capabilities
│   │   ├── formula.rs      # formula capabilities
│   │   ├── profile.rs      # profiling capabilities
│   │   ├── sort.rs         # sort capabilities
│   │   ├── stream.rs       # streaming capabilities
│   │   ├── validate.rs     # validation capabilities
│   │   └── workflow.rs     # workflow capabilities
│   ├── cli/                # CLI implementation
│   │   ├── mod.rs          # CLI definition (clap), command dispatch
│   │   ├── handler.rs      # CLI command handler logic
│   │   ├── format.rs       # output formatting (table, JSON, CSV)
│   │   ├── runtime.rs      # CLI runtime and execution context
│   │   └── commands/       # individual command implementations
│   ├── columnar/           # columnar format support
│   │   ├── mod.rs          # module root
│   │   ├── parquet.rs      # Parquet reader/writer
│   │   └── avro.rs         # Avro reader/writer
│   └── profiling/          # data profiling
│       ├── mod.rs          # module root
│       ├── profiler.rs     # profiling engine
│       ├── analysis.rs     # column analysis (type inference, stats)
│       ├── quality.rs      # quality assessment
│       └── types.rs        # profiling types
├── examples/
│   ├── *.rs                # Rust API examples (write_xls, write_rich_xls)
│   └── *.csv, *.xlsx, ...  # sample data files for examples and tests
├── tests/
│   ├── test_*.rs           # integration tests (per-module and end-to-end)
│   └── common/             # shared test utilities
├── Cargo.toml              # package metadata, deps, features (mcp, parquet, avro, gsheets, watch, completions)
└── Cargo.lock
```

## The Loop

### 1. Complete Remaining TODO Items
Pick the next highest-priority item from `TODO.md` (or `ARCHITECTURE.md` if the task is architectural). Implement it with minimal, focused changes. Do not add speculative features.

### 2. Create Tests and Examples
For every new capability:
- Add integration tests in `tests/` that exercise the feature end-to-end
- Add unit tests for core logic where appropriate (inline `#[cfg(test)] mod tests`)
- Provide a minimal usage example in `examples/` if the feature is library-facing
- Add a CLI smoke test to `tests/test_cli_integration.rs` if there's a CLI dispatch path

### 3. Ensure `cargo test` Passes
Run the full test suite:
```bash
cargo test                  # all unit tests + integration tests
cargo test --examples       # examples compile and run
cargo clippy --all-targets --all-features                # lint pass (warnings acceptable but noted)
cargo bench --bench performance  # performance benchmarks
```
Some tests require feature flags (`parquet`, `avro`, `gsheets`). Run with `--all-features` if needed. Fix any failures before proceeding. See [`MEMORY.md`](MEMORY.md#testing-patterns) for testing conventions and patterns.

### 4. Harvest to MEMORY.md
After each completed feature, extract patterns and best practices:
- **Success patterns**: What worked well and should be repeated
- **Anti-patterns**: What to avoid in future implementations
- **Spreadsheet domain knowledge**: Format-specific quirks (XLSX OOXML, XLS BIFF, ODS, CSV type inference), cell type handling, formula evaluation pitfalls
- **Rust patterns**: xls-rs-specific conventions for the Cell/Row/Sheet/Workbook types, handler registry, CLI dispatch, and MCP capabilities
- **Testing patterns**: How to assert on cell values, round-trip fidelity, fixture data files, regression cases

Add these to `MEMORY.md` with clear categories and references to specific files/lines.

### 5. Loop Back to Step 1
Return to `TODO.md` and pick the next item. Repeat until the backlog is clear.

### 6. Audit and Optimize
After each batch of features, perform a quality pass:
- **Maintainability**: Are functions small and well-named? Is the module structure logical?
- **Leanness**: Remove dead code, unused imports, and speculative abstractions
- **Wiring**: Ensure all new features are properly integrated into `lib.rs`, the handler registry, CLI dispatch, and MCP capabilities
- **Small footprint**: Avoid unnecessary dependencies; prefer standard library or lightweight crates
- **Consistency**: Match existing code style and patterns (Rust 2024 edition, `anyhow` for errors, `clap` for CLI, `serde` for serialization) - See [`MEMORY.md`](MEMORY.md#common-pitfalls-and-anti-patterns).
- **Data fidelity**: Verify round-trip read/write preserves data (types, styles, formulas) across formats - See [`MEMORY.md`](MEMORY.md#round-trip-test-structure).

### 7. Competitive Intelligence
Research similar open-source spreadsheet and data tools (openpyxl, excelize, SheetJS, calamine, xsv, polars, duckdb, miller, csvkit, visidata, pandas, etc.). Identify capabilities they have that this project lacks. Add the most valuable ones to the `TODO.md` brainstorming section. Prioritize features that provide clear competitive advantage.

### 8. Update Documentation
Keep all project docs aligned with the current implementation. Root docs (required):

- **`README.md`**: Quick start, CLI usage, feature list, crate API summary
- **`ARCHITECTURE.md`**: Module relationships, data flow, design decisions
- **`TODO.md`**: Mark completed items, move them to Done, keep brainstorming current
- **`SPEC.md`**: CLI subcommands, supported formats, MCP tool catalog, data operations
- **`MEMORY.md`**: Harvested patterns, domain knowledge, technical conventions (enhanced)

Update **`AGENTS.md`** if the loop itself evolves.

## Memory System (MEMORY.md)

### Purpose
`MEMORY.md` is the institutional knowledge repository that accelerates development by:
- **Preventing wheel reinvention**: Reuse proven patterns instead of guessing
- **Domain knowledge preservation**: Capture spreadsheet format quirks and data handling rules that may be counter-intuitive
- **Onboarding acceleration**: New contributors (human or AI) can understand patterns quickly
- **Quality consistency**: Ensure all features follow established conventions

### Structure
Organize `MEMORY.md` into these sections:

#### 1. Core Types & Handler Patterns
- `Cell`, `Row`, `Sheet`, `Workbook` type design and conventions
- `CellValue` variants and type inference rules
- `DataHandler` trait and handler registry dispatch
- Format detection logic and edge cases

#### 2. Excel Format Patterns
- XLSX (OOXML/zip) reader/writer conventions: styles, charts, conditional formatting, sparklines
- XLS (BIFF) reader/writer conventions and BIFF version differences
- ODS reader conventions (OpenDocument XML)
- Cell type inference and formatting preservation
- Feature detection patterns (what features a file uses)

#### 3. Formula Engine Patterns
- Formula parser conventions (A1/R1C1 references, functions, operators)
- Evaluator design: cell reference resolution, cross-sheet refs, function dispatch
- Built-in function implementation patterns
- Edge cases: circular refs, error propagation, type coercion

#### 4. Data Operations & Analysis Patterns
- Pandas-style operations: groupby, merge, pivot, filter, transform
- Statistical operations: describe, correlation, regression
- Streaming-aware operations for large files
- Profiling and quality assessment conventions

#### 5. Columnar Format Patterns
- Parquet reader/writer conventions (arrow integration)
- Avro reader/writer conventions
- Type mapping between columnar and spreadsheet types
- Feature flag gating (`parquet`, `avro`)

#### 6. CLI & MCP Patterns
- `clap` command structure and dispatch conventions
- Output formatting (table, JSON, CSV)
- MCP server implementation and capability registration
- Config loading and option override patterns

#### 7. Testing Patterns
- Round-trip test structure (write → read → compare)
- Fixture data files in `examples/` and their usage
- CLI smoke test structure in `tests/test_cli_integration.rs`
- Regression cases for format-specific edge cases
- Feature-flag-gated test conventions

## Principles

- **Simplicity over flexibility**: Solve the problem at hand, not every hypothetical future problem
- **Surgical changes**: Touch only what you must; clean up only your own mess
- **Goal-driven**: Every change should have a verifiable success criterion
- **Test before ship**: No feature is complete until it has passing tests
- **Docs are code**: Documentation drift is a bug
- **Data fidelity**: Never compromise on data accuracy for convenience — round-trip must preserve data
- **Memory first**: Always check `MEMORY.md` before starting a new feature
- **Pattern harvesting**: After success, update `MEMORY.md` to share the learning

## File Positioning and Value

### README.md
- **Value**: User-facing documentation and project overview
- **Audience**: Users, contributors, stakeholders
- **Position**: Entry point for anyone discovering the project
- **Focus**: Features, quick start, CLI usage, crate API summary, architecture summary
- **Last updated**: 2026-08-08

### MEMORY.md
- **Value**: Institutional knowledge and pattern library
- **Audience**: Development team (accelerates onboarding and consistency)
- **Position**: Development acceleration and quality consistency
- **Focus**: Proven patterns, domain knowledge, technical conventions
- **Last updated**: 2026-08-08

### TODO.md
- **Value**: Feature roadmap and backlog management
- **Audience**: Development team (human and AI agents)
- **Position**: Development planning and prioritization
- **Focus**: What to build next, what's done, competitive intelligence

### ARCHITECTURE.md
- **Value**: Module relationships, data flow, and design decisions
- **Audience**: Contributors maintaining or extending the crate
- **Position**: Structural reference for the codebase
- **Focus**: Module boundaries, data flow, deployment topology

### SPEC.md
- **Value**: Interface specification for the CLI, formats, and MCP tools
- **Audience**: Users and contributors integrating with xls-rs
- **Position**: Contract definition for inputs and outputs
- **Focus**: CLI subcommands, supported formats, MCP tool catalog, data operations

### MEMORY.md
- **Value**: Institutional knowledge and pattern library
- **Audience**: Development team (accelerates onboarding and consistency)
- **Position**: Development acceleration and quality consistency
- **Focus**: Proven patterns, domain knowledge, technical conventions

### AGENTS.md (this file)
- **Value**: Development process and workflow definition
- **Audience**: AI agents and human developers following the development loop
- **Position**: Process automation and continuous improvement
- **Focus**: How we work, the loop, memory system, principles
- **Update**: This file should be updated when the development loop itself evolves or when new process patterns emerge

## How These Files Work Together

1. **README.md** tells stakeholders what the project is and how to use it
2. **SPEC.md** defines the CLI, format, and MCP tool contract
3. **ARCHITECTURE.md** describes how the modules fit together
4. **TODO.md** tells developers what to build next (driven by competitive intelligence)
5. **AGENTS.md** tells agents how to work through the TODO items with quality and memory
6. **MEMORY.md** captures what we learned so we don't repeat mistakes

The loop reinforces these files:
- Complete TODO → Test → Harvest to MEMORY → Optimize → Research → Update TODO

This creates a flywheel of continuous improvement with institutional knowledge preservation.
