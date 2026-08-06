//! SRS (Super Rotation System) piece shapes and wall-kick tables.
//!
//! Shape and kick data are transcribed from the standard Tetris Guideline SRS
//! specification. Kick offsets are stored in the conventional `(dx, dy)` form
//! where `+x` is right and `+y` is *up*; because this engine's `row` increases
//! downward, a kick is applied as `col += dx; row -= dy`.

use crate::board::Board;
use crate::piece::{ActivePiece, PieceKind, Rotation};

/// The 4 occupied cells of `kind` in `rotation`, as (row, col) offsets from the
/// piece's bounding-box top-left.
pub fn shape(kind: PieceKind, rotation: Rotation) -> [(i32, i32); 4] {
    use Rotation::*;
    match kind {
        PieceKind::O => [(0, 0), (0, 1), (1, 0), (1, 1)],
        PieceKind::I => match rotation {
            R0 => [(1, 0), (1, 1), (1, 2), (1, 3)],
            R => [(0, 2), (1, 2), (2, 2), (3, 2)],
            R2 => [(2, 0), (2, 1), (2, 2), (2, 3)],
            L => [(0, 1), (1, 1), (2, 1), (3, 1)],
        },
        PieceKind::J => match rotation {
            R0 => [(0, 0), (1, 0), (1, 1), (1, 2)],
            R => [(0, 1), (0, 2), (1, 1), (2, 1)],
            R2 => [(1, 0), (1, 1), (1, 2), (2, 2)],
            L => [(0, 1), (1, 1), (2, 0), (2, 1)],
        },
        PieceKind::L => match rotation {
            R0 => [(0, 2), (1, 0), (1, 1), (1, 2)],
            R => [(0, 1), (1, 1), (2, 1), (2, 2)],
            R2 => [(1, 0), (1, 1), (1, 2), (2, 0)],
            L => [(0, 0), (0, 1), (1, 1), (2, 1)],
        },
        PieceKind::S => match rotation {
            R0 => [(0, 1), (0, 2), (1, 0), (1, 1)],
            R => [(0, 1), (1, 1), (1, 2), (2, 2)],
            R2 => [(1, 1), (1, 2), (2, 0), (2, 1)],
            L => [(0, 0), (1, 0), (1, 1), (2, 1)],
        },
        PieceKind::Z => match rotation {
            R0 => [(0, 0), (0, 1), (1, 1), (1, 2)],
            R => [(0, 2), (1, 1), (1, 2), (2, 1)],
            R2 => [(1, 0), (1, 1), (2, 1), (2, 2)],
            L => [(0, 1), (1, 0), (1, 1), (2, 0)],
        },
        PieceKind::T => match rotation {
            R0 => [(0, 1), (1, 0), (1, 1), (1, 2)],
            R => [(0, 1), (1, 1), (1, 2), (2, 1)],
            R2 => [(1, 0), (1, 1), (1, 2), (2, 1)],
            L => [(0, 1), (1, 0), (1, 1), (2, 1)],
        },
    }
}

/// The 5 kick offsets to try, in order (first is always (0,0), the unkicked attempt).
fn jlstz_kicks(from: Rotation, to: Rotation) -> [(i32, i32); 5] {
    use Rotation::*;
    match (from, to) {
        (R0, R) => [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
        (R, R0) => [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
        (R, R2) => [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
        (R2, R) => [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
        (R2, L) => [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
        (L, R2) => [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
        (L, R0) => [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
        (R0, L) => [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
        _ => [(0, 0); 5],
    }
}

fn i_kicks(from: Rotation, to: Rotation) -> [(i32, i32); 5] {
    use Rotation::*;
    match (from, to) {
        (R0, R) => [(0, 0), (-2, 0), (1, 0), (-2, -1), (1, 2)],
        (R, R0) => [(0, 0), (2, 0), (-1, 0), (2, 1), (-1, -2)],
        (R, R2) => [(0, 0), (-1, 0), (2, 0), (-1, 2), (2, -1)],
        (R2, R) => [(0, 0), (1, 0), (-2, 0), (1, -2), (-2, 1)],
        (R2, L) => [(0, 0), (2, 0), (-1, 0), (2, 1), (-1, -2)],
        (L, R2) => [(0, 0), (-2, 0), (1, 0), (-2, -1), (1, 2)],
        (L, R0) => [(0, 0), (1, 0), (-2, 0), (1, -2), (-2, 1)],
        (R0, L) => [(0, 0), (-1, 0), (2, 0), (-1, 2), (2, -1)],
        _ => [(0, 0); 5],
    }
}

fn kicks_for(kind: PieceKind, from: Rotation, to: Rotation) -> [(i32, i32); 5] {
    match kind {
        PieceKind::O => [(0, 0); 5],
        PieceKind::I => i_kicks(from, to),
        _ => jlstz_kicks(from, to),
    }
}

pub fn piece_fits(board: &Board, piece: &ActivePiece) -> bool {
    piece
        .occupied_cells()
        .iter()
        .all(|&(row, col)| !board.is_occupied(row, col))
}

/// Attempts to rotate `piece` clockwise (`cw = true`) or counter-clockwise,
/// trying each SRS kick offset in order. Returns the first legal resulting
/// piece, or `None` if all kicks fail.
pub fn try_rotate(board: &Board, piece: &ActivePiece, cw: bool) -> Option<ActivePiece> {
    let to = if cw {
        piece.rotation.cw()
    } else {
        piece.rotation.ccw()
    };
    let from = piece.rotation;
    for (dx, dy) in kicks_for(piece.kind, from, to) {
        let candidate = piece.with_rotation(to, piece.row - dy, piece.col + dx);
        if piece_fits(board, &candidate) {
            return Some(candidate);
        }
    }
    None
}
