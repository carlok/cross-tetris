pub mod bitboard;
pub mod dellacherie;
pub mod greedy;
pub mod placement;

pub use greedy::{
    best_cross_placement, best_placement, play_best_cross_move, play_best_move, CrossPlacement, Placement, Weights,
    DEFAULT_WEIGHTS,
};
