pub mod bitboard;
pub mod greedy;

pub use greedy::{
    best_cross_placement, best_placement, play_best_cross_move, play_best_move, CrossPlacement, Placement, Weights,
    DEFAULT_WEIGHTS,
};
