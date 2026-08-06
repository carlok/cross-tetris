use engine::piece::{ActivePiece, PieceKind, Rotation};
use engine::rotation::{piece_fits, try_rotate};
use engine::{Action, Board, GameState, SevenBag};
use proptest::prelude::*;
use std::collections::{HashMap, HashSet};

fn arb_piece_kind() -> impl Strategy<Value = PieceKind> {
    prop_oneof![
        Just(PieceKind::I),
        Just(PieceKind::J),
        Just(PieceKind::L),
        Just(PieceKind::O),
        Just(PieceKind::S),
        Just(PieceKind::Z),
        Just(PieceKind::T),
    ]
}

fn arb_action() -> impl Strategy<Value = Action> {
    prop_oneof![
        Just(Action::MoveLeft),
        Just(Action::MoveRight),
        Just(Action::RotateCw),
        Just(Action::RotateCcw),
        Just(Action::SoftDropStart),
        Just(Action::SoftDropEnd),
        Just(Action::HardDrop),
        Just(Action::Hold),
        (0.0f64..900.0).prop_map(Action::Tick),
    ]
}

proptest! {
    /// Rotating a piece 4x (in either direction) from an arbitrary unobstructed
    /// starting position returns it to its original occupied cell set.
    #[test]
    fn rotation_topology_preserved(kind in arb_piece_kind(), row in 10i32..30, col in 2i32..6, cw in any::<bool>()) {
        let board = Board::new();
        let start = ActivePiece { kind, rotation: Rotation::R0, row, col };
        prop_assume!(piece_fits(&board, &start));

        let mut current = start;
        for _ in 0..4 {
            current = try_rotate(&board, &current, cw)
                .expect("rotation on an empty, unobstructed board must always succeed");
        }
        let start_cells: HashSet<_> = start.occupied_cells().into_iter().collect();
        let end_cells: HashSet<_> = current.occupied_cells().into_iter().collect();
        prop_assert_eq!(start_cells, end_cells);
    }

    /// Any placement `try_rotate` returns as `Some` must be fully in bounds and
    /// non-overlapping with locked cells.
    #[test]
    fn successful_wall_kicks_are_always_legal(
        kind in arb_piece_kind(),
        row in 10i32..30,
        col in 0i32..10,
        cw in any::<bool>(),
        fill_seed in any::<u64>(),
    ) {
        let mut board = Board::new();
        // Scatter some locked cells (deterministically, from fill_seed) so kicks
        // sometimes have to route around obstacles instead of always hitting an
        // empty board.
        let mut rng = engine::Rng::new(fill_seed.max(1));
        for _ in 0..40 {
            let r = (rng.next_below(40)) as i32;
            let c = (rng.next_below(10)) as i32;
            board.set(r, c, Some(PieceKind::J));
        }
        let start = ActivePiece { kind, rotation: Rotation::R0, row, col };
        prop_assume!(piece_fits(&board, &start));

        if let Some(result) = try_rotate(&board, &start, cw) {
            prop_assert!(piece_fits(&board, &result));
        }
    }

    /// Every consecutive, non-overlapping window of 7 draws from the bag
    /// contains each of the 7 piece kinds exactly once, for any seed.
    #[test]
    fn bag_fairness_holds_for_arbitrary_seeds(seed in any::<u64>(), windows in 1usize..20) {
        let mut bag = SevenBag::new(seed);
        for _ in 0..windows {
            let mut counts: HashMap<PieceKind, u32> = HashMap::new();
            for _ in 0..7 {
                *counts.entry(bag.next()).or_insert(0) += 1;
            }
            prop_assert_eq!(counts.len(), 7);
            prop_assert!(counts.values().all(|&c| c == 1));
        }
    }

    /// Replaying an arbitrary valid action sequence against the same seed twice
    /// produces identical final state.
    #[test]
    fn determinism_holds_for_arbitrary_action_sequences(
        seed in any::<u64>(),
        actions in prop::collection::vec(arb_action(), 0..30),
    ) {
        let mut a = GameState::new(seed);
        let mut b = GameState::new(seed);
        for action in &actions {
            a.apply(*action);
            b.apply(*action);
        }
        prop_assert_eq!(a, b);
    }
}
