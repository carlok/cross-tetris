//! Mode A ("Independent Cross"): four standard Tetris wells arranged as a
//! cross, each behaving as a fully independent board — its own active piece,
//! its own queue/bag, its own hold slot. The only cross-board coupling in
//! this mode is bookkeeping: a combined score and a combined game-over check.
//! Shared resources (Milestone 5) and garbage coupling (Milestone 6) are not
//! implemented here.
//!
//! Per the project spec ("rotate every well into a canonical internal
//! orientation so the same board-processing code can be reused for all four
//! arms"), each arm reuses the identical single-board `GameState` untouched;
//! the "canonical orientation" requirement only matters once arms need to
//! exchange pieces/garbage directionally (a later milestone), so it is not
//! yet modeled here — documented simplification.

use crate::game::GameState;

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

/// Decorrelates a single master seed into 4 per-arm seeds so the arms don't
/// all draw the exact same piece sequence. Not cryptographic — just enough
/// mixing (splitmix64-style) that arm bags look independent.
fn derive_seed(master: u64, arm_index: usize) -> u64 {
    let mut z = master.wrapping_add(0x9E3779B97F4A7C15u64.wrapping_mul(arm_index as u64 + 1));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

#[derive(Clone, PartialEq, Debug)]
pub struct CrossGame {
    pub arms: [GameState; 4],
}

impl CrossGame {
    pub fn new(master_seed: u64) -> Self {
        CrossGame {
            arms: Arm::ALL.map(|arm| GameState::new(derive_seed(master_seed, arm.index()))),
        }
    }

    pub fn arm(&self, arm: Arm) -> &GameState {
        &self.arms[arm.index()]
    }

    pub fn arm_mut(&mut self, arm: Arm) -> &mut GameState {
        &mut self.arms[arm.index()]
    }

    /// Advances gravity/lock-delay on every arm by the same `dt_ms`. Topped-out
    /// arms are no-ops (`GameState::apply` ignores actions once `game_over`).
    pub fn tick_all(&mut self, dt_ms: f64) {
        for arm in self.arms.iter_mut() {
            arm.apply(crate::actions::Action::Tick(dt_ms));
        }
    }

    pub fn total_score(&self) -> u32 {
        self.arms.iter().map(|a| a.score).sum()
    }

    /// The game ends when any single arm tops out (the primary rule
    /// suggested by the spec for Mode A; "all arms" is a documented
    /// alternative it also allows, not implemented here since it changes
    /// what "game over" means for the human/AI comparison this mode exists
    /// to support).
    pub fn is_game_over(&self) -> bool {
        self.arms.iter().any(|a| a.game_over)
    }
}
