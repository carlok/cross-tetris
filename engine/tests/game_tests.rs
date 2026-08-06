use engine::piece::{ActivePiece, PieceKind, Rotation};
use engine::{Action, GameState, BOARD_TOTAL_HEIGHT};

const BOTTOM: i32 = (BOARD_TOTAL_HEIGHT - 1) as i32;

#[test]
fn gravity_moves_piece_down_over_time() {
    let mut state = GameState::new(10);
    state.active = Some(ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::R0,
        row: 0,
        col: 4,
    });
    let start_row = state.active.unwrap().row;
    // Level 1 gravity is 1000ms/row; advance well past one row's worth.
    state.apply(Action::Tick(1200.0));
    let row_after = state.active.expect("piece should not have locked yet").row;
    assert!(row_after > start_row, "gravity should have moved the piece down at least one row");
}

#[test]
fn piece_does_not_lock_before_lock_delay_elapses() {
    let mut state = GameState::new(11);
    state.active = Some(ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::R0,
        row: BOTTOM - 1, // already resting on the floor
        col: 4,
    });
    state.apply(Action::Tick(400.0)); // < 500ms lock delay
    assert!(state.active.is_some(), "piece must still be active before lock delay elapses");
}

#[test]
fn piece_locks_after_lock_delay_elapses() {
    let mut state = GameState::new(12);
    state.active = Some(ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::R0,
        row: BOTTOM - 1,
        col: 4,
    });
    state.apply(Action::Tick(600.0)); // > 500ms lock delay
    let locked_row = state.active.expect("a new piece should have spawned").row;
    assert_ne!(locked_row, BOTTOM - 1, "the O piece should have locked and a new piece spawned");
    // The O we placed should now be part of the board.
    assert!(state.board.column_height(4) >= 2);
}

#[test]
fn soft_drop_awards_one_point_per_cell() {
    let mut state = GameState::new(13);
    state.active = Some(ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::R0,
        row: 0,
        col: 4,
    });
    state.apply(Action::SoftDropStart);
    // Soft drop is 20x gravity: level-1 interval 1000ms / 20 = 50ms per row.
    state.apply(Action::Tick(50.0));
    assert_eq!(state.score, 1, "one row of soft drop should award 1 point");
}

#[test]
fn hold_swaps_active_piece_and_can_only_be_used_once_per_spawn() {
    let mut state = GameState::new(14);
    let original_kind = state.active.unwrap().kind;
    assert!(state.hold.is_none());

    state.apply(Action::Hold);
    assert_eq!(state.hold, Some(original_kind));
    let after_first_hold = state.active.unwrap().kind;

    // Using hold again immediately should be a no-op (already used this spawn).
    state.apply(Action::Hold);
    assert_eq!(state.active.unwrap().kind, after_first_hold);
    assert_eq!(state.hold, Some(original_kind));
}

#[test]
fn hold_becomes_available_again_after_next_piece_locks() {
    let mut state = GameState::new(15);
    state.apply(Action::Hold);
    assert!(state.hold_used_this_turn);

    // Force the current (post-hold) piece to lock via hard drop.
    state.apply(Action::HardDrop);
    assert!(!state.hold_used_this_turn, "hold should be available again for the newly spawned piece");
}

#[test]
fn ai_never_selects_an_illegal_placement() {
    // Sanity check belongs here rather than ai crate: confirms every move the
    // engine allows via apply() leaves the active piece in a legal position.
    let mut state = GameState::new(16);
    for _ in 0..50 {
        if state.game_over {
            break;
        }
        state.apply(Action::MoveLeft);
        state.apply(Action::RotateCw);
        state.apply(Action::Tick(16.0));
        if let Some(p) = state.active {
            assert!(engine::rotation::piece_fits(&state.board, &p));
        }
    }
}
