//! Thin wasm-bindgen adapter over the `engine`/`ai` crates. Contains no game
//! logic of its own — every decision happens in `engine::GameState` or
//! `ai::greedy`; this layer only translates between JS-friendly types and the
//! native Rust API.

use ai::{play_best_move, Weights, DEFAULT_WEIGHTS};
use engine::{Action, GameState};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmGame {
    state: GameState,
    ai_weights: Weights,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64) -> WasmGame {
        WasmGame {
            state: GameState::new(seed),
            ai_weights: DEFAULT_WEIGHTS,
        }
    }

    pub fn move_left(&mut self) {
        self.state.apply(Action::MoveLeft);
    }

    pub fn move_right(&mut self) {
        self.state.apply(Action::MoveRight);
    }

    pub fn rotate_cw(&mut self) {
        self.state.apply(Action::RotateCw);
    }

    pub fn rotate_ccw(&mut self) {
        self.state.apply(Action::RotateCcw);
    }

    pub fn soft_drop_start(&mut self) {
        self.state.apply(Action::SoftDropStart);
    }

    pub fn soft_drop_end(&mut self) {
        self.state.apply(Action::SoftDropEnd);
    }

    pub fn hard_drop(&mut self) {
        self.state.apply(Action::HardDrop);
    }

    pub fn hold(&mut self) {
        self.state.apply(Action::Hold);
    }

    /// Advances gravity/lock-delay timers by `dt_ms` milliseconds.
    pub fn tick(&mut self, dt_ms: f64) {
        self.state.apply(Action::Tick(dt_ms));
    }

    /// Computes and applies the greedy rule-based AI's best move for the
    /// current active piece, via a hard drop.
    pub fn ai_step(&mut self) {
        play_best_move(&mut self.state, &self.ai_weights);
    }

    /// Flat visible-board buffer, row-major top-to-bottom, one byte per cell
    /// (0 = empty, 1..=7 = PieceKind), with the active piece composited in.
    /// The only per-frame hot-path call; everything else below is a cheap
    /// scalar getter.
    pub fn board_buffer(&self) -> Vec<u8> {
        self.state.rendered_board().visible_buffer()
    }

    pub fn score(&self) -> u32 {
        self.state.score
    }

    pub fn level(&self) -> u32 {
        self.state.level
    }

    pub fn lines_cleared(&self) -> u32 {
        self.state.lines_cleared_total
    }

    pub fn is_game_over(&self) -> bool {
        self.state.game_over
    }

    /// 0 if there is no active piece, else 1..=7 (PieceKind).
    pub fn active_piece_kind(&self) -> u8 {
        self.state.active.map(|p| p.kind.as_u8()).unwrap_or(0)
    }

    /// -1 if the hold slot is empty, else 1..=7 (PieceKind).
    pub fn hold_piece_kind(&self) -> i8 {
        self.state.hold.map(|k| k.as_u8() as i8).unwrap_or(-1)
    }

    /// The next `n` upcoming piece kinds (1..=7 each), without consuming them.
    pub fn next_queue(&mut self, n: usize) -> Vec<u8> {
        self.state.next_queue(n).into_iter().map(|k| k.as_u8()).collect()
    }
}
