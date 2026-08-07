use engine::piece::{ActivePiece, PieceKind, Rotation};
use engine::{Action, GameState, BOARD_TOTAL_HEIGHT, BOARD_WIDTH};

const BOTTOM: i32 = (BOARD_TOTAL_HEIGHT - 1) as i32;

fn fill_row_except(state: &mut GameState, row: i32, keep_open: &[i32]) {
    for col in 0..BOARD_WIDTH as i32 {
        if !keep_open.contains(&col) {
            state.board.set(row, col, Some(PieceKind::J));
        }
    }
}

#[test]
fn single_line_clear_scores_and_empties_board() {
    let mut state = GameState::new(1);
    fill_row_except(&mut state, BOTTOM, &[0, 1, 2, 3]);
    state.active = Some(ActivePiece {
        kind: PieceKind::I,
        rotation: Rotation::R0,
        row: BOTTOM - 1, // I's R0 shape occupies box-relative row 1 -> lands on BOTTOM
        col: 0,
    });

    state.apply(Action::HardDrop);

    assert_eq!(state.lines_cleared_total, 1);
    assert_eq!(state.score, 100); // Single at level 1
    for col in 0..BOARD_WIDTH as i32 {
        assert_eq!(state.board.column_height(col as usize), 0, "board should be empty after the only filled row clears");
    }
}

#[test]
fn double_line_clear_scores_300() {
    let mut state = GameState::new(2);
    fill_row_except(&mut state, BOTTOM, &[0, 1]);
    fill_row_except(&mut state, BOTTOM - 1, &[0, 1]);
    state.active = Some(ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::R0,
        row: BOTTOM - 1,
        col: 0,
    });

    state.apply(Action::HardDrop);

    assert_eq!(state.lines_cleared_total, 2);
    assert_eq!(state.score, 300);
}

#[test]
fn hard_drop_awards_two_points_per_cell_dropped() {
    let mut state = GameState::new(3);
    // Force a piece far above the floor on an otherwise empty board so we can
    // measure exactly how far it drops.
    state.active = Some(ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::R0,
        row: 0,
        col: 4,
    });
    let expected_drop_rows = BOTTOM - 1; // O's cells land on rows BOTTOM-1..=BOTTOM
    state.apply(Action::HardDrop);
    assert_eq!(state.score, (expected_drop_rows as u64) * 2);
}

/// Score is `u64`, not `u32`: an AI benchmark run (200,000 pieces at
/// dev-seed difficulty) reached ~4.0 billion points — only ~270 million
/// below `u32::MAX` (4,294,967,295) at "just" level ~8000, well within reach
/// of a longer game. `score` must accumulate past `u32::MAX` without
/// silently wrapping, since neither the engine nor its release build has
/// overflow checks enabled by default.
#[test]
fn score_accumulates_correctly_past_u32_max() {
    let mut state = GameState::new(1);
    state.score = u32::MAX as u64 - 50;
    fill_row_except(&mut state, BOTTOM, &[0, 1, 2, 3]);
    state.active = Some(ActivePiece { kind: PieceKind::I, rotation: Rotation::R0, row: BOTTOM - 1, col: 0 });

    state.apply(Action::HardDrop); // Single at level 1 = 100 points

    assert_eq!(state.score, u32::MAX as u64 - 50 + 100);
    assert!(state.score > u32::MAX as u64, "score must have crossed u32::MAX, not wrapped back below it");
}

#[test]
fn top_out_sets_game_over_when_next_spawn_is_blocked() {
    let mut state = GameState::new(4);
    let active = state.active.expect("game starts with an active piece");

    // Fill the entire board except one column, so no row is ever complete
    // (preventing a clear from reopening space) and every spawn area is blocked.
    for row in 0..BOARD_TOTAL_HEIGHT as i32 {
        for col in 0..BOARD_WIDTH as i32 {
            if col != BOARD_WIDTH as i32 - 1 && !active.occupied_cells().contains(&(row, col)) {
                state.board.set(row, col, Some(PieceKind::J));
            }
        }
    }

    state.apply(Action::HardDrop);

    assert!(state.game_over, "spawning into a fully blocked area must top out");
}
