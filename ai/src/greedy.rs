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

use engine::board::{Board, BOARD_WIDTH};
use engine::piece::{ActivePiece, PieceKind, Rotation};
use engine::rotation::{piece_fits, shape};
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

fn column_heights(board: &Board) -> [u32; BOARD_WIDTH] {
    let mut heights = [0u32; BOARD_WIDTH];
    for (col, h) in heights.iter_mut().enumerate() {
        *h = board.column_height(col);
    }
    heights
}

fn count_holes(board: &Board) -> u32 {
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

fn extract_features(board: &Board, lines_cleared: u32) -> Features {
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
/// and the number of lines that placement clears.
fn simulate_lock(board: &Board, piece: &ActivePiece) -> (Board, u32) {
    let mut b = board.clone();
    for (row, col) in piece.occupied_cells() {
        b.set(row, col, Some(piece.kind));
    }
    let cleared = b.clear_full_rows();
    (b, cleared)
}

/// Every legal final resting placement of `kind` on `board`: for each SRS
/// rotation state and every horizontally in-bounds column, the piece dropped
/// straight down to where it first collides.
fn enumerate_placements(board: &Board, kind: PieceKind) -> Vec<ActivePiece> {
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
            // Start well above the board (always legal, per Board::get's open-above-top rule)
            // and fall until the next row down would collide.
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
    let candidates = enumerate_placements(&state.board, active.kind);
    candidates
        .into_iter()
        .map(|piece| {
            let (resulting_board, lines_cleared) = simulate_lock(&state.board, &piece);
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
