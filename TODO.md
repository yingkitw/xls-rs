# TODO

## North star

- [ ] Maintain **capability parity** across **library** (`xls_rs`), **CLI** (`xls-rs`), and **MCP server** (`XlsRsMcpServer`) so the same operations and formats are available everywhere with consistent semantics, errors, and defaults.

## Parity work (library ↔ CLI ↔ MCP)

- [x] Define a single “capability catalog” (operations + I/O formats + options) and track parity gaps. (`src/capability_catalog.rs`)
- [ ] Ensure every CLI command maps 1:1 to a library entry point (no hidden behavior in CLI).
- [ ] Ensure every MCP tool maps 1:1 to a library entry point (no bespoke MCP-only logic).
- [ ] Normalize error surface:
  - [ ] Stable error codes/messages for CLI + MCP (same root causes, same wording)
  - [ ] Structured MCP error payloads with actionable fields (file, sheet, range, row/col)
- [~] Add parity tests that run the same use case through:
  - [x] library API
  - [x] CLI command (smoke) (`tests/test_parity_smoke.rs`)
  - [ ] MCP tool (needs harness)
  - [ ] compare normalized outputs for deterministic parity

## XLS/XLSX manipulation (core)

- [ ] **Read parity**:
  - [ ] Range reads behave identically across formats (CSV/XLSX/XLS/ODS) where possible
  - [ ] Sheet selection behavior consistent (default sheet, missing sheet errors)
- [ ] **Write parity**:
  - [ ] XLSX writer: ensure formulas/styles/charts/sparklines/condfmt APIs are reachable from CLI + MCP
  - [ ] Cell typing rules (number/date/string/empty) consistent across writers
- [ ] **Edit operations** (in-place style transforms):
  - [ ] “apply formula” to a range (not just a single cell)
  - [ ] “write range” that can expand sheet bounds safely
  - [ ] preserve/overwrite behavior explicitly configurable

## Format coverage & fidelity

- [ ] Confirm support matrix and document it in README (what’s read-only vs read/write):
  - [ ] `.xlsx`
  - [ ] `.xls`
  - [ ] `.ods`
  - [ ] `.csv`
  - [ ] `.parquet`
  - [ ] `.avro`
- [ ] Ensure round-trip expectations are tested:
  - [ ] CSV → XLSX → CSV (headers, ordering, types where representable)
  - [ ] XLSX → Parquet/Avro → CSV (schema + headers preserved)
- [ ] Add explicit constraints for unsupported features (merged cells, pivot tables, etc.) and fail with clear errors.

## CLI UX & reliability

- [x] Add `--config <path>` to override config discovery.
- [x] Add `--quiet` and `--verbose` modes.
- [x] Add guardrails for destructive overwrites (`--overwrite` required).
- [x] Add `xls-rs examples-generate` to generate `examples/` artifacts deterministically.
- [ ] Add `--format` defaults that are consistent with config + subcommands.
- [ ] Improve output consistency:
  - [ ] data output goes to stdout
  - [ ] progress/logs go to stderr

## MCP server (tooling completeness)

- [ ] Tool naming: consistent verbs and nouns (read/write/convert/sort/filter/…).
- [ ] Add missing tools for advanced operations (validation/profile/chart/encrypt/batch/stream) if not already exposed.
- [ ] Ensure MCP tools accept the same option schema as CLI flags (sheet, range, format, etc.).
- [x] Add an MCP “capabilities” tool that returns the supported operations + formats at runtime.

## Performance & large files

- [ ] Streaming mode parity (CLI + library + MCP):
  - [ ] chunked reads/writes for big CSV and big XLSX where feasible
  - [ ] avoid loading whole datasets when not needed (head/tail/schema/info)
- [ ] Add basic benchmarks for key paths (read XLSX, write XLSX, convert to parquet).

## Safety & correctness

- [ ] Keep CSV formula-injection sanitization consistent across all write paths.
- [ ] Path validation rules consistent for CLI commands that write files.

## Testing & fixtures

- [x] Consolidate example/fixture generation in one place and make it deterministic (CLI `examples-generate` + test fixtures).
- [ ] Add golden-file tests for XLSX writer output structure (beyond current smoke checks).
- [ ] Add property-like tests for range parsing and column name resolution.

## Hygiene

- [ ] Keep `.gitignore` aligned with generated artifacts (e.g. `target/`, `examples/`).

