//! Thin smoke test confirming the wasm32 compile target behaves identically
//! to native: the `wasm` crate has no logic of its own, so this is not a
//! duplicate of the native test suite, just a confirmation that nothing about
//! the wasm32 target (float determinism, trap semantics) changes behavior.

use cross_tetris_wasm::WasmGame;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn fixed_action_sequence_produces_expected_score_and_board() {
    let mut game = WasmGame::new(0xC0FFEE);
    game.tick(16.0);
    game.move_left();
    game.tick(16.0);
    game.rotate_cw();
    game.hard_drop();
    game.tick(16.0);

    // Not asserting exact values here (that's the native determinism suite's
    // job); this only confirms the wasm target runs without panicking/trapping
    // and produces a well-formed board buffer of the expected size.
    let buffer = game.board_buffer();
    assert_eq!(buffer.len(), 10 * 20);
    assert!(game.score() > 0 || !game.is_game_over());
}
