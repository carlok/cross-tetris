use ai::{best_placement, play_best_move, DEFAULT_WEIGHTS};
use engine::piece::{ActivePiece, PieceKind, Rotation};
use engine::rotation::piece_fits;
use engine::{GameState, BOARD_TOTAL_HEIGHT, BOARD_WIDTH};

const BOTTOM: i32 = (BOARD_TOTAL_HEIGHT - 1) as i32;

#[test]
fn best_placement_is_deterministic_for_a_given_board_and_piece() {
    let state = GameState::new(42);
    let a = best_placement(&state, &DEFAULT_WEIGHTS);
    let b = best_placement(&state, &DEFAULT_WEIGHTS);
    assert_eq!(a, b);
}

#[test]
fn best_placement_is_always_legal() {
    let mut state = GameState::new(7);
    for _ in 0..30 {
        if state.game_over {
            break;
        }
        play_best_move(&mut state, &DEFAULT_WEIGHTS);
        if let Some(active) = state.active {
            assert!(piece_fits(&state.board, &active));
        }
    }
}

#[test]
fn greedy_ai_completes_an_obviously_available_line_clear() {
    // Fill the bottom row except a 4-wide gap that exactly fits a horizontal I.
    let mut state = GameState::new(1);
    for col in 0..BOARD_WIDTH as i32 {
        if !(0..4).contains(&col) {
            state.board.set(BOTTOM, col, Some(PieceKind::J));
        }
    }
    state.active = Some(ActivePiece {
        kind: PieceKind::I,
        rotation: Rotation::R,
        row: 15,
        col: 5,
    });

    play_best_move(&mut state, &DEFAULT_WEIGHTS);

    assert_eq!(state.lines_cleared_total, 1, "the greedy AI should find and take the available line clear");
}

fn count_holes(board: &engine::Board) -> u32 {
    let mut holes = 0;
    for col in 0..BOARD_WIDTH as i32 {
        let mut found_filled = false;
        for row in 0..BOARD_TOTAL_HEIGHT as i32 {
            if board.is_occupied(row, col) {
                found_filled = true;
            } else if found_filled {
                holes += 1;
            }
        }
    }
    holes
}

#[test]
fn greedy_ai_avoids_an_obviously_worse_placement_with_holes() {
    // An almost-flat board with a 1-wide notch at column 5: dropping the O
    // piece straddling the notch buries a hole underneath; dropping it flush
    // elsewhere does not. The AI should prefer a flush, hole-free placement.
    let mut state = GameState::new(2);
    for col in 0..BOARD_WIDTH as i32 {
        let height = if col == 5 { 0 } else { 2 };
        for h in 0..height {
            state.board.set(BOTTOM - h, col, Some(PieceKind::J));
        }
    }
    state.active = Some(ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::R0,
        row: 10,
        col: 4,
    });

    play_best_move(&mut state, &DEFAULT_WEIGHTS);

    assert_eq!(
        count_holes(&state.board),
        0,
        "the greedy AI should not have buried a hole when a flush, hole-free placement was available"
    );
}
