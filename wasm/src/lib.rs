//! Thin wasm-bindgen adapter over the `engine`/`ai` crates. Contains no game
//! logic of its own — every decision happens in `engine::GameState` or
//! `ai::greedy`; this layer only translates between JS-friendly types and the
//! native Rust API.

use ai::{play_best_cross_move, play_best_move, Weights, DEFAULT_WEIGHTS};
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

/// Mode A ("Independent Cross") as actually specified: four wells sharing
/// ONE piece stream. Only one piece falls at a time; `select_well` commits
/// the next queued piece to a well, then the usual real-time actions
/// (move/rotate/drop/hold/tick) act on whichever piece is currently falling
/// — no arm parameter needed for those, since there's only ever one active
/// piece. No shared resources or garbage coupling yet (later milestones).
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

    /// True when no piece is currently falling — call `select_well` before
    /// anything else will happen.
    pub fn awaiting_well_selection(&self) -> bool {
        self.cross.awaiting_well_selection()
    }

    /// Commits the next queued piece to `arm`. Returns `false` (no-op) if a
    /// piece is already falling or that well has topped out.
    pub fn select_well(&mut self, arm: WasmArm) -> bool {
        self.cross.select_well(arm.into())
    }

    /// The well currently holding the falling piece, or `-1` if none.
    pub fn active_arm(&self) -> i8 {
        self.cross.active_arm().map(|a| a.index() as i8).unwrap_or(-1)
    }

    pub fn move_left(&mut self) {
        self.cross.apply(Action::MoveLeft);
    }

    pub fn move_right(&mut self) {
        self.cross.apply(Action::MoveRight);
    }

    pub fn rotate_cw(&mut self) {
        self.cross.apply(Action::RotateCw);
    }

    pub fn rotate_ccw(&mut self) {
        self.cross.apply(Action::RotateCcw);
    }

    pub fn soft_drop_start(&mut self) {
        self.cross.apply(Action::SoftDropStart);
    }

    pub fn soft_drop_end(&mut self) {
        self.cross.apply(Action::SoftDropEnd);
    }

    pub fn hard_drop(&mut self) {
        self.cross.apply(Action::HardDrop);
    }

    pub fn hold(&mut self) {
        self.cross.apply(Action::Hold);
    }

    /// Advances gravity/lock-delay for whichever piece is currently falling.
    /// No-op while `awaiting_well_selection()` is true.
    pub fn tick(&mut self, dt_ms: f64) {
        self.cross.apply(Action::Tick(dt_ms));
    }

    /// Computes the best (well, rotation, column) for the upcoming piece and
    /// plays it via a hard drop. No-op if a piece is already falling.
    pub fn ai_step(&mut self) {
        play_best_cross_move(&mut self.cross, &self.ai_weights);
    }

    pub fn board_buffer(&self, arm: WasmArm) -> Vec<u8> {
        self.cross.rendered_board(arm.into()).visible_buffer()
    }

    pub fn score(&self, arm: WasmArm) -> u32 {
        self.cross.well(arm.into()).score
    }

    pub fn total_score(&self) -> u32 {
        self.cross.total_score()
    }

    pub fn level(&self, arm: WasmArm) -> u32 {
        self.cross.well(arm.into()).level
    }

    pub fn lines_cleared(&self, arm: WasmArm) -> u32 {
        self.cross.well(arm.into()).lines_cleared_total
    }

    pub fn arm_game_over(&self, arm: WasmArm) -> bool {
        self.cross.well(arm.into()).game_over
    }

    /// True once any single well has topped out (this mode's game-over rule).
    pub fn is_game_over(&self) -> bool {
        self.cross.is_game_over()
    }

    /// 0 if no piece is currently falling, else 1..=7 (PieceKind).
    pub fn active_piece_kind(&self) -> u8 {
        self.cross.active_piece().map(|p| p.kind.as_u8()).unwrap_or(0)
    }

    /// -1 if `arm`'s hold slot is empty, else 1..=7 (PieceKind).
    pub fn hold_piece_kind(&self, arm: WasmArm) -> i8 {
        self.cross.well(arm.into()).hold.map(|k| k.as_u8() as i8).unwrap_or(-1)
    }

    /// The next `n` upcoming piece kinds from the one shared queue.
    pub fn next_queue(&mut self, n: usize) -> Vec<u8> {
        self.cross.next_queue(n).into_iter().map(|k| k.as_u8()).collect()
    }
}
