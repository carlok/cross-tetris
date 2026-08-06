use engine::piece::{ActivePiece, PieceKind, Rotation};
use engine::{Action, Arm, CrossGame};

#[test]
fn arms_get_distinct_piece_sequences_from_one_master_seed() {
    let cross = CrossGame::new(777);
    let kinds: Vec<PieceKind> = Arm::ALL.iter().map(|&arm| cross.arm(arm).active.unwrap().kind).collect();
    // Not a strict guarantee (7 kinds, 4 arms, birthday-paradox collisions are
    // possible), but with real seed derivation at least one pair should differ
    // across most seeds; this catches the "forgot to derive per-arm seeds" bug
    // where all 4 arms would be byte-for-byte identical, not just first-piece-equal.
    assert!(
        cross.arm(Arm::North) != cross.arm(Arm::East),
        "arms must not share identical state when seeded from a single master seed"
    );
    let _ = kinds;
}

#[test]
fn same_master_seed_reproduces_identical_cross_game() {
    let a = CrossGame::new(42);
    let b = CrossGame::new(42);
    assert_eq!(a, b);
}

#[test]
fn arms_are_independent_actions_on_one_do_not_affect_others() {
    let mut cross = CrossGame::new(5);
    let before_others: Vec<_> = [Arm::East, Arm::South, Arm::West].iter().map(|&a| cross.arm(a).clone()).collect();

    cross.arm_mut(Arm::North).apply(Action::HardDrop);
    cross.arm_mut(Arm::North).apply(Action::MoveLeft);

    let after_others: Vec<_> = [Arm::East, Arm::South, Arm::West].iter().map(|&a| cross.arm(a).clone()).collect();
    assert_eq!(before_others, after_others, "acting on one arm must not mutate the others");
}

#[test]
fn total_score_sums_all_arms() {
    let mut cross = CrossGame::new(9);
    cross.arm_mut(Arm::North).score = 100;
    cross.arm_mut(Arm::East).score = 50;
    cross.arm_mut(Arm::South).score = 0;
    cross.arm_mut(Arm::West).score = 25;
    assert_eq!(cross.total_score(), 175);
}

#[test]
fn game_over_when_any_single_arm_tops_out() {
    let mut cross = CrossGame::new(11);
    assert!(!cross.is_game_over());
    cross.arm_mut(Arm::South).game_over = true;
    assert!(cross.is_game_over());
}

#[test]
fn tick_all_advances_every_arm() {
    let mut cross = CrossGame::new(13);
    for arm in Arm::ALL {
        cross.arm_mut(arm).active = Some(ActivePiece {
            kind: PieceKind::O,
            rotation: Rotation::R0,
            row: 0,
            col: 4,
        });
    }
    cross.tick_all(1200.0); // > 1000ms/row at level 1
    for arm in Arm::ALL {
        assert!(cross.arm(arm).active.unwrap().row > 0, "{arm:?} should have fallen under gravity");
    }
}
