# MEMORY

Institutional knowledge and pattern library for xls-rs — **the pure-Rust spreadsheet toolkit**.

**Version**: 0.1.14 | **Last updated**: 2026-08-10

## Core Types & Handler Patterns

### `Cell`, `Row`, `Sheet`, `Workbook` Type Design
- **`CellValue` variants**: `String`, `Number(f64)`, `Boolean(bool)`, `Empty`, `Error(String)`, `Formula(String)`, `DateTime(String)`
- **Type inference rules**: Numbers detected via `parse::<f64>()`, booleans via "TRUE"/"FALSE" (case-insensitive), formulas start with "=", empty cells are `""` or whitespace-only
- **Cell reference format**: A1-style (e.g., "A1", "B2") and R1C1-style supported in formulas
- **Type coercion**: String → Number attempted for numeric operations; errors propagate rather than silently convert

### `DataHandler` Trait and Handler Registry
- **Handler registration**: File extensions map to specific handlers in `handler_registry.rs`
- **Extension mapping**: `.csv` → `CsvHandler`, `.xlsx` → `ExcelHandler`, `.xls` → `ExcelHandler`, `.ods` → `OdsHandler`, `.parquet` → `ParquetHandler`, `.avro` → `AvroHandler`
- **Format detection**: Primary by extension; secondary by content (zip magic bytes for XLSX, OLE2 magic for XLS)

### Format Detection Logic and Edge Cases
- **XLSX detection**: ZIP magic bytes `50 4B 03 04` + `[Content_Types].xml` in archive
- **XLS detection**: OLE2 magic bytes `D0 CF 11 E0 A1 B1 1A E1` + workbook stream
- **ODS detection**: ZIP magic bytes + `mimetype="application/vnd.oasis.opendocument.spreadsheet"`
- **CSV detection**: Text file with comma/tab delimiters; heuristics in `format_detector.rs`
- **Ambiguous extensions**: Always prefer extension over content detection for speed

## Excel Format Patterns

### XLSX (OOXML/ZIP) Reader/Writer Conventions
- **ZIP structure**: `[Content_Types].xml`, `_rels/.rels`, `xl/workbook.xml`, `xl/worksheets/sheetN.xml`, `xl/styles.xml`, `xl/sharedStrings.xml`, optionally `xl/vbaProject.bin` for macro-enabled files
- **Shared strings**: String values > 0 characters stored in shared string table to reduce file size; numbers stored inline
- **Cell storage**: `<c r="A1"><v>123</v></c>` for values, `<c r="A1" t="s"><v>0</v></c>` for shared string references
- **Style inheritance**: Cell styles reference XF records in `styles.xml`; XF records reference font/fill/border IDs
- **Formula storage**: `<f>SUM(A1:A10)</f>` element with optional cached result in `<v>`

- **Styles**: Font families, sizes, colors, bold/italic/underline patterns stored in `fonts.xml`
- **Charts**: Defined in `xl/charts/chartN.xml` with series data, axes, and layout
- **Conditional formatting**: Rules in `xl/worksheets/sheetN.xml` under `<conditionalFormatting>` elements
- **Sparklines**: Tiny charts in cells defined via `<extLst>` extensions in worksheet XML
- **Merged cells**: `<mergeCells>` element with `ref` attribute (e.g., "A1:B2")
- **Hyperlinks**: `<hyperlinks>` element with `<hyperlink ref="A1" r:id="rId1" />`
- **Comments**: Separate `xl/commentsN.xml` files with `author` and `text` elements
- **Data validation**: `<dataValidation>` elements with type, operator, and formula constraints
- **Print setup**: `<pageSetup>` element in worksheet XML with orientation, scale, paper size
- **Freeze panes**: `<pane>` element with `xSplit`, `ySplit`, and `state="frozen"`
- **Auto-filter**: `<autoFilter ref="A1:Z1000"/>` element defining filter range
- **Row/column grouping**: `outlineLevel` attributes on `<row>` and `<col>` elements
- **VBA macros**: `xl/vbaProject.bin` is an OLE2 compound document. When present, `[Content_Types].xml` must include `<Default Extension="bin" ContentType="application/vnd.ms-office.vbaProject"/>` and workbook content type changes to `application/vnd.ms-excel.sheet.macroEnabled.main+xml`. API: `XlsxReader::vba_project() -> Option<&[u8]>`, `XlsxWriter::set_vba_project(Vec<u8>)`. VBA bin stored uncompressed (Stored compression method).
- **Password-protected XLSX**: Encrypted XLSX is an OLE2 (CFB) container with `EncryptionInfo` and `EncryptedPackage` streams. Agile Encryption (v4): EncryptionInfo is 4-byte version header + XML with encryption params. Key derivation: PBKDF2-HMAC-SHA512(password_utf16le, salt, spinCount, keySize). Package decryption: AES-256-CBC with IV = first 16 bytes of keyData salt. EncryptedPackage stream: 4-byte size + 4-byte padding + encrypted ZIP data. Behind `password` feature flag. API: `XlsxReader::from_reader_with_password(reader, password)`. CFB reader's `get_stream` uses mini-FAT for streams < 4096 bytes — test containers must pad streams to >= 4096 or set up mini-FAT properly.
- **Structured tables**: `xl/tables/tableN.xml` files define named table ranges with headers, banded rows, and styles. Worksheet XML references tables via `<tableParts><tablePart r:id="rIdN"/></tableParts>`. Worksheet rels map rId to `../tables/tableN.xml`. Content type: `application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml`. Table XML has `<table name="..." displayName="..." ref="A1:C4"><tableColumns count="N"><tableColumn id="1" name="..."/></tableColumns><tableStyleInfo name="TableStyleMedium2" .../></table>`. Multiple tables per sheet supported — each gets its own global index and rel ID. API: `XlsxWriter::add_table(Table { ... })`, `XlsxReader::tables(sheet) -> Option<&[XlsxTableInfo]>`. XmlScanner's `find_open_tag` advances to end of data when searching for a tag that doesn't exist — must save/restore `scanner.pos` when bounding searches within a parent element.

### XLS (BIFF) Reader/Writer Conventions
- **BIFF8 format**: OLE2 container + BIFF8 records; sector size 512 bytes
- **Record structure**: 2-byte record ID + 2-byte length + record body
- **Cell storage**: `LABEL` (string), `NUMBER` (IEEE 754), `BOOLERR` (boolean/error), `FORMULA` (RPN tokens)
- **String encoding**: BIFF8 uses 16-bit length prefix + UTF-16LE characters; earlier BIFF used codepages
- **Formula encoding**: Reverse Polish Notation (RPN) with PTG (Parsed Thing) tokens
- **Sheet structure**: `BOF` (Begin of File) → records → `EOF` (End of File)
- **BoundSheet records**: Define sheet names (UTF-16LE) and stream positions
- **SST (Shared String Table)**: String values > 0 characters to reduce file size
- **BIFF version differences**: BIFF8 (Excel 97-2003) supports Unicode, earlier versions use codepages

### XLSX Streaming Reader
- **`XlsxStreamingReader`**: Row-by-row XLSX parsing without full materialization. Reads shared strings + styles upfront, then streams sheet XML via `BufReader` on the ZIP entry.
- **Row extraction**: `RowIterator` reads chunks into an internal buffer, searches for `<row>...</row>` or `<row/>` elements, drains consumed bytes, and parses each row with `XmlScanner`.
- **Lifetime pattern**: `RowIterator<'a>` borrows `&'a [String]` (shared strings) from `XlsxStreamingReader`. The `ZipFile` inside the iterator borrows the `ZipArchive` owned by the reader.
- **Cell parsing**: Reuses `parse_cell_ref` (now `pub(crate)`) and `XmlScanner` from `xlsx_reader.rs`. Same cell type handling as full reader.
- **Testing**: Compare streaming output against `XlsxReader` full materialization to verify parity. See `tests/test_xlsx_streaming.rs`.

### ODS Reader Conventions
- **OpenDocument XML**: ZIP container with `content.xml`, `styles.xml`, `meta.xml`
- **Cell storage**: `<table:table-cell>` elements with `office:value-type` attribute
- **Repeated values**: `table:number-columns-repeated` and `table:number-rows-repeated` attributes for compression
- **Type handling**: `float`, `string`, `boolean`, `date`, `time`, `currency` types
- **Formula syntax**: Uses `of:` namespace prefix for functions (e.g., `of:sum`)
- **Style inheritance**: Styles defined in `styles.xml` with automatic/conditional styles

### Cell Type Inference and Formatting Preservation
- **Number detection**: Try `parse::<f64>()` first; if fails, treat as string
- **Date detection**: Excel serial dates (1 = Jan 1, 1900) vs ISO 8601 strings vs locale-specific formats
- **Boolean detection**: Case-insensitive "TRUE"/"FALSE"; "1"/"0" treated as numbers
- **Empty cells**: `""` or whitespace-only strings are `CellValue::Empty`
- **Formatting loss**: Round-trip may lose locale-specific formatting (currency symbols, date formats)
- **Formula preservation**: Formulas stored as strings; evaluation happens on read

### Feature Detection Patterns
- **XLSX features**: Scan worksheet XML for `<conditionalFormatting>`, `<extLst>` (sparklines), `<mergeCells>`
- **Chart detection**: Look for `xl/charts/` directory in ZIP archive
- **Style detection**: Parse `styles.xml` for non-default fonts, fills, borders
- **Macro detection**: Look for `xl/vbaProject.bin` (XLSM files)
- **Pivot table detection**: Look for `xl/pivotCache/` and `xl/pivotTables/` directories

## Formula Engine Patterns

### Formula Parser Conventions
- **Reference styles**: A1-style (column letter + row number) and R1C1-style (row + column offsets)
- **Function calls**: `FUNCTION_NAME(arg1, arg2, ...)`; names are case-insensitive
- **Operators**: Arithmetic (`+`, `-`, `*`, `/`, `^`), comparison (`=`, `<`, `>`, `<=`, `>=`, `<>`)
- **String literals**: Double-quoted; escape double-quotes with `""`
- **Array formulas**: `{=SUM(A1:A10*B1:B10)}`; curly braces indicate array context
- **Range operators**: Colon (`:`) for contiguous ranges, comma (`,`) for union, space for intersection

### Evaluator Design
- **Cell reference resolution**: Parse A1 references to (row, col) coordinates; resolve against current sheet
- **Cross-sheet references**: `SheetName!A1` syntax; must load target sheet data
- **Function dispatch**: Lookup function name in registry; call with evaluated arguments
- **Operator precedence**: Standard Excel precedence: `^` > `*/` > `+-` > comparison
- **Error propagation**: Errors bubble up through expression tree; stop evaluation on first error

### Built-in Function Implementation Patterns
- **Aggregation functions**: `SUM`, `AVERAGE`, `MIN`, `MAX`, `COUNT` iterate over cell ranges
- **Conditional functions**: `IF(condition, true_value, false_value)` evaluates condition first
- **Lookup functions**: `VLOOKUP(lookup_value, table_range, col_index, range_lookup)`
- **Math functions**: `ABS`, `ROUND`, `SQRT`, `POWER`, `LOG` operate on numeric arguments
- **Text functions**: `LEFT`, `RIGHT`, `MID`, `LEN`, `CONCATENATE` operate on strings
- **Date functions**: `TODAY()`, `NOW()`, `DATE(year, month, day)` handle Excel serial dates

### Edge Cases
- **Circular references**: Detect when formula references its own cell (directly or indirectly)
- **Error propagation**: `#DIV/0!`, `#VALUE!`, `#REF!`, `#NAME?`, `#NUM!`, `#N/A`, `#NULL!`
- **Type coercion**: Strings automatically converted to numbers for arithmetic; may produce `#VALUE!`
- **Array context**: Some functions (SUM, AVERAGE) automatically expand to array context
- **Range size limits**: BIFF8 limits ranges to 65,536 rows × 256 columns; XLSX limits are much higher

## Data Operations & Analysis Patterns

### Pandas-Style Operations
- **GroupBy**: `groupby(column_name).aggregate()` produces aggregated rows per unique value
- **Merge/Join**: Inner/outer/left/right joins on key columns; NaN for missing values
- **Pivot**: Reshape from long to wide format; index columns + columns + values
- **Melt**: Reshape from wide to long format; id_vars + value_vars
- **Filter**: WHERE-style conditions; boolean expressions on columns
- **Sort**: Stable sort on one or more columns; ascending/descending
- **Transform**: Apply functions to columns; preserve row count
- **Aggregations**: `describe()`, `value_counts()`, unique, count, sum, mean, median, std

### Statistical Operations
- **Describe**: Count, mean, std, min, 25th/50th/75th percentiles, max for numeric columns
- **Correlation**: Pearson (linear), Spearman (rank), Kendall tau-b (rank with ties)
- **Regression**: Simple linear regression (y = mx + b) with R², slope, intercept
- **Percentiles**: Linear interpolation method for percentile calculation (NumPy-compatible)
- **Skewness**: Third standardized moment; measures asymmetry
- **Kurtosis**: Fourth standardized moment (excess kurtosis = Fisher-1); measures tail heaviness

### Streaming-Aware Operations
- **Head**: Read first N rows without loading entire file
- **Tail**: Read last N rows using ring buffer to avoid loading entire file
- **Schema inference**: Scan first N rows to detect column types
- **Chunked processing**: Process large CSVs in fixed-size chunks (e.g., 10,000 rows)
- **Streaming sort**: External sort for files larger than memory (sort + merge chunks)

### Profiling and Quality Assessment
- **Type inference**: Heuristic detection of column types (string, number, boolean, date)
- **Null analysis**: Count missing values per column; calculate null percentage
- **Outlier detection**: Z-score, Modified Z-score (MAD-based), IQR, percentile methods
- **Value distribution**: Unique value counts, frequency analysis, pattern detection
- **Data quality scoring**: Composite score based on completeness, validity, consistency, uniqueness

## Columnar Format Patterns

### Parquet Reader/Writer Conventions
- **Arrow integration**: Use `arrow` crate for in-memory representation
- **Schema evolution**: Parquet schemas can evolve; writer may need to reconcile schema differences
- **Compression**: Default to Snappy compression; balance speed vs compression ratio
- **Row groups**: Parquet files organized into row groups; optimize for query patterns
- **Column pruning**: Read only needed columns from Parquet files
- **Predicate pushdown**: Push filter conditions to Parquet reader for efficiency

### Avro Reader/Writer Conventions
- **Schema generation**: Infer schema from CSV headers or Excel first row
- **JSON schema**: Avro schemas defined in JSON with field names and types
- **Record encoding**: Binary encoding with schema ID prefix
- **Union types**: Handle Avro union types (e.g., `["null", "string"]`)
- **Nested structures**: Flatten nested Avro records for spreadsheet representation

### Type Mapping Between Columnar and Spreadsheet Types
- **Parquet/Arrow → Spreadsheet**: `INT32/INT64` → `Number`, `FLOAT/DOUBLE` → `Number`, `STRING` → `String`, `BOOLEAN` → `Boolean`, `NULL` → `Empty`
- **Avro → Spreadsheet**: Similar mapping; handle union types by choosing non-null variant
- **Spreadsheet → Columnar**: String values attempted as numbers; empty cells → NULL
- **Date handling**: Excel serial dates → ISO 8601 strings; preserve timezone if available

### Feature Flag Gating
- **`parquet` feature**: Enables Parquet read/write via `arrow` and `parquet` crates
- **`avro` feature**: Enables Avro read/write via `apache-avro` crate
- **Conditional compilation**: Use `#[cfg(feature = "parquet")]` for Parquet-specific code
- **Feature detection**: Check `cfg!(feature = "parquet")` at runtime for capability availability

## CLI & MCP Patterns

### Clap Command Structure and Dispatch Conventions
- **Command hierarchy**: `xls-rs <subcommand> [args]` with 50+ subcommands organized into groups
- **Global flags**: `--config`, `--quiet`, `--verbose`, `--overwrite` (must come before subcommand)
- **Subcommand groups**: I/O (`read`, `write`, `convert`), Transforms (`sort`, `filter`), Analytics (`describe`, `corr`), Advanced (`formula`, `chart`)
- **Argument parsing**: Use `clap` derive macros for type-safe argument parsing
- **Help generation**: Auto-generated help from `clap` definitions; include examples

### Output Formatting
- **Table format**: Pretty-printed tables with aligned columns for terminal output
- **JSON format**: Structured JSON with consistent field names; useful for piping
- **CSV format**: Standard CSV with comma delimiters; quote strings containing commas
- **Markdown format**: GitHub-flavored markdown tables; good for documentation
- **HTML format**: Basic HTML tables; limited CSS styling
- **Format selection**: `--format` flag defaults to config setting; falls back to CSV

### MCP Server Implementation and Capability Registration
- **Server type**: `XlsRsMcpServer` implements MCP tool interface
- **Tool registration**: Each tool delegates to `CapabilityRegistry::execute` with tool name
- **Error responses**: Structured `error.data` with `code`, `file`, `sheet`, `range`, `cell` fields
- **Capability catalog**: Runtime catalog of available tools returned by `capabilities` tool
- **Request enrichment**: `mcp_enrichment.rs` adds context to errors (input/output paths, sheet, range)
- **Stdio transport**: `xls-rs serve` creates a tokio multi-threaded runtime and calls `XlsRsMcpServer::new().serve(stdio()).await`. The `tokio` dependency is gated behind the `mcp` feature. Pattern: `tokio::runtime::Builder::new_multi_thread().enable_all().build()` then `runtime.block_on(async { ... })` since `main()` is sync.
- **End-to-end testing**: Spawn the CLI binary as a child process, send JSON-RPC initialize over stdin, read response from stdout with timeout. See `tests/test_mcp_serve.rs`.

### Config Loading and Option Override Patterns
- **Config discovery**: `.xls-rs.toml` → `~/.xls-rs.toml` → `$XDG_CONFIG_HOME/xls-rs/config.toml`
- **Config structure**: TOML with sections for Google Sheets (`google_sheets.access_token`, `google_sheets.api_key`)
- **Option precedence**: CLI flags > config file > defaults
- **Type-safe config**: Use `serde` for deserialization into `Config` struct
- **Validation**: Validate config values on load; fail fast on invalid config

## Testing Patterns

### Round-Trip Test Structure
- **Pattern**: Write file → Read file → Compare data structures → Assert equality
- **Format coverage**: CSV → XLSX → CSV, XLSX → Parquet → CSV, XLSX → XLS → XLSX
- **Data preservation**: Verify types, values, ordering are preserved
- **Tolerance**: Floating-point comparisons use approximate equality (epsilon = 1e-9)
- **Edge cases**: Empty files, single cell, large files, special characters, unicode

### Fixture Data Files in `examples/` and Their Usage
- **Generation**: `xls-rs examples-generate` creates deterministic fixture files
- **File types**: `sales.csv`, `multi_sheet.xlsx`, `formulas.xlsx`, `styled.xlsx`
- **Test data**: Small, predictable datasets for unit tests; larger files for integration tests
- **Cleanup**: Generated files tracked in `.gitignore`; regenerated before tests
- **Versioning**: Fixture generation is deterministic; same inputs produce same outputs

### CLI Smoke Test Structure in `tests/test_cli_integration.rs`
- **Pattern**: Call CLI command → Capture output → Parse/validate → Assert success
- **Command coverage**: Test each major subcommand (read, write, convert, sort, filter)
- **Exit codes**: Assert exit code 0 for success, non-zero for errors
- **Output validation**: Check stdout contains expected data, stderr contains expected logs
- **Error handling**: Test error cases (missing files, invalid formats, permission errors)

### Regression Cases for Format-Specific Edge Cases
- **XLSX**: Merged cells (only top-left value exposed), pivot tables (not expanded), charts (read support limited)
- **XLS**: Sheet name length limits (31 chars), string length limits (255 chars), BIFF version quirks
- **CSV**: Formula injection attempts, delimiter detection, encoding issues, quote handling
- **ODS**: Repeated cell values, number-columns-repeated overflow, namespace handling

### Feature-Flag-Gated Test Conventions
- **Conditional compilation**: Use `#[cfg(feature = "parquet")]` for Parquet-specific tests
- **Test discovery**: Run `cargo test --all-features` to include all feature-flagged tests
- **Skip patterns**: Use `#[cfg(not(feature = "parquet"))]` to skip tests when feature disabled
- **Feature matrix**: Document which tests require which features in test comments

### CSV Index Patterns (`tests/test_csv_index.rs`)
- **CRLF handling**: `CsvIndex::build` must treat `\r\n` as a single record boundary, not two. Use `prev_was_cr` flag: when `\n` follows `\r`, skip the offset push (the `\r` already set `at_record_start`). Standalone `\r` (old Mac) and standalone `\n` (Unix) each work as single boundaries.
- **Index file format**: Magic `XLSRSIDX` (8 bytes) + version (4 LE) + file_size (8 LE) + count (8 LE) + offsets (count × 8 LE). Version field allows future format changes.
- **Stale index detection**: `load_or_build` compares CSV mtime vs `.idx` mtime and file_size match. If CSV is newer or size differs, rebuilds automatically.
- **Test pattern for stale index**: Use `tempfile::tempdir()` + manual `fs::File` (not `NamedTempFile` which deletes on drop). Write initial rows, build index, append rows, verify rebuild.
- **Fallback pattern**: `streaming_ops::tail` tries `CsvIndex::load_or_build` first, falls back to sequential `VecDeque` scan on any error. This ensures correctness even if indexing fails.

### Piping Ergonomics Patterns (`tests/test_piping.rs`)
- **`--quiet` dual-mode output**: Commands with text labels (value-counts, unique, dtypes, corr) check `crate::cli::runtime::get().quiet`. When quiet, suppress label headers and switch to CSV format (e.g., `value,count` instead of `  value: count`). This enables clean pipe composition.
- **Proper CSV writer for stdout**: `print_csv` and `print_data(Csv)` must use `csv::WriterBuilder` with `has_headers(false)` and `flexible(true)` writing to `std::io::stdout()`. Naive `row.join(",")` breaks on values containing commas, quotes, or newlines.
- **Stdin/stdout convention**: `-` as input/output path means stdin/stdout. `Converter::read_any` and `write_any` handle this. `ensure_safe_input` and `ensure_can_write` skip validation for `-`.
- **Log messages to stderr**: `crate::cli::runtime::log()` writes to stderr, keeping stdout clean for data. This is critical for pipe compatibility.
- **Test pattern**: Use `Command::new(xls_rs_exe())` with `Stdio::piped()` for stdin/stdout/stderr. Write input to stdin, close it, then `wait_with_output()`. For multi-stage chains, feed previous stage's stdout as next stage's stdin.

### CLI Analytics Patterns (`tests/test_cli_analytics.rs`)
- **Anomaly detection CLI**: `anomaly-detect` subcommand wraps `AnomalyDetector` with 4 methods (zscore, modified-zscore, iqr, percentile). Outputs CSV with `row,column,value,score,reason` header. Uses `--quiet` to suppress summary on stderr.
- **Resample CLI**: `resample` subcommand wraps `TimeSeriesProcessor::resample`. Takes `--date-column`, `--value-column`, `--interval`, `--agg`, optional `--date-format`. Outputs resampled time-series as CSV with date,value columns. Supports stdin/stdout piping.
- **Short option conflicts**: When adding CLI subcommands, check for short option collisions. `input` uses `-i`, so `interval` must use `-l` (or another non-conflicting short). clap panics at startup on duplicate short options.
- **Analytics handler pattern**: New analytics handlers live in `src/cli/commands/advanced/analytics.rs`, re-exported via `advanced/mod.rs`, dispatched through `AdvancedCommandHandler` methods, and wired in `handler.rs` match arms.

## Common Pitfalls and Anti-Patterns

### Excel Format Pitfalls
- **Date handling confusion**: Excel serial dates (1 = Jan 1, 1900) vs ISO 8601 strings vs locale formats
- **Merged cells**: Only top-left value visible; other cells in merged range contain data but aren't displayed
- **Formula evaluation**: Circular references cause infinite loops if not detected
- **Shared strings**: Forgetting to store strings in shared string table bloats XLSX file size
- **Style inheritance**: Not reusing style definitions creates duplicate XF records

### Data Operations Pitfalls
- **Type coercion**: Strings silently converted to numbers may produce unexpected results
- **Memory leaks**: Loading entire large datasets into memory causes OOM; use streaming for large files
- **Sort stability**: Unstable sorts lose relative order of equal elements
- **GroupBy cardinality: Too many unique values cause memory explosion
- **Null handling**: Different null representations (Empty, NaN, NULL) cause inconsistent behavior

### Testing Pitfalls
- **Flaky tests**: Tests that depend on file system state or timing; make tests deterministic
- **Hardcoded paths**: Use temporary directories and relative paths; avoid hardcoded absolute paths
- **Insufficient coverage**: Only testing happy paths; add error cases and edge cases
- **Fixture drift**: Generated fixtures changing over time; make generation deterministic
- **Platform differences**: Path separators, line endings, and case sensitivity vary across platforms

### Performance Pitfalls
- **String allocations**: Excessive `String::from()` or `format!()` calls in hot loops
- **Unnecessary copies**: Cloning large data structures when references suffice
- **Inefficient algorithms**: O(n²) algorithms where O(n log n) exists
- **Missing caching**: Repeatedly parsing or computing the same values
- **I/O blocking**: Not using async I/O for network operations (Google Sheets API)

## Performance Optimization Patterns

### Memory Optimization
- **Reuse buffers**: Pre-allocate buffers and reuse them instead of allocating new ones
- **Avoid clones**: Use references (`&str`, `&[T]`) instead of cloning data
- **Streaming readers**: Process data in chunks instead of loading entire files
- **String interning**: Reuse string values (shared string table for XLSX)
- **Lazy evaluation**: Defer computation until results are needed

### Algorithm Optimization
- **Sort once**: Replace multiple sorts with single sort and derive results
- **Use appropriate data structures**: Hash maps for lookups, B-trees for range queries
- **Early exit**: Return early from functions when result is known
- **Memoization**: Cache results of expensive computations
- **Batch operations**: Process multiple items in batches to reduce overhead

### I/O Optimization
- **Buffered I/O**: Use `BufReader` and `BufWriter` to reduce system calls
- **Bulk operations**: Batch database/API calls instead of one-by-one
- **Parallel processing**: Use rayon for parallel CPU-bound operations
- **Memory mapping**: Use `memmap2` for zero-copy file access (where safe)
- **Compression**: Compress large files on disk to reduce I/O

### Measurement and Profiling
- **Criterion benchmarks**: Use `cargo bench` with Criterion for accurate performance measurements
- **Flame graphs**: Generate flame graphs to identify hot spots
- **Memory profiling**: Use `valgrind` or `heaptrack` to find memory leaks
- **CI benchmarks**: Run benchmarks in CI to catch performance regressions
- **Profile-guided optimization**: Use `cargo pgo` for profile-guided optimization

## Security Considerations

### CSV Formula Injection
- **Attack vector**: Malicious CSV files with formulas starting with `=`, `+`, `-`, `@`
- **Mitigation**: Sanitize CSV output on all write paths (`sanitize_csv_row`, `write_records_safe`)
- **Detection**: Check first character of cell values; prefix with `'` or escape if formula-like
- **Context matters**: Only sanitize when CSV will be opened in Excel (not for data exchange)

### Path Traversal Prevention
- **Attack vector**: File paths with `..` components to escape intended directory
- **Mitigation**: Validate paths reject `..` and embedded nulls (`ensure_safe_input`, `ensure_can_write`)
- **Absolute vs relative**: Prefer relative paths; resolve absolute paths against safe base directory
- **Symlinks**: Be careful with symlink resolution; may bypass directory restrictions

### Resource Limits
- **Attack vector**: Malicious files that cause OOM or CPU exhaustion
- **Mitigation**: Hard caps defined in `limits.rs` for dense grids, ODS repeats, ZIP/CSV sizes
- **Timeouts**: Use timeouts for network operations (Google Sheets API)
- **Allocation limits**: Limit maximum allocations per operation

### Input Validation
- **File type validation**: Verify file magic bytes, not just extensions
- **Schema validation**: Validate data against expected schema for structured formats
- **Range validation**: Check that cell references and ranges are within valid bounds
- **Type validation**: Validate that values match expected types before operations

## Version History

- **0.1.x**: Initial release series. CSV/XLSX read/write, Parquet/Avro, XLS (BIFF8) read/write from scratch, Google Sheets, MCP server, streaming, data profiling, template generation, password-protected XLSX decryption, style reading, `.xlsm` support, structured tables, LaTeX/HTML export, advanced analytics (correlation, regression, anomaly detection, sampling).
- **Breaking changes**: `XlsRsError` → `XlsError`; `ExcelHandler` methods reorganized; Parquet/Avro moved to optional features; `--input`/`--output` standardized.
- **Deprecation notices**: Use `write_records_safe` instead of `write_records` for CSV writes. Use style presets instead of manual style creation.

## Glossary of Technical Terms

- **BIFF**: Binary Interchange File Format - Excel binary file format
- **BIFF8**: BIFF version 8 - Excel 97-2003 binary format
- **OOXML**: Office Open XML - XML-based format for Office 2007+
- **OLE2**: Object Linking and Embedding - Compound document binary format
- **CFB**: Compound File Binary - OLE2 container format
- **PTG**: Parsed Thing - Token in Excel formula RPN representation
- **RPN**: Reverse Polish Notation - Expression evaluation with operators after operands
- **SAX**: Simple API for XML - Event-driven XML parsing
- **MCP**: Model Context Protocol - Protocol for AI agent tool integration
- **RMCP**: Rust MCP - Rust implementation of MCP protocol
- **MSRV**: Minimum Supported Rust Version - Oldest Rust version that compiles the crate
- **Z-score**: Standard score - (value - mean) / std_dev
- **MAD**: Median Absolute Deviation - Robust measure of variability
- **IQR**: Interquartile Range - Difference between 75th and 25th percentiles
- **ISO 8601**: International standard for date/time representation
- **TOML**: Tom's Obvious Minimal Language - Configuration file format
- **XDG**: X Desktop Group - Linux desktop environment standards