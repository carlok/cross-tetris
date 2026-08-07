//! Simplest-possible rule-based baseline: one-ply, no lookahead beyond the
//! current piece, no hold usage. For every legal final resting placement of
//! the active piece, score the resulting board with a manually-set weighted
//! linear feature sum and pick the argmax.
//!
//! Placements are enumerated directly from `engine::rotation::shape` at each
//! of the 4 SRS rotation states, rather than by walking `try_rotate`'s
//! kick-validated path from the piece's current orientation. This is a
//! documented simplification: some placements this AI proposes might not be
//! reachable by a legal sequence of single-step rotate/move actions from spawn
//! (a small minority of kick-dependent placements), but per the project's
//! placement-level action model (spec section 4.1) the simulator is expected
//! to execute a chosen placement directly rather than derive it from
//! real-time input, so this does not misrepresent what the AI "solves".

use crate::bitboard::{self, BitBoard};
use engine::board::BOARD_WIDTH;
use engine::cross::{Arm, CrossGame};
use engine::piece::{ActivePiece, PieceKind, Rotation};
use engine::rotation::shape;
use engine::{Action, GameState};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Weights {
    pub agg_height: f32,
    pub holes: f32,
    pub bumpiness: f32,
    pub height_variance: f32,
    pub lines_cleared: f32,
}

/// A well-known reasonable starting point (not evolved/tuned) for the greedy
/// baseline; manually configurable by constructing a different `Weights`.
pub const DEFAULT_WEIGHTS: Weights = Weights {
    agg_height: -0.51,
    holes: -0.76,
    bumpiness: -0.18,
    height_variance: -0.10,
    lines_cleared: 0.76,
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Placement {
    pub rotation: Rotation,
    /// Column of the piece's rotation bounding-box top-left at rest.
    pub column: i32,
    pub row: i32,
}

#[derive(Copy, Clone, Debug)]
struct Features {
    agg_height: u32,
    holes: u32,
    bumpiness: u32,
    height_variance: f32,
    lines_cleared: u32,
}

fn column_heights(board: &BitBoard) -> [u32; BOARD_WIDTH] {
    let mut heights = [0u32; BOARD_WIDTH];
    for (col, h) in heights.iter_mut().enumerate() {
        *h = board.column_height(col);
    }
    heights
}

fn count_holes(board: &BitBoard) -> u32 {
    let mut holes = 0;
    for col in 0..BOARD_WIDTH as i32 {
        let mut found_filled = false;
        for row in 0..engine::board::BOARD_TOTAL_HEIGHT as i32 {
            if board.is_occupied(row, col) {
                found_filled = true;
            } else if found_filled {
                holes += 1;
            }
        }
    }
    holes
}

fn extract_features(board: &BitBoard, lines_cleared: u32) -> Features {
    let heights = column_heights(board);
    let agg_height: u32 = heights.iter().sum();
    let bumpiness: u32 = heights.windows(2).map(|w| (w[0] as i32 - w[1] as i32).unsigned_abs()).sum();
    let mean = agg_height as f32 / BOARD_WIDTH as f32;
    let height_variance = heights.iter().map(|&h| (h as f32 - mean).powi(2)).sum::<f32>() / BOARD_WIDTH as f32;
    Features {
        agg_height,
        holes: count_holes(board),
        bumpiness,
        height_variance,
        lines_cleared,
    }
}

fn score(features: &Features, weights: &Weights) -> f32 {
    weights.agg_height * features.agg_height as f32
        + weights.holes * features.holes as f32
        + weights.bumpiness * features.bumpiness as f32
        + weights.height_variance * features.height_variance
        + weights.lines_cleared * features.lines_cleared as f32
}

/// Resulting board (after locking `piece` and clearing any complete lines)
/// and the number of lines that placement clears. `board: BitBoard` is
/// `Copy` (80 bytes) — this is a stack copy, not the heap allocation
/// `engine::Board::clone()` used to be, done once per candidate placement
/// (~40 times per well per decision).
fn simulate_lock(board: BitBoard, piece: &ActivePiece) -> (BitBoard, u32) {
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
fn col_top(board: &BitBoard, col: i32) -> i32 {
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
fn enumerate_placements(board: &BitBoard, kind: PieceKind) -> Vec<ActivePiece> {
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

/// The highest-scoring legal placement for the active piece, or `None` if
/// there is no active piece. Ties break toward the first-found candidate
/// (stable rotation/column iteration order) for determinism.
pub fn best_placement(state: &GameState, weights: &Weights) -> Option<Placement> {
    let active = state.active?;
    let bb = BitBoard::from_board(&state.board);
    let candidates = enumerate_placements(&bb, active.kind);
    candidates
        .into_iter()
        .map(|piece| {
            let (resulting_board, lines_cleared) = simulate_lock(bb, &piece);
            let features = extract_features(&resulting_board, lines_cleared);
            (piece, score(&features, weights))
        })
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(piece, _)| Placement {
            rotation: piece.rotation,
            column: piece.col,
            row: piece.row,
        })
}

/// Computes and immediately applies the greedy best move for the active
/// piece via a hard drop. No-op if there is no active piece or the game is over.
pub fn play_best_move(state: &mut GameState, weights: &Weights) {
    let Some(placement) = best_placement(state, weights) else {
        return;
    };
    let Some(active) = state.active else { return };
    let drop_distance = (placement.row - active.row).max(0) as u32;
    if let Some(active) = state.active.as_mut() {
        active.rotation = placement.rotation;
        active.row = placement.row;
        active.col = placement.column;
    }
    state.apply(Action::HardDrop);
    // The piece is already at its resting row when HardDrop applies (0
    // simulated drop distance from there), so credit the hard-drop score the
    // piece would have earned falling from its original spawn row.
    state.score += drop_distance * engine::scoring::HARD_DROP_POINTS_PER_CELL;
}

/// A cross-mode placement: which well plus the same rotation/column/row a
/// single-board `Placement` carries, since choosing the target arm is part
/// of one placement decision here (spec section 4.1), not a separate step.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CrossPlacement {
    pub arm: Arm,
    pub rotation: Rotation,
    pub column: i32,
    pub row: i32,
}

/// The highest-scoring legal placement for the *upcoming* piece across every
/// selectable well (not topped out, not already over the max well-imbalance
/// limit — see `CrossGame::is_well_selectable`) — the AI evaluates all
/// eligible boards in parallel and picks one (arm, rotation, column). `None`
/// if a piece is already falling (call only while `awaiting_well_selection()`)
/// or no well is currently selectable.
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
            // One conversion per well per decision, not per candidate — the
            // board doesn't change across the ~40 candidates a well yields.
            let bb = BitBoard::from_board(&cross.well(arm).board);
            enumerate_placements(&bb, kind)
                .into_iter()
                .map(move |piece| {
                    let (resulting_board, lines_cleared) = simulate_lock(bb, &piece);
                    let features = extract_features(&resulting_board, lines_cleared);
                    (arm, piece, score(&features, weights))
                })
                .collect::<Vec<_>>()
        })
        .max_by(|(_, _, a), (_, _, b)| a.partial_cmp(b).unwrap())
        .map(|(arm, piece, _)| CrossPlacement { arm, rotation: piece.rotation, column: piece.col, row: piece.row })
}

/// Computes the best cross placement for the upcoming piece, commits it to
/// that well, and hard-drops it there. No-op if a piece is already falling
/// or every well has topped out.
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

#[cfg(test)]
mod landing_row_tests {
    use super::*;
    use engine::board::Board;
    use engine::rng::Rng;
    use engine::rotation::piece_fits;

    /// Deliberately independent reimplementation of "drop straight down one
    /// row at a time" (the technique enumerate_placements used before the
    /// skirt-based landing-row shortcut), so this test can catch a
    /// regression in the fast path rather than just re-testing itself.
    fn naive_enumerate_placements(board: &Board, kind: PieceKind) -> Vec<ActivePiece> {
        const ROTATIONS: [Rotation; 4] = [Rotation::R0, Rotation::R, Rotation::R2, Rotation::L];
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
                let mut piece = ActivePiece { kind, rotation, row: -40, col };
                loop {
                    let next = ActivePiece { row: piece.row + 1, ..piece };
                    if piece_fits(board, &next) {
                        piece = next;
                    } else {
                        break;
                    }
                }
                if piece_fits(board, &piece) {
                    out.push(piece);
                }
            }
            if kind == PieceKind::O {
                break;
            }
        }
        out
    }

    // Rotation isn't Hash, so compare as a sorted Vec instead of a HashSet.
    fn placement_key(p: &ActivePiece) -> (i32, i32, u8) {
        let r = match p.rotation {
            Rotation::R0 => 0,
            Rotation::R => 1,
            Rotation::R2 => 2,
            Rotation::L => 3,
        };
        (p.col, p.row, r)
    }

    fn placement_set(pieces: &[ActivePiece]) -> Vec<(i32, i32, u8)> {
        let mut v: Vec<_> = pieces.iter().map(placement_key).collect();
        v.sort_unstable();
        v
    }

    /// Boards with varied stack shapes: empty, flat, staggered, a well, an
    /// overhang, and a column filled all the way to the top (the edge case
    /// the landing-row shortcut's doc comment calls out specifically).
    fn test_boards() -> Vec<Board> {
        let mut boards = vec![Board::new()];

        let mut flat = Board::new();
        for col in 0..BOARD_WIDTH as i32 {
            for row in 35..40 {
                flat.set(row, col, Some(PieceKind::J));
            }
        }
        boards.push(flat);

        let mut staggered = Board::new();
        for col in 0..BOARD_WIDTH as i32 {
            let height = 30 + (col % 4) * 2;
            for row in height..40 {
                staggered.set(row, col, Some(PieceKind::L));
            }
        }
        boards.push(staggered);

        let mut well = Board::new();
        for col in 0..BOARD_WIDTH as i32 {
            if col == 5 {
                continue;
            }
            for row in 34..40 {
                well.set(row, col, Some(PieceKind::T));
            }
        }
        boards.push(well);

        let mut overhang = Board::new();
        for col in 0..BOARD_WIDTH as i32 {
            overhang.set(20, col, Some(PieceKind::S)); // a floating row near the top
        }
        for col in 0..3 {
            for row in 35..40 {
                overhang.set(row, col, Some(PieceKind::Z));
            }
        }
        boards.push(overhang);

        let mut full_column = Board::new();
        for row in 0..40 {
            full_column.set(row, 4, Some(PieceKind::O));
        }
        boards.push(full_column);

        // A batch of pseudo-random boards built by dropping random pieces
        // via the same landing-row logic being tested, seeded for
        // reproducibility (this only builds test fixtures, not the
        // assertions themselves).
        let mut rng = Rng::new(777);
        for _ in 0..10 {
            let mut b = Board::new();
            for _ in 0..15 {
                let kind = PieceKind::ALL[rng.next_below(7) as usize];
                let candidates = naive_enumerate_placements(&b, kind);
                if candidates.is_empty() {
                    break;
                }
                let pick = &candidates[rng.next_below(candidates.len() as u32) as usize];
                for (row, col) in pick.occupied_cells() {
                    b.set(row, col, Some(kind));
                }
                b.clear_full_rows();
            }
            boards.push(b);
        }

        boards
    }

    #[test]
    fn skirt_landing_row_matches_naive_drop_loop_on_every_kind_and_board() {
        for board in test_boards() {
            let bb = BitBoard::from_board(&board);
            for &kind in &PieceKind::ALL {
                let fast = placement_set(&enumerate_placements(&bb, kind));
                let naive = placement_set(&naive_enumerate_placements(&board, kind));
                assert_eq!(fast, naive, "mismatch for {kind:?} on board:\n{board:?}");
            }
        }
    }
}
