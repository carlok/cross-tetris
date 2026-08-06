/// Standard Tetris Guideline base scores for a single lock's line clears,
/// before the `x level` multiplier. No back-to-back or combo bonus in this
/// milestone (documented simplification).
pub fn line_clear_score(lines_cleared: u32, level: u32) -> u32 {
    let base = match lines_cleared {
        0 => 0,
        1 => 100,
        2 => 300,
        3 => 500,
        _ => 800, // 4 = Tetris; anything higher can't happen with one piece but clamp defensively
    };
    base * level.max(1)
}

pub const SOFT_DROP_POINTS_PER_CELL: u32 = 1;
pub const HARD_DROP_POINTS_PER_CELL: u32 = 2;

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
