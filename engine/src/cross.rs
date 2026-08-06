//! Mode A ("Independent Cross") as actually specified for this game: four
//! standard Tetris wells arranged N/E/S/W sharing **one** piece stream. Only
//! one piece is ever falling at a time; before it spawns, the player/AI
//! commits it to a well (`select_well`), and it then plays out as ordinary
//! real-time Tetris in that well (move/rotate/soft/hard drop, gravity, lock
//! delay) until it locks — at which point the next piece from the shared
//! queue needs a well selected again. This is deliberately *not* four
//! independent simultaneous games: the four wells are locked stacks waiting
//! their turn, and the single shared queue is what makes them one game
//! rather than four unrelated ones.
//!
//! Each well keeps its own board, score, level, hold slot, and top-out flag.
//! No garbage coupling or shared action budget yet (later milestones).

use crate::bag::SevenBag;
use crate::board::Board;
use crate::game::{LINES_PER_LEVEL, LOCK_DELAY_MS, MAX_LOCK_RESETS, SOFT_DROP_MULTIPLIER, SPAWN_ROW};
use crate::piece::{ActivePiece, PieceKind, Rotation};
use crate::rotation::{piece_fits, try_rotate};
use crate::scoring::{gravity_ms_per_row, line_clear_score, HARD_DROP_POINTS_PER_CELL, SOFT_DROP_POINTS_PER_CELL};
use crate::Action;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Arm {
    North,
    East,
    South,
    West,
}

impl Arm {
    pub const ALL: [Arm; 4] = [Arm::North, Arm::East, Arm::South, Arm::West];

    pub fn index(self) -> usize {
        match self {
            Arm::North => 0,
            Arm::East => 1,
            Arm::South => 2,
            Arm::West => 3,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Well {
    pub board: Board,
    pub score: u32,
    pub level: u32,
    pub lines_cleared_total: u32,
    pub hold: Option<PieceKind>,
    pub game_over: bool,
}

impl Well {
    fn new() -> Self {
        Well {
            board: Board::new(),
            score: 0,
            level: 1,
            lines_cleared_total: 0,
            hold: None,
            game_over: false,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
struct ActiveInWell {
    arm: Arm,
    piece: ActivePiece,
}

#[derive(Clone, PartialEq, Debug)]
pub struct CrossGame {
    pub wells: [Well; 4],
    bag: SevenBag,
    active: Option<ActiveInWell>,
    hold_used_this_piece: bool,
    soft_dropping: bool,
    gravity_accum_ms: f64,
    lock_timer_ms: Option<f64>,
    lock_reset_count: u32,
}

impl CrossGame {
    pub fn new(seed: u64) -> Self {
        CrossGame {
            wells: [Well::new(), Well::new(), Well::new(), Well::new()],
            bag: SevenBag::new(seed),
            active: None,
            hold_used_this_piece: false,
            soft_dropping: false,
            gravity_accum_ms: 0.0,
            lock_timer_ms: None,
            lock_reset_count: 0,
        }
    }

    pub fn well(&self, arm: Arm) -> &Well {
        &self.wells[arm.index()]
    }

    /// True when no piece is currently falling — the player/AI must call
    /// `select_well` before anything else will happen.
    pub fn awaiting_well_selection(&self) -> bool {
        self.active.is_none()
    }

    /// Which well currently has the falling piece, if any.
    pub fn active_arm(&self) -> Option<Arm> {
        self.active.map(|a| a.arm)
    }

    pub fn active_piece(&self) -> Option<ActivePiece> {
        self.active.map(|a| a.piece)
    }

    /// Directly overwrites the falling piece's rotation/position. Placement-
    /// level AI convenience that bypasses real-time kick-path movement (same
    /// documented simplification as the single-board greedy AI's use of
    /// `engine::rotation::shape` directly). No-op if no piece is active.
    pub fn force_active_placement(&mut self, rotation: Rotation, row: i32, col: i32) {
        if let Some(a) = self.active {
            self.active = Some(ActiveInWell { arm: a.arm, piece: ActivePiece { rotation, row, col, ..a.piece } });
        }
    }

    /// The upcoming piece kinds, without consuming them — the same queue
    /// regardless of which well they eventually go to.
    pub fn next_queue(&mut self, n: usize) -> Vec<PieceKind> {
        self.bag.preview(n)
    }

    /// Commits the next piece from the shared queue to `arm`. Fails (returns
    /// `false`, no state change) if a piece is already falling or that well
    /// has topped out. If the well can't legally hold a fresh spawn, the well
    /// tops out and the piece is consumed but never becomes active.
    pub fn select_well(&mut self, arm: Arm) -> bool {
        if self.active.is_some() || self.wells[arm.index()].game_over {
            return false;
        }
        let kind = self.bag.next();
        self.spawn_in_well(arm, kind);
        true
    }

    fn spawn_in_well(&mut self, arm: Arm, kind: PieceKind) {
        let piece = ActivePiece::spawn(kind, SPAWN_ROW);
        let well = &mut self.wells[arm.index()];
        if !piece_fits(&well.board, &piece) {
            well.game_over = true;
            self.active = None;
            return;
        }
        self.active = Some(ActiveInWell { arm, piece });
        self.hold_used_this_piece = false;
        self.soft_dropping = false;
        self.gravity_accum_ms = 0.0;
        self.lock_timer_ms = None;
        self.lock_reset_count = 0;
    }

    fn is_grounded(&self) -> bool {
        match self.active {
            None => true,
            Some(a) => {
                let dropped = ActivePiece { row: a.piece.row + 1, ..a.piece };
                !piece_fits(&self.wells[a.arm.index()].board, &dropped)
            }
        }
    }

    fn try_move(&mut self, dr: i32, dc: i32) -> bool {
        let Some(a) = self.active else { return false };
        let candidate = ActivePiece { row: a.piece.row + dr, col: a.piece.col + dc, ..a.piece };
        if piece_fits(&self.wells[a.arm.index()].board, &candidate) {
            self.active = Some(ActiveInWell { arm: a.arm, piece: candidate });
            self.on_successful_shift();
            true
        } else {
            false
        }
    }

    fn on_successful_shift(&mut self) {
        if self.lock_timer_ms.is_some() && self.is_grounded() {
            if self.lock_reset_count < MAX_LOCK_RESETS {
                self.lock_timer_ms = Some(0.0);
                self.lock_reset_count += 1;
            }
        } else if !self.is_grounded() {
            self.lock_timer_ms = None;
        }
    }

    fn lock_active_piece(&mut self) {
        let Some(a) = self.active else { return };
        let well = &mut self.wells[a.arm.index()];
        for (row, col) in a.piece.occupied_cells() {
            well.board.set(row, col, Some(a.piece.kind));
        }
        let cleared = well.board.clear_full_rows();
        well.score += line_clear_score(cleared, well.level);
        well.lines_cleared_total += cleared;
        well.level = well.lines_cleared_total / LINES_PER_LEVEL + 1;
        self.active = None;
        self.soft_dropping = false;
    }

    /// Applies a real-time action to whichever piece is currently falling.
    /// No-op (including `Tick`) if no piece is active — the game is paused
    /// between pieces until `select_well` is called.
    pub fn apply(&mut self, action: Action) {
        let Some(a) = self.active else { return };
        if self.wells[a.arm.index()].game_over {
            return;
        }
        match action {
            Action::MoveLeft => {
                self.try_move(0, -1);
            }
            Action::MoveRight => {
                self.try_move(0, 1);
            }
            Action::RotateCw | Action::RotateCcw => {
                let cw = matches!(action, Action::RotateCw);
                if let Some(rotated) = try_rotate(&self.wells[a.arm.index()].board, &a.piece, cw) {
                    self.active = Some(ActiveInWell { arm: a.arm, piece: rotated });
                    self.on_successful_shift();
                }
            }
            Action::SoftDropStart => self.soft_dropping = true,
            Action::SoftDropEnd => self.soft_dropping = false,
            Action::HardDrop => {
                let mut dropped_rows = 0u32;
                while self.try_move(1, 0) {
                    dropped_rows += 1;
                }
                self.wells[a.arm.index()].score += dropped_rows * HARD_DROP_POINTS_PER_CELL;
                self.lock_active_piece();
            }
            Action::Hold => self.apply_hold(),
            Action::Tick(dt_ms) => self.tick(dt_ms),
        }
    }

    fn apply_hold(&mut self) {
        if self.hold_used_this_piece {
            return;
        }
        let Some(a) = self.active else { return };
        let incoming = self.wells[a.arm.index()].hold.replace(a.piece.kind);
        let kind = match incoming {
            Some(k) => k,
            None => self.bag.next(),
        };
        let piece = ActivePiece::spawn(kind, SPAWN_ROW);
        let well = &mut self.wells[a.arm.index()];
        if !piece_fits(&well.board, &piece) {
            well.game_over = true;
            self.active = None;
            return;
        }
        self.active = Some(ActiveInWell { arm: a.arm, piece });
        self.hold_used_this_piece = true;
        self.gravity_accum_ms = 0.0;
        self.lock_timer_ms = None;
        self.lock_reset_count = 0;
    }

    fn tick(&mut self, dt_ms: f64) {
        let Some(a) = self.active else { return };
        let level = self.wells[a.arm.index()].level;
        let interval = gravity_ms_per_row(level) / if self.soft_dropping { SOFT_DROP_MULTIPLIER } else { 1.0 };
        self.gravity_accum_ms += dt_ms;
        while self.gravity_accum_ms >= interval {
            self.gravity_accum_ms -= interval;
            if self.try_move(1, 0) {
                if self.soft_dropping {
                    let arm = self.active.unwrap().arm;
                    self.wells[arm.index()].score += SOFT_DROP_POINTS_PER_CELL;
                }
            } else {
                break;
            }
        }
        if self.active.is_none() {
            return; // locked out mid-loop (shouldn't happen for Tick, but stay safe)
        }
        if self.is_grounded() {
            let elapsed = self.lock_timer_ms.unwrap_or(0.0) + dt_ms;
            if elapsed >= LOCK_DELAY_MS {
                self.lock_active_piece();
            } else {
                self.lock_timer_ms = Some(elapsed);
            }
        } else {
            self.lock_timer_ms = None;
        }
    }

    /// A copy of `arm`'s board with the active piece composited in (if it's
    /// the well currently holding the falling piece), for rendering.
    pub fn rendered_board(&self, arm: Arm) -> Board {
        let mut b = self.wells[arm.index()].board.clone();
        if let Some(a) = self.active {
            if a.arm == arm {
                for (row, col) in a.piece.occupied_cells() {
                    b.set(row, col, Some(a.piece.kind));
                }
            }
        }
        b
    }

    pub fn total_score(&self) -> u32 {
        self.wells.iter().map(|w| w.score).sum()
    }

    /// The game ends when any single well tops out.
    pub fn is_game_over(&self) -> bool {
        self.wells.iter().any(|w| w.game_over)
    }
}
