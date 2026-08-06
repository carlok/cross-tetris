use engine::piece::{ActivePiece, PieceKind, Rotation};
use engine::rotation::piece_fits;
use engine::{Board, BOARD_TOTAL_HEIGHT, BOARD_WIDTH};

#[test]
fn piece_fits_on_empty_board() {
    let board = Board::new();
    let piece = ActivePiece::spawn(PieceKind::T, 18);
    assert!(piece_fits(&board, &piece));
}

#[test]
fn piece_does_not_fit_past_left_wall() {
    let board = Board::new();
    let piece = ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::R0,
        row: 18,
        col: -1,
    };
    assert!(!piece_fits(&board, &piece));
}

#[test]
fn piece_does_not_fit_past_right_wall() {
    let board = Board::new();
    let piece = ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::R0,
        row: 18,
        col: (BOARD_WIDTH - 1) as i32,
    };
    assert!(!piece_fits(&board, &piece));
}

#[test]
fn piece_does_not_fit_past_floor() {
    let board = Board::new();
    let piece = ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::R0,
        row: (BOARD_TOTAL_HEIGHT - 1) as i32,
        col: 4,
    };
    assert!(!piece_fits(&board, &piece));
}

#[test]
fn piece_fits_above_the_top_of_the_board() {
    // Hidden spawn area allows pieces to float above row 0 without colliding.
    let board = Board::new();
    let piece = ActivePiece {
        kind: PieceKind::I,
        rotation: Rotation::R0,
        row: -5,
        col: 3,
    };
    assert!(piece_fits(&board, &piece));
}

#[test]
fn piece_does_not_fit_into_locked_cells() {
    let mut board = Board::new();
    board.set(19, 4, Some(PieceKind::J));
    let piece = ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::R0,
        row: 18,
        col: 4,
    };
    assert!(!piece_fits(&board, &piece));
}

#[test]
fn piece_fits_next_to_but_not_overlapping_locked_cells() {
    let mut board = Board::new();
    board.set(19, 4, Some(PieceKind::J));
    let piece = ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::R0,
        row: 17,
        col: 4,
    };
    assert!(piece_fits(&board, &piece));
}
