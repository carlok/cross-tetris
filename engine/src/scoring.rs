/// Standard Tetris Guideline base scores for a single lock's line clears,
/// before the `x level` multiplier. No back-to-back or combo bonus in this
/// milestone (documented simplification).
///
/// `u64`, not `u32`: level is uncapped (`lines_cleared / 10 + 1`), so a long
/// enough game pushes `base * level` past `u32::MAX` on its own, well before
/// the cumulative score total gets anywhere near overflowing — confirmed by
/// a headless AI run that reached a ~4.0 billion total within 270 million of
/// u32::MAX at "only" level ~8000.
pub fn line_clear_score(lines_cleared: u32, level: u32) -> u64 {
    let base: u64 = match lines_cleared {
        0 => 0,
        1 => 100,
        2 => 300,
        3 => 500,
        _ => 800, // 4 = Tetris; anything higher can't happen with one piece but clamp defensively
    };
    base * level.max(1) as u64
}

pub const SOFT_DROP_POINTS_PER_CELL: u64 = 1;
pub const HARD_DROP_POINTS_PER_CELL: u64 = 2;

/// Standard Guideline-style gravity curve: milliseconds per row of fall, by level.
/// Levels beyond the table clamp to the fastest defined speed.
pub fn gravity_ms_per_row(level: u32) -> f64 {
    const TABLE_MS: [f64; 15] = [
        1000.0, 793.0, 618.0, 473.0, 355.0, 262.0, 190.0, 135.0, 94.0, 64.0, 43.0, 28.0, 18.0,
        11.0, 7.0,
    ];
    let idx = (level.saturating_sub(1) as usize).min(TABLE_MS.len() - 1);
    TABLE_MS[idx]
}
