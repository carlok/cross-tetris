use crate::actions::Action;
use crate::bag::SevenBag;
use crate::board::{Board, BOARD_HIDDEN_ROWS};
use crate::piece::{ActivePiece, PieceKind};
use crate::rotation::{piece_fits, try_rotate};
use crate::scoring::{gravity_ms_per_row, line_clear_score, HARD_DROP_POINTS_PER_CELL, SOFT_DROP_POINTS_PER_CELL};

const LOCK_DELAY_MS: f64 = 500.0;
const MAX_LOCK_RESETS: u32 = 15;
const SOFT_DROP_MULTIPLIER: f64 = 20.0;
const LINES_PER_LEVEL: u32 = 10;
const SPAWN_ROW: i32 = BOARD_HIDDEN_ROWS as i32 - 2;

#[derive(Clone, PartialEq, Debug)]
pub struct GameState {
    pub board: Board,
    pub bag: SevenBag,
    pub active: Option<ActivePiece>,
    pub hold: Option<PieceKind>,
    pub hold_used_this_turn: bool,
    pub score: u32,
    pub level: u32,
    pub lines_cleared_total: u32,
    pub game_over: bool,
    soft_dropping: bool,
    gravity_accum_ms: f64,
    lock_timer_ms: Option<f64>,
    lock_reset_count: u32,
}

impl GameState {
    pub fn new(seed: u64) -> Self {
        let mut state = GameState {
            board: Board::new(),
            bag: SevenBag::new(seed),
            active: None,
            hold: None,
            hold_used_this_turn: false,
            score: 0,
            level: 1,
            lines_cleared_total: 0,
            game_over: false,
            soft_dropping: false,
            gravity_accum_ms: 0.0,
            lock_timer_ms: None,
            lock_reset_count: 0,
        };
        state.spawn_next();
        state
    }

    fn spawn_next(&mut self) {
        let kind = self.bag.next();
        let piece = ActivePiece::spawn(kind, SPAWN_ROW);
        if !piece_fits(&self.board, &piece) {
            self.game_over = true;
        }
        self.active = Some(piece);
        self.hold_used_this_turn = false;
        self.gravity_accum_ms = 0.0;
        self.lock_timer_ms = None;
        self.lock_reset_count = 0;
    }

    pub fn next_queue(&mut self, n: usize) -> Vec<PieceKind> {
        self.bag.preview(n)
    }

    fn is_grounded(&self) -> bool {
        match &self.active {
            None => true,
            Some(p) => {
                let dropped = ActivePiece { row: p.row + 1, ..*p };
                !piece_fits(&self.board, &dropped)
            }
        }
    }

    fn try_move(&mut self, dr: i32, dc: i32) -> bool {
        let Some(p) = self.active else { return false };
        let candidate = ActivePiece {
            row: p.row + dr,
            col: p.col + dc,
            ..p
        };
        if piece_fits(&self.board, &candidate) {
            self.active = Some(candidate);
            self.on_successful_shift();
            true
        } else {
            false
        }
    }

    /// Resets the lock-delay timer after a successful move/rotate while
    /// grounded, up to a capped number of resets (prevents infinite-lock lock
    /// evasion, per the documented lock-delay simplification).
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
        let Some(p) = self.active else { return };
        for (row, col) in p.occupied_cells() {
            self.board.set(row, col, Some(p.kind));
        }
        let cleared = self.board.clear_full_rows();
        self.score += line_clear_score(cleared, self.level);
        self.lines_cleared_total += cleared;
        self.level = self.lines_cleared_total / LINES_PER_LEVEL + 1;
        self.active = None;
        self.soft_dropping = false;
        self.spawn_next();
    }

    pub fn apply(&mut self, action: Action) {
        if self.game_over {
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
                let Some(p) = self.active else { return };
                let cw = matches!(action, Action::RotateCw);
                if let Some(rotated) = try_rotate(&self.board, &p, cw) {
                    self.active = Some(rotated);
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
                self.score += dropped_rows * HARD_DROP_POINTS_PER_CELL;
                self.lock_active_piece();
            }
            Action::Hold => self.apply_hold(),
            Action::Tick(dt_ms) => self.tick(dt_ms),
        }
    }

    fn apply_hold(&mut self) {
        if self.hold_used_this_turn {
            return;
        }
        let Some(active) = self.active else { return };
        let incoming = self.hold.replace(active.kind);
        let kind = match incoming {
            Some(k) => k,
            None => self.bag.next(),
        };
        let piece = ActivePiece::spawn(kind, SPAWN_ROW);
        if !piece_fits(&self.board, &piece) {
            self.game_over = true;
        }
        self.active = Some(piece);
        self.hold_used_this_turn = true;
        self.gravity_accum_ms = 0.0;
        self.lock_timer_ms = None;
        self.lock_reset_count = 0;
    }

    fn tick(&mut self, dt_ms: f64) {
        if self.active.is_none() {
            return;
        }
        let interval = gravity_ms_per_row(self.level)
            / if self.soft_dropping { SOFT_DROP_MULTIPLIER } else { 1.0 };
        self.gravity_accum_ms += dt_ms;
        while self.gravity_accum_ms >= interval {
            self.gravity_accum_ms -= interval;
            if self.try_move(1, 0) {
                if self.soft_dropping {
                    self.score += SOFT_DROP_POINTS_PER_CELL;
                }
            } else {
                break;
            }
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

    /// A copy of the board with the active piece composited in, for rendering.
    pub fn rendered_board(&self) -> Board {
        let mut b = self.board.clone();
        if let Some(p) = &self.active {
            for (row, col) in p.occupied_cells() {
                b.set(row, col, Some(p.kind));
            }
        }
        b
    }
}
