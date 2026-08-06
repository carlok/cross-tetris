use engine::piece::{ActivePiece, PieceKind, Rotation};
use engine::rotation::{piece_fits, shape, try_rotate};
use engine::Board;
use std::collections::HashSet;

fn absolute_cells(piece: &ActivePiece) -> HashSet<(i32, i32)> {
    piece.occupied_cells().into_iter().collect()
}

#[test]
fn four_clockwise_rotations_return_to_original_shape() {
    for &kind in PieceKind::ALL.iter() {
        let board = Board::new();
        let start = ActivePiece::spawn(kind, 18);
        let mut current = start;
        for _ in 0..4 {
            current = try_rotate(&board, &current, true)
                .unwrap_or_else(|| panic!("{kind:?} rotation should always succeed on an empty board"));
        }
        assert_eq!(
            absolute_cells(&current),
            absolute_cells(&start),
            "{kind:?}: 4x CW rotation must return to the original cell set"
        );
    }
}

#[test]
fn four_counterclockwise_rotations_return_to_original_shape() {
    for &kind in PieceKind::ALL.iter() {
        let board = Board::new();
        let start = ActivePiece::spawn(kind, 18);
        let mut current = start;
        for _ in 0..4 {
            current = try_rotate(&board, &current, false).unwrap();
        }
        assert_eq!(absolute_cells(&current), absolute_cells(&start));
    }
}

#[test]
fn o_piece_shape_is_identical_across_all_rotation_states() {
    let base = shape(PieceKind::O, Rotation::R0);
    for &r in &[Rotation::R, Rotation::R2, Rotation::L] {
        assert_eq!(shape(PieceKind::O, r), base);
    }
}

#[test]
fn every_shape_occupies_exactly_four_cells_within_its_bounding_box() {
    for &kind in PieceKind::ALL.iter() {
        for &r in &[Rotation::R0, Rotation::R, Rotation::R2, Rotation::L] {
            let cells = shape(kind, r);
            let unique: HashSet<(i32, i32)> = cells.into_iter().collect();
            assert_eq!(unique.len(), 4, "{kind:?}/{r:?} must occupy 4 distinct cells");
            let box_size = kind.box_size();
            for (row, col) in cells {
                assert!(row >= 0 && row < box_size, "{kind:?}/{r:?} row out of box");
                assert!(col >= 0 && col < box_size, "{kind:?}/{r:?} col out of box");
            }
        }
    }
}

#[test]
fn i_piece_wall_kick_shifts_off_left_wall() {
    // I piece vertical (R), hugging the left wall: rotating to R2 (horizontal)
    // without a kick would put part of the piece out of bounds, so a kick must apply.
    let board = Board::new();
    let piece = ActivePiece {
        kind: PieceKind::I,
        rotation: Rotation::R,
        row: 18,
        col: -2,
    };
    assert!(piece_fits(&board, &piece), "test setup must itself be legal");
    let rotated = try_rotate(&board, &piece, true).expect("kick should resolve the wall conflict");
    assert!(piece_fits(&board, &rotated));
    assert_ne!(
        (rotated.row, rotated.col),
        (piece.row, piece.col),
        "a kick offset must have been applied"
    );
}

#[test]
fn rotation_fails_when_fully_enclosed() {
    // Surround a T piece's spawn footprint entirely with locked cells so every
    // one of the 5 SRS kick candidates is blocked.
    let mut board = Board::new();
    let piece = ActivePiece::spawn(PieceKind::T, 18);
    for row in (piece.row - 3)..=(piece.row + 3) {
        for col in (piece.col - 3)..=(piece.col + 4) {
            if !piece.occupied_cells().contains(&(row, col)) {
                board.set(row, col, Some(PieceKind::J));
            }
        }
    }
    assert!(piece_fits(&board, &piece));
    assert!(try_rotate(&board, &piece, true).is_none());
}
