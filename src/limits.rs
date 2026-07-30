//! Hard caps to mitigate memory bombs and hang-on-allocate paths.
//!
//! Spreadsheet formats can declare enormous dimensions (far corner cells,
//! ODS `number-*-repeated`, cyclic CFB FAT chains). Without bounds, dense
//! materialization and chain walks can exhaust RAM or loop forever.

/// Excel absolute maxima (XFD / 1048576).
pub const MAX_SHEET_ROWS: usize = 1_048_576;
pub const MAX_SHEET_COLS: usize = 16_384;

/// Max cells allowed when materializing a sparse sheet into a dense grid.
/// One far-corner cell must not allocate a full Excel-sized matrix.
pub const MAX_DENSE_CELLS: usize = 10_000_000;

/// Cap ODS `number-columns-repeated` / `number-rows-repeated` expansions.
pub const MAX_ODS_CELL_REPEAT: usize = MAX_SHEET_COLS;
pub const MAX_ODS_ROW_REPEAT: usize = 100_000;

/// Reject ZIP entries larger than this when slurping into memory.
pub const MAX_ZIP_ENTRY_BYTES: u64 = 512 * 1024 * 1024; // 512 MiB

/// Max output rows from a many-to-many join before aborting.
pub const MAX_JOIN_OUTPUT_ROWS: usize = 5_000_000;

/// Max characters considered for string-distance DP tables.
pub const MAX_STRING_DISTANCE_CHARS: usize = 10_000;

/// Default profiler sample when no sample_size is set (avoids full-data copy).
pub const DEFAULT_PROFILE_SAMPLE_ROWS: usize = 100_000;

/// Max unique values retained in frequency maps during profiling.
pub const MAX_PROFILE_FREQUENCY_KEYS: usize = 50_000;

/// Max formula evaluation recursion depth.
pub const MAX_FORMULA_DEPTH: usize = 64;

/// Max cells a formula range may span (pre-allocation / iteration budget).
pub const MAX_FORMULA_RANGE_CELLS: usize = 1_000_000;

/// Max FAT/mini-FAT sector hops when walking a CFB stream (cycle guard).
pub const MAX_CFB_SECTOR_HOPS: usize = 4_000_000;

/// Clamp dense grid dimensions so `rows * cols <= MAX_DENSE_CELLS`.
/// Prefer keeping row count; shrink columns when over budget.
pub fn clamp_dense_dims(max_row: usize, max_col: usize) -> (usize, usize) {
    let mut rows = (max_row + 1).min(MAX_SHEET_ROWS);
    let mut cols = (max_col + 1).min(MAX_SHEET_COLS);

    if rows == 0 || cols == 0 {
        return (0, 0);
    }

    if rows.saturating_mul(cols) > MAX_DENSE_CELLS {
        cols = (MAX_DENSE_CELLS / rows).max(1);
        if rows.saturating_mul(cols) > MAX_DENSE_CELLS {
            rows = (MAX_DENSE_CELLS / cols).max(1);
        }
    }

    (rows, cols)
}

/// Cap a repeat count so appending `repeat` items does not exceed `limit`.
pub fn capped_repeat(repeat: usize, already: usize, limit: usize) -> usize {
    if already >= limit {
        return 0;
    }
    repeat.min(limit - already)
}
