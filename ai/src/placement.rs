//! Board/geometry mechanics shared by every evaluator (`greedy`, `dellacherie`):
//! landing-row enumeration and lock simulation. No scoring or feature
//! extraction lives here — those stay per-evaluator.

use crate::bitboard::{self, BitBoard};
use engine::board::BOARD_WIDTH;
use engine::piece::{ActivePiece, PieceKind, Rotation};
use engine::rotation::shape;

/// Resulting board (after locking `piece` and clearing any complete lines)
/// and the number of lines that placement clears. `board: BitBoard` is
/// `Copy` (80 bytes) — this is a stack copy, not a heap allocation.
pub fn simulate_lock(board: BitBoard, piece: &ActivePiece) -> (BitBoard, u32) {
    let mut b = board;
    b.place(piece);
    let cleared = b.clear_full_rows();
    (b, cleared)
}

/// Row of the topmost filled cell in `col`, or `BOARD_TOTAL_HEIGHT` (the
/// floor) if the column is empty — `BitBoard::column_height` already encodes
/// this (`height = BOARD_TOTAL_HEIGHT - top_row`, `0` for an empty column,
/// which inverts back to exactly `BOARD_TOTAL_HEIGHT`), so this is just a
/// unit conversion, not a rescan.
pub fn col_top(board: &BitBoard, col: i32) -> i32 {
    engine::board::BOARD_TOTAL_HEIGHT as i32 - board.column_height(col as usize) as i32
}

/// Every legal final resting placement of `kind` on `board`: for each SRS
/// rotation state and every horizontally in-bounds column, the piece dropped
/// straight down to where it first collides.
///
/// The landing row is computed directly (the "skirt" technique) instead of
/// walking down one row at a time with a `piece_fits` check per step — for
/// each occupied cell `(dr, dc)` of the piece, the highest row it could rest
/// at is `col_top(col + dc) - dr - 1` (one row above the first filled cell
/// in that column, or the floor); the piece's actual landing row is the
/// *smallest* (most constraining) of those across its 4 cells. Column tops
/// are precomputed once per board (not per candidate — the board doesn't
/// change across the ~40 candidates this function considers), turning what
/// was up to ~40 `piece_fits` calls per candidate into 4 array reads.
///
/// `piece_fits` is still called once per candidate at the computed landing
/// row, as a cheap correctness safety net (e.g. a column already full to the
/// very top yields a landing row that doesn't fit anything — the old
/// iterative version would simply never find a valid resting spot there
/// either, so this preserves identical behavior at that edge case too).
pub fn enumerate_placements(board: &BitBoard, kind: PieceKind) -> Vec<ActivePiece> {
    const ROTATIONS: [Rotation; 4] = [Rotation::R0, Rotation::R, Rotation::R2, Rotation::L];
    let col_tops: [i32; BOARD_WIDTH] = core::array::from_fn(|c| col_top(board, c as i32));
    let mut out = Vec::new();
    for &rotation in &ROTATIONS {
        let cells = shape(kind, rotation);
        let min_col = cells.iter().map(|&(_, c)| c).min().unwrap();
        let max_col = cells.iter().map(|&(_, c)| c).max().unwrap();
        let lo = -min_col;
        let hi = BOARD_WIDTH as i32 - 1 - max_col;
        if lo > hi {
            continue;
        }
        for col in lo..=hi {
            // min over all 4 cells is equivalent to first taking, per
            // distinct dc, the cell with the largest dr (the piece's
            // "skirt" in that column) and then the min over columns — for a
            // fixed dc the term is monotonically decreasing in dr, so the
            // overall minimum is the same either way.
            let row = cells.iter().map(|&(dr, dc)| col_tops[(col + dc) as usize] - dr - 1).min().unwrap();
            let piece = ActivePiece { kind, rotation, row, col };
            if bitboard::piece_fits(board, &piece) {
                out.push(piece);
            }
        }
        if kind == PieceKind::O {
            break; // all 4 rotation states are identical; no need to repeat
        }
    }
    out
}
