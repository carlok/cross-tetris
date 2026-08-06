//! Thin wasm-bindgen adapter over the `engine`/`ai` crates. Contains no game
//! logic of its own — every decision happens in `engine::GameState` or
//! `ai::greedy`; this layer only translates between JS-friendly types and the
//! native Rust API.

use ai::{play_best_move, play_best_move_all, Weights, DEFAULT_WEIGHTS};
use engine::{Action, Arm, CrossGame, GameState};
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

/// JS-facing arm selector, index-compatible with `engine::cross::Arm`.
#[wasm_bindgen]
#[derive(Copy, Clone)]
pub enum WasmArm {
    North,
    East,
    South,
    West,
}

impl From<WasmArm> for Arm {
    fn from(arm: WasmArm) -> Arm {
        match arm {
            WasmArm::North => Arm::North,
            WasmArm::East => Arm::East,
            WasmArm::South => Arm::South,
            WasmArm::West => Arm::West,
        }
    }
}

/// Mode A ("Independent Cross"): four independent boards driven through one
/// bridge object. Each arm is a fully independent `engine::GameState`; this
/// type only fans actions out to the selected arm and aggregates read-only
/// bookkeeping (total score, any-arm-topped-out). No shared resources or
/// garbage coupling yet (later milestones).
#[wasm_bindgen]
pub struct WasmCrossGame {
    cross: CrossGame,
    ai_weights: Weights,
}

#[wasm_bindgen]
impl WasmCrossGame {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64) -> WasmCrossGame {
        WasmCrossGame {
            cross: CrossGame::new(seed),
            ai_weights: DEFAULT_WEIGHTS,
        }
    }

    pub fn move_left(&mut self, arm: WasmArm) {
        self.cross.arm_mut(arm.into()).apply(Action::MoveLeft);
    }

    pub fn move_right(&mut self, arm: WasmArm) {
        self.cross.arm_mut(arm.into()).apply(Action::MoveRight);
    }

    pub fn rotate_cw(&mut self, arm: WasmArm) {
        self.cross.arm_mut(arm.into()).apply(Action::RotateCw);
    }

    pub fn rotate_ccw(&mut self, arm: WasmArm) {
        self.cross.arm_mut(arm.into()).apply(Action::RotateCcw);
    }

    pub fn soft_drop_start(&mut self, arm: WasmArm) {
        self.cross.arm_mut(arm.into()).apply(Action::SoftDropStart);
    }

    pub fn soft_drop_end(&mut self, arm: WasmArm) {
        self.cross.arm_mut(arm.into()).apply(Action::SoftDropEnd);
    }

    pub fn hard_drop(&mut self, arm: WasmArm) {
        self.cross.arm_mut(arm.into()).apply(Action::HardDrop);
    }

    pub fn hold(&mut self, arm: WasmArm) {
        self.cross.arm_mut(arm.into()).apply(Action::Hold);
    }

    /// Advances gravity/lock-delay on every arm by `dt_ms` milliseconds.
    pub fn tick(&mut self, dt_ms: f64) {
        self.cross.tick_all(dt_ms);
    }

    /// Runs the greedy AI on every non-topped-out arm independently.
    pub fn ai_step_all(&mut self) {
        play_best_move_all(&mut self.cross, &self.ai_weights);
    }

    /// Runs the greedy AI on a single arm (for a "human controls one arm,
    /// AI controls the rest" mix, if the UI wants that later).
    pub fn ai_step(&mut self, arm: WasmArm) {
        let state = self.cross.arm_mut(arm.into());
        if !state.game_over {
            play_best_move(state, &self.ai_weights);
        }
    }

    pub fn board_buffer(&self, arm: WasmArm) -> Vec<u8> {
        self.cross.arm(arm.into()).rendered_board().visible_buffer()
    }

    pub fn score(&self, arm: WasmArm) -> u32 {
        self.cross.arm(arm.into()).score
    }

    pub fn total_score(&self) -> u32 {
        self.cross.total_score()
    }

    pub fn level(&self, arm: WasmArm) -> u32 {
        self.cross.arm(arm.into()).level
    }

    pub fn lines_cleared(&self, arm: WasmArm) -> u32 {
        self.cross.arm(arm.into()).lines_cleared_total
    }

    pub fn arm_game_over(&self, arm: WasmArm) -> bool {
        self.cross.arm(arm.into()).game_over
    }

    /// True once any single arm has topped out (this mode's game-over rule).
    pub fn is_game_over(&self) -> bool {
        self.cross.is_game_over()
    }

    pub fn active_piece_kind(&self, arm: WasmArm) -> u8 {
        self.cross.arm(arm.into()).active.map(|p| p.kind.as_u8()).unwrap_or(0)
    }

    pub fn hold_piece_kind(&self, arm: WasmArm) -> i8 {
        self.cross.arm(arm.into()).hold.map(|k| k.as_u8() as i8).unwrap_or(-1)
    }

    pub fn next_queue(&mut self, arm: WasmArm, n: usize) -> Vec<u8> {
        self.cross.arm_mut(arm.into()).next_queue(n).into_iter().map(|k| k.as_u8()).collect()
    }
}
