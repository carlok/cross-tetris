//! Dellacherie's six classical hand-derived Tetris features (landing_height,
//! eroded_piece_cells, row_transitions, column_transitions, holes,
//! well_sums), scored with the published weights. A well-known, distinctly
//! stronger feature set than the naive height/bumpiness family in
//! `greedy.rs`, but still one-ply and non-ML — no lookahead, no learning.
//!
//! Kept as an independent evaluator alongside `greedy.rs` (not a replacement
//! of it): `greedy`'s public API (`Weights`, `play_best_move`,
//! `play_best_cross_move`, ...) is depended on by `wasm/src/lib.rs` and must
//! keep working unchanged; this module reuses `greedy::CrossPlacement` (an
//! evaluator-agnostic result type) and the shared placement mechanics in
//! `placement.rs`, but has its own `Weights`/`Features`/scoring.

use crate::bitboard::BitBoard;
use crate::greedy::CrossPlacement;
use crate::placement::{enumerate_placements, simulate_lock};
use engine::board::{BOARD_TOTAL_HEIGHT, BOARD_WIDTH};
use engine::cross::{Arm, CrossGame};
use engine::piece::ActivePiece;
use engine::{Action, GameState};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Weights {
    pub landing_height: f32,
    pub eroded_piece_cells: f32,
    pub row_transitions: f32,
    pub column_transitions: f32,
    pub holes: f32,
    pub well_sums: f32,
}

/// Published Dellacherie weights (see plan.md) — a matched set, not to be
/// mixed with `greedy::DEFAULT_WEIGHTS`. Kept at full published precision
/// even though `f32` only retains ~7 significant digits, for provenance.
#[allow(clippy::excessive_precision)]
pub const DELLACHERIE_WEIGHTS: Weights = Weights {
    landing_height: -4.500158825082766,
    eroded_piece_cells: 3.4181268101392694,
    row_transitions: -3.2178882868487753,
    column_transitions: -9.348695305445199,
    holes: -7.899265427351652,
    well_sums: -3.3855972247263626,
};

#[derive(Copy, Clone, Debug)]
struct Features {
    landing_height: f32,
    eroded_piece_cells: u32,
    row_transitions: u32,
    column_transitions: u32,
    holes: u32,
    well_sums: u32,
}

/// Mean, over the piece's 4 occupied cells (at its final resting position),
/// of the cell's height above the floor.
fn landing_height(piece: &ActivePiece) -> f32 {
    let sum: i32 = piece.occupied_cells().iter().map(|&(row, _)| BOARD_TOTAL_HEIGHT as i32 - row).sum();
    sum as f32 / 4.0
}

/// (number of the piece's own cells that land in a row which becomes full)
/// times (number of rows cleared by this placement) — Dellacherie's
/// "erosion" measure of how directly this placement contributed to clearing
/// lines. Computed against `board` *before* `simulate_lock`'s
/// `clear_full_rows` call, since that collapses rows and destroys which ones
/// were full.
fn eroded_piece_cells(board: &BitBoard, piece: &ActivePiece) -> u32 {
    let mut placed = *board;
    placed.place(piece);
    let full_rows: u32 = (0..BOARD_TOTAL_HEIGHT).filter(|&r| placed.is_row_full(r)).count() as u32;
    let piece_cells_in_full_rows =
        piece.occupied_cells().iter().filter(|&&(row, _)| row >= 0 && (row as usize) < BOARD_TOTAL_HEIGHT && placed.is_row_full(row as usize)).count() as u32;
    piece_cells_in_full_rows * full_rows
}

/// Filled/empty transitions along each row (scanned left to right, walls on
/// both sides counted as filled), summed over every row from the topmost
/// filled row down to the floor. Rows entirely above the stack are skipped —
/// otherwise every such row would contribute exactly 2 (wall-in, wall-out),
/// biasing the feature toward taller stacks purely from empty headroom.
fn row_transitions(board: &BitBoard) -> u32 {
    let mut count = 0u32;
    for row in board.topmost_filled_row()..BOARD_TOTAL_HEIGHT {
        let mut prev_filled = true; // left wall
        for col in 0..BOARD_WIDTH as i32 {
            let filled = board.is_occupied(row as i32, col);
            if filled != prev_filled {
                count += 1;
            }
            prev_filled = filled;
        }
        if !prev_filled {
            count += 1; // right wall
        }
    }
    count
}

/// Filled/empty transitions down each column (scanned from the topmost
/// filled row to the floor, which counts as filled), summed over all 10
/// columns. Unlike `row_transitions`, the floor boundary is always scored —
/// every column transitions into it eventually, even on an empty board (one
/// transition per column, from open sky into the floor).
fn column_transitions(board: &BitBoard) -> u32 {
    let top = board.topmost_filled_row();
    let mut count = 0u32;
    for col in 0..BOARD_WIDTH as i32 {
        let mut prev_filled = false; // nothing above the scanned range
        for row in top..BOARD_TOTAL_HEIGHT {
            let filled = board.is_occupied(row as i32, col);
            if filled != prev_filled {
                count += 1;
            }
            prev_filled = filled;
        }
        if !prev_filled {
            count += 1; // floor
        }
    }
    count
}

/// Sum, over every column, of the triangular depth of each "well" — a
/// contiguous vertical run of empty cells with a filled cell (or wall) on
/// both sides. Implemented as a running depth counter that resets whenever
/// the well condition breaks: a cell `depth` rows into a well contributes
/// `depth` to the sum (1 + 2 + ... for a well of depth N sums to N(N+1)/2),
/// which is the standard well_sums definition.
fn well_sums(board: &BitBoard) -> u32 {
    let mut sum = 0u32;
    for col in 0..BOARD_WIDTH as i32 {
        let mut depth = 0u32;
        for row in 0..BOARD_TOTAL_HEIGHT as i32 {
            let is_well_cell =
                !board.is_occupied(row, col) && board.is_occupied(row, col - 1) && board.is_occupied(row, col + 1);
            if is_well_cell {
                depth += 1;
                sum += depth;
            } else {
                depth = 0;
            }
        }
    }
    sum
}

fn extract_features(board_before: &BitBoard, piece: &ActivePiece, board_after: &BitBoard) -> Features {
    Features {
        landing_height: landing_height(piece),
        eroded_piece_cells: eroded_piece_cells(board_before, piece),
        row_transitions: row_transitions(board_after),
        column_transitions: column_transitions(board_after),
        holes: board_after.count_holes(),
        well_sums: well_sums(board_after),
    }
}

fn score(features: &Features, weights: &Weights) -> f32 {
    weights.landing_height * features.landing_height
        + weights.eroded_piece_cells * features.eroded_piece_cells as f32
        + weights.row_transitions * features.row_transitions as f32
        + weights.column_transitions * features.column_transitions as f32
        + weights.holes * features.holes as f32
        + weights.well_sums * features.well_sums as f32
}

/// The highest-scoring legal placement for the *upcoming* piece across every
/// selectable well, per Dellacherie's feature set. Mirrors
/// `greedy::best_cross_placement`'s shape exactly.
pub fn best_cross_placement(cross: &mut CrossGame, weights: &Weights) -> Option<CrossPlacement> {
    if !cross.awaiting_well_selection() {
        return None;
    }
    let kind = *cross.next_queue(1).first()?;
    Arm::ALL
        .iter()
        .copied()
        .filter(|&arm| cross.is_well_selectable(arm))
        .flat_map(|arm| {
            let bb = BitBoard::from_board(&cross.well(arm).board);
            enumerate_placements(&bb, kind)
                .into_iter()
                .map(move |piece| {
                    let (resulting_board, _lines_cleared) = simulate_lock(bb, &piece);
                    let features = extract_features(&bb, &piece, &resulting_board);
                    (arm, piece, score(&features, weights))
                })
                .collect::<Vec<_>>()
        })
        .max_by(|(_, _, a), (_, _, b)| a.partial_cmp(b).unwrap())
        .map(|(arm, piece, _)| CrossPlacement { arm, rotation: piece.rotation, column: piece.col, row: piece.row })
}

/// Computes the best cross placement for the upcoming piece per the
/// Dellacherie evaluator, commits it to that well, and hard-drops it there.
pub fn play_best_cross_move(cross: &mut CrossGame, weights: &Weights) {
    let Some(placement) = best_cross_placement(cross, weights) else {
        return;
    };
    cross.select_well(placement.arm);
    let Some(active) = cross.active_piece() else { return };
    let drop_distance = (placement.row - active.row).max(0) as u32;
    cross.force_active_placement(placement.rotation, placement.row, placement.column);
    cross.apply(Action::HardDrop);
    cross.wells[placement.arm.index()].score += drop_distance * engine::scoring::HARD_DROP_POINTS_PER_CELL;
}

/// Single-board equivalent of `best_cross_placement`, mirroring
/// `greedy::best_placement`. Not used by the cross-mode game loop, kept for
/// API symmetry and single-board testing.
pub fn best_placement(state: &GameState, weights: &Weights) -> Option<crate::greedy::Placement> {
    let active = state.active?;
    let bb = BitBoard::from_board(&state.board);
    enumerate_placements(&bb, active.kind)
        .into_iter()
        .map(|piece| {
            let (resulting_board, _lines_cleared) = simulate_lock(bb, &piece);
            let features = extract_features(&bb, &piece, &resulting_board);
            (piece, score(&features, weights))
        })
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(piece, _)| crate::greedy::Placement { rotation: piece.rotation, column: piece.col, row: piece.row })
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::piece::{PieceKind, Rotation};

    /// Single filled cell at the bottom-left corner. Hand-computed:
    /// row_transitions: wall(filled) -> col0(filled): no transition; col0 ->
    /// col1(empty): +1; col1..col8 all empty: no transitions; col9(empty) ->
    /// wall(filled): +1. Total 2.
    /// column_transitions: col0 (empty above range, then filled at the one
    /// scanned row): +1 transition into filled, then floor is also filled so
    /// no further transition = 1. col1..col9: stay empty for the whole
    /// (single-row) scan, then transition into the floor: +1 each = 9. Total
    /// 1 + 9 = 10.
    /// well_sums: 0 (the only filled cell has no empty cell flanked by fill
    /// on both sides anywhere).
    /// holes: 0 (nothing below the filled cell).
    #[test]
    fn single_corner_cell_hand_computed_features() {
        let mut board = BitBoard::empty();
        board.set(39, 0);

        assert_eq!(row_transitions(&board), 2);
        assert_eq!(column_transitions(&board), 10);
        assert_eq!(well_sums(&board), 0);
        assert_eq!(board.count_holes(), 0);
    }

    /// A single-column well of depth 3 (cols 0 and 2 filled for 3 rows, col 1
    /// empty for those same 3 rows). well_sums for a well of depth N is
    /// N(N+1)/2 = 3*4/2 = 6.
    #[test]
    fn well_sums_hand_computed_for_depth_three_well() {
        let mut board = BitBoard::empty();
        for row in 37..40 {
            board.set(row, 0);
            board.set(row, 2);
        }
        assert_eq!(well_sums(&board), 6);
    }

    /// Two separate depth-1 wells (col 1 empty between filled col0/col2 at
    /// row 39, and col 4 empty between filled col3/col5 at row 39) sum to
    /// 1 + 1 = 2, not 6 — confirms depth resets are per-column and
    /// non-adjacent wells don't accumulate into each other.
    #[test]
    fn well_sums_hand_computed_for_two_separate_depth_one_wells() {
        let mut board = BitBoard::empty();
        for col in [0, 2, 3, 5] {
            board.set(39, col);
        }
        assert_eq!(well_sums(&board), 2);
    }

    /// Row 39 filled for columns 2..=9 (8 cells), columns 0 and 1 open. An O
    /// piece dropped at row=38,col=0 occupies (38,0) (38,1) (39,0) (39,1),
    /// completing row 39 (now fully filled) while row 38 stays partial (only
    /// 2 of 10 columns). Exactly 2 of the piece's 4 cells land in the one
    /// full row, so eroded_piece_cells = 2 cells * 1 cleared row = 2.
    #[test]
    fn eroded_piece_cells_hand_computed() {
        let mut board = BitBoard::empty();
        for col in 2..BOARD_WIDTH as i32 {
            board.set(39, col);
        }
        let piece = ActivePiece { kind: PieceKind::O, rotation: Rotation::R0, row: 38, col: 0 };
        assert_eq!(eroded_piece_cells(&board, &piece), 2);
    }

    /// An O piece resting at the floor (row=38, so its cells occupy absolute
    /// rows 38,38,39,39 — the bottommost two valid rows) has heights
    /// (40-38)=2 twice and (40-39)=1 twice, mean = (2+2+1+1)/4 = 1.5.
    #[test]
    fn landing_height_hand_computed_for_floor_row() {
        let piece = ActivePiece { kind: PieceKind::O, rotation: Rotation::R0, row: 38, col: 0 };
        assert_eq!(landing_height(&piece), 1.5);
    }
}
